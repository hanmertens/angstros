#![no_std]
#![no_main]
#![feature(abi_efiapi, asm)]

mod allocator;

use allocator::BootAllocator;
use common::{
    boot::{offset, BootInfo, FrameBuffer, MemoryMap},
    elf::Elf,
    println,
};
use core::{mem, panic::PanicInfo, slice};
use uefi::{
    prelude::*,
    proto::console::gop::GraphicsOutput,
    table::{boot::MemoryDescriptor, runtime::ResetType},
    Handle,
};
use x86_64::{
    registers::control::Cr3,
    structures::paging::{Mapper, OffsetPageTable, PageTable, PageTableFlags, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

mod config {
    include!(concat!(env!("XTASK_OUT_DIR"), "/cfg_uefi_stub.rs"));
}

const KERNEL_SIZE: usize = include_bytes!(env!("KERNEL_PATH")).len();
const KERNEL_BYTES: [u8; KERNEL_SIZE] = *include_bytes!(env!("KERNEL_PATH"));

/// Put kernel ELF in memory
static KERNEL: Elf<KERNEL_SIZE> = Elf::new(KERNEL_BYTES);

fn shutdown(system_table: SystemTable<Boot>) -> ! {
    let rt = system_table.runtime_services();
    rt.reset(ResetType::Shutdown, Status::SUCCESS, None);
}

struct Setup {
    kernel_page_table: &'static PageTable,
    stack: u64,
    entry_point: u64,
    boot_info: *mut BootInfo,
    mmap: &'static mut [u8],
}

fn setup_boot(
    system_table: &SystemTable<Boot>,
) -> Result<(Setup, Option<FrameBuffer>), &'static str> {
    common::init(config::LOG_LEVEL)?;

    // Reset UEFI text and background colors and print newline
    println!("\x1b[0m");
    println!(
        "== ÅngstrÖS UEFI boot stub v{} ==",
        env!("CARGO_PKG_VERSION")
    );
    println!();

    let boot_serv = system_table.boot_services();
    let mut boot_alloc = BootAllocator::new(&boot_serv);

    // Setup graphics protocol and frame buffer
    let fb = boot_serv
        .locate_protocol::<GraphicsOutput>()
        .log_warning()
        .map_or_else(
            |e| {
                log::error!("Failed to locate graphics output: {:?}", e.status());
                None
            },
            |gop| Some(FrameBuffer::new(unsafe { &mut *gop.get() }, offset::USIZE)),
        );

    // Setup basic mappings for kernel
    let uefi_page_table = {
        let phys_addr = Cr3::read().0.start_address();
        let virt_addr = VirtAddr::new(phys_addr.as_u64());
        unsafe { virt_addr.as_mut_ptr::<PageTable>().as_mut() }.unwrap()
    };
    let kernel_page_table = {
        let virt_addr = VirtAddr::new(boot_alloc.allocate_pages(1)?);
        let ptr: *mut PageTable = virt_addr.as_mut_ptr();
        unsafe { ptr.write(PageTable::new()) };
        unsafe { ptr.as_mut() }.unwrap()
    };
    kernel_page_table[offset::PAGE_TABLE_INDEX] = uefi_page_table[0].clone();
    let mut offset_kpt = unsafe { OffsetPageTable::new(kernel_page_table, VirtAddr::new(0)) };
    let kernel_info = KERNEL.info(false)?;
    kernel_info.setup_mappings(&mut offset_kpt, &mut boot_alloc)?;

    // Map pages around context switch
    log::info!(
        "Identity mapping around kernel context switch at {:?}",
        switch_to_kernel as *const ()
    );
    let addr = PhysAddr::new(VirtAddr::from_ptr(switch_to_kernel as *const ()).as_u64());
    let frame = PhysFrame::<Size4KiB>::containing_address(addr);
    for frame in PhysFrame::range_inclusive(frame, frame + 1) {
        log::debug!("Identity mapping {:?} to be sure", frame);
        unsafe { offset_kpt.identity_map(frame, PageTableFlags::PRESENT, &mut boot_alloc) }
            .map_err(|_| "Mapping error")?
            .ignore();
    }

    let stack = boot_alloc.allocate_pages(16)? + 15 * 0x1000;
    let boot_info = {
        let size = mem::size_of::<BootInfo>();
        // Align as guaranteed by allocate_pool
        assert!(mem::align_of::<BootInfo>() <= 8);
        let ptr = boot_alloc.allocate_pool(size)?;
        ptr as *mut BootInfo
    };
    let mmap = {
        // Size may increase between now and exiting boot services
        let mmap_size = boot_serv.memory_map_size() + 256;
        let mmap_ptr = boot_alloc.allocate_pool(mmap_size)?;
        // Creating a &[u8] containing uninitialized memory is UB
        unsafe { mmap_ptr.write_bytes(0, mmap_size) };
        unsafe { slice::from_raw_parts_mut(mmap_ptr, mmap_size) }
    };

    Ok((
        Setup {
            kernel_page_table,
            stack,
            entry_point: kernel_info.entry_point(),
            boot_info,
            mmap,
        },
        fb,
    ))
}

#[entry]
fn efi_main(image_handler: Handle, system_table: SystemTable<Boot>) -> Status {
    let (setup, fb) = match setup_boot(&system_table) {
        Ok(s) => s,
        Err(s) => {
            log::error!("{}", s);
            shutdown(system_table);
        }
    };

    log::info!("Exiting boot services and performing final setup");

    let (uefi_system_table, mut mmap_iter) = system_table
        .exit_boot_services(image_handler, setup.mmap)?
        .log();

    // Figure out distance between elements in memory descriptor slice
    let size = if let (Some(fst), Some(snd)) = (mmap_iter.next(), mmap_iter.next()) {
        let fst = fst as *const _ as usize;
        let snd = snd as *const _ as usize;
        snd - fst
    } else {
        mem::size_of::<MemoryDescriptor>()
    };
    let len = mmap_iter.len();
    // Drop the mutable borrow of setup.mmap
    mem::drop(mmap_iter);
    // We use wrapping_add because the resulting pointer points to unmapped memory
    let ptr = setup.mmap.as_ptr().wrapping_add(offset::USIZE).cast();
    let memory_map = unsafe { MemoryMap::new(ptr, size, len) };

    unsafe {
        setup.boot_info.write(BootInfo {
            uefi_system_table,
            memory_map,
            fb,
        })
    };

    log::info!("Switching to kernel");

    switch_to_kernel(setup);
}

#[inline(never)]
fn switch_to_kernel(setup: Setup) -> ! {
    unsafe {
        asm!(
            "mov cr3, {}; mov rsp, {}; jmp {}",
            in(reg) setup.kernel_page_table as *const _ as usize,
            in(reg) setup.stack as usize + offset::USIZE,
            in(reg) setup.entry_point,
            in("rdi") setup.boot_info as usize + offset::USIZE,
            options(noreturn)
        );
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    common::panic_handler(info);
}
