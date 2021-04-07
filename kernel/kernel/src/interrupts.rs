use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Once;
use x86_64::{
    instructions::interrupts,
    registers::control::Cr2,
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode},
};

mod gdt {
    use spin::Once;
    use x86_64::{
        instructions::{segmentation, tables},
        structures::{
            gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
            tss::TaskStateSegment,
        },
        VirtAddr,
    };

    /// Global descriptor table and relevant selectors
    struct Gdt {
        gdt: GlobalDescriptorTable,
        code_selector: SegmentSelector,
        tss_selector: SegmentSelector,
    }

    pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

    static GDT: Once<Gdt> = Once::new();
    static TSS: Once<TaskStateSegment> = Once::new();

    /// Initialize everything related to the GDT
    ///
    /// This includes, specifically:
    /// - Set up double fault stack in task state segment
    /// - Initialize and load global descriptor table
    /// - Reset nonsensical segment registers
    /// - Set up code and task state segment selectors
    pub fn init() {
        let tss = TSS.call_once(|| {
            let mut tss = TaskStateSegment::new();
            // Set up stack for double fault handler
            tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
                const STACK_SIZE: usize = 4096 * 5;
                // Not thread-safe
                static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

                let stack_start = VirtAddr::from_ptr(unsafe { &STACK });
                stack_start + STACK_SIZE
            };
            tss
        });
        let gdt = GDT.call_once(|| {
            let mut gdt = GlobalDescriptorTable::new();
            let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
            let tss_selector = gdt.add_entry(Descriptor::tss_segment(&tss));
            Gdt {
                gdt,
                code_selector,
                tss_selector,
            }
        });

        // Reset segment register if set by UEFI firmware
        unsafe { asm!("mov ss, {:r}", in(reg) 0) };

        gdt.gdt.load();
        unsafe {
            segmentation::set_cs(gdt.code_selector);
            tables::load_tss(gdt.tss_selector);
        }
    }
}

mod pic {
    use pic8259_simple::ChainedPics;
    use spin::Mutex;
    use x86_64::instructions::port::Port;

    pub const PIC_1_OFFSET: u8 = 0x20;
    pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

    pub static PICS: Mutex<ChainedPics> =
        Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

    pub fn init() {
        // Lock PICS before (manually) writing to ports
        let mut pics = PICS.lock();
        unsafe {
            // UEFI masks all interrupt, so unmask at least the ones we want
            Port::<u8>::new(0x21).write(0b10111000);
            Port::<u8>::new(0xa1).write(0b10001110);
            pics.initialize();
        }
    }
}

const TIMER_INTERRUPT_ID: u8 = pic::PIC_1_OFFSET;

static IDT: Once<InterruptDescriptorTable> = Once::new();

extern "x86-interrupt" fn breakpoint_handler(stack_frame: &mut InterruptStackFrame) {
    log::warn!("Breakpoint in {:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    let address = Cr2::read();

    log::error!(
        "Page fault {:?} at {:?} in {:#?}",
        error_code,
        address,
        stack_frame
    );

    // We can't recover at the moment, so we go looping
    panic!("page fault");
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) -> ! {
    log::error!("Double fault in {:#?}", stack_frame);

    // We can't recover, so we remain looping
    panic!("double fault");
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    let count = COUNT.fetch_add(1, Ordering::Relaxed);
    if count % 1000 == 0 {
        log::info!("Handling timer interrupt #{}", count);
    }
    unsafe { pic::PICS.lock().notify_end_of_interrupt(TIMER_INTERRUPT_ID) };
}

/// Initialize everything related to interrupts; should be called only once
///
/// This includes, specifically:
/// - Everything related to the global descriptor table (see [`gdt::init`])
/// - Initialize and load the interrupt descriptor table
pub fn init() {
    gdt::init();
    let idt = IDT.call_once(|| {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt[TIMER_INTERRUPT_ID as usize].set_handler_fn(timer_interrupt_handler);
        idt
    });
    idt.load();
    pic::init();
    interrupts::enable();
}

#[cfg(test)]
mod tests {
    use x86_64::instructions::interrupts;

    #[test_case]
    fn int3() {
        interrupts::int3();
    }
}
