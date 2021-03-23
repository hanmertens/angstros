#![no_std]
#![no_main]
#![feature(asm)]

use common::boot::{BootInfo, KernelMain};
use core::panic::PanicInfo;

// Type-check of kernel entry point
const _: KernelMain = _start;

/// Kernel entry point
#[no_mangle]
pub unsafe extern "C" fn _start(_boot: &'static BootInfo) -> ! {
    loop {
        asm!("hlt");
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") };
    }
}
