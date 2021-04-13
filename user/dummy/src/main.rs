#![no_std]
#![no_main]
#![feature(asm)]

use core::panic::PanicInfo;

#[no_mangle]
extern "C" fn _start() {
    let _a = 2;
    unsafe { asm!("syscall") };
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
