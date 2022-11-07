//! Heap allocation implementation
//!
//! Before using anything that requires heap allocations, like vectors
//! or strings, you are required to call [init_heap] exactly once! You
//! don't have to do anything else, as the module uses
//! `#[global_allocator]` to set the allocator globally.

pub mod fixed_size_block;
pub mod linked_list;

use x86_64::structures::paging::mapper::MapToError;
use x86_64::structures::paging::{
    FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
};
use x86_64::VirtAddr;

use fixed_size_block::FixedSizeBlockAllocator;

#[global_allocator]
static ALLOCATOR: Locked<FixedSizeBlockAllocator> =
    Locked::new(FixedSizeBlockAllocator::new());

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 100 * 1024;

pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe { mapper.map_to(page, frame, flags, frame_allocator)?.flush() };
    }

    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    Ok(())
}

/// Align the given address `addr` upwards to alignment `align`.
///
/// Requires that `align` is a power of two, which it normally should
/// anyway. Note that alloc::alloc::Layout requires this.
fn align_up(addr: usize, align: usize) -> usize {
    // This will be called with every memory allocation, so we should
    // make it reasonably fast. Since we're guaranteed to get powers of
    // two, we can use some clever bit manipulation to get the desired
    // behavior.

    // Mask which sets to 0 the bits required to align by `align`. But
    // if we use it as is, it will align *downwards* instead of upwards.
    let align_mask = !(align - 1);

    // Bump address so that it is bumped into the next block of size
    // `align`. So when it is aligned downwards it will be as if the
    // original address was aligned upwards. Note that thatks to the
    // `-1` term, if `addr` was already aligned, it is *not* bumped to
    // the next block. Then when it is aligned downards we get the
    // original `addr`, as we should.
    let addr_in_next_block = addr + align - 1;

    addr_in_next_block & align_mask
}

/// A wrapper around spin::Mutex to permit trait implementations.
pub struct Locked<A> {
    inner: spin::Mutex<A>,
}

impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: spin::Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> spin::MutexGuard<A> {
        self.inner.lock()
    }
}
