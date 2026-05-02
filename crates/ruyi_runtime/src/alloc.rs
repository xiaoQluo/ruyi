use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU32, Ordering};

/// Memory management strategy for an object.
///
/// Ruyi supports both garbage collection (GC) and automatic reference
/// counting (ARC). The strategy is chosen per-object at allocation time.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryStrategy {
    /// Garbage collected — managed by the GC collector.
    GC = 0,
    /// Automatic reference counting — managed by explicit retain/release.
    ARC = 1,
}

/// Bit flags stored in `GcObjectHeader.flags`.
pub mod flags {
    /// Object has been marked during GC tracing.
    pub const MARKED: u32 = 1 << 0;
    /// Object is pinned and must not be moved.
    pub const PINNED: u32 = 1 << 1;
    /// Mask for the memory strategy bits (bits 2–3).
    pub const STRATEGY_MASK: u32 = 0b11 << 2;
    /// Mask for the generation bits (bits 4–5).
    ///
    /// Generation encoding:
    /// - 0 = Eden (young)
    /// - 1 = Survivor
    /// - 2 = Old
    /// - 3 = Reserved
    pub const GENERATION_MASK: u32 = 0b11 << 4;
}

use flags::*;

/// Runtime type information for every heap-allocated Ruyi object.
///
/// `TypeInfo` describes how to trace an object for GC roots and how to
/// destroy it when the memory is reclaimed.
#[repr(C)]
#[derive(Debug)]
pub struct TypeInfo {
    /// Unique type identifier (used by exception tables and RTTI).
    pub type_id: u64,
    /// Human-readable type name.
    pub type_name: &'static str,
    /// Destructor called before the object memory is freed.
    pub destructor: Option<unsafe extern "C" fn(*mut u8)>,
    /// GC tracing function. Invoked with the object payload pointer and a
    /// callback that must be called for every interior GC pointer field.
    ///
    /// The callback receives the **address** of the pointer field
    /// (`*mut *mut u8`) so that the collector can both read the current
    /// child and overwrite it with a new value during copying/compaction.
    pub trace_fn: Option<unsafe fn(*mut u8, trace: &mut dyn FnMut(*mut *mut u8))>,
}

/// Universal object header for all Ruyi heap allocations.
///
/// The header is designed to support **both** GC and ARC (Task 25) so that
/// the runtime does not need two separate object layouts.
///
/// Memory layout (64-bit):
/// ```text
///  0..4   flags:       mark | pinned | strategy | generation
///  4..8   ref_count:   atomic reference count (ARC mode)
///  8..16  type_info:   *mut TypeInfo
/// 16..24  size:        usize (payload size in bytes)
/// 24..32  forwarding:  *mut u8 (used during copying/compaction)
/// ```
#[repr(C)]
#[derive(Debug)]
pub struct GcObjectHeader {
    flags: u32,
    ref_count: AtomicU32,
    pub type_info: *mut TypeInfo,
    pub size: usize,
    pub forwarding_ptr: *mut u8,
}

impl GcObjectHeader {
    /// Size of the header in bytes.
    pub const SIZE: usize = std::mem::size_of::<Self>();

    /// Create a new header with the given parameters.
    ///
    /// # Safety
    ///
    /// `type_info` must remain valid for the lifetime of the object.
    pub unsafe fn new(size: usize, type_info: *mut TypeInfo, strategy: MemoryStrategy) -> Self {
        Self {
            flags: ((strategy as u32) << 2),
            ref_count: AtomicU32::new(1),
            type_info,
            size,
            forwarding_ptr: std::ptr::null_mut(),
        }
    }

    /// Return a pointer to the payload that follows this header.
    pub fn payload(&self) -> *mut u8 {
        unsafe { (self as *const Self).add(1) as *mut u8 }
    }

    /// Return the header from a payload pointer (inverse of `payload`).
    ///
    /// # Safety
    ///
    /// `ptr` must have been obtained from `GcObjectHeader::payload`.
    pub unsafe fn from_payload(ptr: *mut u8) -> *mut Self {
        ptr.sub(Self::SIZE) as *mut Self
    }

    // --- mark bit ---

    pub fn is_marked(&self) -> bool {
        (self.flags & MARKED) != 0
    }

    pub fn set_marked(&mut self) {
        self.flags |= MARKED;
    }

    pub fn clear_marked(&mut self) {
        self.flags &= !MARKED;
    }

    // --- pinned bit ---

    pub fn is_pinned(&self) -> bool {
        (self.flags & PINNED) != 0
    }

    pub fn set_pinned(&mut self) {
        self.flags |= PINNED;
    }

    pub fn clear_pinned(&mut self) {
        self.flags &= !PINNED;
    }

    // --- strategy ---

    pub fn strategy(&self) -> MemoryStrategy {
        match (self.flags & STRATEGY_MASK) >> 2 {
            1 => MemoryStrategy::ARC,
            _ => MemoryStrategy::GC,
        }
    }

    pub fn set_strategy(&mut self, strategy: MemoryStrategy) {
        self.flags = (self.flags & !STRATEGY_MASK) | ((strategy as u32) << 2);
    }

    // --- generation ---

    pub fn generation(&self) -> u8 {
        ((self.flags & GENERATION_MASK) >> 4) as u8
    }

    pub fn set_generation(&mut self, generation: u8) {
        self.flags = (self.flags & !GENERATION_MASK) | ((generation as u32) << 4);
    }

    // --- reference counting (ARC) ---

    /// Atomically increment the reference count.
    pub fn retain(&self) -> u32 {
        self.ref_count.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Atomically decrement the reference count, returning the previous value.
    pub fn release(&self) -> u32 {
        self.ref_count.fetch_sub(1, Ordering::Release) - 1
    }

    pub fn ref_count(&self) -> u32 {
        self.ref_count.load(Ordering::Acquire)
    }
}

/// A simple bump/free-list hybrid heap used by the Ruyi runtime.
///
/// In v1 this delegates to the system allocator. Future iterations may
/// switch to a dedicated memory arena.
#[derive(Debug, Default)]
pub struct Heap {
    _private: (),
}

impl Heap {
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Allocate raw memory with the given layout.
    ///
    /// # Safety
    ///
    /// Follows the same safety contract as `std::alloc::GlobalAlloc::alloc`.
    pub unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        System.alloc(layout)
    }

    /// Deallocate raw memory.
    ///
    /// # Safety
    ///
    /// Follows the same safety contract as `std::alloc::GlobalAlloc::dealloc`.
    pub unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
    }

    /// Reallocate raw memory.
    ///
    /// # Safety
    ///
    /// Follows the same safety contract as `std::alloc::GlobalAlloc::realloc`.
    pub unsafe fn realloc(&self, ptr: *mut u8, old_layout: Layout, new_size: usize) -> *mut u8 {
        System.realloc(ptr, old_layout, new_size)
    }
}

/// Allocate a Ruyi heap object with the given payload size and type info.
///
/// The returned pointer points to the **payload** (after the header). Use
/// `GcObjectHeader::from_payload` to recover the header.
///
/// # Safety
///
/// `type_info` must remain valid for the lifetime of the object.
pub unsafe fn ruyi_alloc(size: usize, type_info: *mut TypeInfo, strategy: MemoryStrategy) -> *mut u8 {
    let total = size + GcObjectHeader::SIZE;
    let layout = Layout::from_size_align(total, align_of::<GcObjectHeader>()).unwrap();
    let ptr = System.alloc(layout);
    if ptr.is_null() {
        std::alloc::handle_alloc_error(layout);
    }

    let header = ptr as *mut GcObjectHeader;
    header.write(GcObjectHeader::new(size, type_info, strategy));

    (*header).payload()
}

/// Deallocate a Ruyi heap object given its payload pointer.
///
/// # Safety
///
/// `ptr` must have been returned by `ruyi_alloc` and not already freed.
pub unsafe fn ruyi_dealloc(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }
    let header = GcObjectHeader::from_payload(ptr);
    let size = (*header).size + GcObjectHeader::SIZE;
    let layout = Layout::from_size_align(size, align_of::<GcObjectHeader>()).unwrap();

    // Call destructor if present.
    if let Some(dtor) = (*(*header).type_info).destructor {
        dtor(ptr);
    }

    System.dealloc(header as *mut u8, layout);
}

/// Reallocate a Ruyi heap object to a new payload size.
///
/// The header is preserved and moved to the new location.
///
/// # Safety
///
/// `ptr` must have been returned by `ruyi_alloc`.
pub unsafe fn ruyi_realloc(ptr: *mut u8, new_size: usize) -> *mut u8 {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let old_header = GcObjectHeader::from_payload(ptr);
    let old_size = (*old_header).size + GcObjectHeader::SIZE;
    let old_layout = Layout::from_size_align(old_size, align_of::<GcObjectHeader>()).unwrap();

    let new_total = new_size + GcObjectHeader::SIZE;
    let new_layout = Layout::from_size_align(new_total, align_of::<GcObjectHeader>()).unwrap();

    let new_ptr = System.realloc(old_header as *mut u8, old_layout, new_total);
    if new_ptr.is_null() {
        std::alloc::handle_alloc_error(new_layout);
    }

    let new_header = new_ptr as *mut GcObjectHeader;
    (*new_header).size = new_size;
    (*new_header).payload()
}

/// Allocate a raw buffer (no header) using the system allocator.
pub fn allocate(layout: Layout) -> *mut u8 {
    unsafe { System.alloc(layout) }
}

/// Deallocate a raw buffer.
pub fn deallocate(ptr: *mut u8, layout: Layout) {
    unsafe { System.dealloc(ptr, layout) }
}

/// Reallocate a raw buffer.
pub fn reallocate(ptr: *mut u8, old_layout: Layout, new_layout: Layout) -> *mut u8 {
    if new_layout.size() <= old_layout.size() {
        return ptr;
    }
    let new_ptr = allocate(new_layout);
    unsafe {
        std::ptr::copy_nonoverlapping(ptr, new_ptr, old_layout.size());
        deallocate(ptr, old_layout);
    }
    new_ptr
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ruyi_alloc_dealloc() {
        static mut TYPE_INFO: TypeInfo = TypeInfo {
            type_id: 1,
            type_name: "test",
            destructor: None,
            trace_fn: None,
        };

        unsafe {
            let ptr = ruyi_alloc(64, &raw mut TYPE_INFO, MemoryStrategy::GC);
            assert!(!ptr.is_null());

            let header = GcObjectHeader::from_payload(ptr);
            assert_eq!((*header).size, 64);
            assert_eq!((*header).strategy(), MemoryStrategy::GC);
            assert!(!(*header).is_marked());

            ruyi_dealloc(ptr);
        }
    }

    #[test]
    fn test_header_flags() {
        let mut header = unsafe { GcObjectHeader::new(32, std::ptr::null_mut(), MemoryStrategy::GC) };
        assert!(!header.is_marked());
        header.set_marked();
        assert!(header.is_marked());
        header.clear_marked();
        assert!(!header.is_marked());

        assert!(!header.is_pinned());
        header.set_pinned();
        assert!(header.is_pinned());

        assert_eq!(header.strategy(), MemoryStrategy::GC);
        header.set_strategy(MemoryStrategy::ARC);
        assert_eq!(header.strategy(), MemoryStrategy::ARC);

        header.set_generation(2);
        assert_eq!(header.generation(), 2);
    }

    #[test]
    fn test_arc_ref_count() {
        let header = unsafe { GcObjectHeader::new(16, std::ptr::null_mut(), MemoryStrategy::ARC) };
        assert_eq!(header.ref_count(), 1);
        assert_eq!(header.retain(), 2);
        assert_eq!(header.retain(), 3);
        assert_eq!(header.release(), 2);
        assert_eq!(header.release(), 1);
        assert_eq!(header.release(), 0);
    }

    #[test]
    fn test_ruyi_realloc() {
        static mut TYPE_INFO: TypeInfo = TypeInfo {
            type_id: 2,
            type_name: "realloc_test",
            destructor: None,
            trace_fn: None,
        };

        unsafe {
            let ptr = ruyi_alloc(16, &raw mut TYPE_INFO, MemoryStrategy::GC);
            let header = GcObjectHeader::from_payload(ptr);
            assert_eq!((*header).size, 16);

            let new_ptr = ruyi_realloc(ptr, 128);
            let new_header = GcObjectHeader::from_payload(new_ptr);
            assert_eq!((*new_header).size, 128);
            assert_eq!((*new_header).strategy(), MemoryStrategy::GC);

            ruyi_dealloc(new_ptr);
        }
    }
}
