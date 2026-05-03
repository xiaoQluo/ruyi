use std::cell::RefCell;
use std::ffi::CStr;
use std::alloc::{alloc, Layout};
use crate::exception::types::ExceptionObject;

thread_local! {
    static PENDING_EXCEPTION: RefCell<*const i8> = RefCell::new(std::ptr::null());
}

#[no_mangle]
pub extern "C" fn ruyi_throw(msg: *const i8) {
    PENDING_EXCEPTION.with(|e| {
        *e.borrow_mut() = msg;
    });
}

#[no_mangle]
pub extern "C" fn ruyi_get_pending_exception() -> *const i8 {
    PENDING_EXCEPTION.with(|e| *e.borrow())
}

#[no_mangle]
pub extern "C" fn ruyi_clear_pending_exception() {
    PENDING_EXCEPTION.with(|e| {
        let old = *e.borrow();
        *e.borrow_mut() = std::ptr::null();
        eprintln!("clear_pending: old={:?}", old);
    });
}

#[no_mangle]
pub extern "C" fn ruyi_get_pending_exception() -> *const i8 {
    let val = PENDING_EXCEPTION.with(|e| *e.borrow());
    eprintln!("get_pending: val={:?}", val);
    val
}

#[no_mangle]
pub extern "C" fn ruyi_str_concat(a: *const i8, b: *const i8) -> *mut i8 {
    unsafe {
        if a.is_null() || b.is_null() {
            return std::ptr::null_mut();
        }
        let str_a = CStr::from_ptr(a).to_bytes();
        let str_b = CStr::from_ptr(b).to_bytes();
        let total = str_a.len() + str_b.len() + 1;
        let layout = Layout::from_size_align(total, 1).unwrap();
        let out = alloc(layout) as *mut i8;
        if out.is_null() {
            return std::ptr::null_mut();
        }
        std::ptr::copy_nonoverlapping(str_a.as_ptr(), out as *mut u8, str_a.len());
        std::ptr::copy_nonoverlapping(str_b.as_ptr(), out.add(str_a.len()) as *mut u8, str_b.len());
        *out.add(str_a.len() + str_b.len()) = 0;
        out
    }
}

#[no_mangle]
pub extern "C" fn ruyi_begin_catch(exc: *mut u8) -> *mut ExceptionObject {
    unsafe { crate::exception::runtime::ruyi_begin_catch(exc) }
}

#[no_mangle]
pub extern "C" fn ruyi_end_catch() {
    crate::exception::runtime::ruyi_end_catch();
}
