//! Code relevant to booting (mostly shared between bootloader and kernel).

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
    pub memory_map: MemoryMap,
}

/// Description of memory map and iterator over it
///
/// Note that this structure itself is an iterator, so you need to clone it if
/// retaining access to previous elements is desired.
#[derive(Clone)]
pub struct MemoryMap {
    ptr: *const u8,
    size: usize,
    len: usize,
}

// Safe because you need a mutable reference to use the pointer
unsafe impl Send for MemoryMap {}

impl MemoryMap {
    /// Create new memory map description
    ///
    /// # Safety
    /// Pointer should point to the first element of the memory map, size the
    /// distance between elements and len the total number of elements. The
    /// lifetime of the memory map should be `'static`.
    pub unsafe fn new(ptr: *const u8, size: usize, len: usize) -> Self {
        Self { ptr, size, len }
    }
}

impl Iterator for MemoryMap {
    type Item = &'static MemoryDescriptor;

    fn next(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            return None;
        }
        let current = self.ptr;
        self.ptr = self.ptr.wrapping_add(self.size);
        self.len -= 1;
        Some(unsafe { &*(current as *const MemoryDescriptor) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl ExactSizeIterator for MemoryMap {}
