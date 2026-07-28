use inkwell::context::Context;
/**
 * Built-in function implementations for Ruyi code generation.
 *
 * Provides LLVM IR for runtime functions like print, gc_alloc, etc.
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */
use inkwell::module::Module;
use inkwell::types::{BasicType, FunctionType};
use inkwell::values::{BasicValueEnum, FunctionValue};

use crate::cli::gc_mode::GcMode;
use crate::typechecker::types::Type;

use super::builtins_table::{
    params_to_metadata, sig_to_basic_type, BuiltinDecl, BuiltinSig, BUILTINS,
};

/// Declare built-in runtime functions in the LLVM module.
///
/// `gc_mode` selects which heap allocator symbol to declare:
/// `Stub` → `@cc_alloc`, `Real` → `@ruyi_gc_alloc`. All other runtime
/// functions are mode-independent.
///
/// The 55 `__builtin_*` / `__string_*` / `__math_*` / `__time_*` / `__json_*`
/// FFI entries are declared by iterating the static `BUILTINS` table; the
/// 26 internal `ruyi_*` / `printf` / GC helpers below are kept as
/// bespoke `declare_*` wrappers because they take extra arguments
/// (e.g. `gc_mode`) or call into `gc_alloc::GcAllocFn`.
pub fn declare_builtins<'ctx>(context: &'ctx Context, module: &Module<'ctx>, gc_mode: GcMode) {
    declare_printf(context, module);
    declare_alloc(context, module, gc_mode);
    declare_gc_collect(context, module);
    declare_gc_add_root(context, module);
    declare_gc_remove_root(context, module);
    declare_gc_write_barrier(context, module);
    declare_ruyi_throw(context, module);
    declare_ruyi_get_pending_exception(context, module);
    declare_ruyi_clear_pending_exception(context, module);
    declare_ruyi_str_concat(context, module);
    declare_ruyi_begin_catch(context, module);
    declare_ruyi_end_catch(context, module);
    declare_ruyi_async_poll(context, module);
    declare_ruyi_await(context, module);
    declare_ruyi_spawn(context, module);
    declare_ruyi_wake_task(context, module);
    declare_ruyi_run_scheduler(context, module);
    declare_ruyi_obj_keys(context, module);
    declare_ruyi_iter_next(context, module);
    declare_ruyi_bigint_from_str(context, module);
    declare_ruyi_bigint_eq(context, module);
    declare_ruyi_obj_get(context, module);
    declare_ruyi_int_to_string(context, module);
    declare_ruyi_float_to_string(context, module);
    declare_ruyi_bool_to_string(context, module);
    declare_pow(context, module);

    // 55 FFI entries (array, map, set, string, math, time, json) are
    // declared by walking the static BUILTINS table.
    for decl in BUILTINS {
        declare_builtin_from_table(context, module, decl);
    }
}

/// Declare one FFI entry from a `BuiltinDecl` record.
///
/// `Builtin's LLVM ABI is constructed by mapping each `BuiltinSig`
/// variant through `sig_to_basic_type` (return) / `params_to_metadata`
/// (parameters); the resulting `FunctionType` is registered against
/// `module` so call sites can resolve the symbol via `module.get_function`.
fn declare_builtin_from_table<'ctx>(
    context: &'ctx Context,
    module: &Module<'ctx>,
    decl: &BuiltinDecl,
) {
    let param_tys = params_to_metadata(context, decl.params);
    let fn_type: FunctionType<'_> = match decl.ret {
        BuiltinSig::Void => context.void_type().fn_type(&param_tys, false),
        ret => sig_to_basic_type(context, ret).fn_type(&param_tys, false),
    };
    module.add_function(decl.name, fn_type, None);
}

fn declare_printf<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.ptr_type(Default::default());
    let i32_ty = context.i32_type();
    let fn_type = i32_ty.fn_type(&[i8_ptr.into()], true);
    module.add_function("printf", fn_type, None);
}

fn declare_alloc<'ctx>(context: &'ctx Context, module: &Module<'ctx>, gc_mode: GcMode) {
    use super::gc_alloc::GcAllocFn;
    let alloc = GcAllocFn::for_mode(gc_mode);
    if module.get_function(alloc.fn_name()).is_some() {
        return;
    }
    let i64_ty = context.i64_type();
    let i8_ptr = context.ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i64_ty.into()], false);
    module.add_function(alloc.fn_name(), fn_type, None);
}

fn declare_gc_collect<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let void_ty = context.void_type();
    let fn_type = void_ty.fn_type(&[], false);
    module.add_function("ruyi_gc_collect", fn_type, None);
}

fn declare_gc_add_root<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.ptr_type(Default::default());
    let void_ty = context.void_type();
    let fn_type = void_ty.fn_type(&[i8_ptr.into()], false);
    module.add_function("ruyi_gc_add_root", fn_type, None);
}

fn declare_gc_remove_root<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.ptr_type(Default::default());
    let void_ty = context.void_type();
    let fn_type = void_ty.fn_type(&[i8_ptr.into()], false);
    module.add_function("ruyi_gc_remove_root", fn_type, None);
}

fn declare_gc_write_barrier<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.ptr_type(Default::default());
    let void_ty = context.void_type();
    let fn_type = void_ty.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
    module.add_function("ruyi_gc_write_barrier", fn_type, None);
}

fn declare_ruyi_throw<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let void_ty = context.void_type();
    let i8_ptr = context.ptr_type(Default::default());
    let fn_type = void_ty.fn_type(&[i8_ptr.into()], false);
    module.add_function("ruyi_throw", fn_type, None);
}

fn declare_ruyi_get_pending_exception<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[], false);
    module.add_function("ruyi_get_pending_exception", fn_type, None);
}

fn declare_ruyi_clear_pending_exception<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let void_ty = context.void_type();
    let fn_type = void_ty.fn_type(&[], false);
    module.add_function("ruyi_clear_pending_exception", fn_type, None);
}

fn declare_ruyi_str_concat<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
    module.add_function("ruyi_str_concat", fn_type, None);
}

fn declare_ruyi_int_to_string<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.ptr_type(Default::default());
    let i64_ty = context.i64_type();
    let fn_type = i8_ptr.fn_type(&[i64_ty.into()], false);
    module.add_function("ruyi_int_to_string", fn_type, None);
}

fn declare_ruyi_float_to_string<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.ptr_type(Default::default());
    let f64_ty = context.f64_type();
    let fn_type = i8_ptr.fn_type(&[f64_ty.into()], false);
    module.add_function("ruyi_float_to_string", fn_type, None);
}

fn declare_ruyi_bool_to_string<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.ptr_type(Default::default());
    let i1_ty = context.bool_type();
    let fn_type = i8_ptr.fn_type(&[i1_ty.into()], false);
    module.add_function("ruyi_bool_to_string", fn_type, None);
}

fn declare_pow<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let f64_ty = context.f64_type();
    let fn_type = f64_ty.fn_type(&[f64_ty.into(), f64_ty.into()], false);
    module.add_function("pow", fn_type, None);
}

pub fn build_ruyi_get_pending_exception<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
) -> inkwell::values::PointerValue<'ctx> {
    let fn_val = module
        .get_function("ruyi_get_pending_exception")
        .expect("ruyi_get_pending_exception not declared");
    builder
        .build_call(fn_val, &[], "get_pending_exc")
        .unwrap()
        .try_as_basic_value()
        .unwrap_basic()
        .into_pointer_value()
}

pub fn build_ruyi_clear_pending_exception<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
) {
    let fn_val = module
        .get_function("ruyi_clear_pending_exception")
        .expect("ruyi_clear_pending_exception not declared");
    builder.build_call(fn_val, &[], "clear_pending_exc").unwrap();
}

fn declare_ruyi_begin_catch<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into()], false);
    module.add_function("ruyi_begin_catch", fn_type, None);
}

fn declare_ruyi_end_catch<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let void_ty = context.void_type();
    let fn_type = void_ty.fn_type(&[], false);
    module.add_function("ruyi_end_catch", fn_type, None);
}

/// Build a call to the built-in `print` function.
pub fn build_print<'ctx>(
    context: &'ctx Context,
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    value: BasicValueEnum<'ctx>,
    ty: &Type,
    function: FunctionValue<'ctx>,
) {
    match ty {
        Type::Array(_elem_ty) => {
            build_print_array(context, builder, module, value, function);
        }
        Type::Bool => {
            build_print_bool(context, builder, module, value.into_int_value(), function);
        }
        _ => {
            build_print_primitive(builder, module, value);
        }
    }
}

fn build_print_bool<'ctx>(
    context: &'ctx Context,
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    value: inkwell::values::IntValue<'ctx>,
    function: FunctionValue<'ctx>,
) {
    let printf = module.get_function("printf").expect("printf not declared");
    let true_bb = context.append_basic_block(function, "print_bool_true");
    let false_bb = context.append_basic_block(function, "print_bool_false");
    let merge_bb = context.append_basic_block(function, "print_bool_merge");
    builder.build_conditional_branch(value, true_bb, false_bb).unwrap();
    builder.position_at_end(true_bb);
    let fmt_true = builder.build_global_string_ptr("true\n", "fmt_bool_true").unwrap();
    builder.build_call(
        printf,
        &[fmt_true.as_pointer_value().into()],
        "print_bool_true",
    ).unwrap();
    builder.build_unconditional_branch(merge_bb).unwrap();
    builder.position_at_end(false_bb);
    let fmt_false = builder.build_global_string_ptr("false\n", "fmt_bool_false").unwrap();
    builder.build_call(
        printf,
        &[fmt_false.as_pointer_value().into()],
        "print_bool_false",
    ).unwrap();
    builder.build_unconditional_branch(merge_bb).unwrap();
    builder.position_at_end(merge_bb);
}

/// Print a primitive value (int, float, string, pointer).
fn build_print_primitive<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    value: BasicValueEnum<'ctx>,
) {
    let printf = module.get_function("printf").expect("printf not declared");

    match value {
        BasicValueEnum::IntValue(v) => {
            let fmt = builder.build_global_string_ptr("%ld\n", "fmt_int").unwrap();
            builder.build_call(
                printf,
                &[fmt.as_pointer_value().into(), v.into()],
                "print_int",
            ).unwrap();
        }
        BasicValueEnum::FloatValue(v) => {
            let fmt = builder.build_global_string_ptr("%f\n", "fmt_float").unwrap();
            builder.build_call(
                printf,
                &[fmt.as_pointer_value().into(), v.into()],
                "print_float",
            ).unwrap();
        }
        BasicValueEnum::PointerValue(v) => {
            let fmt = builder.build_global_string_ptr("%s\n", "fmt_str").unwrap();
            builder.build_call(
                printf,
                &[fmt.as_pointer_value().into(), v.into()],
                "print_str",
            ).unwrap();
        }
        _ => {
            let fmt = builder.build_global_string_ptr("<unknown>\n", "fmt_unknown").unwrap();
            builder.build_call(printf, &[fmt.as_pointer_value().into()], "print_unknown").unwrap();
        }
    }
}

/// Print an array value as `[elem1, elem2, ...]`.
///
/// Array memory layout:
///   offset 0: length (i64)
///   offset 8: element 0 (i64)
///   offset 16: element 1 (i64)
///   ...
fn build_print_array<'ctx>(
    context: &'ctx Context,
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    array_ptr: BasicValueEnum<'ctx>,
    function: FunctionValue<'ctx>,
) {
    use inkwell::IntPredicate;

    let printf = module.get_function("printf").expect("printf not declared");
    let array_ptr = array_ptr.into_pointer_value();
    let i64_ty = context.i64_type();
    let i32_ty = context.i32_type();
    let i64_ptr_ty = context.ptr_type(Default::default());

    let len_ptr = builder
        .build_bit_cast(array_ptr, i64_ptr_ty, "len_ptr").unwrap()
        .into_pointer_value();
    let len = builder.build_load(i64_ty, len_ptr, "len").unwrap().into_int_value();

    let entry_bb = builder.get_insert_block().unwrap();
    let loop_header = context.append_basic_block(function, "array_loop_header");
    let loop_body = context.append_basic_block(function, "array_loop_body");
    let print_comma_bb = context.append_basic_block(function, "array_print_comma");
    let print_elem_bb = context.append_basic_block(function, "array_print_elem");
    let loop_increment = context.append_basic_block(function, "array_loop_inc");
    let loop_merge = context.append_basic_block(function, "array_loop_merge");

    builder.position_at_end(entry_bb);
    let fmt_open = builder.build_global_string_ptr("[", "fmt_open").unwrap();
    builder.build_call(printf, &[fmt_open.as_pointer_value().into()], "print_open").unwrap();
    builder.build_unconditional_branch(loop_header).unwrap();

    builder.position_at_end(loop_header);
    let phi = builder.build_phi(i64_ty, "array_idx_phi").unwrap();
    let i = phi.as_basic_value().into_int_value();
    let cond = builder.build_int_compare(IntPredicate::ULT, i, len, "array_loop_cond").unwrap();
    builder.build_conditional_branch(cond, loop_body, loop_merge).unwrap();

    builder.position_at_end(loop_body);
    let zero = i64_ty.const_int(0, false);
    let is_first = builder.build_int_compare(IntPredicate::EQ, i, zero, "is_first").unwrap();
    builder.build_conditional_branch(is_first, print_elem_bb, print_comma_bb).unwrap();

    builder.position_at_end(print_comma_bb);
    let fmt_comma = builder.build_global_string_ptr(", ", "fmt_comma").unwrap();
    builder.build_call(
        printf,
        &[fmt_comma.as_pointer_value().into()],
        "print_comma",
    ).unwrap();
    builder.build_unconditional_branch(print_elem_bb).unwrap();

    builder.position_at_end(print_elem_bb);
    let one = i64_ty.const_int(1, false);
    let eight = i64_ty.const_int(8, false);
    let sixteen = i64_ty.const_int(16, false);
    let elem_byte_offset = builder.build_int_mul(i, eight, "elem_byte_offset").unwrap();
    let elem_offset_with_header =
        builder.build_int_add(sixteen, elem_byte_offset, "elem_offset_hdr").unwrap();
    let elem_offset_i32 =
        builder.build_int_cast(elem_offset_with_header, i32_ty, "elem_offset_i32").unwrap();
    let elem_ptr = unsafe { builder.build_gep(context.i8_type(), array_ptr, &[elem_offset_i32], "elem_ptr").unwrap() };
    let elem_i64_ptr = builder
        .build_bit_cast(elem_ptr, i64_ptr_ty, "elem_i64_ptr").unwrap()
        .into_pointer_value();
    let elem_val = builder
        .build_load(i64_ty, elem_i64_ptr, "elem_val").unwrap()
        .into_int_value();

    let fmt_elem = builder.build_global_string_ptr("%ld", "fmt_elem").unwrap();
    builder.build_call(
        printf,
        &[fmt_elem.as_pointer_value().into(), elem_val.into()],
        "print_elem",
    ).unwrap();
    builder.build_unconditional_branch(loop_increment).unwrap();

    builder.position_at_end(loop_increment);
    let next_i = builder.build_int_add(i, one, "next_i").unwrap();
    builder.build_unconditional_branch(loop_header).unwrap();

    phi.add_incoming(&[(&zero, entry_bb), (&next_i, loop_increment)]);

    builder.position_at_end(loop_merge);
    let fmt_close = builder.build_global_string_ptr("]\n", "fmt_close").unwrap();
    builder.build_call(
        printf,
        &[fmt_close.as_pointer_value().into()],
        "print_close",
    ).unwrap();
}

/// Build a call to the active allocator via [`crate::codegen::gc_alloc::GcAllocFn`].
///
/// Kept as a thin compat shim for callers that don't yet carry a
/// `GcMode` directly. Returns `cc_alloc` (stub) by default.
pub fn build_gc_alloc<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    size: inkwell::values::IntValue<'ctx>,
) -> inkwell::values::PointerValue<'ctx> {
    use super::gc_alloc::GcAllocFn;
    use crate::cli::gc_mode::GcMode;
    GcAllocFn::for_mode(GcMode::default()).emit(builder, module, size)
}

/// Build a call to `ruyi_gc_add_root`.
pub fn build_gc_add_root<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    ptr: inkwell::values::PointerValue<'ctx>,
) {
    let fn_val = module
        .get_function("ruyi_gc_add_root")
        .expect("ruyi_gc_add_root not declared");
    builder.build_call(fn_val, &[ptr.into()], "gc_add_root").unwrap();
}

/// Build a call to `ruyi_gc_remove_root`.
pub fn build_gc_remove_root<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    ptr: inkwell::values::PointerValue<'ctx>,
) {
    let fn_val = module
        .get_function("ruyi_gc_remove_root")
        .expect("ruyi_gc_remove_root not declared");
    builder.build_call(fn_val, &[ptr.into()], "gc_remove_root").unwrap();
}

/// Build a call to `ruyi_gc_write_barrier`.
pub fn build_gc_write_barrier<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    parent: inkwell::values::PointerValue<'ctx>,
    field: inkwell::values::PointerValue<'ctx>,
) {
    let fn_val = module
        .get_function("ruyi_gc_write_barrier")
        .expect("ruyi_gc_write_barrier not declared");
    builder.build_call(fn_val, &[parent.into(), field.into()], "gc_write_barrier").unwrap();
}

fn declare_ruyi_async_poll<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i32_ty = context.i32_type();
    let i8_ptr = context.ptr_type(Default::default());
    let fn_type = i32_ty.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
    module.add_function("ruyi_async_poll", fn_type, None);
}

fn declare_ruyi_await<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into()], false);
    module.add_function("ruyi_await", fn_type, None);
}

fn declare_ruyi_spawn<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into()], false);
    module.add_function("ruyi_spawn", fn_type, None);
}

fn declare_ruyi_wake_task<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.ptr_type(Default::default());
    let void_ty = context.void_type();
    let fn_type = void_ty.fn_type(&[i8_ptr.into()], false);
    module.add_function("ruyi_wake_task", fn_type, None);
}

fn declare_ruyi_run_scheduler<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let void_ty = context.void_type();
    let fn_type = void_ty.fn_type(&[], false);
    module.add_function("ruyi_run_scheduler", fn_type, None);
}

fn declare_ruyi_obj_keys<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into()], false);
    module.add_function("ruyi_obj_keys", fn_type, None);
}

fn declare_ruyi_iter_next<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into()], false);
    module.add_function("ruyi_iter_next", fn_type, None);
}

/// Build a call to `ruyi_async_poll`.
pub fn build_ruyi_async_poll<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    future: inkwell::values::PointerValue<'ctx>,
    waker: inkwell::values::PointerValue<'ctx>,
) -> inkwell::values::IntValue<'ctx> {
    let fn_val = module
        .get_function("ruyi_async_poll")
        .expect("ruyi_async_poll not declared");
    builder
        .build_call(fn_val, &[future.into(), waker.into()], "async_poll")
        .unwrap()
        .try_as_basic_value()
        .unwrap_basic()
        .into_int_value()
}

pub fn build_ruyi_await<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    future: inkwell::values::PointerValue<'ctx>,
) -> inkwell::values::PointerValue<'ctx> {
    let fn_val = module
        .get_function("ruyi_await")
        .expect("ruyi_await not declared");
    builder
        .build_call(fn_val, &[future.into()], "await")
        .unwrap()
        .try_as_basic_value()
        .unwrap_basic()
        .into_pointer_value()
}

/// Build a call to `ruyi_spawn`.
pub fn build_ruyi_spawn<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    future: inkwell::values::PointerValue<'ctx>,
) -> inkwell::values::PointerValue<'ctx> {
    let fn_val = module
        .get_function("ruyi_spawn")
        .expect("ruyi_spawn not declared");
    builder
        .build_call(fn_val, &[future.into()], "spawn")
        .unwrap()
        .try_as_basic_value()
        .unwrap_basic()
        .into_pointer_value()
}

/// Returns `true` if a Ruyi type is managed by the GC (heap allocated).
pub fn is_gc_managed(ty: &crate::typechecker::types::Type) -> bool {
    use crate::typechecker::types::Type;
    match ty {
        Type::Int
        | Type::Float
        | Type::Bool
        | Type::Byte
        | Type::Null
        | Type::Void
        | Type::Never
        | Type::Error
        | Type::String
        | Type::Function { .. }
        | Type::Dynamic
        | Type::TypeVar(_)
        | Type::Generic { .. } => false,
        // Single uppercase letter named types (T, U, V) are type parameters, not GC-managed
        Type::Named(name, _)
            if name.len() == 1
                && name
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_uppercase())
                    .unwrap_or(false) =>
        {
            false
        }
        Type::Nullable(inner) => is_gc_managed(inner),
        _ => true,
    }
}

fn declare_ruyi_bigint_from_str<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into()], false);
    module.add_function("ruyi_bigint_from_str", fn_type, None);
}

pub fn build_ruyi_bigint_from_str<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    string_ptr: inkwell::values::PointerValue<'ctx>,
) -> Result<inkwell::values::PointerValue<'ctx>, String> {
    let fn_val = module
        .get_function("ruyi_bigint_from_str")
        .ok_or("ruyi_bigint_from_str not declared")?;
    let result = builder
        .build_call(fn_val, &[string_ptr.into()], "bigint_from_str")
        .unwrap()
        .try_as_basic_value()
        .unwrap_basic()
        .into_pointer_value();
    Ok(result)
}

fn declare_ruyi_bigint_eq<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.ptr_type(Default::default());
    let fn_type = context
        .i8_type()
        .fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
    module.add_function("ruyi_bigint_eq", fn_type, None);
}

pub fn build_ruyi_bigint_eq<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    a: inkwell::values::PointerValue<'ctx>,
    b: inkwell::values::PointerValue<'ctx>,
) -> Result<inkwell::values::IntValue<'ctx>, String> {
    let fn_val = module
        .get_function("ruyi_bigint_eq")
        .ok_or("ruyi_bigint_eq not declared")?;
    let result = builder
        .build_call(fn_val, &[a.into(), b.into()], "bigint_eq")
        .unwrap()
        .try_as_basic_value()
        .unwrap_basic()
        .into_int_value();
    Ok(result)
}

fn declare_ruyi_obj_get<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
    module.add_function("ruyi_obj_get", fn_type, None);
}

pub fn build_ruyi_obj_get<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    obj: inkwell::values::PointerValue<'ctx>,
    key: inkwell::values::PointerValue<'ctx>,
) -> inkwell::values::PointerValue<'ctx> {
    let fn_val = module
        .get_function("ruyi_obj_get")
        .expect("ruyi_obj_get not declared");
    builder
        .build_call(fn_val, &[obj.into(), key.into()], "obj_get")
        .unwrap()
        .try_as_basic_value()
        .unwrap_basic()
        .into_pointer_value()
}

/// Build a call to `__builtin_array_set(arr, index, value)`.
pub fn build_builtin_array_set<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    arr: inkwell::values::PointerValue<'ctx>,
    index: inkwell::values::IntValue<'ctx>,
    value: inkwell::values::IntValue<'ctx>,
) {
    let fn_val = module
        .get_function("__builtin_array_set")
        .expect("__builtin_array_set not declared");
    builder.build_call(
        fn_val,
        &[arr.into(), index.into(), value.into()],
        "array_set",
    ).unwrap();
}

/// Build a call to `__builtin_array_get(arr, index)`.
///
/// Returns the element as an `i64`. Bounds checking and negative-index
/// handling are performed by the runtime function.
pub fn build_builtin_array_get<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    arr: inkwell::values::PointerValue<'ctx>,
    index: inkwell::values::IntValue<'ctx>,
) -> inkwell::values::IntValue<'ctx> {
    let fn_val = module
        .get_function("__builtin_array_get")
        .expect("__builtin_array_get not declared");
    builder
        .build_call(fn_val, &[arr.into(), index.into()], "array_get")
        .unwrap()
        .try_as_basic_value()
        .unwrap_basic()
        .into_int_value()
}
