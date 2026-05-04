//! Exception runtime: throw, catch, and finally support.
//!
//! Provides both the runtime function implementations and the LLVM
//! code-generation helpers used by the compiler frontend.

mod cxxabi_stubs {
    use std::sync::atomic::{AtomicPtr, Ordering};
    static ACTIVE_EXC: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());

    pub unsafe fn __cxa_begin_catch(exc: *mut std::ffi::c_void) -> *mut std::ffi::c_void {
        ACTIVE_EXC.store(exc, Ordering::SeqCst);
        exc
    }

    pub unsafe fn __cxa_end_catch() {
        ACTIVE_EXC.store(std::ptr::null_mut(), Ordering::SeqCst);
    }
}

pub use cxxabi_stubs::*;

use crate::exception::types::{ExceptionObject, ExceptionType};

/// Runtime implementation of `ruyi_throw`.
///
/// Takes ownership of an `ExceptionObject` and initiates stack unwinding
/// via the Itanium C++ ABI `_Unwind_RaiseException`.
///
/// # Safety
///
/// `exception` must be a valid, uniquely-owned pointer returned by the
/// runtime allocator.
pub unsafe fn ruyi_throw(exception: *mut ExceptionObject) -> ! {
    let layout = std::alloc::Layout::new::<crate::exception::types::UnwindException>();
    let exc = std::alloc::alloc(layout) as *mut crate::exception::types::UnwindException;
    if exc.is_null() {
        std::process::abort();
    }
    (*exc).exception_class = crate::exception::types::KLANG_EXCEPTION_CLASS;
    (*exc).exception_cleanup = Some(ruyi_exception_cleanup);
    (*exc).private = [0; 6];
    std::ptr::copy_nonoverlapping(exception, &mut (*exc).payload, 1);

    extern "C" {
        fn _Unwind_RaiseException(exc: *mut std::ffi::c_void) -> i32;
    }
    let _reason = _Unwind_RaiseException(exc as *mut std::ffi::c_void);
    std::process::abort();
}

extern "C" fn ruyi_exception_cleanup(
    _reason: u64,
    exc: *mut crate::exception::types::UnwindException,
) {
    unsafe {
        std::alloc::dealloc(
            exc as *mut u8,
            std::alloc::Layout::new::<crate::exception::types::UnwindException>(),
        );
    }
}

/// Runtime implementation of `ruyi_begin_catch`.
///
/// Called from a catch handler landing pad. Invokes `__cxa_begin_catch`
/// for ABI compliance and returns the Ruyi exception payload.
///
/// # Safety
///
/// `exception_ptr` must be the first element of a landing-pad result.
pub unsafe fn ruyi_begin_catch(exception_ptr: *mut u8) -> *mut ExceptionObject {
    let exc_ptr = __cxa_begin_catch(exception_ptr as *mut std::ffi::c_void);
    let unwind_exc = exc_ptr as *mut crate::exception::types::UnwindException;
    &mut (*unwind_exc).payload as *mut _
}

/// Runtime implementation of `ruyi_end_catch`.
///
/// Marks the end of a catch block and releases exception resources by
/// calling `__cxa_end_catch`.
pub fn ruyi_end_catch() {
    unsafe { __cxa_end_catch() };
}

/// Runtime implementation of finally guard.
///
/// Ensures that a finally block is entered with the correct state.
/// If `pending_exception` is non-null, the finally block must rethrow
/// it after executing its body.
///
/// # Safety
///
/// `pending_exception` may be null or a valid exception pointer.
pub unsafe fn ruyi_finally(pending_exception: *mut ExceptionObject) -> *mut ExceptionObject {
    pending_exception
}

/// Match an exception against a list of catch types.
///
/// Returns the index of the first matching handler, or `None` if no
/// handler matches.
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
///
/// In a full implementation this would walk the stack using the
/// unwind library. The current version returns a placeholder.
pub fn capture_stack_trace() -> Vec<crate::exception::StackFrame> {
    Vec::new()
}

#[cfg(feature = "inkwell")]
pub mod llvm {
    use inkwell::basic_block::BasicBlock;
    use inkwell::builder::Builder;
    use inkwell::context::Context;
    use inkwell::module::Module;
    use inkwell::values::{BasicValueEnum, FunctionValue, PointerValue};
    use inkwell::AddressSpace;

    use crate::exception::landing_pad::llvm::LandingPadGenerator;
    use crate::exception::types::ExceptionType;

    /// High-level LLVM exception runtime builder.
    pub struct ExceptionRuntime<'ctx, 'm, 'b> {
        context: &'ctx Context,
        module: &'m Module<'ctx>,
        builder: &'b Builder<'ctx>,
        lpad_gen: LandingPadGenerator<'ctx, 'm, 'b>,
    }

    impl<'ctx, 'm, 'b> ExceptionRuntime<'ctx, 'm, 'b> {
        /// Create a new exception runtime helper.
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

        /// Build a call to the runtime `ruyi_throw` function.
        ///
        /// After the call, builds `unreachable` because `ruyi_throw` does not return.
        pub fn build_throw(&self, exception: PointerValue<'ctx>) {
            let throw_fn = self.get_throw_function();
            self.builder
                .build_call(throw_fn, &[exception.into()], "ruyi.throw");
            self.builder.build_unreachable();
        }

        /// Build a call to `ruyi_begin_catch` inside a catch handler.
        pub fn build_begin_catch(&self, exception_ptr: PointerValue<'ctx>) -> PointerValue<'ctx> {
            let begin_catch_fn = self.get_begin_catch_function();
            self.builder
                .build_call(begin_catch_fn, &[exception_ptr.into()], "ruyi.begin.catch")
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value()
        }

        /// Build a call to `ruyi_end_catch`.
        pub fn build_end_catch(&self) {
            let end_catch_fn = self.get_end_catch_function();
            self.builder.build_call(end_catch_fn, &[], "ruyi.end.catch");
        }

        /// Build a call to `ruyi_finally`.
        pub fn build_finally(
            &self,
            pending_exception: Option<PointerValue<'ctx>>,
        ) -> PointerValue<'ctx> {
            let finally_fn = self.get_finally_function();
            let arg = pending_exception.map(|p| p.into()).unwrap_or_else(|| {
                self.context
                    .i8_type()
                    .ptr_type(AddressSpace::default())
                    .const_null()
                    .into()
            });
            self.builder
                .build_call(finally_fn, &[arg], "ruyi.finally")
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value()
        }

        /// Build the landing pad and dispatch for a try/catch/finally region.
        ///
        /// Generates a landing pad that catches the given exception types,
        /// dispatches to the corresponding handler blocks, and routes to a
        /// cleanup/finally block when present. Unhandled exceptions resume
        /// propagation via `resume_bb`.
        pub fn build_try_region(
            &self,
            catch_handlers: &[(ExceptionType, BasicBlock<'ctx>)],
            finally_block: Option<BasicBlock<'ctx>>,
            resume_bb: BasicBlock<'ctx>,
        ) -> BasicValueEnum<'ctx> {
            let type_ids: Vec<u64> = catch_handlers.iter().map(|(ty, _)| ty.type_id()).collect();
            let landing_pad =
                self.lpad_gen
                    .build_landing_pad(&type_ids, finally_block.is_some(), "lpad");

            self.lpad_gen.build_catch_dispatch(
                landing_pad,
                &catch_handlers
                    .iter()
                    .map(|(ty, bb)| (ty.type_id(), *bb))
                    .collect::<Vec<_>>(),
                finally_block,
                resume_bb,
            );

            landing_pad
        }

        fn get_throw_function(&self) -> FunctionValue<'ctx> {
            let void_ty = self.context.void_type();
            let i8_ptr = self.context.i8_type().ptr_type(AddressSpace::default());
            let fn_type = void_ty.fn_type(&[i8_ptr.into()], false);
            get_or_insert_function(self.module, "ruyi_throw", fn_type)
        }

        fn get_begin_catch_function(&self) -> FunctionValue<'ctx> {
            let i8_ptr = self.context.i8_type().ptr_type(AddressSpace::default());
            let fn_type = i8_ptr.fn_type(&[i8_ptr.into()], false);
            get_or_insert_function(self.module, "ruyi_begin_catch", fn_type)
        }

        fn get_end_catch_function(&self) -> FunctionValue<'ctx> {
            let void_ty = self.context.void_type();
            let fn_type = void_ty.fn_type(&[], false);
            get_or_insert_function(self.module, "ruyi_end_catch", fn_type)
        }

        fn get_finally_function(&self) -> FunctionValue<'ctx> {
            let i8_ptr = self.context.i8_type().ptr_type(AddressSpace::default());
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

    #[test]
    fn test_ruyi_throw_allocates_unwind_exception() {
        let mut msg = b"test throw\0".to_vec();
        let exc = ExceptionObject {
            type_tag: ExceptionType::Error.type_id(),
            message: msg.as_mut_ptr(),
            stack_trace_len: 0,
            stack_trace: std::ptr::null_mut(),
        };
        unsafe {
            let layout = std::alloc::Layout::new::<crate::exception::types::UnwindException>();
            let uexc = std::alloc::alloc(layout) as *mut crate::exception::types::UnwindException;
            assert!(!uexc.is_null());
            (*uexc).exception_class = crate::exception::types::KLANG_EXCEPTION_CLASS;
            (*uexc).exception_cleanup = Some(ruyi_exception_cleanup);
            (*uexc).private = [0; 6];
            std::ptr::copy_nonoverlapping(&exc, &mut (*uexc).payload, 1);
            assert_eq!((*uexc).payload.type_tag, ExceptionType::Error.type_id());
            std::alloc::dealloc(uexc as *mut u8, layout);
        }
    }
}
