/**
 * Built-in runtime functions for Ruyi.
 *
 * Provides C-ABI runtime helpers for string concat, array/object
 * allocation, bigint conversion, and member access.
 *
 * All allocations use the system allocator (malloc/free equivalent)
 * with GC integration deferred to a later milestone.
 *
 * @author Ruyi Team
 * @date 2026-05-02
 */

use std::alloc::{alloc, Layout};
use std::ffi::CStr;

/// Concatenate two null-terminated C strings.
///
/// Returns a newly allocated null-terminated string containing `lhs`
/// followed by `rhs`. The caller is responsible for freeing the
/// returned pointer.
///
/// # Safety
///
/// `lhs` and `rhs` must each be null-terminated or null.
#[no_mangle]
pub extern "C" fn ruyi_string_concat(lhs: *const i8, rhs: *const i8) -> *mut i8 {
    unsafe {
        let lhs_bytes = if lhs.is_null() {
            &[]
        } else {
            CStr::from_ptr(lhs).to_bytes()
        };
        let rhs_bytes = if rhs.is_null() {
            &[]
        } else {
            CStr::from_ptr(rhs).to_bytes()
        };

        let total = lhs_bytes.len() + rhs_bytes.len();
        let layout = Layout::from_size_align(total + 1, 1).unwrap();
        let out = alloc(layout) as *mut i8;
        if out.is_null() {
            return std::ptr::null_mut();
        }

        std::ptr::copy_nonoverlapping(lhs_bytes.as_ptr(), out as *mut u8, lhs_bytes.len());
        std::ptr::copy_nonoverlapping(
            rhs_bytes.as_ptr(),
            out.add(lhs_bytes.len()) as *mut u8,
            rhs_bytes.len(),
        );
        *out.add(total) = 0;
        out
    }
}

/// Allocate a Ruyi array with the given capacity.
///
/// Layout: `[len: i64][cap: i64][data: *mut i8 * cap]`
///
/// Returns a pointer to the array header. The caller is responsible
/// for freeing the returned pointer.
#[no_mangle]
pub extern "C" fn ruyi_array_alloc(capacity: i64) -> *mut i8 {
    unsafe {
        let cap = if capacity < 0 { 0 } else { capacity as usize };
        let header_size = std::mem::size_of::<i64>() * 2;
        let data_size = cap * std::mem::size_of::<*mut i8>();
        let layout = Layout::from_size_align(
            header_size + data_size,
            std::mem::align_of::<i64>(),
        )
        .unwrap();
        let ptr = alloc(layout) as *mut i8;
        if ptr.is_null() {
            return std::ptr::null_mut();
        }
        *(ptr as *mut i64) = 0; // len
        *(ptr.add(std::mem::size_of::<i64>()) as *mut i64) = cap as i64; // cap
        // Zero-initialize the data slots.
        std::ptr::write_bytes(ptr.add(header_size), 0, data_size);
        ptr
    }
}

/// Allocate a Ruyi object with the given field count.
///
/// Layout: `[field_count: i64][fields: *mut i8 * field_count]`
///
/// Returns a pointer to the object header. The caller is responsible
/// for freeing the returned pointer.
#[no_mangle]
pub extern "C" fn ruyi_object_alloc(field_count: i64) -> *mut i8 {
    unsafe {
        let count = if field_count < 0 { 0 } else { field_count as usize };
        let header_size = std::mem::size_of::<i64>();
        let data_size = count * std::mem::size_of::<*mut i8>();
        let layout = Layout::from_size_align(
            header_size + data_size,
            std::mem::align_of::<i64>(),
        )
        .unwrap();
        let ptr = alloc(layout) as *mut i8;
        if ptr.is_null() {
            return std::ptr::null_mut();
        }
        *(ptr as *mut i64) = count as i64;
        // Zero-initialize the field slots.
        std::ptr::write_bytes(ptr.add(header_size), 0, data_size);
        ptr
    }
}

/// Create a bigint from a decimal string.
///
/// In this staged implementation the bigint is stored as an opaque
/// heap-allocated copy of the input string. Future iterations will
/// switch to a real arbitrary-precision representation.
///
/// # Safety
///
/// `s` must be a valid null-terminated string.
#[no_mangle]
pub extern "C" fn ruyi_bigint_from_str(s: *const i8) -> *mut i8 {
    unsafe {
        if s.is_null() {
            return std::ptr::null_mut();
        }
        let bytes = CStr::from_ptr(s).to_bytes();
        let layout = Layout::from_size_align(bytes.len() + 1, 1).unwrap();
        let out = alloc(layout) as *mut i8;
        if out.is_null() {
            return std::ptr::null_mut();
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out as *mut u8, bytes.len());
        *out.add(bytes.len()) = 0;
        out
    }
}

/// Access a field of a Ruyi object by offset.
///
/// `obj` is treated as a pointer to an object layout where the first
/// `i64` is the field count and the remaining slots are `*mut i8`
/// fields. `offset` is a zero-based index into the fields. The
/// return value is the pointer stored at that slot.
///
/// # Safety
///
/// `obj` must be a valid pointer returned by `ruyi_object_alloc`.
/// `offset` must be non-negative and less than the field count.
#[no_mangle]
pub extern "C" fn ruyi_member_access(obj: *mut i8, offset: i64) -> *mut i8 {
    unsafe {
        if obj.is_null() || offset < 0 {
            return std::ptr::null_mut();
        }
        let fields = obj.add(std::mem::size_of::<i64>()) as *mut *mut i8;
        *fields.add(offset as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::{dealloc, Layout};
    use std::ffi::CString;

    #[test]
    fn test_ruyi_string_concat_basic() {
        let a = CString::new("Hello, ").unwrap();
        let b = CString::new("World!").unwrap();
        unsafe {
            let result = ruyi_string_concat(a.as_ptr(), b.as_ptr());
            assert!(!result.is_null());
            let cstr = CStr::from_ptr(result);
            assert_eq!(cstr.to_str().unwrap(), "Hello, World!");
            dealloc(result as *mut u8, Layout::from_size_align(14, 1).unwrap());
        }
    }

    #[test]
    fn test_ruyi_string_concat_with_null() {
        let a = CString::new("solo").unwrap();
        unsafe {
            let result = ruyi_string_concat(a.as_ptr(), std::ptr::null());
            assert!(!result.is_null());
            let cstr = CStr::from_ptr(result);
            assert_eq!(cstr.to_str().unwrap(), "solo");
            dealloc(result as *mut u8, Layout::from_size_align(5, 1).unwrap());
        }
    }

    #[test]
    fn test_ruyi_string_concat_both_null() {
        unsafe {
            let result = ruyi_string_concat(std::ptr::null(), std::ptr::null());
            assert!(!result.is_null());
            let cstr = CStr::from_ptr(result);
            assert_eq!(cstr.to_str().unwrap(), "");
            dealloc(result as *mut u8, Layout::from_size_align(1, 1).unwrap());
        }
    }

    #[test]
    fn test_ruyi_array_alloc() {
        unsafe {
            let arr = ruyi_array_alloc(5);
            assert!(!arr.is_null());
            assert_eq!(*(arr as *mut i64), 0); // len
            assert_eq!(*(arr.add(std::mem::size_of::<i64>()) as *mut i64), 5); // cap
            let layout = Layout::from_size_align(
                std::mem::size_of::<i64>() * 2 + 5 * std::mem::size_of::<*mut i8>(),
                std::mem::align_of::<i64>(),
            )
            .unwrap();
            dealloc(arr as *mut u8, layout);
        }
    }

    #[test]
    fn test_ruyi_array_alloc_negative() {
        unsafe {
            let arr = ruyi_array_alloc(-1);
            assert!(!arr.is_null());
            assert_eq!(*(arr as *mut i64), 0); // len
            assert_eq!(*(arr.add(std::mem::size_of::<i64>()) as *mut i64), 0i64);
            let layout = Layout::from_size_align(
                std::mem::size_of::<i64>() * 2,
                std::mem::align_of::<i64>(),
            )
            .unwrap();
            dealloc(arr as *mut u8, layout);
        }
    }

    #[test]
    fn test_ruyi_object_alloc() {
        unsafe {
            let obj = ruyi_object_alloc(3);
            assert!(!obj.is_null());
            assert_eq!(*(obj as *mut i64), 3); // field_count
            let layout = Layout::from_size_align(
                std::mem::size_of::<i64>() + 3 * std::mem::size_of::<*mut i8>(),
                std::mem::align_of::<i64>(),
            )
            .unwrap();
            dealloc(obj as *mut u8, layout);
        }
    }

    #[test]
    fn test_ruyi_object_alloc_negative() {
        unsafe {
            let obj = ruyi_object_alloc(-1);
            assert!(!obj.is_null());
            assert_eq!(*(obj as *mut i64), 0i64);
            let layout = Layout::from_size_align(
                std::mem::size_of::<i64>(),
                std::mem::align_of::<i64>(),
            )
            .unwrap();
            dealloc(obj as *mut u8, layout);
        }
    }

    #[test]
    fn test_ruyi_bigint_from_str() {
        let s = CString::new("12345678901234567890").unwrap();
        unsafe {
            let result = ruyi_bigint_from_str(s.as_ptr());
            assert!(!result.is_null());
            let cstr = CStr::from_ptr(result);
            assert_eq!(cstr.to_str().unwrap(), "12345678901234567890");
            dealloc(
                result as *mut u8,
                Layout::from_size_align(21, 1).unwrap(),
            );
        }
    }

    #[test]
    fn test_ruyi_bigint_from_str_null() {
        let result = ruyi_bigint_from_str(std::ptr::null());
        assert!(result.is_null());
    }

    #[test]
    fn test_ruyi_member_access() {
        unsafe {
            let obj = ruyi_object_alloc(3);
            let fields = obj.add(std::mem::size_of::<i64>()) as *mut *mut i8;
            let dummy: *mut i8 = 0x1234 as *mut i8;
            *fields.add(0) = dummy;
            *fields.add(1) = std::ptr::null_mut();
            *fields.add(2) = dummy;

            assert_eq!(ruyi_member_access(obj, 0), dummy);
            assert!(ruyi_member_access(obj, 1).is_null());
            assert_eq!(ruyi_member_access(obj, 2), dummy);
            assert!(ruyi_member_access(std::ptr::null_mut(), 0).is_null());
            assert!(ruyi_member_access(obj, -1).is_null());

            let layout = Layout::from_size_align(
                std::mem::size_of::<i64>() + 3 * std::mem::size_of::<*mut i8>(),
                std::mem::align_of::<i64>(),
            )
            .unwrap();
            dealloc(obj as *mut u8, layout);
        }
    }
}
