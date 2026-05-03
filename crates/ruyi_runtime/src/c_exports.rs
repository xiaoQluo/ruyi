use crate::exception::types::ExceptionObject;

#[no_mangle]
pub extern "C" fn ruyi_throw(msg: *const i8) {
    eprintln!("ruyi_throw: {}", unsafe {
        if msg.is_null() {
            "null".to_string()
        } else {
            std::ffi::CStr::from_ptr(msg).to_string_lossy().into_owned()
        }
    });
    std::process::abort();
}

#[no_mangle]
pub extern "C" fn ruyi_begin_catch(exc: *mut u8) -> *mut ExceptionObject {
    unsafe { crate::exception::runtime::ruyi_begin_catch(exc) }
}

#[no_mangle]
pub extern "C" fn ruyi_end_catch() {
    crate::exception::runtime::ruyi_end_catch();
}
