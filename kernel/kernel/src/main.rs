#![no_std]
#![no_main]
#![feature(
    abi_x86_interrupt,
    alloc_error_handler,
    asm,
    const_mut_refs,
    custom_test_frameworks
)]
#![allow(clippy::inconsistent_digit_grouping)]
#![test_runner(test::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

mod allocator;
mod interrupts;
#[cfg(test)]
mod test;

use allocator::RegionFrameAllocator;
use common::{
    boot::{offset, BootInfo, KernelMain},
    println,
};
use core::alloc::Layout;
use log::LevelFilter;
use x86_64::{
    instructions,
    registers::control::Cr3,
    structures::paging::{OffsetPageTable, PageTable},
};

// Type-check of kernel entry point
const _: KernelMain = _start;

fn init(boot_info: &'static BootInfo) {
    let level = if cfg!(test) {
        LevelFilter::Off
    } else {
        LevelFilter::Trace
    };
    common::init(level).unwrap();
    let page_table_addr = offset::VIRT_ADDR + Cr3::read().0.start_address().as_u64();
    let page_table_ref = unsafe { &mut *page_table_addr.as_mut_ptr::<PageTable>() };
    let mut page_table = unsafe { OffsetPageTable::new(page_table_ref, offset::VIRT_ADDR) };
    let mut alloca = RegionFrameAllocator::new(&boot_info.memory_map());
    allocator::init(&mut page_table, &mut alloca).unwrap();
    interrupts::init();
}

/// Kernel entry point
#[no_mangle]
pub unsafe extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
    init(boot_info);

    #[cfg(test)]
    test_main();

    // Single line to prevent race condition with first timer interrupt
    println!("\n== ÅngstrÖS v{} ==\n", env!("CARGO_PKG_VERSION"));

    log::info!("Boot complete");

    loop {
        instructions::hlt();
    }
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    common::panic_handler(info);
}

#[alloc_error_handler]
fn alloc_error(layout: Layout) -> ! {
    panic!("Out of memory requesting {:#?}", layout);
}
