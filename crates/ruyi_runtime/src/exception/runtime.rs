//! Exception runtime: throw, catch, and finally support.
//!
//! Uses a global pending-exception model. `ruyi_throw*` functions store
//! the exception in a global slot and return normally; the generated code
//! checks the slot after potentially-throwing operations and dispatches to
//! the matching catch handler. This avoids `setjmp`/`longjmp`, which are
//! unreliable when crossing Rust `extern "C"` frames on macOS.

use crate::exception::types::{ExceptionObject, ExceptionType};
use std::sync::atomic::{AtomicPtr, Ordering};

// ── Global exception state ────────────────────────────────────

/// Represents a single active try/catch frame.
#[derive(Debug)]
pub struct ExceptionFrame {
    /// Exception types this frame's catch clauses accept.
    pub catch_types: Vec<ExceptionType>,
    /// `true` if this frame has a `finally` clause.
    pub has_finally: bool,
    /// Whether this frame was already consumed (caught an exception).
    pub consumed: bool,
}

/// Active try/catch frames, innermost last.
static mut EXCEPTION_STACK: Vec<ExceptionFrame> = Vec::new();

/// Currently thrown exception (set by throw functions, read by dispatch).
static PENDING_EXCEPTION: AtomicPtr<ExceptionObject> = AtomicPtr::new(std::ptr::null_mut());

/// Enter a try block. Pushes a frame onto the exception stack and
/// returns 0. The generated code uses `ruyi_find_handler` to dispatch
/// when a pending exception is detected.
#[no_mangle]
pub unsafe extern "C" fn ruyi_try_begin(
    catch_type_ids: *const u64,
    catch_type_count: usize,
    _handler_indices: *const u64,
    _handler_count: usize,
    has_finally: bool,
) -> i32 {
    let catch_types: Vec<ExceptionType> = if catch_type_ids.is_null() || catch_type_count == 0 {
        vec![ExceptionType::Error]
    } else {
        std::slice::from_raw_parts(catch_type_ids, catch_type_count)
            .iter()
            .filter_map(|&id| ExceptionType::from_type_id(id))
            .collect()
    };

    let frame = ExceptionFrame {
        catch_types,
        has_finally,
        consumed: false,
    };

    (&raw mut EXCEPTION_STACK).as_mut().unwrap().push(frame);
    0
}

/// Exit a try block normally (no exception thrown). Pops the exception
/// frame from the stack.
#[no_mangle]
pub unsafe extern "C" fn ruyi_try_end() {
    if let Some(frame) = (&raw mut EXCEPTION_STACK).as_mut().unwrap().pop() {
        let _ = frame;
    }
}

/// Throw an exception. Stores the exception in the global pending slot
/// and returns normally. The caller must check the pending exception
/// and dispatch to the appropriate handler.
///
/// # Safety
///
/// `exception` must be a valid, uniquely-owned `ExceptionObject` pointer.
#[no_mangle]
pub unsafe extern "C" fn ruyi_throw(exception: *mut ExceptionObject) {
    PENDING_EXCEPTION.store(exception, Ordering::SeqCst);
}

/// Get the pending exception object (for use by catch handlers).
#[no_mangle]
pub extern "C" fn ruyi_get_pending_exception() -> *mut ExceptionObject {
    PENDING_EXCEPTION.load(Ordering::SeqCst)
}

/// Clear the pending exception (mark it as caught).
#[no_mangle]
pub extern "C" fn ruyi_clear_pending_exception() {
    PENDING_EXCEPTION.store(std::ptr::null_mut(), Ordering::SeqCst);
}

/// Find the index of the first catch clause that matches `exception`.
///
/// Returns a 0-based handler index, or `-1` if no clause matches.
/// A `catch_type_id` of `u64::MAX` (catch-all) matches any exception;
/// `Error` matches all built-in subtypes; otherwise an exact match is
/// required.
#[no_mangle]
pub unsafe extern "C" fn ruyi_find_handler(
    exception: *mut ExceptionObject,
    catch_type_ids: *const u64,
    catch_type_count: usize,
) -> i64 {
    if exception.is_null() || catch_type_ids.is_null() || catch_type_count == 0 {
        return -1;
    }

    let thrown_type =
        ExceptionType::from_type_id((*exception).type_tag).unwrap_or(ExceptionType::Error);
    let ids = std::slice::from_raw_parts(catch_type_ids, catch_type_count);

    for (i, &id) in ids.iter().enumerate() {
        // Codegen stores 32-bit `TryTypeId` values widened to i64; accept
        // both widths so catch-all keeps matching regardless of which
        // sentinel the caller used.
        if id == u64::MAX || id == ruyi_exception::landing_pad::CATCH_ALL_TYPE_ID as u64 {
            return i as i64;
        }
        if let Some(catch_ty) = ExceptionType::from_type_id(id) {
            if catch_ty.matches(thrown_type) {
                return i as i64;
            }
        }
    }

    -1
}

/// Mark the innermost non-consumed exception frame as consumed.
#[no_mangle]
pub unsafe extern "C" fn ruyi_mark_frame_consumed() {
    if let Some(frame) = (&raw mut EXCEPTION_STACK)
        .as_mut()
        .unwrap()
        .iter_mut()
        .rev()
        .find(|f| !f.consumed)
    {
        frame.consumed = true;
    }
}

/// Compatibility stub for the old `ruyi_begin_catch` interface.
#[no_mangle]
pub unsafe extern "C" fn ruyi_begin_catch(_exception_ptr: *mut u8) -> *mut ExceptionObject {
    ruyi_get_pending_exception()
}

/// Compatibility stub for the old `ruyi_end_catch` interface.
#[no_mangle]
pub extern "C" fn ruyi_end_catch() {
    ruyi_clear_pending_exception();
}

/// Compatibility stub for the old `ruyi_finally` interface.
#[no_mangle]
pub unsafe extern "C" fn ruyi_finally(
    pending_exception: *mut ExceptionObject,
) -> *mut ExceptionObject {
    pending_exception
}

/// Match an exception against a list of catch types.
pub fn ruyi_match_exception(
    exception: &ExceptionObject,
    catch_types: &[ExceptionType],
) -> Option<usize> {
    let thrown = ExceptionType::from_type_id(exception.type_tag)?;
    for (idx, catch_ty) in catch_types.iter().enumerate() {
        if catch_ty.matches(thrown) {
            return Some(idx);
        }
    }
    None
}

/// Capture a stack trace for the current call stack.
pub fn capture_stack_trace() -> Vec<crate::exception::StackFrame> {
    use std::backtrace::Backtrace;

    let bt = Backtrace::capture();
    let bt_str = format!("{}", bt);

    let mut frames = Vec::new();

    let lines: Vec<&str> = bt_str.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();
        if let Some(rest) = line.strip_prefix("  ") {
            let rest = rest.trim();
            if let Some(colon_pos) = rest.find(": ") {
                let func_name = rest[colon_pos + 2..].trim();
                if !func_name.contains("capture_stack_trace")
                    && !func_name.contains("throw_exception")
                    && !func_name.contains("ruyi_throw")
                {
                    let mut file = String::new();
                    let mut line_num = 0u32;
                    if i + 1 < lines.len() {
                        let loc = lines[i + 1].trim();
                        if let Some(at_rest) = loc.strip_prefix("at ") {
                            let at_rest = at_rest.trim();
                            if let Some(last_colon) = at_rest.rfind(':') {
                                file = at_rest[..last_colon].to_string();
                                line_num = at_rest[last_colon + 1..].parse().unwrap_or(0);
                            }
                            i += 1;
                        }
                    }
                    frames.push(crate::exception::StackFrame::new(
                        func_name,
                        if file.is_empty() { "<unknown>" } else { &file },
                        line_num,
                    ));
                }
            }
        }
        i += 1;
    }

    frames.truncate(64);
    frames
}

#[cfg(feature = "inkwell")]
pub mod llvm {
    use inkwell::basic_block::BasicBlock;
    use inkwell::builder::Builder;
    use inkwell::context::Context;
    use inkwell::module::Module;
    use inkwell::values::{BasicValueEnum, FunctionValue, PointerValue};
    use inkwell::AddressSpace;

    use crate::exception::landing_pad::{LandingPadGenerator, TryTypeId};
    use crate::exception::types::ExceptionType;

    pub struct ExceptionRuntime<'ctx, 'm, 'b> {
        context: &'ctx Context,
        module: &'m Module<'ctx>,
        builder: &'b Builder<'ctx>,
        lpad_gen: LandingPadGenerator<'ctx, 'm, 'b>,
    }

    impl<'ctx, 'm, 'b> ExceptionRuntime<'ctx, 'm, 'b> {
        pub fn new(
            context: &'ctx Context,
            module: &'m Module<'ctx>,
            builder: &'b Builder<'ctx>,
        ) -> Self {
            let lpad_gen = LandingPadGenerator::new(context, module, builder);
            let this = Self {
                context,
                module,
                builder,
                lpad_gen,
            };
            this.get_throw_function();
            this.get_begin_catch_function();
            this.get_end_catch_function();
            this.get_finally_function();
            this
        }

        pub fn build_throw(&self, exception: PointerValue<'ctx>) {
            let throw_fn = self.get_throw_function();
            self.builder
                .build_call(throw_fn, &[exception.into()], "ruyi.throw")
                .unwrap();
        }

        pub fn build_begin_catch(&self, exception_ptr: PointerValue<'ctx>) -> PointerValue<'ctx> {
            let begin_catch_fn = self.get_begin_catch_function();
            self.builder
                .build_call(begin_catch_fn, &[exception_ptr.into()], "ruyi.begin.catch")
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic()
                .into_pointer_value()
        }

        pub fn build_end_catch(&self) {
            let end_catch_fn = self.get_end_catch_function();
            self.builder
                .build_call(end_catch_fn, &[], "ruyi.end.catch")
                .unwrap();
        }

        pub fn build_finally(
            &self,
            pending_exception: Option<PointerValue<'ctx>>,
        ) -> PointerValue<'ctx> {
            let finally_fn = self.get_finally_function();
            let arg = pending_exception.map(|p| p.into()).unwrap_or_else(|| {
                self.context
                    .ptr_type(AddressSpace::default())
                    .const_null()
                    .into()
            });
            self.builder
                .build_call(finally_fn, &[arg], "ruyi.finally")
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic()
                .into_pointer_value()
        }

        pub fn build_try_region(
            &self,
            catch_handlers: &[(ExceptionType, BasicBlock<'ctx>)],
            finally_block: Option<BasicBlock<'ctx>>,
            resume_bb: BasicBlock<'ctx>,
        ) -> BasicValueEnum<'ctx> {
            let type_ids: Vec<TryTypeId> = catch_handlers
                .iter()
                .map(|(ty, _)| ty.type_id() as TryTypeId)
                .collect();
            let landing_pad =
                self.lpad_gen
                    .build_landing_pad(&type_ids, finally_block.is_some(), "lpad");

            self.lpad_gen.build_catch_dispatch(
                landing_pad,
                &catch_handlers
                    .iter()
                    .map(|(ty, bb)| (ty.type_id() as TryTypeId, *bb))
                    .collect::<Vec<_>>(),
                finally_block,
                resume_bb,
            );

            landing_pad
        }

        fn get_throw_function(&self) -> FunctionValue<'ctx> {
            let void_ty = self.context.void_type();
            let i8_ptr = self.context.ptr_type(AddressSpace::default());
            let fn_type = void_ty.fn_type(&[i8_ptr.into()], false);
            get_or_insert_function(self.module, "ruyi_throw", fn_type)
        }

        fn get_begin_catch_function(&self) -> FunctionValue<'ctx> {
            let i8_ptr = self.context.ptr_type(AddressSpace::default());
            let fn_type = i8_ptr.fn_type(&[i8_ptr.into()], false);
            get_or_insert_function(self.module, "ruyi_begin_catch", fn_type)
        }

        fn get_end_catch_function(&self) -> FunctionValue<'ctx> {
            let void_ty = self.context.void_type();
            let fn_type = void_ty.fn_type(&[], false);
            get_or_insert_function(self.module, "ruyi_end_catch", fn_type)
        }

        fn get_finally_function(&self) -> FunctionValue<'ctx> {
            let i8_ptr = self.context.ptr_type(AddressSpace::default());
            let fn_type = i8_ptr.fn_type(&[i8_ptr.into()], false);
            get_or_insert_function(self.module, "ruyi_finally", fn_type)
        }
    }

    fn get_or_insert_function<'ctx>(
        module: &inkwell::module::Module<'ctx>,
        name: &str,
        fn_type: inkwell::types::FunctionType<'ctx>,
    ) -> FunctionValue<'ctx> {
        module
            .get_function(name)
            .unwrap_or_else(|| module.add_function(name, fn_type, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Codegen widens the 32-bit `CATCH_ALL_TYPE_ID` into an i64 slot; the
    /// selector must recognise that sentinel (regression: it only matched
    /// `u64::MAX`, so every `catch (e)` clause silently failed to match).
    #[test]
    fn test_ruyi_find_handler_accepts_u32_catch_all() {
        let exc = ExceptionObject {
            type_tag: ExceptionType::TypeError.type_id(),
            message: std::ptr::null_mut(),
            stack_trace_len: 0,
            stack_trace: std::ptr::null_mut(),
        };
        let ids = [ruyi_exception::landing_pad::CATCH_ALL_TYPE_ID as u64];
        unsafe {
            assert_eq!(
                ruyi_find_handler(&exc as *const _ as *mut _, ids.as_ptr(), ids.len()),
                0
            );
        }
    }

    #[test]
    fn test_ruyi_match_exception() {
        let exc = ExceptionObject {
            type_tag: ExceptionType::TypeError.type_id(),
            message: std::ptr::null_mut(),
            stack_trace_len: 0,
            stack_trace: std::ptr::null_mut(),
        };

        assert_eq!(
            ruyi_match_exception(&exc, &[ExceptionType::Error, ExceptionType::TypeError]),
            Some(0)
        );

        assert_eq!(
            ruyi_match_exception(&exc, &[ExceptionType::RangeError, ExceptionType::TypeError]),
            Some(1)
        );

        assert_eq!(
            ruyi_match_exception(&exc, &[ExceptionType::RangeError]),
            None
        );
    }

    #[test]
    fn test_ruyi_finally_preserves_exception() {
        let exc = ExceptionObject {
            type_tag: ExceptionType::Error.type_id(),
            message: std::ptr::null_mut(),
            stack_trace_len: 0,
            stack_trace: std::ptr::null_mut(),
        };

        unsafe {
            let result = ruyi_finally(&exc as *const _ as *mut _);
            assert!(!result.is_null());
            assert_eq!((*result).type_tag, ExceptionType::Error.type_id());
        }
    }

    #[test]
    fn test_ruyi_finally_with_null() {
        unsafe {
            let result = ruyi_finally(std::ptr::null_mut());
            assert!(result.is_null());
        }
    }

    #[test]
    fn test_ruyi_end_catch_does_not_panic() {
        ruyi_end_catch();
    }
}
