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
use inkwell::values::{BasicValueEnum, FunctionValue};

use crate::typechecker::types::Type;

/// Declare built-in runtime functions in the LLVM module.
pub fn declare_builtins<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    declare_printf(context, module);
    declare_gc_alloc(context, module);
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
    declare_ruyi_obj_get(context, module);
    declare_ruyi_int_to_string(context, module);
    declare_ruyi_float_to_string(context, module);
    declare_pow(context, module);
}

fn declare_printf<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let i32_ty = context.i32_type();
    let fn_type = i32_ty.fn_type(&[i8_ptr.into()], true);
    module.add_function("printf", fn_type, None);
}

fn declare_gc_alloc<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i64_ty = context.i64_type();
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i64_ty.into()], false);
    module.add_function("ruyi_gc_alloc", fn_type, None);
}

fn declare_gc_collect<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let void_ty = context.void_type();
    let fn_type = void_ty.fn_type(&[], false);
    module.add_function("ruyi_gc_collect", fn_type, None);
}

fn declare_gc_add_root<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let void_ty = context.void_type();
    let fn_type = void_ty.fn_type(&[i8_ptr.into()], false);
    module.add_function("ruyi_gc_add_root", fn_type, None);
}

fn declare_gc_remove_root<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let void_ty = context.void_type();
    let fn_type = void_ty.fn_type(&[i8_ptr.into()], false);
    module.add_function("ruyi_gc_remove_root", fn_type, None);
}

fn declare_gc_write_barrier<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let void_ty = context.void_type();
    let fn_type = void_ty.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
    module.add_function("ruyi_gc_write_barrier", fn_type, None);
}

fn declare_ruyi_throw<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let void_ty = context.void_type();
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = void_ty.fn_type(&[i8_ptr.into()], false);
    module.add_function("ruyi_throw", fn_type, None);
}

fn declare_ruyi_get_pending_exception<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[], false);
    module.add_function("ruyi_get_pending_exception", fn_type, None);
}

fn declare_ruyi_clear_pending_exception<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let void_ty = context.void_type();
    let fn_type = void_ty.fn_type(&[], false);
    module.add_function("ruyi_clear_pending_exception", fn_type, None);
}

fn declare_ruyi_str_concat<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
    module.add_function("ruyi_str_concat", fn_type, None);
}

fn declare_ruyi_int_to_string<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let i64_ty = context.i64_type();
    let fn_type = i8_ptr.fn_type(&[i64_ty.into()], false);
    module.add_function("ruyi_int_to_string", fn_type, None);
}

fn declare_ruyi_float_to_string<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let f64_ty = context.f64_type();
    let fn_type = i8_ptr.fn_type(&[f64_ty.into()], false);
    module.add_function("ruyi_float_to_string", fn_type, None);
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
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_pointer_value()
}

pub fn build_ruyi_clear_pending_exception<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
) {
    let fn_val = module
        .get_function("ruyi_clear_pending_exception")
        .expect("ruyi_clear_pending_exception not declared");
    builder.build_call(fn_val, &[], "clear_pending_exc");
}

fn declare_ruyi_begin_catch<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
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
        _ => {
            build_print_primitive(builder, module, value);
        }
    }
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
            let fmt = builder.build_global_string_ptr("%ld\n", "fmt_int");
            builder.build_call(
                printf,
                &[fmt.as_pointer_value().into(), v.into()],
                "print_int",
            );
        }
        BasicValueEnum::FloatValue(v) => {
            let fmt = builder.build_global_string_ptr("%f\n", "fmt_float");
            builder.build_call(
                printf,
                &[fmt.as_pointer_value().into(), v.into()],
                "print_float",
            );
        }
        BasicValueEnum::PointerValue(v) => {
            let fmt = builder.build_global_string_ptr("%s\n", "fmt_str");
            builder.build_call(
                printf,
                &[fmt.as_pointer_value().into(), v.into()],
                "print_str",
            );
        }
        _ => {
            let fmt = builder.build_global_string_ptr("<unknown>\n", "fmt_unknown");
            builder.build_call(printf, &[fmt.as_pointer_value().into()], "print_unknown");
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
    let i64_ptr_ty = i64_ty.ptr_type(Default::default());

    let len_ptr = builder
        .build_bitcast(array_ptr, i64_ptr_ty, "len_ptr")
        .into_pointer_value();
    let len = builder.build_load(len_ptr, "len").into_int_value();

    let entry_bb = builder.get_insert_block().unwrap();
    let loop_header = context.append_basic_block(function, "array_loop_header");
    let loop_body = context.append_basic_block(function, "array_loop_body");
    let print_comma_bb = context.append_basic_block(function, "array_print_comma");
    let print_elem_bb = context.append_basic_block(function, "array_print_elem");
    let loop_increment = context.append_basic_block(function, "array_loop_inc");
    let loop_merge = context.append_basic_block(function, "array_loop_merge");

    builder.position_at_end(entry_bb);
    let fmt_open = builder.build_global_string_ptr("[", "fmt_open");
    builder.build_call(printf, &[fmt_open.as_pointer_value().into()], "print_open");
    builder.build_unconditional_branch(loop_header);

    builder.position_at_end(loop_header);
    let phi = builder.build_phi(i64_ty, "array_idx_phi");
    let i = phi.as_basic_value().into_int_value();
    let cond = builder.build_int_compare(IntPredicate::ULT, i, len, "array_loop_cond");
    builder.build_conditional_branch(cond, loop_body, loop_merge);

    builder.position_at_end(loop_body);
    let zero = i64_ty.const_int(0, false);
    let is_first = builder.build_int_compare(IntPredicate::EQ, i, zero, "is_first");
    builder.build_conditional_branch(is_first, print_elem_bb, print_comma_bb);

    builder.position_at_end(print_comma_bb);
    let fmt_comma = builder.build_global_string_ptr(", ", "fmt_comma");
    builder.build_call(
        printf,
        &[fmt_comma.as_pointer_value().into()],
        "print_comma",
    );
    builder.build_unconditional_branch(print_elem_bb);

    builder.position_at_end(print_elem_bb);
    let one = i64_ty.const_int(1, false);
    let eight = i64_ty.const_int(8, false);
    let elem_idx = builder.build_int_add(i, one, "elem_idx");
    let elem_byte_offset = builder.build_int_mul(elem_idx, eight, "elem_byte_offset");
    let elem_offset_i32 = builder.build_int_cast(elem_byte_offset, i32_ty, "elem_offset_i32");
    let elem_ptr = unsafe { builder.build_gep(array_ptr, &[elem_offset_i32], "elem_ptr") };
    let elem_i64_ptr = builder
        .build_bitcast(elem_ptr, i64_ptr_ty, "elem_i64_ptr")
        .into_pointer_value();
    let elem_val = builder
        .build_load(elem_i64_ptr, "elem_val")
        .into_int_value();

    let fmt_elem = builder.build_global_string_ptr("%ld", "fmt_elem");
    builder.build_call(
        printf,
        &[fmt_elem.as_pointer_value().into(), elem_val.into()],
        "print_elem",
    );
    builder.build_unconditional_branch(loop_increment);

    builder.position_at_end(loop_increment);
    let next_i = builder.build_int_add(i, one, "next_i");
    builder.build_unconditional_branch(loop_header);

    phi.add_incoming(&[(&zero, entry_bb), (&next_i, loop_increment)]);

    builder.position_at_end(loop_merge);
    let fmt_close = builder.build_global_string_ptr("]\n", "fmt_close");
    builder.build_call(
        printf,
        &[fmt_close.as_pointer_value().into()],
        "print_close",
    );
}

/// Build a call to `ruyi_gc_alloc`.
pub fn build_gc_alloc<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    size: inkwell::values::IntValue<'ctx>,
) -> inkwell::values::PointerValue<'ctx> {
    let alloc_fn = module
        .get_function("ruyi_gc_alloc")
        .expect("ruyi_gc_alloc not declared");
    builder
        .build_call(alloc_fn, &[size.into()], "gc_alloc")
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_pointer_value()
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
    builder.build_call(fn_val, &[ptr.into()], "gc_add_root");
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
    builder.build_call(fn_val, &[ptr.into()], "gc_remove_root");
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
    builder.build_call(fn_val, &[parent.into(), field.into()], "gc_write_barrier");
}

fn declare_ruyi_async_poll<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i32_ty = context.i32_type();
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i32_ty.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
    module.add_function("ruyi_async_poll", fn_type, None);
}

fn declare_ruyi_await<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into()], false);
    module.add_function("ruyi_await", fn_type, None);
}

fn declare_ruyi_spawn<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into()], false);
    module.add_function("ruyi_spawn", fn_type, None);
}

fn declare_ruyi_wake_task<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
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
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into()], false);
    module.add_function("ruyi_obj_keys", fn_type, None);
}

fn declare_ruyi_iter_next<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
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
        .try_as_basic_value()
        .left()
        .unwrap()
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
        .try_as_basic_value()
        .left()
        .unwrap()
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
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_pointer_value()
}

/// Returns `true` if a Ruyi type is managed by the GC (heap allocated).
pub fn is_gc_managed(ty: &crate::typechecker::types::Type) -> bool {
    use crate::typechecker::types::Type;
    match ty {
        Type::Int
        | Type::Float
        | Type::Bool
        | Type::Null
        | Type::Void
        | Type::Never
        | Type::Error
        | Type::String
        | Type::Function { .. } => false,
        Type::Nullable(inner) => is_gc_managed(inner),
        _ => true,
    }
}

fn declare_ruyi_bigint_from_str<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
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
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_pointer_value();
    Ok(result)
}

fn declare_ruyi_obj_get<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
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
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_pointer_value()
}
