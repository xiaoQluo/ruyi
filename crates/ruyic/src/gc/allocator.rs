use std::alloc::{GlobalAlloc, Layout};

pub struct Allocator;

impl Allocator {
    pub fn new() -> Self {
        Self
    }

    pub fn allocate(&self, layout: Layout) -> *mut u8 {
        unsafe { std::alloc::alloc(layout) }
    }

    pub fn deallocate(&self, ptr: *mut u8, layout: Layout) {
        unsafe { std::alloc::dealloc(ptr, layout) }
    }
}