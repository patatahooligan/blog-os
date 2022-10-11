use x86_64::structures::paging::{OffsetPageTable, PageTable};
use x86_64::VirtAddr;

/// Initialize a new OffsetPageTable
///
/// It is unsafe because the caller must guarantee that the entire
/// physical address is mapped at the offset given as the argument.
/// Additionally, it must never be called more than once.
pub unsafe fn init(
    physical_memory_offset: VirtAddr,
) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

/// Get a reference to the level 4 page table.
///
/// It is unsafe because the caller must guarantee that the entire
/// physical address is mapped at the offset given as the argument.
/// Additionally, it must never be called more than once, as that would
/// create aliasing `&mut`s which is undefined behavior.
unsafe fn active_level_4_table(
    physical_memory_offset: VirtAddr,
) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr
}
