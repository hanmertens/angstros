//! A simple frame allocator based on memory regions

use core::slice::Iter;
use uefi::table::boot::{MemoryDescriptor, MemoryType};
use x86_64::{
    structures::paging::{frame::PhysFrameRange, FrameAllocator, PageSize, PhysFrame, Size4KiB},
    PhysAddr,
};

/// Frame allocator based on memory regions
///
/// Currently only allocates pages in regions marked conventional by UEFI.
pub struct RegionFrameAllocator {
    frames: PhysFrameRange,
    regions: Iter<'static, MemoryDescriptor>,
}

unsafe impl FrameAllocator<Size4KiB> for RegionFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        // Switch to a new region if current one is out of frames
        self.frames.next().map_or_else(
            || {
                // Only allocate if a new region exists; recursion should be
                // limited as next_region skips regions without usable frames
                self.next_region().and_then(|_| self.allocate_frame())
            },
            Some,
        )
    }
}

fn region_to_frames<S>(region: &MemoryDescriptor) -> PhysFrameRange<S>
where
    S: PageSize,
{
    PhysFrame::range(
        // Align up to make sure the frame falls completely in the region
        PhysFrame::containing_address(PhysAddr::new(region.phys_start).align_up(S::SIZE)),
        // Ending is exclusive, so no manual alignment is necessary
        PhysFrame::containing_address(PhysAddr::new(
            region.phys_start + S::SIZE * region.page_count,
        )),
    )
}

impl RegionFrameAllocator {
    pub fn new(memory_map: &'static [MemoryDescriptor]) -> Self {
        // This is just a dummy value
        let frame_zero = PhysFrame::containing_address(PhysAddr::new(0));
        let mut allocator = Self {
            frames: PhysFrame::range(frame_zero, frame_zero),
            regions: memory_map.iter(),
        };
        // Replace dummy value with the actual first usable frame
        allocator.next_region();
        allocator
    }

    /// Find next usable region containing at least one frame
    ///
    /// Should only be called if all frames in the current region are exhausted.
    /// Also updates list of frames with those in the newly found current region.
    fn next_region(&mut self) -> Option<MemoryDescriptor> {
        self.regions
            .by_ref()
            .find(|region| {
                region.ty == MemoryType::CONVENTIONAL
                    && !region_to_frames::<Size4KiB>(region).is_empty()
            })
            .map(|region| {
                self.frames = region_to_frames(region);
                log::trace!(
                    "New region for allocations {:?}..{:?}",
                    self.frames.start,
                    self.frames.end
                );
                *region
            })
    }
}
