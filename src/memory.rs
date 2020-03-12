use alloc_cortex_m::CortexMHeap;
use cortex_m_rt::heap_start;
use crate::config::HEAP_SIZE;

#[global_allocator]
pub(crate) static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

pub fn init() {
    let start = heap_start() as usize;
    unsafe { ALLOCATOR.init(start, HEAP_SIZE) }
}

#[alloc_error_handler]
fn alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Alloc layout {:?} failed!", layout)
}