use spin::Once;
use x86_64::{
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
        gdt.gdt.load();
        unsafe {
            segmentation::set_cs(gdt.code_selector);
            tables::load_tss(gdt.tss_selector);
        }
    }
}

static IDT: Once<InterruptDescriptorTable> = Once::new();

extern "x86-interrupt" fn breakpoint_handler(stack_frame: &mut InterruptStackFrame) {
    log::warn!("Breakpoint in {:#?}", stack_frame);

    // It should be possible to remove this panic, but then we get a general
    // protection fault resulting in a double fault...
    panic!("breakpoint");
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
        idt
    });
    idt.load();
}
