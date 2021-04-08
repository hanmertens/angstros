//! Everything allocator-related
//!
//! This includes both frame allocators governing physical memory and "normal"
//! allocators governing virtual memory.

mod bump;
mod region_frame;

pub use bump::BumpAllocator;
pub use region_frame::RegionFrameAllocator;

use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};

pub const HEAP_START: VirtAddr = VirtAddr::new_truncate(0o1_000_000_0000);
pub const HEAP_SIZE: u64 = 0o1_000_0000;

/// Our global allocator
#[global_allocator]
pub static ALLOC: BumpAllocator = BumpAllocator::new();

pub fn init<M, A>(mapper: &mut M, allocator: &mut A) -> Result<(), MapToError<Size4KiB>>
where
    M: Mapper<Size4KiB>,
    A: FrameAllocator<Size4KiB>,
{
    log::debug!(
        "Initializing heap at {:?}..{:?}",
        HEAP_START,
        HEAP_START + HEAP_SIZE
    );
    for page in Page::range_inclusive(
        Page::containing_address(HEAP_START),
        Page::containing_address(HEAP_START + (HEAP_SIZE - 1)),
    ) {
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        let frame = allocator.allocate_frame().unwrap();
        unsafe { mapper.map_to(page, frame, flags, allocator)? }.flush();
    }
    unsafe { ALLOC.init(HEAP_START.as_u64(), HEAP_SIZE) };
    Ok(())
}

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;

    #[test_case]
    fn boxed() {
        let mut boxed = Box::new(10);
        *boxed += 10;
        assert_eq!(*boxed, 20);
    }
}
