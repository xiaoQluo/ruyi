use ruyi_runtime::*;
use std::alloc::{dealloc, Layout};
use std::ffi::CString;

#[test]
fn test_ruyi_string_concat_smoke() {
    let a = CString::new("Hello, ").unwrap();
    let b = CString::new("World!").unwrap();
    unsafe {
        let result = ruyi_string_concat(a.as_ptr(), b.as_ptr());
        assert!(!result.is_null());
        let cstr = std::ffi::CStr::from_ptr(result);
        assert_eq!(cstr.to_str().unwrap(), "Hello, World!");
        dealloc(result as *mut u8, Layout::from_size_align(14, 1).unwrap());
    }
}

#[test]
fn test_ruyi_array_alloc_smoke() {
    unsafe {
        let arr = ruyi_array_alloc(4);
        assert!(!arr.is_null());
        assert_eq!(*(arr as *mut i64), 0);
        assert_eq!(*(arr.add(std::mem::size_of::<i64>()) as *mut i64), 4);
        let layout = Layout::from_size_align(
            std::mem::size_of::<i64>() * 2 + 4 * std::mem::size_of::<*mut i8>(),
            std::mem::align_of::<i64>(),
        )
        .unwrap();
        dealloc(arr as *mut u8, layout);
    }
}

#[test]
fn test_ruyi_object_alloc_smoke() {
    unsafe {
        let obj = ruyi_object_alloc(2);
        assert!(!obj.is_null());
        assert_eq!(*(obj as *mut i64), 2);
        let layout = Layout::from_size_align(
            std::mem::size_of::<i64>() + 2 * std::mem::size_of::<*mut i8>(),
            std::mem::align_of::<i64>(),
        )
        .unwrap();
        dealloc(obj as *mut u8, layout);
    }
}

#[test]
fn test_ruyi_bigint_from_str_smoke() {
    let s = CString::new("99999999999999999999").unwrap();
    unsafe {
        let result = ruyi_bigint_from_str(s.as_ptr());
        assert!(!result.is_null());
        let cstr = std::ffi::CStr::from_ptr(result);
        assert_eq!(cstr.to_str().unwrap(), "99999999999999999999");
        dealloc(
            result as *mut u8,
            Layout::from_size_align(21, 1).unwrap(),
        );
    }
}

#[test]
fn test_ruyi_member_access_smoke() {
    unsafe {
        let obj = ruyi_object_alloc(2);
        let fields = obj.add(std::mem::size_of::<i64>()) as *mut *mut i8;
        let dummy = 0xABCD as *mut i8;
        *fields.add(0) = dummy;
        *fields.add(1) = std::ptr::null_mut();

        assert_eq!(ruyi_member_access(obj, 0), dummy);
        assert!(ruyi_member_access(obj, 1).is_null());

        let layout = Layout::from_size_align(
            std::mem::size_of::<i64>() + 2 * std::mem::size_of::<*mut i8>(),
            std::mem::align_of::<i64>(),
        )
        .unwrap();
        dealloc(obj as *mut u8, layout);
    }
}
