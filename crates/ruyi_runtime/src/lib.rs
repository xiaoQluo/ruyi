pub mod alloc;
pub mod arc;
pub mod async_exports;
pub mod async_gc_roots;
pub mod async_runtime;
pub mod atomic_ffi;
pub mod builtins;
mod c_exports;
pub mod compress_ffi;
pub mod crypto_ffi;
pub mod exception;
pub mod fmt_ffi;
pub mod float_ffi;
pub mod gc;
pub mod gc_exports;
pub mod io_ffi;
pub mod json_ffi;
pub mod math_ffi;
pub mod mutex_ffi;
pub mod net_ffi;
pub mod path_ffi;
pub mod process_ffi;
pub mod random_ffi;
pub mod time_ffi;
pub mod tls_ffi;

pub use alloc::{
    allocate, deallocate, reallocate, ruyi_alloc, ruyi_dealloc, ruyi_realloc, GcObjectHeader, Heap,
    MemoryStrategy, TypeInfo,
};
pub use arc::{
    ruyi_arc_alloc, ruyi_arc_ref_count, ruyi_arc_release, ruyi_arc_retain, ruyi_arc_weak,
    ruyi_arc_weak_drop, ruyi_arc_weak_load, ruyi_is_arc, ruyi_is_gc, ruyi_release_any,
    ruyi_retain_any, CycleDetector, WeakRef, WeakTable,
};
pub use async_runtime::{
    register_async_roots, ruyi_await, JoinAll, Poll, Race, RuyiFuture, Scheduler, Task, TaskId,
    Waker, WorkStealingDeque, GLOBAL_SCHEDULER,
};
pub use builtins::{
    ruyi_array_alloc, ruyi_array_get, ruyi_array_length, ruyi_array_pop, ruyi_array_push,
    ruyi_array_set, ruyi_bigint_eq, ruyi_bigint_from_str, ruyi_bool_to_string,
    ruyi_float_to_string, ruyi_int_to_string, ruyi_member_access, ruyi_object_alloc,
    __random_bool, __random_bytes, __random_float, __random_int, __random_new,
    ruyi_string_concat,
};
pub use c_exports::cc_alloc;
#[cfg(feature = "inkwell")]
pub use exception::landing_pad::llvm::LandingPadGenerator;
#[cfg(feature = "inkwell")]
pub use exception::runtime::llvm::ExceptionRuntime;
pub use exception::runtime::{
    capture_stack_trace, ruyi_begin_catch, ruyi_end_catch, ruyi_finally, ruyi_match_exception,
    ruyi_throw,
};
pub use exception::types::{
    ExceptionObject, ExceptionType, UnwindException, KLANG_EXCEPTION_CLASS,
};
pub use exception::{
    builtin_type_ids, fresh_type_id, throw_exception, CatchClause, ExceptionTableEntry,
    ExceptionTableRegistry, FunctionExceptionTable, LandingPadAction, LandingPadDescriptor,
    RuyiException, StackFrame, TypeId,
};
pub use fmt_ffi::__string_replace_all;
pub use gc::{
    barrier::WriteBarrier, generational::GenerationalCollector, old::OldGeneration, roots::RootSet,
    young::YoungGeneration, Collector, GcAllocator, MarkSweepCollector,
};

#[cfg(feature = "inkwell")]
use inkwell::context::Context;
#[cfg(feature = "inkwell")]
use inkwell::types::{BasicTypeEnum, FloatType, IntType, PointerType, StructType, VoidType};

/// Ruyi primitive types that map directly to LLVM IR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuyiType {
    Int,
    Float,
    Bool,
    String,
    Null,
    Void,
    Dyn,
    Never,
    /// 8-bit unsigned integer (`byte`)
    Byte,
    /// A user-defined or generic type identified by name.
    Named(&'static str),
    /// An async future wrapping a value type.
    Future,
}

#[cfg(feature = "inkwell")]
/// Map a Ruyi type to its LLVM equivalent.
///
/// | Ruyi  | LLVM                |
/// |--------|---------------------|
/// | int    | i64                 |
/// | float  | f64                 |
/// | bool   | i1                  |
/// | string | *ruyiString        |
/// | null   | i8* (null pointer)  |
/// | void   | void                |
/// | dyn    | { i64, i8* }        |
/// | never  | void (poison)       |
pub fn ruyi_type_to_llvm<'ctx>(context: &'ctx Context, ty: RuyiType) -> BasicTypeEnum<'ctx> {
    match ty {
        RuyiType::Int => BasicTypeEnum::IntType(context.i64_type()),
        RuyiType::Float => BasicTypeEnum::FloatType(context.f64_type()),
        RuyiType::Bool => BasicTypeEnum::IntType(context.bool_type()),
        RuyiType::String => {
            BasicTypeEnum::PointerType(context.i8_type().ptr_type(Default::default()))
        }
        RuyiType::Null => {
            BasicTypeEnum::PointerType(context.i8_type().ptr_type(Default::default()))
        }
        RuyiType::Dyn => {
            // Tagged union: { type_id: i64, payload: i8* }
            let dyn_type = context.struct_type(
                &[
                    context.i64_type().into(),
                    context.i8_type().ptr_type(Default::default()).into(),
                ],
                false,
            );
            BasicTypeEnum::StructType(dyn_type)
        }
        RuyiType::Void | RuyiType::Never => {
            // void cannot be represented as BasicTypeEnum; use i8 as placeholder.
            BasicTypeEnum::IntType(context.i8_type())
        }
        RuyiType::Byte => BasicTypeEnum::IntType(context.i8_type()),
        RuyiType::Named(_) => {
            // Opaque pointer for user-defined types in the baseline.
            BasicTypeEnum::PointerType(context.i8_type().ptr_type(Default::default()))
        }
        RuyiType::Future => {
            BasicTypeEnum::PointerType(context.i8_type().ptr_type(Default::default()))
        }
    }
}

#[cfg(feature = "inkwell")]
/// Return the LLVM `i64` type (used for Ruyi `int`).
pub fn llvm_int_type<'ctx>(context: &'ctx Context) -> IntType<'ctx> {
    context.i64_type()
}

#[cfg(feature = "inkwell")]
/// Return the LLVM `f64` type (used for Ruyi `float`).
pub fn llvm_float_type<'ctx>(context: &'ctx Context) -> FloatType<'ctx> {
    context.f64_type()
}

#[cfg(feature = "inkwell")]
/// Return the LLVM `i1` type (used for Ruyi `bool`).
pub fn llvm_bool_type<'ctx>(context: &'ctx Context) -> IntType<'ctx> {
    context.bool_type()
}

#[cfg(feature = "inkwell")]
/// Return the LLVM pointer type (used for Ruyi `string`, objects, etc.).
pub fn llvm_ptr_type<'ctx>(context: &'ctx Context) -> PointerType<'ctx> {
    context.i8_type().ptr_type(Default::default())
}

#[cfg(feature = "inkwell")]
/// Return the LLVM struct type for `dyn` (tagged union).
pub fn llvm_dyn_type<'ctx>(context: &'ctx Context) -> StructType<'ctx> {
    context.struct_type(
        &[
            context.i64_type().into(),
            context.i8_type().ptr_type(Default::default()).into(),
        ],
        false,
    )
}

#[cfg(feature = "inkwell")]
/// Return the LLVM void type (used for Ruyi `void`).
pub fn llvm_void_type<'ctx>(context: &'ctx Context) -> VoidType<'ctx> {
    context.void_type()
}

#[cfg(feature = "inkwell")]
/// Ruyi runtime context that wraps an inkwell LLVM `Context`.
///
/// This is the primary handle used by the compiler frontend when
/// generating LLVM IR. It holds the LLVM context and provides
/// convenience accessors for Ruyi-specific types.
pub struct RuyiContext {
    context: Context,
}

#[cfg(feature = "inkwell")]
impl RuyiContext {
    pub fn new() -> Self {
        Self {
            context: Context::create(),
        }
    }

    pub fn llvm(&self) -> &Context {
        &self.context
    }

    pub fn int_type(&self) -> IntType<'_> {
        llvm_int_type(&self.context)
    }

    pub fn float_type(&self) -> FloatType<'_> {
        llvm_float_type(&self.context)
    }

    pub fn bool_type(&self) -> IntType<'_> {
        llvm_bool_type(&self.context)
    }

    pub fn ptr_type(&self) -> PointerType<'_> {
        llvm_ptr_type(&self.context)
    }

    pub fn dyn_type(&self) -> StructType<'_> {
        llvm_dyn_type(&self.context)
    }

    pub fn void_type(&self) -> VoidType<'_> {
        llvm_void_type(&self.context)
    }

    pub fn ruyi_type(&self, ty: RuyiType) -> BasicTypeEnum<'_> {
        ruyi_type_to_llvm(&self.context, ty)
    }
}

#[cfg(feature = "inkwell")]
impl Default for RuyiContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(test, feature = "inkwell"))]
mod tests {
    use super::*;

    #[test]
    fn test_ruyi_context_creation() {
        let ctx = RuyiContext::new();
        assert_eq!(ctx.int_type().get_bit_width(), 64);
        assert_eq!(ctx.bool_type().get_bit_width(), 1);
        let dyn_ty = ctx.dyn_type();
        assert_eq!(dyn_ty.count_fields(), 2);
    }

    #[test]
    fn test_type_mapping() {
        let ctx = RuyiContext::new();
        assert!(matches!(
            ctx.ruyi_type(RuyiType::Int),
            BasicTypeEnum::IntType(_)
        ));
        assert!(matches!(
            ctx.ruyi_type(RuyiType::Float),
            BasicTypeEnum::FloatType(_)
        ));
        assert!(matches!(
            ctx.ruyi_type(RuyiType::Bool),
            BasicTypeEnum::IntType(_)
        ));
        assert!(matches!(
            ctx.ruyi_type(RuyiType::String),
            BasicTypeEnum::PointerType(_)
        ));
        assert!(matches!(
            ctx.ruyi_type(RuyiType::Dyn),
            BasicTypeEnum::StructType(_)
        ));
    }
}
