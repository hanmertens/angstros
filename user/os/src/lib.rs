#![no_std]

pub use sys;

use core::mem::{self, MaybeUninit};
use sys::{syscall, FrameBuffer, SyscallCode};

/// Exit with specified exit code
pub fn exit(code: u64) -> ! {
    unsafe { syscall(SyscallCode::Exit, code, 0) };
    unreachable!("Process should have been killed by OS");
}

/// Log message
pub fn log(msg: &str) {
    let code = unsafe { syscall(SyscallCode::Log, msg.as_ptr() as u64, msg.len() as u64) };
    // Return code should be zero as message is guaranteed to be valid (valid
    // pointer/length combination and valid UTF-8).
    debug_assert_eq!(code, 0);
}

/// Obtain frame buffer
pub fn frame_buffer() -> Option<FrameBuffer> {
    let fb = MaybeUninit::<FrameBuffer>::uninit();
    let code = unsafe {
        syscall(
            SyscallCode::FrameBuffer,
            &fb as *const _ as u64,
            mem::size_of::<FrameBuffer>() as u64,
        )
    };
    if code != 0 {
        return None;
    }
    Some(unsafe { fb.assume_init() })
}
