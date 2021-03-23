//! Convenience wrappers for allocations

use uefi::{
    prelude::*,
    table::boot::{AllocateType, MemoryType},
};
use x86_64::{
    structures::paging::{FrameAllocator, PhysFrame, Size4KiB},
    PhysAddr,
};

/// Wrapper around [`BootServices`] for more ergonomic allocations.
pub struct BootAllocator<'a>(&'a BootServices);

impl<'a> BootAllocator<'a> {
    /// Create allocator struct by borrowing [`BootServices`].
    pub fn new(boot_serv: &'a BootServices) -> Self {
        Self(boot_serv)
    }

    /// Allocate from pool
    ///
    /// Convenience function for [`BootServices::allocate_pool`]. Log any
    /// warnings and use a static string as error message.
    pub fn allocate_pool(&self, count: usize) -> Result<*mut u8, &'static str> {
        self.0
            .allocate_pool(MemoryType::LOADER_DATA, count)
            .log_warning()
            .map_err(|_| "Failed to allocate pool")
    }

    /// Allocate pages
    ///
    /// Convenience function for [`BootServices::allocate_pages`]. Log any
    /// warnings and use a static string as error message.
    pub fn allocate_pages(&self, count: usize) -> Result<u64, &'static str> {
        self.0
            .allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, count)
            .log_warning()
            .map_err(|_| "Failed to allocate pages")
    }
}

/// Convenience wrapper for interopability with [`x86_64`] crate.
unsafe impl FrameAllocator<Size4KiB> for BootAllocator<'_> {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        self.allocate_pages(1).ok().map(|addr| {
            PhysFrame::from_start_address(PhysAddr::new(addr))
                .expect("UEFI returned invalid allocated page")
        })
    }
}
