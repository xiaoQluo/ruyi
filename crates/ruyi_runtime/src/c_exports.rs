use crate::exception::types::ExceptionObject;
use std::alloc::{alloc, Layout};
use std::ffi::CStr;
use std::sync::atomic::{AtomicPtr, Ordering};

/// Deprecated pending exception pointer.
///
/// Since `ruyi_throw` now connects to `_Unwind_RaiseException`, exceptions
/// no longer flow through this atomic pointer.  Retained for ABI compatibility
/// — codegen still emits calls to `ruyi_get_pending_exception` /
/// `ruyi_clear_pending_exception` in legacy `call`-based code paths.
static PENDING_EXCEPTION: AtomicPtr<i8> = AtomicPtr::new(std::ptr::null_mut());

#[no_mangle]
pub extern "C" fn ruyi_throw(msg: *const i8) {
    unsafe {
        let c_str = CStr::from_ptr(msg);
        let message = c_str.to_string_lossy().into_owned();
        let exc = crate::exception::RuyiException::new(
            crate::exception::builtin_type_ids::ERROR,
            message,
        );
        crate::exception::throw_exception(exc);
    }
}

#[no_mangle]
pub extern "C" fn ruyi_clear_pending_exception() {
    PENDING_EXCEPTION.store(std::ptr::null_mut(), Ordering::SeqCst);
}

#[no_mangle]
pub extern "C" fn ruyi_get_pending_exception() -> *const i8 {
    PENDING_EXCEPTION.load(Ordering::SeqCst) as *const i8
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

/// Stub allocator for `--gc=stub` mode.
///
/// 接收字节数，返回对齐的内存指针。**不**追踪、不回收、**不**调用任何 GC。
/// 等价语义：在程序运行期间不真正回收内存，仅作占位实现。
///
/// 命名约定：`cc_alloc` 是 "C compiler alloc" 的缩写，对应 LLVM IR 中的
/// `declare i8* @cc_alloc(i64)`。Codegen 在 `--gc=stub` 模式 emit
/// `call i8* @cc_alloc(i64 %size)`，链接器从 `libruyi_runtime.a` 解析此符号。
///
/// 与 `--gc=real` 模式共存于同一 `libruyi_runtime.a`：链接时由 IR 中
/// 实际引用的符号决定哪个函数被拉入二进制，互不干扰。
///
/// # ABI contract
///
/// - `size == 0` → 返回 null（与 `malloc(0)` 语义一致）。
/// - 否则返回 `std::alloc::alloc` 分配出的 8 字节对齐指针，**不**调用
///   `ruyi_gc_alloc`（避免循环依赖；stub 模式下不走 GC）。
/// - 不进行任何 GC bookkeeping / write barrier / 类型元数据登记。
/// - 调用方负责在不再使用该内存时通过 `cc_alloc` 自身的语义（或外部
///   进程退出）来释放；本函数不提供配套的 `cc_free`，对应 LLVM IR
///   中也未引用 `cc_dealloc` / `cc_realloc`。
///
/// @author luozegang
/// @date 2026-07-10
#[no_mangle]
pub extern "C" fn cc_alloc(size: u64) -> *mut u8 {
    if size == 0 {
        return std::ptr::null_mut();
    }
    let layout = match std::alloc::Layout::from_size_align(size as usize, 8) {
        Ok(l) => l,
        Err(_) => return std::ptr::null_mut(),
    };
    unsafe { std::alloc::alloc(layout) }
}
