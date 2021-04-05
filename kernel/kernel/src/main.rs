#![no_std]
#![no_main]
#![feature(abi_x86_interrupt, asm)]

mod interrupts;

use common::{
    boot::{BootInfo, KernelMain},
    println,
};
use core::panic::PanicInfo;
use log::LevelFilter;
use x86_64::instructions;

// Type-check of kernel entry point
const _: KernelMain = _start;

fn init() {
    common::init(LevelFilter::Trace).unwrap();
    interrupts::init();
}

/// Kernel entry point
#[no_mangle]
pub unsafe extern "C" fn _start(_boot: &'static BootInfo) -> ! {
    init();

    // Single line to prevent race condition with first timer interrupt
    println!("\n== ÅngstrÖS v{} ==\n", env!("CARGO_PKG_VERSION"));

    log::info!("Boot complete");

    instructions::interrupts::int3();

    loop {
        instructions::hlt();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    common::panic_handler(info);
}
