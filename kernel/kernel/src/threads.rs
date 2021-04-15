use crate::Init;
use common::elf::ElfInfo;
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
    asm!(
        "mov [{}], rsp; mov rcx, {}; mov rsp, {}; mov r11, {}; sysretq; return_syscall:",
        in(reg) &STACK,
        // rip is read from rcx
        in(reg) elf.entry_point(),
        in(reg) stack_start + stack_length * 0x1000,
        // rflags is read from r11
        const 0x0202,
        // These registers are clobbered
        out("rcx") _,
        out("r11") _,
        // The rest is not preserved
        lateout("rax") _,
        lateout("rbx") _,
        lateout("rdx") _,
        lateout("rsi") _,
        lateout("rdi") _,
        lateout("r8") _,
        lateout("r9") _,
        lateout("r10") _,
        lateout("r12") _,
        lateout("r13") _,
        lateout("r14") _,
        lateout("r15") _,
    );
    log::info!("Back in kernelspace");
    instructions::interrupts::enable();
}

unsafe extern "C" fn syscall_handler() {
    asm!("mov rsp, [{}]; jmp return_syscall", in(reg) &STACK);
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
