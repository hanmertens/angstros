#![no_std]
#![no_main]
#![feature(asm)]

use core::panic::PanicInfo;

#[no_mangle]
extern "C" fn _start() {
    let hw = "Hello kernel from userspace!";
    unsafe { asm!("syscall", in("rdi") 1, in("rsi") hw.as_ptr(), in("rdx") hw.len()) };
    unsafe { asm!("syscall", in("rdi") 0) };
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
