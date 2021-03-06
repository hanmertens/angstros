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
mod threads;

use allocator::{RegionFrameAllocator, UserFrameAllocator};
use common::{
    boot::{offset, BootInfo, KernelMain},
    elf::Elf,
};
use core::alloc::Layout;
use x86_64::{
    registers::control::Cr3,
    structures::paging::{OffsetPageTable, PageTable},
};

mod config {
    include!(concat!(env!("XTASK_OUT_DIR"), "/cfg_kernel.rs"));
}

const USER_SIZE: usize = include_bytes!(env!("USER_PATH")).len();
const USER_BYTES: [u8; USER_SIZE] = *include_bytes!(env!("USER_PATH"));

/// Put userspace ELF in memory
static USER: Elf<USER_SIZE> = Elf::new(USER_BYTES);

// Type-check of kernel entry point
const _: KernelMain = _start;

pub struct Init {
    boot_info: &'static BootInfo,
    page_table: OffsetPageTable<'static>,
    frame_allocator: UserFrameAllocator<RegionFrameAllocator>,
}

fn init(boot_info: &'static BootInfo) -> Init {
    common::init(config::LOG_LEVEL).unwrap();
    let page_table_addr = offset::VIRT_ADDR + Cr3::read().0.start_address().as_u64();
    let page_table_ref = unsafe { &mut *page_table_addr.as_mut_ptr::<PageTable>() };
    let mut page_table = unsafe { OffsetPageTable::new(page_table_ref, offset::VIRT_ADDR) };
    let mut frame_allocator = RegionFrameAllocator::new(boot_info.memory_map.clone());
    allocator::init(&mut page_table, &mut frame_allocator).unwrap();
    interrupts::init();
    let frame_allocator = UserFrameAllocator::new(frame_allocator);
    Init {
        boot_info,
        page_table,
        frame_allocator,
    }
}

// Kernel entry point for tests
#[cfg(test)]
#[no_mangle]
pub unsafe extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
    let init = init(boot_info);
    test::run_tests(init);
}

/// Kernel entry point
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
    let mut init = init(boot_info);

    // Single line to prevent race condition with first timer interrupt
    common::println!("\n== ??ngstr??S v{} ==\n", env!("CARGO_PKG_VERSION"));

    log::info!("Boot complete");
    threads::spawn_user(&mut init, &USER.info(true).unwrap());
    log::info!("Rerunning user process");
    threads::spawn_user(&mut init, &USER.info(true).unwrap());
    log::info!("Going to halt");

    loop {
        x86_64::instructions::hlt();
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
