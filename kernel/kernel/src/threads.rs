use crate::Init;
use common::elf::ElfInfo;
use core::{slice, str};
use x86_64::{
    instructions,
    registers::model_specific::LStar,
    structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags},
    VirtAddr,
};

static mut STACK: u64 = 0;

/// Simple test of user space
///
/// Blocks until userspace thread returns, does not clean up ELF mappings.
pub unsafe fn spawn_user(init: &mut Init, elf: &ElfInfo) {
    elf.setup_mappings(&mut init.page_table, &mut init.frame_allocator)
        .unwrap();
    let stack_start = 0x2000;
    let stack_length = 1;
    for i in 0..stack_length {
        let page = Page::containing_address(VirtAddr::new(stack_start)) + i;
        let frame = init.frame_allocator.allocate_frame().unwrap();
        let flags =
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;
        init.page_table
            .map_to(page, frame, flags, &mut init.frame_allocator)
            .unwrap()
            .flush();
    }
    LStar::write(VirtAddr::from_ptr(syscall_handler as *const ()));
    log::info!("Switching to userspace");
    syscall_loop(elf.entry_point(), stack_start + stack_length * 0x1000);
    log::info!("Back in kernelspace");
    instructions::interrupts::enable();
}

/// Loop while handling syscalls
unsafe fn syscall_loop(entry_point: u64, stack_end: u64) {
    let mut rip = entry_point;
    let mut rsp = stack_end;
    loop {
        let code: u64;
        let ptr: u64;
        let len: u64;
        asm!(
            "mov [{}], rsp; mov rsp, {}; sysretq; return_syscall:",
            in(reg) &STACK,
            in(reg) rsp,
            // rip is read from rcx
            inout("rcx") rip,
            // rflags is read from r11
            inlateout("r11") 0x0202 => _,
            // The rest is not preserved
            lateout("rax") _,
            lateout("rbx") rsp,
            lateout("rdx") len,
            lateout("rsi") ptr,
            lateout("rdi") code,
            lateout("r8") _,
            lateout("r9") _,
            lateout("r10") _,
            lateout("r12") _,
            lateout("r13") _,
            lateout("r14") _,
            lateout("r15") _,
        );
        match code {
            0 => return,
            1 => {
                // TODO add checks for pointer and length
                let s = slice::from_raw_parts(ptr as _, len as _);
                match str::from_utf8(s) {
                    Ok(s) => log::info!("User message: {}", s),
                    Err(_) => log::warn!("User message not valid UTF-8"),
                }
            }
            _ => log::warn!("Ignoring unknown syscall {}", code),
        }
    }
}

unsafe extern "C" fn syscall_handler() {
    asm!("mov rbx, rsp; mov rsp, [{}]; jmp return_syscall", in(reg) &STACK);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn dummy() {
        let mut guard = crate::test::INIT.lock();
        let init = guard.as_mut().unwrap();
        unsafe { spawn_user(init, &crate::USER.info(true).unwrap()) };
    }
}
