use crate::exception::builtin_type_ids;
use crate::exception::TypeId;
use std::alloc::{alloc, Layout};
use std::ffi::CStr;

/// EX-H3: Map class name to builtin type ID.
///
/// Returns `builtin_type_ids::ERROR` for unknown class names (safe fallback).
///
/// @author Ruyi Team
/// @date 2026-07-26
pub fn class_name_to_type_id(name: &str) -> TypeId {
    match name {
        "Error" => builtin_type_ids::ERROR,
        "TypeError" => builtin_type_ids::TYPE_ERROR,
        "RangeError" => builtin_type_ids::RANGE_ERROR,
        "RuntimeError" => builtin_type_ids::RUNTIME_ERROR,
        "LogicError" => builtin_type_ids::LOGIC_ERROR,
        "AssertionError" => builtin_type_ids::ASSERTION_ERROR,
        "ArgumentError" => builtin_type_ids::ARGUMENT_ERROR,
        "NullError" => builtin_type_ids::NULL_ERROR,
        "ArithmeticError" => builtin_type_ids::ARITHMETIC_ERROR,
        "IteratorError" => builtin_type_ids::ITERATOR_ERROR,
        "ParseError" => builtin_type_ids::PARSE_ERROR,
        "NullAssertionError" => builtin_type_ids::NULL_ASSERTION_ERROR,
        "IOError" => builtin_type_ids::IO_ERROR,
        _ => builtin_type_ids::ERROR,
    }
}

/// EX-H3: Throw with explicit type ID.
///
/// Stores the exception in the global pending-exception slot and returns.
/// The generated code checks the pending exception and dispatches to the
/// matching catch handler.
///
/// @author Ruyi Team
/// @date 2026-07-26
#[no_mangle]
pub extern "C" fn ruyi_throw_with_type(type_id: u64, msg: *const i8) {
    unsafe {
        let c_str = CStr::from_ptr(msg);
        let message = c_str.to_string_lossy().into_owned();
        let exc = crate::exception::RuyiException::new(type_id, message);
        crate::exception::throw_exception(exc);
    }
}

#[no_mangle]
pub extern "C" fn ruyi_throw_typed(class_name: *const i8, msg: *const i8) {
    unsafe {
        let name = CStr::from_ptr(class_name).to_string_lossy();
        let message = CStr::from_ptr(msg).to_string_lossy().into_owned();
        let type_id = class_name_to_type_id(&name);
        let exc = crate::exception::RuyiException::new(type_id, message);
        crate::exception::throw_exception(exc);
    }
}

/// Rethrow an exception object captured by a landing pad.
///
/// Stores the exception in the global pending-exception slot and returns.
#[no_mangle]
pub extern "C" fn ruyi_rethrow(exc_ptr: *const i8) {
    let exc = crate::exception::RuyiException::new(
        builtin_type_ids::ERROR,
        format!("rethrown exception at {:p}", exc_ptr),
    );
    crate::exception::throw_exception(exc);
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
