#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::alloc::{alloc, dealloc, Layout};
use std::ptr;

#[allow(dead_code)]
pub fn allocate(layout: Layout) -> *mut u8 {
    unsafe { alloc(layout) }
}

#[allow(dead_code)]
pub fn deallocate(ptr: *mut u8, layout: Layout) {
    unsafe { dealloc(ptr, layout) }
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
