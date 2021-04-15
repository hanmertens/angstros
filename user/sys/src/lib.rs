#![no_std]
#![feature(asm)]

/// System call codes
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SyscallCode {
    /// Exit with code in rsi
    Exit = 0,
    /// Log message, raw parts of UTF-8 slice passed through rsi for the pointer
    /// and rdx for the length.
    Log = 1,
}

/// Perform a system call
///
/// The raw return code is returned. All registers are marked as clobbered.
pub fn syscall(code: SyscallCode, rsi: u64, rdx: u64) -> u64 {
    let rax: u64;
    unsafe {
        asm!(
            "syscall",
            inout("rdi") code as u64 => _,
            inout("rsi") rsi => _,
            inout("rdx") rdx => _,
            out("rax") rax,
            out("rbx") _,
            out("rcx") _,
            out("r8") _,
            out("r9") _,
            out("r10") _,
            out("r11") _,
            out("r12") _,
            out("r13") _,
            out("r14") _,
            out("r15") _,
        )
    };
    rax
}
