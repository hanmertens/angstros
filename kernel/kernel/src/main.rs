#![no_std]
#![no_main]

use common::{
    boot::{BootInfo, KernelMain},
    println,
};
use core::panic::PanicInfo;
use log::LevelFilter;

// Type-check of kernel entry point
const _: KernelMain = _start;

fn init() {
    common::init(LevelFilter::Trace).unwrap();
}

/// Kernel entry point
#[no_mangle]
pub unsafe extern "C" fn _start(_boot: &'static BootInfo) -> ! {
    init();

    println!();
    println!("== ÅngstrÖS v{} ==", env!("CARGO_PKG_VERSION"));
    println!();

    log::info!("Boot complete");

    panic!("The kernel is still young; there's nothing more to do!");
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    common::panic_handler(info);
}
