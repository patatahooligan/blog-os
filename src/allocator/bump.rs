use super::{align_up, Locked};
use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr;

pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
    allocations: usize,
}

/// Simple bump allocator. All it does is keep track of the first
/// not yet allocated address and returning that (after aligning it)
/// whenever a new allocation is requested. It does not generally track
/// and reuse freed memory, so it constantly creeps forward and will
/// eventually run out of memory. The only exception is that if *every*
/// allocation gets freed, it will reset to the start of the heap.
impl BumpAllocator {
    /// Create new empty allocator
    pub const fn new() -> Self {
        BumpAllocator {
            heap_start: 0,
            heap_end: 0,
            next: 0,
            allocations: 0,
        }
    }

    /// Initialize allocator with given heap bounds.
    ///
    /// This is unsafe because the caller must ensure that the given
    /// memory range is unused. Additionally, this method must never be
    /// called more than once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        self.next = heap_start;
    }
}

unsafe impl GlobalAlloc for Locked<BumpAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut bump_allocator = self.lock();

        // TODO alignment and bounds check
        let alloc_start = align_up(bump_allocator.next, layout.align());
        let alloc_end = match alloc_start.checked_add(layout.size()) {
            Some(end) => end,
            None => return ptr::null_mut(),
        };

        if alloc_end > bump_allocator.heap_end {
            // Out of memory
            ptr::null_mut()
        }
        else {
            bump_allocator.next = alloc_end;
            bump_allocator.allocations += 1;
            alloc_start as *mut u8
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        let mut bump_allocator = self.lock();

        bump_allocator.allocations -= 1;

        if bump_allocator.allocations == 0 {
            // The bump allocator is pretty dumb. Generally, it cannot
            // reuse memory. However, only in the case where every
            // single allocation has been deallocated, it can reset and
            // start from BumpAllocator::heap_start again.
            bump_allocator.next = bump_allocator.heap_start;
        }
    }
}
