use std::alloc::{GlobalAlloc, Layout};
use std::ptr;

pub fn allocate(layout: Layout) -> *mut u8 {
    unsafe { ptr::alloc(layout) }
}

pub fn deallocate(ptr: *mut u8, layout: Layout) {
    unsafe { ptr::dealloc(ptr, layout) }
}

pub fn reallocate(ptr: *mut u8, old_layout: Layout, new_layout: Layout) -> *mut u8 {
    if new_layout.size() <= old_layout.size() {
        return ptr;
    }
    let new_ptr = allocate(new_layout);
    unsafe {
        ptr::copy_nonoverlapping(ptr, new_ptr, old_layout.size());
        deallocate(ptr, old_layout);
    }
    new_ptr
}