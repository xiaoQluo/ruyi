use ruyi_runtime::gc_exports::{
    ruyi_gc_alloc,
    ruyi_gc_collect,
    ruyi_gc_add_root,
    ruyi_gc_remove_root,
    ruyi_gc_write_barrier,
};
use ruyi_runtime::GcObjectHeader;

#[test]
fn test_gc_alloc_returns_unique() {
    let mut pointers: Vec<*mut u8> = Vec::with_capacity(100);

    for size in 1..=100 {
        let ptr = ruyi_gc_alloc(size);
        assert!(!ptr.is_null(), "allocation with size {} returned null", size);
        pointers.push(ptr);
    }

    for (i, &p1) in pointers.iter().enumerate() {
        for (j, &p2) in pointers.iter().enumerate() {
            if i != j {
                assert_ne!(p1, p2, "duplicate pointer detected at indices {} and {}", i, j);
            }
        }
    }
}

#[test]
fn test_gc_alloc_zero_size() {
    let ptr = ruyi_gc_alloc(0);
    let ptr2 = ruyi_gc_alloc(0);
    if !ptr.is_null() && !ptr2.is_null() {
        assert_ne!(ptr, ptr2, "zero-size allocations should be distinct if non-null");
    }
}

#[test]
fn test_gc_collect_survives_reachable() {
    let obj = ruyi_gc_alloc(64);
    assert!(!obj.is_null());

    ruyi_gc_add_root(obj);

    unsafe { *(obj as *mut u64) = 0xDEADBEEF; }

    ruyi_gc_collect();

    unsafe {
        let header = GcObjectHeader::from_payload(obj);
        assert!(!header.is_null() && (*header).is_marked(), "object should survive with root");
    }

    ruyi_gc_remove_root(obj);
}

#[test]
fn test_gc_collect_frees_unreachable() {
    let _a = ruyi_gc_alloc(32);
    let _b = ruyi_gc_alloc(64);
    let _c = ruyi_gc_alloc(128);

    ruyi_gc_collect();

    let new_obj = ruyi_gc_alloc(256);
    assert!(!new_obj.is_null(), "allocation after GC should succeed");
}

#[test]
fn test_gc_add_remove_root() {
    let obj = ruyi_gc_alloc(32);
    assert!(!obj.is_null());

    unsafe { *(obj as *mut u64) = 0xCAFEBABE; }

    ruyi_gc_add_root(obj);
    ruyi_gc_collect();

    unsafe {
        assert_eq!(*(obj as *mut u64), 0xCAFEBABE);
    }

    ruyi_gc_remove_root(obj);
    ruyi_gc_collect();

    let check = ruyi_gc_alloc(16);
    assert!(!check.is_null());
}

#[test]
fn test_gc_write_barrier() {
    let young = ruyi_gc_alloc(16);
    let old = ruyi_gc_alloc(16);
    assert!(!young.is_null());
    assert!(!old.is_null());

    unsafe {
        let old_header = GcObjectHeader::from_payload(old);
        (*old_header).set_generation(2);
    }

    ruyi_gc_write_barrier(old, young);

    ruyi_gc_add_root(old);
    ruyi_gc_add_root(young);

    unsafe { *(young as *mut u64) = 0x1234; }
    ruyi_gc_collect();
    unsafe { assert_eq!(*(young as *mut u64), 0x1234); }

    ruyi_gc_remove_root(old);
    ruyi_gc_remove_root(young);
}

#[test]
fn test_gc_stress() {
    const N: usize = 10_000;

    let mut roots: Vec<*mut u8> = Vec::with_capacity(N / 2);

    for i in 0..N {
        let obj = ruyi_gc_alloc(16);
        assert!(!obj.is_null(), "allocation {} failed", i);

        unsafe { *(obj as *mut u64) = i as u64; }

        if i % 2 == 0 {
            ruyi_gc_add_root(obj);
            roots.push(obj);
        }
    }

    for _ in 0..5 {
        ruyi_gc_collect();
    }

    for (i, &obj) in roots.iter().enumerate() {
        let expected = (i * 2) as u64;
        unsafe { assert_eq!(*(obj as *mut u64), expected, "root {} value mismatch", i); }
    }

    for &obj in &roots {
        ruyi_gc_remove_root(obj);
    }

    ruyi_gc_collect();

    let check = ruyi_gc_alloc(1024);
    assert!(!check.is_null());
}
