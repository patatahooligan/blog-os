use super::{align_up, Locked};
use alloc::alloc::{GlobalAlloc, Layout};
use core::{mem, ptr};

struct ListNode {
    size: usize,
    next: Option<&'static mut ListNode>,
}

impl ListNode {
    const fn new(size: usize) -> Self {
        ListNode { size, next: None }
    }

    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

pub struct LinkedListAllocator {
    head: ListNode,
}

/// A basic linked list allocator.
///
/// This type of allocator keeps a linked list of all available regions.
/// The list nodes themselves are stored inside the free regions they
/// represent. Otherwise we would have been required to use a heap to
/// store the list, but this allocator is what has to provide the heap
/// storage!
///
/// Whever an allocation is requested, the list is traversed until a
/// region big enough is found. If the region is larger than the
/// requested size, it is split. One part is returned as the requested
/// allocation and the other is inserted into the list. This design is
/// simple and therefore a great prototype, but it should be replaced
/// eventually because it suffers from severe flaws:
///  - An allocation might have to traverse as much as the entire list,
///    so worst-case performance therefore continuously degrades as the
///    OS is running.
///  - The splitting of unused memory causes fragmentation. As memory is
///    allocated and freed, we might end up with adjacent free regions,
///    but we don't merge them. Merging them is hard because the list is
///    not sorted so you don't know where a regions neighbors might be.
///    If we do switch to sorted lists, then that's another performance
///    hit.
impl LinkedListAllocator {
    /// Create an empty [LinkedListAllocator].
    pub const fn new() -> Self {
        Self {
            head: ListNode::new(0),
        }
    }

    /// Initialize allocator with given heap bounds.
    ///
    /// This is unsafe because the caller must ensure that the given
    /// memory range is unused. Additionally, this method must never be
    /// called more than once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.add_free_region(heap_start, heap_size);
    }

    /// Adds the given memory region to the front of the list.
    unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
        assert_eq!(align_up(addr, mem::align_of::<ListNode>()), addr);
        assert!(size >= mem::size_of::<ListNode>());

        let mut node = ListNode::new(size);
        node.next = self.head.next.take();

        let node_ptr = addr as *mut ListNode;
        node_ptr.write(node);
        self.head.next = Some(&mut *node_ptr);
    }

    /// Look for a free region with the given size and alignment and
    /// remove it from the list.
    ///
    /// Returns a tuple of the list node and the start address of the
    /// allocation.
    fn find_region(
        &mut self,
        size: usize,
        align: usize,
    ) -> Option<(&'static mut ListNode, usize)> {
        // Start with head and iterate through the list until we find a
        // suitable region.
        let mut current = &mut self.head;

        while let Some(ref mut region) = current.next {
            if let Ok(alloc_start) =
                Self::alloc_from_region(&region, size, align)
            {
                let next = region.next.take();
                let ret = Some((current.next.take().unwrap(), alloc_start));
                current.next = next;
                return ret;
            }
            else {
                current = current.next.as_mut().unwrap();
            }
        }

        // We reach this when ListNode::next == None, so we traversed
        // the entire list without finding a suitable region.
        None
    }

    fn alloc_from_region(
        region: &ListNode,
        size: usize,
        align: usize,
    ) -> Result<usize, ()> {
        let alloc_start = align_up(region.start_addr(), align);
        let alloc_end = alloc_start.checked_add(size).ok_or(())?;

        if alloc_end > region.end_addr() {
            // Region too small
            return Err(());
        }

        let excess_size = region.end_addr() - alloc_end;
        if excess_size > 0 && excess_size < mem::size_of::<ListNode>() {
            // Rest of region too small for ListNode. This is required
            // because the unused part of the current region will need
            // to be inserted into the list. Note how if excess_size ==
            // 0, we're ok because there's nothing to add to the list.
            return Err(());
        }

        Ok(alloc_start)
    }

    /// Adjust the given layout so that the resulting allocated memory
    /// region is also capable of storing a ListNode.
    ///
    /// Returns the adjusted size and alignment as a (size, align)
    /// tuple.
    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(mem::align_of::<ListNode>())
            .expect("Adjusting alignment failed")
            .pad_to_align();
        let size = layout.size().max(mem::size_of::<ListNode>());
        (size, layout.align())
    }
}

unsafe impl GlobalAlloc for Locked<LinkedListAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let (size, align) = LinkedListAllocator::size_align(layout);
        let mut allocator = self.lock();

        if let Some((region, alloc_start)) = allocator.find_region(size, align)
        {
            let alloc_end = alloc_start.checked_add(size).expect("Overflow");
            let excess_size = region.end_addr() - alloc_end;
            if excess_size > 0 {
                allocator.add_free_region(alloc_end, excess_size);
            }
            alloc_start as *mut u8
        }
        else {
            ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let (size, _) = LinkedListAllocator::size_align(layout);

        self.lock().add_free_region(ptr as usize, size);
    }
}
