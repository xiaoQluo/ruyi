pub use crate::atomic_ffi::{
    __atomic_i64_cas, __atomic_i64_fetch_add, __atomic_i64_fetch_sub, __atomic_i64_free,
    __atomic_i64_load, __atomic_i64_new, __atomic_i64_store, __atomic_i64_swap,
};
pub use crate::barrier_ffi::{__barrier_free, __barrier_new, __barrier_wait};
pub use crate::channel_ffi::{
    __channel_clone, __channel_clone_free, __channel_clone_send, __channel_free,
    __channel_is_closed, __channel_new, __channel_recv, __channel_recv_timeout,
    __channel_select_add, __channel_select_free, __channel_select_new, __channel_select_wait,
    __channel_send, __channel_try_recv, __channel_try_send,
};
pub use crate::compress_ffi::{
    __compress_deflate, __compress_finish, __compress_gzip, __compress_new, __compress_write,
    __compress_zlib, __decompress_deflate, __decompress_finish, __decompress_gzip,
    __decompress_new, __decompress_write, __decompress_zlib,
};
pub use crate::condvar_ffi::{
    __condvar_free, __condvar_new, __condvar_notify_all, __condvar_notify_one, __condvar_wait,
};
pub use crate::crypto_ffi::{
    __crypto_aes_gcm_decrypt_hex, __crypto_aes_gcm_decrypt_raw, __crypto_aes_gcm_encrypt_hex,
    __crypto_aes_gcm_encrypt_raw, __crypto_hmac_sha256, __crypto_sha1, __crypto_sha256,
    __crypto_sha512, __crypto_x25519_dh_hex, __crypto_x25519_dh_raw, __crypto_x25519_keypair_hex,
    __crypto_x25519_keypair_raw, __crypto_x25519_pubkey_hex, __crypto_x25519_pubkey_raw,
};
pub use crate::fiber_ffi::{
    __fiber_detach, __fiber_id, __fiber_is_finished, __fiber_join, __fiber_sleep, __fiber_spawn,
    __fiber_yield,
};
pub use crate::float_ffi::{__f64_from_bits, __f64_to_bits};
pub use crate::json_ffi::{__json_parse, __json_stringify};
pub use crate::math_ffi::{
    __math_abs, __math_ceil, __math_cos, __math_e, __math_floor, __math_log, __math_max,
    __math_min, __math_pi, __math_pow, __math_round, __math_sin, __math_sqrt, __math_tan,
};
pub use crate::mutex_ffi::{
    __mutex_free, __mutex_lock, __mutex_new, __mutex_try_lock, __mutex_unlock,
};
pub use crate::once_ffi::{__once_do, __once_free, __once_is_completed, __once_new, __once_reset};
/**
 * Built-in runtime functions for Ruyi.
 *
 * Provides C-ABI runtime helpers for string concat, array/object
 * allocation, bigint conversion, and member access.
 *
 * All allocations use the system allocator (malloc/free equivalent)
 * with GC integration deferred to a later milestone.
 *
 * The `random_ffi` module is re-exported here so the five
 * `__random_*` symbols are part of the same public surface as the
 * other builtins. Implementations live in `random_ffi.rs`; this file
 * just re-registers them under the `builtins` namespace.
 *
 * @author Ruyi Team
 * @date 2026-05-02
 */
pub use crate::random_ffi::{
    __random_bool, __random_bytes, __random_float, __random_int, __random_new,
};
pub use crate::rwlock_ffi::{
    __rwlock_free, __rwlock_new, __rwlock_read_lock, __rwlock_read_unlock, __rwlock_try_read_lock,
    __rwlock_try_write_lock, __rwlock_write_lock, __rwlock_write_unlock,
};
pub use crate::semaphore_ffi::{
    __semaphore_acquire, __semaphore_available, __semaphore_free, __semaphore_new,
    __semaphore_release, __semaphore_try_acquire,
};
pub use crate::thread_ffi::{
    __thread_cpu_count, __thread_detach, __thread_id, __thread_is_finished, __thread_join,
    __thread_join_timeout, __thread_sleep, __thread_spawn, __thread_spawn_named,
};
pub use crate::time_ffi::{__time_format, __time_now, __time_sleep, __time_timestamp};
pub use crate::tls_ffi::{
    __tls_close, __tls_config_free, __tls_connect, __tls_free, __tls_read_cstr, __tls_read_raw,
    __tls_server_accept, __tls_server_close, __tls_server_config_new, __tls_server_free,
    __tls_server_read_cstr, __tls_server_read_raw, __tls_server_write, __tls_server_write_raw,
    __tls_write, __tls_write_raw,
};
pub use crate::tls_store_ffi::{
    __tls_clear, __tls_contains, __tls_load, __tls_remove, __tls_store,
};
use std::alloc::{alloc, Layout};
use std::collections::{HashMap, HashSet};
use std::ffi::{CStr, CString};

/// Convert an i64 to a newly allocated null-terminated string.
///
/// The caller is responsible for freeing the returned pointer.
#[no_mangle]
pub extern "C" fn ruyi_int_to_string(n: i64) -> *mut i8 {
    let s = format!("{}", n);
    let bytes = s.into_bytes();
    unsafe {
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

/// Convert an f64 to a newly allocated null-terminated string.
///
/// The caller is responsible for freeing the returned pointer.
#[no_mangle]
pub extern "C" fn ruyi_float_to_string(n: f64) -> *mut i8 {
    let s = format!("{}", n);
    let bytes = s.into_bytes();
    unsafe {
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

/// Convert a bool to a newly allocated null-terminated string ("true" or "false").
///
/// The caller is responsible for freeing the returned pointer.
#[no_mangle]
pub extern "C" fn ruyi_bool_to_string(b: bool) -> *mut i8 {
    let s = if b { "true" } else { "false" };
    let bytes = s.as_bytes();
    unsafe {
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
#[allow(clippy::not_unsafe_ptr_arg_deref)]
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
/// Layout: `[len: i64][cap: i64][data_ptr: *mut i64]` (24-byte header)
/// followed by a separate element buffer of `cap` 8-byte words that
/// `data_ptr` references.
///
/// The header pointer is stable for the array's whole lifetime: growth
/// replaces only the data buffer, so the array keeps reference semantics
/// when passed across function boundaries.
#[no_mangle]
pub extern "C" fn ruyi_array_alloc(capacity: i64) -> *mut i8 {
    unsafe {
        let cap = if capacity < 0 { 0 } else { capacity as usize };
        let header_size = std::mem::size_of::<i64>() * 3;
        let layout =
            Layout::from_size_align(header_size, std::mem::align_of::<i64>()).unwrap();
        let ptr = alloc(layout) as *mut i8;
        if ptr.is_null() {
            return std::ptr::null_mut();
        }
        let data = if cap > 0 {
            let data_size = cap * std::mem::size_of::<i64>();
            let data_layout =
                Layout::from_size_align(data_size, std::mem::align_of::<i64>()).unwrap();
            let data_ptr = alloc(data_layout) as *mut i8;
            if data_ptr.is_null() {
                // Header leaks on this rare OOM path; freeing would need
                // the dealloc import and allocation failure is fatal anyway.
                return std::ptr::null_mut();
            }
            // Zero-initialize the data slots.
            std::ptr::write_bytes(data_ptr, 0, data_size);
            data_ptr
        } else {
            std::ptr::null_mut()
        };
        *(ptr as *mut i64) = 0; // len
        *(ptr.add(std::mem::size_of::<i64>()) as *mut i64) = cap as i64; // cap
        *(ptr.add(std::mem::size_of::<i64>() * 2) as *mut i64) = data as i64; // data_ptr
        ptr
    }
}

/// Read the element-buffer pointer stored in an array header (offset 16).
///
/// Returns null for a null header.
#[inline]
unsafe fn array_data_ptr(arr: *mut i8) -> *mut i64 {
    if arr.is_null() {
        return std::ptr::null_mut();
    }
    let raw = std::ptr::read_unaligned(arr.add(16) as *const i64);
    if (raw as usize) < 0x1000 {
        return std::ptr::null_mut();
    }
    raw as *mut i64
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
        let count = if field_count < 0 {
            0
        } else {
            field_count as usize
        };
        let header_size = std::mem::size_of::<i64>();
        let data_size = count * std::mem::size_of::<*mut i8>();
        let layout =
            Layout::from_size_align(header_size + data_size, std::mem::align_of::<i64>()).unwrap();
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
#[allow(clippy::not_unsafe_ptr_arg_deref)]
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

/// Compare two bigints for equality.
///
/// Returns non-zero (true) if the two bigints represent the same value,
/// zero (false) otherwise. In this staged implementation the bigint
/// payload is a decimal string, so equality is decided by byte-wise
/// comparison of the underlying storage. A real arbitrary-precision
/// representation will replace this placeholder once integrated.
///
/// # Safety
///
/// `a` and `b` must either be null or pointers returned by
/// `ruyi_bigint_from_str`.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn ruyi_bigint_eq(a: *mut i8, b: *mut i8) -> i8 {
    if a.is_null() || b.is_null() {
        return (a == b) as i8;
    }
    unsafe {
        let a_bytes = CStr::from_ptr(a).to_bytes();
        let b_bytes = CStr::from_ptr(b).to_bytes();
        (a_bytes == b_bytes) as i8
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
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn ruyi_member_access(obj: *mut i8, offset: i64) -> *mut i8 {
    unsafe {
        if obj.is_null() || offset < 0 {
            return std::ptr::null_mut();
        }
        let fields = obj.add(std::mem::size_of::<i64>()) as *mut *mut i8;
        *fields.add(offset as usize)
    }
}

/// Build a HashMap of field-name to value pointer from a Ruyi object.
///
/// Object layout: `[field_count: i64][key_0: *mut i8][value_0: *mut i8]...`
/// Each key is expected to be a null-terminated UTF-8 string.
fn object_field_map(obj: *mut i8) -> Option<HashMap<String, *mut i8>> {
    if obj.is_null() {
        return None;
    }
    unsafe {
        let field_count = *(obj as *mut i64);
        if field_count < 0 {
            return Some(HashMap::new());
        }
        let mut map = HashMap::with_capacity(field_count as usize);
        let field_size = 2 * std::mem::size_of::<*mut i8>();
        for i in 0..field_count as usize {
            let slot = obj.add(std::mem::size_of::<i64>() + i * field_size) as *mut *mut i8;
            let key_ptr = *slot;
            let value_ptr = *slot.add(1);
            if key_ptr.is_null() {
                continue;
            }
            // SAFETY: key_ptr is a null-terminated C string owned by the object.
            let key_str = CStr::from_ptr(key_ptr).to_str().ok()?;
            map.insert(key_str.to_string(), value_ptr);
        }
        Some(map)
    }
}

/// Look up a field by name in a Ruyi object.
///
/// Returns a pointer to the field value, or null if the object is null,
/// the key is null, or the key is not present.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn ruyi_obj_get(obj: *mut i8, key: *const i8) -> *mut i8 {
    if key.is_null() {
        return std::ptr::null_mut();
    }
    let map = match object_field_map(obj) {
        Some(m) => m,
        None => return std::ptr::null_mut(),
    };
    // SAFETY: key is a null-terminated C string from the caller.
    let key_bytes = unsafe { CStr::from_ptr(key).to_bytes() };
    let key_str = match std::str::from_utf8(key_bytes) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    match map.get(key_str) {
        Some(&value) => value,
        None => std::ptr::null_mut(),
    }
}

/// Return all field names of a Ruyi object as a Ruyi array of C strings.
///
/// Returns an empty array for a null object or an object with no fields.
#[no_mangle]
pub extern "C" fn ruyi_obj_keys(obj: *mut i8) -> *mut i8 {
    let map = match object_field_map(obj) {
        Some(m) => m,
        None => return ruyi_array_alloc(0),
    };
    let count = map.len() as i64;
    let arr = ruyi_array_alloc(count);
    if arr.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        // Claim the full capacity so ruyi_array_set can write each slot.
        *(arr as *mut i64) = count;
    }
    let mut idx: i64 = 0;
    for key in map.keys() {
        let c_key = match CString::new(key.as_str()) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let ptr = c_key.into_raw();
        ruyi_array_set(arr, idx, ptr as i64);
        idx += 1;
    }
    arr
}

/// Get the length of a Ruyi array.
///
/// Returns 0 if `arr` is null.
#[no_mangle]
pub extern "C" fn ruyi_array_length(arr: *mut i8) -> i64 {
    unsafe {
        if arr.is_null() {
            return 0;
        }
        *(arr as *mut i64)
    }
}

/// Get an element from a Ruyi array by index.
///
/// Returns null if `arr` is null, `index` is out of bounds, or negative.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn ruyi_array_get(arr: *mut i8, index: i64) -> i64 {
    unsafe {
        if arr.is_null() || index < 0 {
            return 0;
        }
        let len = std::ptr::read_unaligned(arr as *const i64);
        if index >= len {
            return 0;
        }
        let data = array_data_ptr(arr);
        if data.is_null() {
            return 0;
        }
        std::ptr::read_unaligned(data.add(index as usize))
    }
}

/// Set an element in a Ruyi array by index.
///
/// Does nothing if `arr` is null or `index` is out of bounds.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn ruyi_array_set(arr: *mut i8, index: i64, value: i64) {
    unsafe {
        if arr.is_null() || index < 0 {
            return;
        }
        let len = std::ptr::read_unaligned(arr as *const i64);
        if index >= len {
            return;
        }
        let data = array_data_ptr(arr);
        if data.is_null() {
            return;
        }
        std::ptr::write_unaligned(data.add(index as usize), value);
    }
}

/// Push an element onto the end of a Ruyi array.
///
/// On capacity overflow a new element buffer is allocated and the
/// header's `data_ptr` is updated in place; the header pointer itself
/// never moves, so every alias of the array observes the growth.
/// Returns the (unchanged) array header pointer, or null on allocation
/// failure.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn ruyi_array_push(arr: *mut i8, value: i64) -> *mut i8 {
    unsafe {
        if arr.is_null() {
            return std::ptr::null_mut();
        }
        let len_ptr = arr as *mut i64;
        let cap_ptr = arr.add(std::mem::size_of::<i64>()) as *mut i64;
        let data_ptr_slot = arr.add(std::mem::size_of::<i64>() * 2) as *mut i64;
        let len = *len_ptr;
        let cap = *cap_ptr;

        if len >= cap {
            let new_cap = if cap == 0 { 4 } else { cap * 2 };
            let new_data_size = new_cap as usize * std::mem::size_of::<i64>();
            let data_layout =
                Layout::from_size_align(new_data_size, std::mem::align_of::<i64>()).unwrap();
            let new_data = alloc(data_layout) as *mut i64;
            if new_data.is_null() {
                return std::ptr::null_mut();
            }

            let old_data = *data_ptr_slot as *mut i64;
            if !old_data.is_null() && cap > 0 {
                std::ptr::copy_nonoverlapping(
                    old_data,
                    new_data,
                    cap as usize,
                );
            }
            *data_ptr_slot = new_data as i64;
            *new_data.add(len as usize) = value;
            *cap_ptr = new_cap;
            *len_ptr = len + 1;
            return arr;
        }

        let data = *data_ptr_slot as *mut i64;
        if data.is_null() {
            return std::ptr::null_mut();
        }
        *data.add(len as usize) = value;
        *len_ptr = len + 1;
        arr
    }
}

/// Pop the last element from a Ruyi array.
///
/// Returns null if `arr` is null or empty.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn ruyi_array_pop(arr: *mut i8) -> i64 {
    unsafe {
        if arr.is_null() {
            return 0;
        }
        let len_ptr = arr as *mut i64;
        let len = *len_ptr;
        if len <= 0 {
            return 0;
        }
        *len_ptr = len - 1;
        let data = array_data_ptr(arr);
        if data.is_null() {
            return 0;
        }
        *data.add((len - 1) as usize)
    }
}

// ============================================================
// __builtin_array_* — stdlib/collections.ry entry points
// ============================================================

#[no_mangle]
pub extern "C" fn __builtin_array_create() -> *mut i8 {
    ruyi_array_alloc(0)
}

#[no_mangle]
pub extern "C" fn __builtin_array_get(arr: *mut i8, index: i64) -> i64 {
    ruyi_array_get(arr, index)
}

#[no_mangle]
pub extern "C" fn __builtin_array_set(arr: *mut i8, index: i64, value: i64) {
    ruyi_array_set(arr, index, value)
}

#[no_mangle]
pub extern "C" fn __builtin_array_push(arr: *mut i8, value: i64) -> *mut i8 {
    ruyi_array_push(arr, value)
}

#[no_mangle]
pub extern "C" fn __builtin_array_pop(arr: *mut i8) -> i64 {
    ruyi_array_pop(arr)
}

#[no_mangle]
pub extern "C" fn __builtin_array_length(arr: *mut i8) -> i64 {
    if arr.is_null() {
        return 0;
    }
    // Use read_unaligned: the codegen sometimes materializes an i64
    // value (boxed Dynamic tag, length, etc.) into this slot when the
    // array is unwrapped through a generic context. Reading without
    // alignment lets us return 0 on garbage instead of aborting the
    // process; legitimate array headers are still read correctly.
    let raw = unsafe { std::ptr::read_unaligned(arr as *const i64) };
    if raw < 0 {
        return 0;
    }
    raw
}

// ============================================================
// __builtin_map_* — HashMap<i64, i64> implementation
// ============================================================

/// Create a new empty map. Returns an opaque pointer to a boxed HashMap.
#[no_mangle]
pub extern "C" fn __builtin_map_create() -> *mut i8 {
    let map: Box<HashMap<i64, i64>> = Box::default();
    Box::into_raw(map) as *mut i8
}

/// Get value by key. Returns the value as i64 (cast to *mut i8), or null if not found.
#[no_mangle]
pub extern "C" fn __builtin_map_get(data: *mut i8, key: *mut i8) -> *mut i8 {
    if data.is_null() || !is_aligned_for_map(data) {
        return std::ptr::null_mut();
    }
    let map = unsafe { &*(data as *const HashMap<i64, i64>) };
    let k = key as i64;
    match map.get(&k) {
        Some(&v) => v as *mut i8,
        None => std::ptr::null_mut(),
    }
}

/// Set a key-value pair in the map.
#[no_mangle]
pub extern "C" fn __builtin_map_set(data: *mut i8, key: *mut i8, value: *mut i8) {
    if data.is_null() || !is_aligned_for_map(data) {
        return;
    }
    let map = unsafe { &mut *(data as *mut HashMap<i64, i64>) };
    map.insert(key as i64, value as i64);
}

/// Delete a key from the map.
#[no_mangle]
pub extern "C" fn __builtin_map_delete(data: *mut i8, key: *mut i8) {
    if data.is_null() || !is_aligned_for_map(data) {
        return;
    }
    let map = unsafe { &mut *(data as *mut HashMap<i64, i64>) };
    map.remove(&(key as i64));
}

/// Check if the map contains a key.
#[no_mangle]
pub extern "C" fn __builtin_map_has(data: *mut i8, key: *mut i8) -> bool {
    if data.is_null() || !is_aligned_for_map(data) {
        return false;
    }
    let map = unsafe { &*(data as *const HashMap<i64, i64>) };
    map.contains_key(&(key as i64))
}

/// Return all keys as a Ruyi array.
#[no_mangle]
pub extern "C" fn __builtin_map_keys(data: *mut i8) -> *mut i8 {
    if data.is_null() || !is_aligned_for_map(data) {
        return ruyi_array_alloc(0);
    }
    let map = unsafe { &*(data as *const HashMap<i64, i64>) };
    let mut arr = ruyi_array_alloc(map.len() as i64);
    for &k in map.keys() {
        arr = ruyi_array_push(arr, k);
    }
    arr
}

/// Return all values as a Ruyi array.
#[no_mangle]
pub extern "C" fn __builtin_map_values(data: *mut i8) -> *mut i8 {
    if data.is_null() || !is_aligned_for_map(data) {
        return ruyi_array_alloc(0);
    }
    let map = unsafe { &*(data as *const HashMap<i64, i64>) };
    let mut arr = ruyi_array_alloc(map.len() as i64);
    for &v in map.values() {
        arr = ruyi_array_push(arr, v);
    }
    arr
}

/// Sanity-check that a `*mut i8` looks like a real HashMap handle.
///
/// The codegen reads `_data` through a generic class field as `i64` and
/// then `inttoptr`'s it back; if the field was never initialized or was
/// overwritten with a tag/length value, the resulting pointer is not
/// dereferenceable. Comparing the address modulo `HashMap`'s alignment
/// cheaply rejects those cases so downstream callers see safe defaults
/// instead of a process-aborting alignment panic.
fn is_aligned_for_map(p: *mut i8) -> bool {
    let align = std::mem::align_of::<HashMap<i64, i64>>();
    (p as usize) % align == 0 && !looks_like_tagged_value(p as usize)
}

/// Heuristic: addresses below 0x1000 are never valid heap pointers in
/// practice. The codegen can materialize small integer tags (1 = Some,
/// 3 = Other, 0x2c = small index) into a pointer slot when a generic
/// class field is read through the wrong view; treating them as
/// misaligned rejects that whole class of mistakes in one check.
fn looks_like_tagged_value(addr: usize) -> bool {
    addr < 0x1000
}

// ============================================================
// __builtin_set_* — HashSet<i64> implementation
// ============================================================

/// Create a new empty set. Returns an opaque pointer to a boxed HashSet.
#[no_mangle]
pub extern "C" fn __builtin_set_create() -> *mut i8 {
    let set: Box<HashSet<i64>> = Box::default();
    Box::into_raw(set) as *mut i8
}

/// Add an element to the set.
#[no_mangle]
pub extern "C" fn __builtin_set_add(data: *mut i8, value: *mut i8) {
    if data.is_null() {
        return;
    }
    let set = unsafe { &mut *(data as *mut HashSet<i64>) };
    set.insert(value as i64);
}

/// Delete an element from the set. Returns true if the element existed.
#[no_mangle]
pub extern "C" fn __builtin_set_delete(data: *mut i8, value: *mut i8) -> bool {
    if data.is_null() {
        return false;
    }
    let set = unsafe { &mut *(data as *mut HashSet<i64>) };
    set.remove(&(value as i64))
}

/// Check if the set contains an element.
#[no_mangle]
pub extern "C" fn __builtin_set_has(data: *mut i8, value: *mut i8) -> bool {
    if data.is_null() {
        return false;
    }
    let set = unsafe { &*(data as *const HashSet<i64>) };
    set.contains(&(value as i64))
}

// ============================================================
// __string_* — stdlib/string.ry entry points
// ============================================================
// NOTE: All __string_* functions that return *mut i8 allocate memory
// via std::alloc::alloc(). These allocations are NOT freed by the caller.
// Memory tracking will be handled by the GC in a future milestone.
// Until then, frequent string operations will leak memory.
// ============================================================

/// Join array elements with a separator string.
/// Array layout: [len: i64][cap: i64][data: *mut i8 * cap]
/// Each element is treated as a null-terminated string pointer.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn __string_join(arr: *mut i8, separator: *const i8) -> *mut i8 {
    if arr.is_null() {
        return ruyi_string_concat(std::ptr::null(), std::ptr::null());
    }
    unsafe {
        let len = *(arr as *const i64);
        if len == 0 {
            return ruyi_string_concat(std::ptr::null(), std::ptr::null());
        }

        let data = array_data_ptr(arr);
        if data.is_null() {
            return ruyi_string_concat(std::ptr::null(), std::ptr::null());
        }
        let sep_bytes = if separator.is_null() {
            &[]
        } else {
            CStr::from_ptr(separator).to_bytes()
        };

        let mut total: usize = 0;
        for i in 0..len {
            let elem = *data.add(i as usize);
            if elem != 0 {
                total += CStr::from_ptr(elem as *const i8).to_bytes().len();
            }
            if i > 0 {
                total += sep_bytes.len();
            }
        }

        let layout = Layout::from_size_align(total + 1, 1).unwrap();
        let out = alloc(layout) as *mut i8;
        if out.is_null() {
            return std::ptr::null_mut();
        }

        let mut pos = 0usize;
        for i in 0..len {
            let elem = *data.add(i as usize);
            if i > 0 {
                std::ptr::copy_nonoverlapping(
                    sep_bytes.as_ptr(),
                    out.add(pos) as *mut u8,
                    sep_bytes.len(),
                );
                pos += sep_bytes.len();
            }
            if elem != 0 {
                let elem_bytes = CStr::from_ptr(elem as *const i8).to_bytes();
                std::ptr::copy_nonoverlapping(
                    elem_bytes.as_ptr(),
                    out.add(pos) as *mut u8,
                    elem_bytes.len(),
                );
                pos += elem_bytes.len();
            }
        }
        *out.add(pos) = 0;
        out
    }
}

/// Create a string from a single Unicode code point.
#[no_mangle]
pub extern "C" fn __string_from_char_code(code: i64) -> *mut i8 {
    let code = code as u32;
    let mut buf = [0u8; 4];
    let encoded = char::from_u32(code)
        .unwrap_or('\u{FFFD}')
        .encode_utf8(&mut buf);
    let bytes = encoded.as_bytes();
    unsafe {
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

/// Create a string from an array of Unicode code points.
/// Array layout: [len: i64][cap: i64][data: i64 * cap]
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn __string_from_char_codes(arr: *mut i8) -> *mut i8 {
    if arr.is_null() {
        return ruyi_string_concat(std::ptr::null(), std::ptr::null());
    }
    unsafe {
        let len = *(arr as *const i64);
        if len == 0 {
            let layout = Layout::from_size_align(1, 1).unwrap();
            let out = alloc(layout) as *mut i8;
            if !out.is_null() {
                *out = 0;
            }
            return out;
        }

        let data = array_data_ptr(arr);
        if data.is_null() {
            let layout = Layout::from_size_align(1, 1).unwrap();
            let out = alloc(layout) as *mut i8;
            if !out.is_null() {
                *out = 0;
            }
            return out;
        }

        let mut total: usize = 0;
        for i in 0..len {
            let code = *data.add(i as usize) as u32;
            if code <= 0x7F {
                total += 1;
            } else if code <= 0x7FF {
                total += 2;
            } else if code <= 0xFFFF {
                total += 3;
            } else {
                total += 4;
            }
        }

        let layout = Layout::from_size_align(total + 1, 1).unwrap();
        let out = alloc(layout) as *mut i8;
        if out.is_null() {
            return std::ptr::null_mut();
        }

        let mut pos = 0usize;
        let mut buf = [0u8; 4];
        for i in 0..len {
            let code = *data.add(i as usize) as u32;
            let ch = char::from_u32(code).unwrap_or('\u{FFFD}');
            let encoded = ch.encode_utf8(&mut buf);
            let bytes = encoded.as_bytes();
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), out.add(pos) as *mut u8, bytes.len());
            pos += bytes.len();
        }
        *out.add(pos) = 0;
        out
    }
}

/// Replace all occurrences of `pattern` in `input` with `replacement`.
///
/// **Deprecated** (v0.5.9 / R3): the canonical string-substitution FFI is
/// now the bounded-buffer `__string_replace_all` exported from
/// `fmt_ffi.rs` (renamed from `ruyi_string_replace_all`). This 3-arg
/// variant is kept under the `_legacy` suffix for source compatibility
/// with out-of-tree code and for `stdlib/fmt.ry`'s loop-over-args
/// pattern. Plan to delete in v0.6.0.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn __string_replace_all_legacy(
    input: *const i8,
    pattern: *const i8,
    replacement: *const i8,
) -> *mut i8 {
    unsafe {
        if input.is_null() || pattern.is_null() {
            return ruyi_string_concat(std::ptr::null(), std::ptr::null());
        }

        let input_bytes = CStr::from_ptr(input).to_bytes();
        let pattern_bytes = CStr::from_ptr(pattern).to_bytes();
        let replacement_bytes = if replacement.is_null() {
            &[]
        } else {
            CStr::from_ptr(replacement).to_bytes()
        };

        if pattern_bytes.is_empty() {
            let layout = Layout::from_size_align(input_bytes.len() + 1, 1).unwrap();
            let out = alloc(layout) as *mut i8;
            if out.is_null() {
                return std::ptr::null_mut();
            }
            std::ptr::copy_nonoverlapping(input_bytes.as_ptr(), out as *mut u8, input_bytes.len());
            *out.add(input_bytes.len()) = 0;
            return out;
        }

        let mut count = 0usize;
        let mut search_start = 0usize;
        while search_start <= input_bytes.len().saturating_sub(pattern_bytes.len()) {
            if input_bytes[search_start..search_start + pattern_bytes.len()] == *pattern_bytes {
                count += 1;
                search_start += pattern_bytes.len();
            } else {
                search_start += 1;
            }
        }

        let output_size =
            input_bytes.len() + count * replacement_bytes.len().saturating_sub(pattern_bytes.len());
        let layout = Layout::from_size_align(output_size + 1, 1).unwrap();
        let out = alloc(layout) as *mut i8;
        if out.is_null() {
            return std::ptr::null_mut();
        }

        let mut pos = 0usize;
        let mut search_start = 0usize;
        while search_start <= input_bytes.len().saturating_sub(pattern_bytes.len()) {
            if input_bytes[search_start..search_start + pattern_bytes.len()] == *pattern_bytes {
                std::ptr::copy_nonoverlapping(
                    replacement_bytes.as_ptr(),
                    out.add(pos) as *mut u8,
                    replacement_bytes.len(),
                );
                pos += replacement_bytes.len();
                search_start += pattern_bytes.len();
            } else {
                *out.add(pos) = input_bytes[search_start] as i8;
                pos += 1;
                search_start += 1;
            }
        }
        while search_start < input_bytes.len() {
            *out.add(pos) = input_bytes[search_start] as i8;
            pos += 1;
            search_start += 1;
        }
        *out.add(pos) = 0;
        out
    }
}

/// Get the byte length of a null-terminated string.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn __string_length(s: *const i8) -> i64 {
    if s.is_null() {
        return 0;
    }
    unsafe { CStr::from_ptr(s).to_bytes().len() as i64 }
}

/// Check if `haystack` contains `needle`.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn __string_equals(lhs: *const i8, rhs: *const i8) -> bool {
    unsafe {
        if lhs.is_null() && rhs.is_null() {
            return true;
        }
        if lhs.is_null() || rhs.is_null() {
            return false;
        }
        let lhs_bytes = CStr::from_ptr(lhs).to_bytes();
        let rhs_bytes = CStr::from_ptr(rhs).to_bytes();
        lhs_bytes == rhs_bytes
    }
}

/// Check if `haystack` contains `needle`.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn __string_contains(haystack: *const i8, needle: *const i8) -> bool {
    unsafe {
        if haystack.is_null() || needle.is_null() {
            return false;
        }
        let haystack_bytes = CStr::from_ptr(haystack).to_bytes();
        let needle_bytes = CStr::from_ptr(needle).to_bytes();
        if needle_bytes.is_empty() {
            return true;
        }
        haystack_bytes
            .windows(needle_bytes.len())
            .any(|w| w == needle_bytes)
    }
}

/// Check if `s` starts with `prefix`.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn __string_starts_with(s: *const i8, prefix: *const i8) -> bool {
    unsafe {
        if s.is_null() || prefix.is_null() {
            return false;
        }
        let s_bytes = CStr::from_ptr(s).to_bytes();
        let prefix_bytes = CStr::from_ptr(prefix).to_bytes();
        if prefix_bytes.is_empty() {
            return true;
        }
        s_bytes.starts_with(prefix_bytes)
    }
}

/// Check if `s` ends with `suffix`.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn __string_ends_with(s: *const i8, suffix: *const i8) -> bool {
    unsafe {
        if s.is_null() || suffix.is_null() {
            return false;
        }
        let s_bytes = CStr::from_ptr(s).to_bytes();
        let suffix_bytes = CStr::from_ptr(suffix).to_bytes();
        if suffix_bytes.is_empty() {
            return true;
        }
        s_bytes.ends_with(suffix_bytes)
    }
}

/// Find the first index of `needle` in `haystack`. Returns -1 if not found.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn __string_index_of(haystack: *const i8, needle: *const i8) -> i64 {
    unsafe {
        if haystack.is_null() || needle.is_null() {
            return -1;
        }
        let haystack_bytes = CStr::from_ptr(haystack).to_bytes();
        let needle_bytes = CStr::from_ptr(needle).to_bytes();
        if needle_bytes.is_empty() {
            return 0;
        }
        haystack_bytes
            .windows(needle_bytes.len())
            .position(|w| w == needle_bytes)
            .map(|i| i as i64)
            .unwrap_or(-1)
    }
}

/// Find the last index of `needle` in `haystack`. Returns -1 if not found.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn __string_last_index_of(haystack: *const i8, needle: *const i8) -> i64 {
    unsafe {
        if haystack.is_null() || needle.is_null() {
            return -1;
        }
        let haystack_bytes = CStr::from_ptr(haystack).to_bytes();
        let needle_bytes = CStr::from_ptr(needle).to_bytes();
        if needle_bytes.is_empty() {
            return haystack_bytes.len() as i64;
        }
        haystack_bytes
            .windows(needle_bytes.len())
            .rposition(|w| w == needle_bytes)
            .map(|i| i as i64)
            .unwrap_or(-1)
    }
}

/// Return the string unchanged. `toString()` on a string is the identity,
/// but codegen dispatches it through the `__string_*` builtin table like any
/// other string method, so an actual symbol is required.
#[no_mangle]
pub extern "C" fn __string_to_string(s: *const i8) -> *mut i8 {
    s as *mut i8
}

/// Get the character at `index`. Returns a new single-character string.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn __string_char_at(s: *const i8, index: i64) -> *mut i8 {
    unsafe {
        if s.is_null() || index < 0 {
            return ruyi_string_concat(std::ptr::null(), std::ptr::null());
        }
        let s_bytes = CStr::from_ptr(s).to_bytes();
        let s_str = match std::str::from_utf8(s_bytes) {
            Ok(s) => s,
            Err(_) => "",
        };
        let ch = s_str.chars().nth(index as usize);
        match ch {
            Some(c) => {
                let mut buf = [0u8; 4];
                let encoded = c.encode_utf8(&mut buf);
                let bytes = encoded.as_bytes();
                let layout = Layout::from_size_align(bytes.len() + 1, 1).unwrap();
                let out = alloc(layout) as *mut i8;
                if out.is_null() {
                    return std::ptr::null_mut();
                }
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), out as *mut u8, bytes.len());
                *out.add(bytes.len()) = 0;
                out
            }
            None => ruyi_string_concat(std::ptr::null(), std::ptr::null()),
        }
    }
}

/// Get the Unicode code point at `index`. Returns -1 if out of bounds.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn __string_char_code_at(s: *const i8, index: i64) -> i64 {
    unsafe {
        if s.is_null() || index < 0 {
            return -1;
        }
        let s_bytes = CStr::from_ptr(s).to_bytes();
        // Validate UTF-8: invalid bytes panic inside `Chars::next` and
        // abort the process. Defensively fall back to an empty str.
        let s_str = match std::str::from_utf8(s_bytes) {
            Ok(s) => s,
            Err(_) => "",
        };
        s_str
            .chars()
            .nth(index as usize)
            .map(|c| c as u32 as i64)
            .unwrap_or(-1)
    }
}

/// Repeat string `s` `count` times.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn __string_repeat(s: *const i8, count: i64) -> *mut i8 {
    unsafe {
        if s.is_null() || count <= 0 {
            return ruyi_string_concat(std::ptr::null(), std::ptr::null());
        }
        let s_bytes = CStr::from_ptr(s).to_bytes();
        let total = s_bytes.len() * count as usize;
        let layout = Layout::from_size_align(total + 1, 1).unwrap();
        let out = alloc(layout) as *mut i8;
        if out.is_null() {
            return std::ptr::null_mut();
        }
        for i in 0..count {
            std::ptr::copy_nonoverlapping(
                s_bytes.as_ptr(),
                out.add(i as usize * s_bytes.len()) as *mut u8,
                s_bytes.len(),
            );
        }
        *out.add(total) = 0;
        out
    }
}

/// Extract substring from `start` to `end`.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn __string_substring(s: *const i8, start: i64, end: i64) -> *mut i8 {
    unsafe {
        if s.is_null() {
            return ruyi_string_concat(std::ptr::null(), std::ptr::null());
        }
        let s_bytes = CStr::from_ptr(s).to_bytes();
        let s_str = std::str::from_utf8_unchecked(s_bytes);
        let len = s_str.len() as i64;
        let start = if start < 0 {
            0
        } else if start > len {
            len
        } else {
            start
        } as usize;
        let end = if end < 0 {
            0
        } else if end > len {
            len
        } else {
            end
        } as usize;
        let end = if end < start { start } else { end };
        let sub = &s_str[start..end];
        let bytes = sub.as_bytes();
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

/// Convert string to uppercase.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn __string_to_upper_case(s: *const i8) -> *mut i8 {
    unsafe {
        if s.is_null() {
            return ruyi_string_concat(std::ptr::null(), std::ptr::null());
        }
        let s_bytes = CStr::from_ptr(s).to_bytes();
        let s_str = std::str::from_utf8_unchecked(s_bytes);
        let upper: String = s_str.to_uppercase();
        let bytes = upper.into_bytes();
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

/// Convert string to lowercase.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn __string_to_lower_case(s: *const i8) -> *mut i8 {
    unsafe {
        if s.is_null() {
            return ruyi_string_concat(std::ptr::null(), std::ptr::null());
        }
        let s_bytes = CStr::from_ptr(s).to_bytes();
        let s_str = std::str::from_utf8_unchecked(s_bytes);
        let lower: String = s_str.to_lowercase();
        let bytes = lower.into_bytes();
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

/// Trim whitespace from both ends.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn __string_trim(s: *const i8) -> *mut i8 {
    unsafe {
        if s.is_null() {
            return ruyi_string_concat(std::ptr::null(), std::ptr::null());
        }
        let s_bytes = CStr::from_ptr(s).to_bytes();
        let s_str = std::str::from_utf8_unchecked(s_bytes);
        let trimmed = s_str.trim();
        let bytes = trimmed.as_bytes();
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

/// Split string by separator into array.
/// Returns array pointer: [len: i64][cap: i64][data: *mut i8 * cap]
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn __string_split(s: *const i8, separator: *const i8) -> *mut i8 {
    unsafe {
        if s.is_null() {
            return ruyi_array_alloc(0);
        }
        let s_bytes = CStr::from_ptr(s).to_bytes();
        let s_str = std::str::from_utf8_unchecked(s_bytes);
        let sep_bytes = if separator.is_null() {
            &[]
        } else {
            CStr::from_ptr(separator).to_bytes()
        };
        let sep_str = std::str::from_utf8_unchecked(sep_bytes);

        let parts: Vec<&str> = if sep_str.is_empty() {
            // Split into individual characters
            s_str.split("").filter(|s| !s.is_empty()).collect()
        } else {
            s_str.split(sep_str).collect()
        };

        let mut arr = ruyi_array_alloc(parts.len() as i64);
        for part in &parts {
            let bytes = part.as_bytes();
            let layout = Layout::from_size_align(bytes.len() + 1, 1).unwrap();
            let out = alloc(layout) as *mut i8;
            if !out.is_null() {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), out as *mut u8, bytes.len());
                *out.add(bytes.len()) = 0;
            }
            arr = ruyi_array_push(arr, out as i64);
        }
        arr
    }
}

// ============================================================
// __string_* tests
// ============================================================
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
            let data = *(arr.add(std::mem::size_of::<i64>() * 2) as *mut i64) as *mut i8;
            assert!(!data.is_null()); // data_ptr
            let header_layout = Layout::from_size_align(
                std::mem::size_of::<i64>() * 3,
                std::mem::align_of::<i64>(),
            )
            .unwrap();
            let data_layout = Layout::from_size_align(
                5 * std::mem::size_of::<i64>(),
                std::mem::align_of::<i64>(),
            )
            .unwrap();
            dealloc(data as *mut u8, data_layout);
            dealloc(arr as *mut u8, header_layout);
        }
    }

    #[test]
    fn test_ruyi_array_alloc_negative() {
        unsafe {
            let arr = ruyi_array_alloc(-1);
            assert!(!arr.is_null());
            assert_eq!(*(arr as *mut i64), 0); // len
            assert_eq!(*(arr.add(std::mem::size_of::<i64>()) as *mut i64), 0i64); // cap
            assert_eq!(*(arr.add(std::mem::size_of::<i64>() * 2) as *mut i64), 0i64); // data_ptr
            let layout = Layout::from_size_align(
                std::mem::size_of::<i64>() * 3,
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
            let layout =
                Layout::from_size_align(std::mem::size_of::<i64>(), std::mem::align_of::<i64>())
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
            dealloc(result as *mut u8, Layout::from_size_align(21, 1).unwrap());
        }
    }

    #[test]
    fn test_ruyi_bigint_from_str_null() {
        let result = ruyi_bigint_from_str(std::ptr::null());
        assert!(result.is_null());
    }

    #[test]
    fn test_ruyi_bigint_eq_same_value() {
        let s = CString::new("12345678901234567890").unwrap();
        unsafe {
            let a = ruyi_bigint_from_str(s.as_ptr());
            let b = ruyi_bigint_from_str(s.as_ptr());
            assert_eq!(ruyi_bigint_eq(a, b), 1);
            dealloc(a as *mut u8, Layout::from_size_align(21, 1).unwrap());
            dealloc(b as *mut u8, Layout::from_size_align(21, 1).unwrap());
        }
    }

    #[test]
    fn test_ruyi_bigint_eq_different_value() {
        let s1 = CString::new("100").unwrap();
        let s2 = CString::new("200").unwrap();
        unsafe {
            let a = ruyi_bigint_from_str(s1.as_ptr());
            let b = ruyi_bigint_from_str(s2.as_ptr());
            assert_eq!(ruyi_bigint_eq(a, b), 0);
            dealloc(a as *mut u8, Layout::from_size_align(4, 1).unwrap());
            dealloc(b as *mut u8, Layout::from_size_align(4, 1).unwrap());
        }
    }

    #[test]
    fn test_ruyi_bigint_eq_both_null() {
        assert_eq!(
            ruyi_bigint_eq(std::ptr::null_mut(), std::ptr::null_mut()),
            1
        );
    }

    #[test]
    fn test_ruyi_bigint_eq_one_null() {
        let s = CString::new("42").unwrap();
        unsafe {
            let a = ruyi_bigint_from_str(s.as_ptr());
            assert_eq!(ruyi_bigint_eq(a, std::ptr::null_mut()), 0);
            assert_eq!(ruyi_bigint_eq(std::ptr::null_mut(), a), 0);
            dealloc(a as *mut u8, Layout::from_size_align(3, 1).unwrap());
        }
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

    #[test]
    fn test_ruyi_int_to_string() {
        unsafe {
            let result = ruyi_int_to_string(42);
            assert!(!result.is_null());
            let cstr = CStr::from_ptr(result);
            assert_eq!(cstr.to_str().unwrap(), "42");
            dealloc(result as *mut u8, Layout::from_size_align(3, 1).unwrap());
        }
    }

    #[test]
    fn test_ruyi_int_to_string_negative() {
        unsafe {
            let result = ruyi_int_to_string(-123);
            assert!(!result.is_null());
            let cstr = CStr::from_ptr(result);
            assert_eq!(cstr.to_str().unwrap(), "-123");
            dealloc(result as *mut u8, Layout::from_size_align(5, 1).unwrap());
        }
    }

    #[test]
    fn test_ruyi_float_to_string() {
        unsafe {
            let result = ruyi_float_to_string(3.14);
            assert!(!result.is_null());
            let cstr = CStr::from_ptr(result);
            assert_eq!(cstr.to_str().unwrap(), "3.14");
            dealloc(result as *mut u8, Layout::from_size_align(5, 1).unwrap());
        }
    }

    #[test]
    fn test_string_join_basic() {
        let sep = CString::new(", ").unwrap();
        let a = CString::new("hello").unwrap();
        let b = CString::new("world").unwrap();
        unsafe {
            let arr = ruyi_array_alloc(2);
            let data = *(arr.add(std::mem::size_of::<i64>() * 2) as *mut i64) as *mut i64;
            *data.add(0) = a.as_ptr() as i64;
            *data.add(1) = b.as_ptr() as i64;
            *(arr as *mut i64) = 2;

            let result = __string_join(arr, sep.as_ptr());
            assert!(!result.is_null());
            let cstr = CStr::from_ptr(result);
            assert_eq!(cstr.to_str().unwrap(), "hello, world");
            dealloc(result as *mut u8, Layout::from_size_align(13, 1).unwrap());
            dealloc(
                data as *mut u8,
                Layout::from_size_align(
                    2 * std::mem::size_of::<i64>(),
                    std::mem::align_of::<i64>(),
                )
                .unwrap(),
            );
            dealloc(
                arr as *mut u8,
                Layout::from_size_align(
                    std::mem::size_of::<i64>() * 3,
                    std::mem::align_of::<i64>(),
                )
                .unwrap(),
            );
        }
    }

    #[test]
    fn test_string_join_empty_array() {
        let sep = CString::new(",").unwrap();
        unsafe {
            let arr = ruyi_array_alloc(0);
            let result = __string_join(arr, sep.as_ptr());
            assert!(!result.is_null());
            let cstr = CStr::from_ptr(result);
            assert_eq!(cstr.to_str().unwrap(), "");
            dealloc(result as *mut u8, Layout::from_size_align(1, 1).unwrap());
            dealloc(
                arr as *mut u8,
                Layout::from_size_align(
                    std::mem::size_of::<i64>() * 3,
                    std::mem::align_of::<i64>(),
                )
                .unwrap(),
            );
        }
    }

    #[test]
    fn test_string_join_null_array() {
        let sep = CString::new(",").unwrap();
        unsafe {
            let result = __string_join(std::ptr::null_mut(), sep.as_ptr());
            assert!(!result.is_null());
            let cstr = CStr::from_ptr(result);
            assert_eq!(cstr.to_str().unwrap(), "");
            dealloc(result as *mut u8, Layout::from_size_align(1, 1).unwrap());
        }
    }

    #[test]
    fn test_string_from_char_code_basic() {
        unsafe {
            let result = __string_from_char_code(65);
            assert!(!result.is_null());
            let cstr = CStr::from_ptr(result);
            assert_eq!(cstr.to_str().unwrap(), "A");
            dealloc(result as *mut u8, Layout::from_size_align(2, 1).unwrap());
        }
    }

    #[test]
    fn test_string_from_char_code_unicode() {
        unsafe {
            let result = __string_from_char_code(0x1F600);
            assert!(!result.is_null());
            let cstr = CStr::from_ptr(result);
            assert_eq!(cstr.to_str().unwrap(), "😀");
            dealloc(result as *mut u8, Layout::from_size_align(5, 1).unwrap());
        }
    }

    #[test]
    fn test_string_replace_all_basic() {
        let input = CString::new("hello world hello").unwrap();
        let pattern = CString::new("hello").unwrap();
        let replacement = CString::new("hi").unwrap();
        unsafe {
            let result =
                __string_replace_all_legacy(input.as_ptr(), pattern.as_ptr(), replacement.as_ptr());
            assert!(!result.is_null());
            let cstr = CStr::from_ptr(result);
            assert_eq!(cstr.to_str().unwrap(), "hi world hi");
            dealloc(result as *mut u8, Layout::from_size_align(10, 1).unwrap());
        }
    }

    #[test]
    fn test_string_replace_all_no_match() {
        let input = CString::new("hello world").unwrap();
        let pattern = CString::new("xyz").unwrap();
        let replacement = CString::new("abc").unwrap();
        unsafe {
            let result =
                __string_replace_all_legacy(input.as_ptr(), pattern.as_ptr(), replacement.as_ptr());
            assert!(!result.is_null());
            let cstr = CStr::from_ptr(result);
            assert_eq!(cstr.to_str().unwrap(), "hello world");
            dealloc(result as *mut u8, Layout::from_size_align(12, 1).unwrap());
        }
    }

    #[test]
    fn test_string_replace_all_empty_pattern() {
        let input = CString::new("hello").unwrap();
        let pattern = CString::new("").unwrap();
        let replacement = CString::new("x").unwrap();
        unsafe {
            let result =
                __string_replace_all_legacy(input.as_ptr(), pattern.as_ptr(), replacement.as_ptr());
            assert!(!result.is_null());
            let cstr = CStr::from_ptr(result);
            assert_eq!(cstr.to_str().unwrap(), "hello");
            dealloc(result as *mut u8, Layout::from_size_align(6, 1).unwrap());
        }
    }

    #[test]
    fn test_string_length_basic() {
        let s = CString::new("hello").unwrap();
        assert_eq!(__string_length(s.as_ptr()), 5);
    }

    #[test]
    fn test_string_length_empty() {
        let s = CString::new("").unwrap();
        assert_eq!(__string_length(s.as_ptr()), 0);
    }

    #[test]
    fn test_string_length_null() {
        assert_eq!(__string_length(std::ptr::null()), 0);
    }
}
