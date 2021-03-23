//! Code relevant to booting (mostly shared between bootloader and kernel).

use core::slice;
use uefi::table::{boot::MemoryDescriptor, Runtime, SystemTable};

/// Offset memory mapping information
pub mod offset {
    use x86_64::VirtAddr;

    /// Index of page table offset entry
    pub const PAGE_TABLE_INDEX: usize = 1;
    /// Offset of kernal mapping
    pub const VIRT_ADDR: VirtAddr = VirtAddr::new_truncate((PAGE_TABLE_INDEX as u64) << 39);
    pub const USIZE: usize = VIRT_ADDR.as_u64() as usize;
}

/// Expected signature of the kernel entry point
pub type KernelMain = unsafe extern "C" fn(&'static BootInfo) -> !;

/// The information provided by the boot stub
pub struct BootInfo {
    /// Access to UEFI system table. Note that this struct contains various
    /// pointers that assume they are identity mapped, which may not be the case
    /// in the kernel page table provided by the bootloader.
    pub uefi_system_table: SystemTable<Runtime>,
    pub memory_map_ptr: *const MemoryDescriptor,
    pub memory_map_len: usize,
}

impl BootInfo {
    /// Slice containing memory layout when exiting UEFI boot services. The
    /// reference is valid in the kernel page table, but the virtual addresses
    /// each memory descriptor refers to are not necessarily up to date.
    pub fn memory_map(&self) -> &'static [MemoryDescriptor] {
        unsafe { slice::from_raw_parts(self.memory_map_ptr, self.memory_map_len) }
    }
}
