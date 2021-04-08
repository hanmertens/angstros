//! A simple bump allocator

use core::{
    alloc::{GlobalAlloc, Layout},
    ptr,
    sync::atomic::{AtomicU64, Ordering},
};
use x86_64::VirtAddr;

/// A simple, lockless, and leaky allocator
///
/// Leaks until all memory is freed, then all memory is reclaimed.
#[derive(Debug, Default)]
pub struct BumpAllocator {
    start: AtomicU64,
    next: AtomicU64,
    end: AtomicU64,
    count: AtomicU64,
}

impl BumpAllocator {
    pub const fn new() -> Self {
        Self {
            start: AtomicU64::new(0),
            next: AtomicU64::new(0),
            end: AtomicU64::new(0),
            count: AtomicU64::new(0),
        }
    }

    /// # Safety
    /// Safe iff virtual addresses `heap_start..heap_start+heap_size` are backed
    /// by unused physical memory.
    pub unsafe fn init(&self, heap_start: u64, heap_size: u64) {
        // Only initialize an empty heap
        assert_eq!(self.count.load(Ordering::Relaxed), 0);
        self.next.store(heap_start, Ordering::Relaxed);
        self.end.store(heap_start + heap_size, Ordering::Relaxed);
        // This acts as a memory fence and allows start reads to use relaxed
        self.start.store(heap_start, Ordering::SeqCst);
    }

    /// Allocate a certain layout
    ///
    /// The virtual address of the first byte of the layout is returned, or
    /// `None` if allocation failed; since this is only used in [`GlobalAlloc`]
    /// no care is put into an error type. This function is safe but it might
    /// leak memory.
    fn allocate(&self, layout: Layout) -> Option<VirtAddr> {
        log::trace!("Allocating {:?}", layout);
        // These are acquire because they need to be done before updating next
        if self.start.load(Ordering::Relaxed) == 0 {
            log::warn!("Allocation requested but allocator uninitialized!");
            return None;
        }
        self.count.fetch_add(1, Ordering::Acquire);
        // These can be relaxed because the order of allocation doesn't matter
        let mut start_addr = VirtAddr::new(0);
        if self
            .next
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
                let addr = VirtAddr::new(next);
                start_addr = addr.align_up(layout.align() as u64);
                let end_addr = start_addr + layout.size();
                if end_addr.as_u64() < self.end.load(Ordering::Relaxed) {
                    Some(end_addr.as_u64())
                } else {
                    None
                }
            })
            .is_ok()
        {
            debug_assert_ne!(start_addr, VirtAddr::new(0));
            Some(start_addr)
        } else {
            // Failed allocation, so decrease allocation count again
            unsafe { self.count_decrease() };
            None
        }
    }

    /// Deallocate memory allocation
    ///
    /// Just the total number of allocations is tracked, so that number is
    /// decreased and if it reaches zero we start reusing memory from the
    /// beginning. This function is thus unsafe as reusing memory while it is
    /// actually still in use can violate Rust's safety guarantees.
    unsafe fn deallocate(&self) {
        log::trace!("Deallocating");
        self.count_decrease();
    }

    /// Convenience function to decrease allocation count, and start reusing
    /// memory if possible.
    ///
    /// That last bit makes the function unsafe; every call should correspond to
    /// a previous increase of the count, see [`deallocate`].
    #[inline]
    unsafe fn count_decrease(&self) {
        let start = self.start.load(Ordering::Relaxed);
        let next = self.next.load(Ordering::Relaxed);
        // This is release so the load of next stays before it
        if self.count.fetch_sub(1, Ordering::Release) == 1 {
            if self
                .next
                .compare_exchange(next, start, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                log::info!("Successfully reset heap");
            } else {
                log::warn!("Resetting heap failed (concurrent allocation?)");
            }
        }
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Call the safe function that has all the guarantees
        self.allocate(layout)
            .map(VirtAddr::as_mut_ptr)
            .unwrap_or(ptr::null_mut())
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        self.deallocate();
    }
}
