#![no_std]
#![no_main]
#![feature(abi_x86_interrupt, asm, custom_test_frameworks)]
#![test_runner(test::test_runner)]
#![reexport_test_harness_main = "test_main"]

mod interrupts;
#[cfg(test)]
mod test;

use common::{
    boot::{BootInfo, KernelMain},
    println,
};
use log::LevelFilter;
use x86_64::instructions;

// Type-check of kernel entry point
const _: KernelMain = _start;

fn init() {
    let level = if cfg!(test) {
        LevelFilter::Off
    } else {
        LevelFilter::Trace
    };
    common::init(level).unwrap();
    interrupts::init();
}

/// Kernel entry point
#[no_mangle]
pub unsafe extern "C" fn _start(_boot: &'static BootInfo) -> ! {
    init();

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
