use super::{linked_list::LinkedListAllocator, Locked};
use alloc::alloc::{GlobalAlloc, Layout};
use core::mem;

/// The available block sizes
///
/// They must be powers of two so we can use them for block alignment.
/// We don't have smaller than 8 bytes because that would be smaller
/// than a 64-bit pointer. Beyond some size, it is best to use a
/// fallback allocator. We have to arbitrarily choose this based on our
/// expectactions on what is large enough to be infrequent.
const BLOCK_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];

struct ListNode {
    next: Option<&'static mut ListNode>,
}

/// A simple fixed size block allocator.
///
/// The allocator works like a collection of linked list allocators with
/// two important differences:
///  - it uses multiple lists of free memory regions
///  - each list has a fixed size, specifically the sizes in BLOCK_SIZES
///
///  This design allows fast allocation times. For small allocations,
///  the allocator rounds up the size to a power of two and simply grabs
///  the first node of the corresponding list. If none exist, then it
///  requests a new block of memory from the fallback allocator. In the
///  worst case the performance is the same as the linked list allocator
///  but in practice you should often have available blocks from the
///  fixed size lists which will be much faster.
///
///  When a block is deallocated, it its size is one of the fixed size
///  lists, it is inserted into that list. For this reason, after a few
///  allocations and deallocations we expect to not need the fallback
///  allocator very often.
///
///  If the requested allocation size is bigger than any element in
///  BLOCK_SIZES, then use the fallback allocator.
///
///  Some important observations on this type of allocator:
///   - Rounding up to fixed sizes means that some parts of the
///     allocated regions are unused. Since we use powers of two, the
///     wasted memory is up to 50% of the block (if it would waste more
///     then it would belong to a smaller power of two block). This
///     wasted memory is intentional because it allows simpler and
///     therefore faster bookkeeping.
///   - We allocate blocks lazily. When the allocator is initialized, it
///     has no blocks of any size to give out and every requested
///     allocation goes through the fallback allocator. This is likely
///     not a great deal because after a block is freed it is reused.
///     But if startup performance seems problematic we could improve it
///     by preallocating a bunch of blocks.
///   - The allocator would greatly benefit from a more sophisticated
///     large size allocator to minimize fragmentation. This will
///     prevent performance degradation and even out-of-memory panics
///     when the kernel runs for too long.
pub struct FixedSizeBlockAllocator {
    list_heads: [Option<&'static mut ListNode>; BLOCK_SIZES.len()],
    fallback_allocator: Locked<LinkedListAllocator>,
}

impl FixedSizeBlockAllocator {
    /// Create an empty FixedSizeBlockAllocator.
    pub const fn new() -> Self {
        const EMPTY: Option<&'static mut ListNode> = None;
        FixedSizeBlockAllocator {
            list_heads: [EMPTY; BLOCK_SIZES.len()],
            fallback_allocator: Locked::new(LinkedListAllocator::new()),
        }
    }

    /// Initialize allocator with given heap bounds.
    ///
    /// This is unsafe because the caller must ensure that the given
    /// memory range is unused. Additionally, this method must never be
    /// called more than once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        // We only need to initialize the fallback allocator because we
        // will lazily get memory from it for our Self::list_heads.
        self.fallback_allocator.lock().init(heap_start, heap_size);
    }
}

/// Find the appropriate block size for the given layout. This is the
/// smallest block that can fit the requested size.
///
/// Return an index into `BLOCK_SIZES` or `None` if the requested size
/// is larger than any available block.
fn list_index(layout: &Layout) -> Option<usize> {
    let required_block_size = layout.size().max(layout.align());
    BLOCK_SIZES.iter().position(|&s| s >= required_block_size)
}

unsafe impl GlobalAlloc for Locked<FixedSizeBlockAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();

        // First check if the requested size should be handled by the
        // primary or fallback allocator.
        match list_index(&layout) {
            Some(index) => {
                // For cases that should be handled by the main
                // allocator, see if there are any available nodes of
                // the appropriate size.
                match allocator.list_heads[index].take() {
                    Some(node) => {
                        // The list had a node. Return and point to the
                        // next one in the list.
                        allocator.list_heads[index] = node.next.take();
                        node as *mut ListNode as *mut u8
                    }
                    None => {
                        // No nodes exist for the appropriate size.
                        // Create one with the fallback allocator.
                        let block_size = BLOCK_SIZES[index];

                        // Only works because we offer block sizes that
                        // are powers of 2.
                        let block_align = block_size;

                        let layout =
                            Layout::from_size_align(block_size, block_align)
                                .unwrap();
                        allocator.fallback_allocator.alloc(layout)
                    }
                }
            }
            None => {
                // Block is too large for main allocator
                allocator.fallback_allocator.alloc(layout)
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut allocator = self.lock();

        // This match mirrors the one we did in Self::alloc. This is
        // important because its return value determines which list we
        // initially used, or if we used the fallback allocator.
        match list_index(&layout) {
            Some(index) => {
                let new_node = ListNode {
                    next: allocator.list_heads[index].take(),
                };

                assert!(mem::size_of::<ListNode>() <= BLOCK_SIZES[index]);
                assert!(mem::align_of::<ListNode>() <= BLOCK_SIZES[index]);

                let new_node_ptr = ptr as *mut ListNode;
                new_node_ptr.write(new_node);
                allocator.list_heads[index] = Some(&mut *new_node_ptr);
            }
            None => {
                allocator.fallback_allocator.dealloc(ptr, layout);
            }
        }
    }
}
