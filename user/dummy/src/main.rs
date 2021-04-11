#![no_std]
#![no_main]
#![feature(asm, link_args)]
#![allow(unused_attributes)]
#![link_args = "--section-start=.text=0x1000"]

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
