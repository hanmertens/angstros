//! Everything related to the linked list allocator

use core::{
    alloc::{GlobalAlloc, Layout},
    borrow::Borrow,
    fmt, mem, ptr,
};
use spin::{mutex::MutexGuard, Mutex};
use x86_64::VirtAddr;

/// Akin to [`Layout`], but uses [`u64`] internally and has the minimum size and
/// alignment requirements of a [`Node`].
#[derive(Copy, Clone, Debug)]
struct NodeLayout {
    size: u64,
    align: u64,
}

impl From<Layout> for NodeLayout {
    fn from(layout: Layout) -> Self {
        let layout = layout
            .align_to(Node::ALIGN as usize)
            .unwrap()
            .pad_to_align();
        Self {
            size: layout.size().max(Node::SIZE as usize) as u64,
            align: layout.align() as u64,
        }
    }
}

/// Describes a free block of memory based on its starting address and size.
#[derive(Copy, Clone, Debug)]
struct Hole {
    addr: VirtAddr,
    size: u64,
}

impl Hole {
    fn new(addr: VirtAddr, size: u64) -> Self {
        Self { addr, size }
    }

    fn start_addr(self) -> VirtAddr {
        self.addr
    }

    fn end_addr(self) -> VirtAddr {
        self.start_addr() + self.size
    }

    /// Create [`Node`] as described by [`Hole`]
    ///
    /// The `next` field of the node is set to [`None`].
    ///
    /// # Panic
    /// Panics if the hole is not lare enough to fit the node or if the hole is
    /// not properly aligned to fit the node.
    ///
    /// # Safety
    /// Starting from `hole.addr`, `hole.size` bytes need to be backed by
    /// physical memory and ownership of that memory is transferred to the node.
    unsafe fn to_static_node(self) -> &'static mut Node {
        assert!(self.size >= Node::SIZE);
        assert!(self.addr.is_aligned(Node::ALIGN));

        let node = Node::new(self.size);
        let node_ptr = self.addr.as_mut_ptr::<Node>();
        node_ptr.write(node);
        &mut *node_ptr
    }

    fn from_alloc(addr: VirtAddr, layout: NodeLayout) -> Self {
        Self::new(addr, layout.size)
    }

    /// Determine if and how a [`NodeLayout`] can fit in a [`Hole`]
    ///
    /// If the layout cannot fit, [`None`] is returned, otherwise a [`VirtAddr`]
    /// is returned for where the layout would fit, along with up to two holes
    /// that fill the remaining space of the hole. It is guaranteed that the
    /// optional first hole's location is the same as `self` and that the
    /// optional second hole's location is after the layout allocation.
    fn fit_alloc(self, layout: NodeLayout) -> Option<(Option<Self>, VirtAddr, Option<Self>)> {
        // Calculate placement of new allocation
        let start = self.start_addr().align_up(layout.align);
        let end = start + layout.size;
        if end > self.end_addr() {
            return None;
        }

        // Calculate placements and necessity of holes before and after
        let excess_before = start - self.start_addr();
        let before = if excess_before == 0 {
            None
        } else if excess_before < Node::SIZE {
            return None;
        } else {
            Some(Self::new(self.start_addr(), excess_before))
        };

        let excess_after = self.end_addr() - end;
        let after = if excess_after == 0 {
            None
        } else if excess_after < Node::SIZE {
            return None;
        } else {
            Some(Self::new(end, excess_after))
        };

        Some((before, start, after))
    }
}

/// Node in linked list of free memory regions
struct Node {
    size: u64,
    next: Option<&'static mut Self>,
}

// Custom implementation to show address and prevent recursion
impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Node")
            .field("addr", &self.start_addr())
            .field("size", &self.size)
            .field("next", &self.next.as_ref().map(|node| node.start_addr()))
            .finish()
    }
}

impl Node {
    const SIZE: u64 = mem::size_of::<Self>() as u64;
    const ALIGN: u64 = mem::align_of::<Self>() as u64;

    /// Initialize a new node with an empty tail
    const fn new(size: u64) -> Self {
        Self { size, next: None }
    }

    fn start_addr(&self) -> VirtAddr {
        VirtAddr::from_ptr(self as _)
    }

    fn end_addr(&self) -> VirtAddr {
        self.start_addr() + self.size
    }

    /// Convenience wrapper around [`Hole::fit_alloc`]
    fn fit_alloc(&self, layout: NodeLayout) -> Option<(Option<Hole>, VirtAddr, Option<Hole>)> {
        Hole::from(self).fit_alloc(layout)
    }

    /// Insert [`Node`] in the linked list immediately after `self`
    ///
    /// The new node should not be part of a linked list.
    fn insert(&mut self, node: &'static mut Self) {
        debug_assert!(node.next.is_none());
        node.next = self.next.take();
        self.next = Some(node);
    }

    /// Convenience wrapper around [`Node::insert`]
    ///
    /// Since the [`Hole`] needs to be converted to a [`Node`], the same
    /// requirements hold as for [`Hole::to_static_node`].
    unsafe fn insert_hole(&mut self, hole: Hole) {
        self.insert(hole.to_static_node())
    }

    /// Unlink the next node from the linked list
    fn remove_next(&mut self) -> Option<&'static mut Node> {
        self.next.take().map(|node| {
            self.next = node.next.take();
            node
        })
    }
}

impl<T: Borrow<Node>> From<T> for Hole {
    fn from(node: T) -> Self {
        let node = node.borrow();
        Self::new(node.start_addr(), node.size)
    }
}

/// A simple iterator over all the nodes in the linked list
///
/// Since a [`Node`] contains a mutable reference to the next element we can't
/// implement [`Iterator`] and hand out mutable references to the nodes.
struct NodeIter<'a>(Option<&'a mut Node>);

impl<'a> NodeIter<'a> {
    /// Create iterator for a given starting node.
    fn new(node: &'a mut Node) -> Self {
        Self(Some(node))
    }

    /// Obtain a reference to the current [`Node`], if any
    ///
    /// [`None`] indicates no further nodes are present.
    fn current(&mut self) -> Option<&mut Node> {
        self.0.as_deref_mut()
    }

    /// Go to the next [`Node`]
    ///
    /// No-ops if called when the linked list is already exhausted.
    fn advance(&mut self) {
        if let Some(current) = self.0.take() {
            self.0 = current.next.as_deref_mut();
        }
    }
}

/// Simple linked-list allocator
///
/// Uses a simple first-fit allocation strategy. Due to internal fragmentation
/// bad performance is expected when a mixture of short and long-lived
/// allocations are performed; for best performance the long-lived allocations
/// should be performed first.
pub struct LinkedListAllocator(Mutex<Node>);

impl fmt::Debug for LinkedListAllocator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut list = f.debug_list();
        let mut head = self.head();
        let mut iter = NodeIter::new(&mut head);
        while let Some(region) = iter.current() {
            list.entry(&Hole::from(region));
            iter.advance();
        }
        list.finish()
    }
}

impl LinkedListAllocator {
    pub const fn new() -> Self {
        Self(Mutex::new(Node::new(0)))
    }

    /// Initialize the allocator by providing a backed memory heap
    ///
    /// Unlike some other allocators, can be called multiple times (with
    /// non-overlapping memory ranges) to grow the heap. These ranges do not
    /// have to be contiguous.
    ///
    /// # Safety
    /// Safe iff virtual addresses `heap_start..heap_start+heap_size` are backed
    /// by unused physical memory.
    pub unsafe fn init(&self, heap_start: u64, heap_size: u64) {
        let hole = Hole::new(VirtAddr::new(heap_start), heap_size);
        self.push(hole);
    }

    /// Lock the heap and get the head node
    fn head(&self) -> MutexGuard<Node> {
        self.0.lock()
    }

    /// Push hole in linked list and merge with other nodes if possible
    unsafe fn push(&self, mut hole: Hole) {
        // Find region after which the hole whould be located
        let mut head = self.head();
        let mut iter = NodeIter::new(&mut head);
        while let Some(region) = iter.current() {
            if let Some(next) = region.next.as_deref_mut() {
                if next.start_addr() < hole.addr {
                    iter.advance();
                    continue;
                }
                // Found location, grow next region if possible and continue
                // below if-let statement
                if next.start_addr() == hole.addr + hole.size {
                    hole.size += next.size;
                    region.next = next.next.take();
                }
            }
            // Grow previous region if possible, insert hole otherwise
            if hole.addr == region.end_addr() {
                region.size += hole.size;
            } else {
                region.insert_hole(hole);
            }
            return;
        }
        unreachable!();
    }

    fn allocate(&self, layout: NodeLayout) -> Option<VirtAddr> {
        log::trace!("Allocating {:?}", layout);
        // Find first hole that fits the desired layout
        let mut head = self.head();
        let mut iter = NodeIter::new(&mut head);
        while let Some(region) = iter.current() {
            if let Some(next) = region.next.as_deref_mut() {
                if let Some((before, start, after)) = next.fit_alloc(layout) {
                    // Update the linked list based on this fit
                    let current = if let Some(before) = before {
                        assert_eq!(next.start_addr(), before.start_addr());
                        next.size = before.size;
                        next
                    } else {
                        assert!(region.remove_next().is_some());
                        region
                    };
                    if let Some(after) = after {
                        unsafe { current.insert_hole(after) };
                    }
                    return Some(start);
                }
            }
            iter.advance();
        }
        None
    }

    /// Deallocate memory and put it back into the linked list
    unsafe fn deallocate(&self, addr: VirtAddr, layout: NodeLayout) {
        log::trace!("Deallocating {:?}", layout);
        let hole = Hole::from_alloc(addr, layout);
        self.push(hole);
    }

    /// Reallocate memory
    ///
    /// Grow allocation if possible, otherwise simple allocate, copy contents
    /// and deallocate otherwise.
    unsafe fn reallocate(
        &self,
        addr: VirtAddr,
        layout: NodeLayout,
        new_size: u64,
    ) -> Option<VirtAddr> {
        let mut hole = Hole::from_alloc(addr, layout);
        let new_layout = Layout::from_size_align(new_size as usize, layout.align as usize)
            .unwrap()
            .into();
        // Small allocations may have been made larger due to NodeLayout
        // size/align requirements and may not require any actual work.
        if let Some((before, start, after)) = hole.fit_alloc(new_layout) {
            // If after isn't None we will need to insert it into the list
            if after.is_none() {
                assert!(before.is_none());
                assert_eq!(addr, start);
                return Some(addr);
            }
        }

        log::trace!("Reallocating {:?} to {:?}", layout, new_layout);
        // Traverse list to find location of hole
        let mut head = self.head();
        let mut iter = NodeIter::new(&mut head);
        while let Some(region) = iter.current() {
            if let Some(next) = region.next.as_deref_mut() {
                if next.start_addr() < hole.addr {
                    iter.advance();
                    continue;
                }
                // Found hole, simply grow or shrink if possible
                if next.start_addr() == hole.end_addr() {
                    hole.size += next.size;
                    if let Some((before, start, after)) = hole.fit_alloc(new_layout) {
                        region.next = next.next.take();
                        assert!(before.is_none());
                        assert_eq!(addr, start);
                        if let Some(after) = after {
                            region.insert_hole(after);
                        }
                        return Some(addr);
                    }
                    hole.size -= next.size;
                }
            } else {
                // Allocation is at the very end, but shrinking might be possible
                if let Some((before, start, after)) = hole.fit_alloc(new_layout) {
                    assert!(before.is_none());
                    assert_eq!(addr, start);
                    if let Some(after) = after {
                        region.insert_hole(after);
                    }
                    return Some(addr);
                }
            }
            // Let's keep track if this a situation worth implementing later
            if hole.addr == region.end_addr() {
                log::info!("Might be able to merge with before block, but this is unimplemented");
            }

            // Can't grow? simply allocate a fresh block, copy and deallocate
            // Drop lock of allocator before trying to allocate
            drop(iter);
            drop(head);
            let new_addr = self.allocate(new_layout);
            if let Some(new_addr) = new_addr {
                ptr::copy_nonoverlapping(
                    addr.as_ptr(),
                    new_addr.as_mut_ptr::<u8>(),
                    layout.size.min(new_layout.size) as usize,
                );
                self.deallocate(addr, layout);
            }
            return new_addr;
        }
        unreachable!();
    }
}

unsafe impl GlobalAlloc for LinkedListAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.allocate(layout.into())
            .map(VirtAddr::as_mut_ptr)
            .unwrap_or(ptr::null_mut())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.deallocate(VirtAddr::from_ptr(ptr), layout.into());
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        self.reallocate(VirtAddr::from_ptr(ptr), layout.into(), new_size as u64)
            .map(VirtAddr::as_mut_ptr)
            .unwrap_or(ptr::null_mut())
    }
}
