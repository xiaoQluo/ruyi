use ruyi_runtime::builtins::*;
use std::alloc::{alloc, dealloc, Layout};
use std::ffi::CString;

unsafe fn alloc_object(field_count: usize) -> *mut i8 {
    let header_size = std::mem::size_of::<i64>();
    let data_size = field_count * 2 * std::mem::size_of::<*mut i8>();
    let layout =
        Layout::from_size_align(header_size + data_size, std::mem::align_of::<i64>()).unwrap();
    let ptr = alloc(layout) as *mut i8;
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    *(ptr as *mut i64) = field_count as i64;
    std::ptr::write_bytes(ptr.add(header_size), 0, data_size);
    ptr
}

unsafe fn set_object_field(obj: *mut i8, index: usize, key: *mut i8, value: *mut i8) {
    let base = obj.add(
        std::mem::size_of::<i64>() + index * 2 * std::mem::size_of::<*mut i8>(),
    ) as *mut *mut i8;
    *base.add(0) = key;
    *base.add(1) = value;
}

unsafe fn dealloc_object(obj: *mut i8, field_count: usize) {
    let header_size = std::mem::size_of::<i64>();
    let data_size = field_count * 2 * std::mem::size_of::<*mut i8>();
    let layout =
        Layout::from_size_align(header_size + data_size, std::mem::align_of::<i64>()).unwrap();
    dealloc(obj as *mut u8, layout);
}

unsafe fn alloc_int(value: i64) -> *mut i8 {
    let layout = Layout::from_size_align(std::mem::size_of::<i64>(), std::mem::align_of::<i64>())
        .unwrap();
    let ptr = alloc(layout) as *mut i64;
    *ptr = value;
    ptr as *mut i8
}

unsafe fn dealloc_int(ptr: *mut i8) {
    let layout = Layout::from_size_align(std::mem::size_of::<i64>(), std::mem::align_of::<i64>())
        .unwrap();
    dealloc(ptr as *mut u8, layout);
}

#[test]
fn test_get_existing_key() {
    let key = CString::new("x").unwrap();
    let value = unsafe { alloc_int(42) };
    let obj = unsafe { alloc_object(1) };
    let key_raw = key.into_raw();
    unsafe {
        set_object_field(obj, 0, key_raw, value);
    }

    let query = CString::new("x").unwrap();
    let result = ruyi_obj_get(obj, query.as_ptr());
    assert!(!result.is_null());

    unsafe {
        dealloc_object(obj, 1);
        dealloc_int(value);
        let _ = CString::from_raw(key_raw);
    }
}

#[test]
fn test_get_missing_key() {
    let key = CString::new("x").unwrap();
    let value = unsafe { alloc_int(42) };
    let obj = unsafe { alloc_object(1) };
    let key_raw = key.into_raw();
    unsafe {
        set_object_field(obj, 0, key_raw, value);
    }

    let query = CString::new("z").unwrap();
    let result = ruyi_obj_get(obj, query.as_ptr());
    assert!(result.is_null());

    unsafe {
        dealloc_object(obj, 1);
        dealloc_int(value);
        let _ = CString::from_raw(key_raw);
    }
}

#[test]
fn test_get_null_object() {
    let query = CString::new("x").unwrap();
    let result = ruyi_obj_get(std::ptr::null_mut(), query.as_ptr());
    assert!(result.is_null());
}

#[test]
fn test_get_null_key() {
    let key = CString::new("x").unwrap();
    let value = unsafe { alloc_int(42) };
    let obj = unsafe { alloc_object(1) };
    let key_raw = key.into_raw();
    unsafe {
        set_object_field(obj, 0, key_raw, value);
    }

    let result = ruyi_obj_get(obj, std::ptr::null());
    assert!(result.is_null());

    unsafe {
        dealloc_object(obj, 1);
        dealloc_int(value);
        let _ = CString::from_raw(key_raw);
    }
}
