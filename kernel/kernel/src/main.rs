#![no_std]
#![no_main]

use common::{
    boot::{BootInfo, KernelMain},
    println,
};
use core::panic::PanicInfo;
use x86_64::instructions;

// Type-check of kernel entry point
const _: KernelMain = _start;

fn init() {
    common::init();
    println!();
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
