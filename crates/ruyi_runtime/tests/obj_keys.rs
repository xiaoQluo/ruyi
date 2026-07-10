use ruyi_runtime::builtins::*;
use std::alloc::{alloc, dealloc, Layout};
use std::ffi::{CStr, CString};

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
    let base = obj.add(std::mem::size_of::<i64>() + index * 2 * std::mem::size_of::<*mut i8>())
        as *mut *mut i8;
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
    let layout =
        Layout::from_size_align(std::mem::size_of::<i64>(), std::mem::align_of::<i64>()).unwrap();
    let ptr = alloc(layout) as *mut i64;
    *ptr = value;
    ptr as *mut i8
}

unsafe fn dealloc_int(ptr: *mut i8) {
    let layout =
        Layout::from_size_align(std::mem::size_of::<i64>(), std::mem::align_of::<i64>()).unwrap();
    dealloc(ptr as *mut u8, layout);
}

unsafe fn dealloc_array(arr: *mut i8) {
    let cap = *(arr.add(std::mem::size_of::<i64>()) as *mut i64);
    let data_size = cap as usize * std::mem::size_of::<i64>();
    let layout = Layout::from_size_align(
        std::mem::size_of::<i64>() * 2 + data_size,
        std::mem::align_of::<i64>(),
    )
    .unwrap();
    dealloc(arr as *mut u8, layout);
}

#[test]
fn test_keys_of_2_field_object() {
    let key_x = CString::new("x").unwrap();
    let key_y = CString::new("y").unwrap();
    let value = unsafe { alloc_int(1) };
    let obj = unsafe { alloc_object(2) };
    let key_x_raw = key_x.into_raw();
    let key_y_raw = key_y.into_raw();
    unsafe {
        set_object_field(obj, 0, key_x_raw, value);
        set_object_field(obj, 1, key_y_raw, value);
    }

    let arr = ruyi_obj_keys(obj);
    assert!(!arr.is_null());
    assert_eq!(ruyi_array_length(arr), 2);

    unsafe {
        let data = arr.add(std::mem::size_of::<i64>() * 2) as *const i64;
        let k0 = CStr::from_ptr(*data.add(0) as *const i8).to_str().unwrap();
        let k1 = CStr::from_ptr(*data.add(1) as *const i8).to_str().unwrap();
        let mut keys = vec![k0, k1];
        keys.sort();
        assert_eq!(keys, vec!["x", "y"]);
    }

    unsafe {
        dealloc_object(obj, 2);
        dealloc_int(value);
        let _ = CString::from_raw(key_x_raw);
        let _ = CString::from_raw(key_y_raw);
        dealloc_array(arr);
    }
}

#[test]
fn test_keys_of_empty_object() {
    let obj = unsafe { alloc_object(0) };

    let arr = ruyi_obj_keys(obj);
    assert!(!arr.is_null());
    assert_eq!(ruyi_array_length(arr), 0);

    unsafe {
        dealloc_object(obj, 0);
        dealloc_array(arr);
    }
}

#[test]
fn test_keys_of_null() {
    let arr = ruyi_obj_keys(std::ptr::null_mut());
    assert!(!arr.is_null());
    assert_eq!(ruyi_array_length(arr), 0);

    unsafe {
        dealloc_array(arr);
    }
}
