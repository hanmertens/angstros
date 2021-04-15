#![no_std]

use sys::{syscall, SyscallCode};

/// Exit with specified exit code
pub fn exit(code: u64) -> ! {
    syscall(SyscallCode::Exit, code, 0);
    unreachable!("Process should have been killed by OS");
}

/// Log message
pub fn log(msg: &str) {
    let code = syscall(SyscallCode::Log, msg.as_ptr() as u64, msg.len() as u64);
    // Return code should be zero as message is guaranteed to be valid (valid
    // pointer/length combination and valid UTF-8).
    debug_assert_eq!(code, 0);
}
