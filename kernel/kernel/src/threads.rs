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
pub unsafe fn spawn_user(init: &mut Init, elf: &ElfInfo) -> ! {
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
        "mov [{}], rsp; mov rcx, {}; mov rsp, {}; mov r11, {}; sysretq",
        in(reg) &STACK,
        // rip is read from rcx
        in(reg) elf.entry_point(),
        in(reg) stack_start + stack_length * 0x1000,
        // rflags is read from r11
        const 0x0202,
        // These registers are clobbered
        out("rcx") _,
        out("r11") _,
    );
    // Function should not return
    unreachable!();
}

unsafe extern "C" fn syscall_handler() -> ! {
    asm!("mov rsp, [{}]", in(reg) &STACK);
    log::info!("Back in kernelspace");
    instructions::interrupts::enable();
    loop {
        instructions::hlt();
    }
}
