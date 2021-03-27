#![no_std]
#![no_main]

#[macro_use]
mod serial;

use common::boot::{BootInfo, KernelMain};
use core::panic::PanicInfo;
use x86_64::instructions;

// Type-check of kernel entry point
const _: KernelMain = _start;

fn init() {
    serial::init();
    // Reset UEFI text and background colors and print newline
    println!("\x1b[0m");
}

/// Kernel entry point
#[no_mangle]
pub unsafe extern "C" fn _start(_boot: &'static BootInfo) -> ! {
    init();

    println!("ÅngstrÖS v{}", env!("CARGO_PKG_VERSION"));

    panic!("The kernel is still young; there's nothing more to do!");
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!();
    println!("Panic in the kernel!");
    println!("{:#?}", info);
    loop {
        instructions::hlt();
    }
}
