use alloc::vec::Vec;
use x86_64::structures::paging::{
    frame::PhysFrameRangeInclusive, FrameAllocator, FrameDeallocator, PhysFrame, Size4KiB,
};

/// Frame allocator storing its own allocations for later deallocation
pub struct UserFrameAllocator<A> {
    backing: A,
    free: Vec<PhysFrameRangeInclusive>,
}

impl<A> UserFrameAllocator<A> {
    pub fn new(backing: A) -> Self {
        Self {
            backing,
            free: Vec::new(),
        }
    }

    /// # Safety
    /// Frame should be unused, as it can be reused later.
    unsafe fn push(&mut self, frame: PhysFrame<Size4KiB>) {
        if let Some(last) = self.free.last_mut() {
            if frame - 1 == last.end {
                last.end = frame;
                return;
            } else if frame + 1 == last.start {
                last.start = frame;
                return;
            }
        }
        self.free.push(PhysFrame::range_inclusive(frame, frame));
    }

    fn pop(&mut self) -> Option<PhysFrame<Size4KiB>> {
        if let Some(last) = self.free.last_mut() {
            let frame = last.end;
            last.end -= 1;
            if last.is_empty() {
                self.free.pop();
            }
            Some(frame)
        } else {
            None
        }
    }
}

unsafe impl<A: FrameAllocator<Size4KiB>> FrameAllocator<Size4KiB> for UserFrameAllocator<A> {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        self.pop().or_else(|| self.backing.allocate_frame())
    }
}

impl<A> FrameDeallocator<Size4KiB> for UserFrameAllocator<A> {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size4KiB>) {
        self.push(frame)
    }
}
