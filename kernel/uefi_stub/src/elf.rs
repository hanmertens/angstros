//! Helpers for dealing with the kernel ELF.

use x86_64::{
    structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags, PhysFrame, Size4KiB},
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
    pub fn setup_mappings<M, A>(&self, map: &mut M, all: &mut A) -> Result<(), &'static str>
    where
        M: Mapper<Size4KiB>,
        A: FrameAllocator<Size4KiB>,
    {
        log::info!("Setting up ELF mappings...");
        for header in self.0.program_iter() {
            match header.get_type()? {
                Type::Load => {
                    self.load_segment(&header, map, all)?;
                }
                ty => {
                    log::debug!("    Skipping section of type {:?}", ty);
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
    ) -> Result<(), &'static str>
    where
        M: Mapper<Size4KiB>,
        A: FrameAllocator<Size4KiB>,
    {
        let virt_len = header.mem_size();
        let phys_len = header.file_size();
        if virt_len == 0 {
            return Ok(());
        }
        let virt_start = VirtAddr::new(header.virtual_addr());
        let virt_end = virt_start + (virt_len - 1);
        let phys_start =
            PhysAddr::new(VirtAddr::from_ptr(self.0.input as *const _ as *const u8).as_u64())
                + header.offset();
        let phys_end = phys_start + (phys_len - 1);
        if virt_len == phys_len {
            log::debug!(
                "    Mapping {:?}..{:?} to {:?}..{:?}",
                virt_start,
                virt_end,
                phys_start,
                phys_end
            );
            for (page, frame) in Page::range_inclusive(
                Page::containing_address(virt_start),
                Page::containing_address(virt_end),
            )
            .zip(PhysFrame::range_inclusive(
                PhysFrame::containing_address(phys_start),
                PhysFrame::containing_address(phys_end),
            )) {
                log::debug!("        Mapping {:?} to {:?}", page, frame);
                unsafe { map.map_to(page, frame, PageTableFlags::PRESENT, all) }
                    .map_err(|e| {
                        log::error!("{:?}", e);
                        "Mapping error"
                    })?
                    .ignore();
            }
            Ok(())
        } else {
            Err("Virtual/physical segment length mismatch")
        }
    }
}
