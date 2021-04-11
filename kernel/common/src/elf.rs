//! Helpers for dealing with the kernel ELF.

use core::ptr;
use x86_64::{
    structures::paging::{
        FrameAllocator, Mapper, Page, PageTableFlags, PhysFrame, Size4KiB, Translate,
    },
    PhysAddr, VirtAddr,
};
use xmas_elf::{
    program::{ProgramHeader, Type},
    ElfFile,
};

/// Align contents on page boundaries.
#[repr(align(4096))]
struct PageAligned<T>(T);

/// Align ELF bytes on page boundaries.
pub struct Elf<const N: usize>(PageAligned<[u8; N]>);

impl<const N: usize> Elf<N> {
    /// Create ELF from raw bytes.
    pub const fn new(bytes: [u8; N]) -> Self {
        Self(PageAligned(bytes))
    }

    /// Parse ELF using [`xmas-elf`].
    pub fn info(&self) -> Result<ElfInfo, &'static str> {
        ElfFile::new(&(self.0).0).map(ElfInfo)
    }
}

/// Extra functionality based on [`xmas-elf`] parsing.
pub struct ElfInfo<'a>(ElfFile<'a>);

impl<'a> ElfInfo<'a> {
    /// Obtain the entry point as encoded in the ELF header
    pub fn entry_point(&self) -> u64 {
        self.0.header.pt2.entry_point()
    }

    /// Setup page table mappings based on desired ELF mappings
    ///
    /// Only supports very rudimentary ELF features
    ///
    /// The `active` parameter indicates whether the passed page table is active
    /// and can provide virtual-to-physical translations. Otherwise, identity
    /// mapping is assumed.
    pub fn setup_mappings<M, A>(
        &self,
        map: &mut M,
        all: &mut A,
        active: bool,
    ) -> Result<(), &'static str>
    where
        M: Mapper<Size4KiB> + Translate,
        A: FrameAllocator<Size4KiB>,
    {
        log::info!("Setting up ELF mappings...");
        for header in self.0.program_iter() {
            match header.get_type()? {
                Type::Load => {
                    if active && header.offset() == 0 {
                        // This section by default overlaps with that of the kernel
                        log::warn!("Skipping conflicting read-only header");
                    } else {
                        self.load_segment(&header, map, all, active)?;
                    }
                }
                ty => {
                    log::debug!("Skipping section of type {:?}", ty);
                }
            }
        }
        Ok(())
    }

    /// Map loadable segment of the executable as requested
    fn load_segment<M, A>(
        &self,
        header: &ProgramHeader,
        map: &mut M,
        all: &mut A,
        active: bool,
    ) -> Result<(), &'static str>
    where
        M: Mapper<Size4KiB> + Translate,
        A: FrameAllocator<Size4KiB>,
    {
        let virt_len = header.mem_size();
        let phys_len = header.file_size();
        if virt_len == 0 {
            return Ok(());
        }
        let flags = {
            let mut flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
            if header.flags().is_write() {
                flags |= PageTableFlags::WRITABLE;
            }
            if !header.flags().is_execute() {
                flags |= PageTableFlags::NO_EXECUTE;
            }
            flags
        };
        let virt_start = VirtAddr::new(header.virtual_addr());
        let virt_end = virt_start + virt_len - 1u64;
        let elf_virt = VirtAddr::from_ptr(self.0.input as *const _ as *const u8) + header.offset();
        let phys_start = if active {
            map.translate_addr(elf_virt).ok_or("Elf not mapped")?
        } else {
            PhysAddr::new(elf_virt.as_u64())
        };
        let phys_end = phys_start + phys_len - 1u64;
        log::debug!(
            "Mapping {:?}..{:?} to {:?}..{:?}",
            virt_start,
            virt_end,
            phys_start,
            phys_end
        );
        let mut page_range = Page::range_inclusive(
            Page::containing_address(virt_start),
            Page::containing_address(virt_end),
        );
        let frame_range = PhysFrame::range_inclusive(
            PhysFrame::containing_address(phys_start),
            PhysFrame::containing_address(phys_end),
        );
        if virt_len > phys_len {
            // Instead of mapping to the last ELF frame, map to fresh frame
            // Other extraneous virtual memory is also backed by fresh frames
            let new_start = Page::containing_address(virt_start + phys_len - 1u64);
            let old_end = page_range.end;
            page_range.end = new_start - 1;
            let new_range = Page::range_inclusive(new_start, old_end);
            for (i, page) in new_range.enumerate() {
                let frame = all.allocate_frame().ok_or("No frame allocated")?;
                log::trace!("Mapping {:?} to fresh {:?}", page, frame);
                unsafe { map.map_to(page, frame, flags, all) }
                    .map_err(|e| {
                        log::error!("{:?}", e);
                        "Mapping error"
                    })?
                    .ignore();
                // Copy data from ELF to first fresh frame
                let zero_start = if i == 0 {
                    let phys_start = phys_start.max(frame_range.end.start_address());
                    let offset = phys_start - phys_start.align_down(4096u64);
                    let count = phys_end - phys_start + 1;
                    let fresh_start = frame.start_address() + offset;
                    log::trace!(
                        "Copying {} bytes from {:?} to {:?}",
                        count,
                        phys_start,
                        fresh_start,
                    );
                    let src = phys_start.as_u64() as *const u8;
                    let dst = fresh_start.as_u64() as *mut u8;
                    unsafe { ptr::copy_nonoverlapping(src, dst, count as usize) };
                    offset + count
                } else {
                    0
                };
                // Zero memory using current identity mapping
                let frame_ptr = (frame.start_address().as_u64() + zero_start) as *mut u8;
                unsafe { ptr::write_bytes(frame_ptr, 0, 4096 - zero_start as usize) };
            }
        }
        // Map directly to ELF as loaded in static variable
        for (page, frame) in page_range.zip(frame_range) {
            log::trace!("Mapping {:?} to {:?}", page, frame);
            unsafe { map.map_to(page, frame, flags, all) }
                .map_err(|e| {
                    log::error!("{:?}", e);
                    "Mapping error"
                })?
                .ignore();
        }
        Ok(())
    }
}
