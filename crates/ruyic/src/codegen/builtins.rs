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
    declare_ruyi_bigint_eq(context, module);
    declare_ruyi_obj_get(context, module);
    declare_ruyi_int_to_string(context, module);
    declare_ruyi_float_to_string(context, module);
    declare_ruyi_bool_to_string(context, module);
    declare_pow(context, module);

    // __builtin_array_* declarations (used by stdlib/collections.ry)
    declare_builtin_array_create(context, module);
    declare_builtin_array_get(context, module);
    declare_builtin_array_set(context, module);
    declare_builtin_array_push(context, module);
    declare_builtin_array_pop(context, module);
    declare_builtin_array_length(context, module);

    // __builtin_map_* declarations (used by stdlib/collections.ry)
    declare_builtin_map_create(context, module);
    declare_builtin_map_get(context, module);
    declare_builtin_map_set(context, module);
    declare_builtin_map_delete(context, module);
    declare_builtin_map_has(context, module);
    declare_builtin_map_keys(context, module);
    declare_builtin_map_values(context, module);

    // __builtin_set_* declarations (used by stdlib/collections.ry)
    declare_builtin_set_create(context, module);
    declare_builtin_set_add(context, module);
    declare_builtin_set_delete(context, module);
    declare_builtin_set_has(context, module);

    // __string_* declarations (used by stdlib/string.ry)
    declare_string_join(context, module);
    declare_string_from_char_code(context, module);
    declare_string_from_char_codes(context, module);
    declare_string_replace_all(context, module);
    declare_string_length(context, module);
    declare_string_contains(context, module);
    declare_string_starts_with(context, module);
    declare_string_ends_with(context, module);
    declare_string_index_of(context, module);
    declare_string_last_index_of(context, module);
    declare_string_char_at(context, module);
    declare_string_char_code_at(context, module);
    declare_string_repeat(context, module);
    declare_string_substring(context, module);
    declare_string_to_upper_case(context, module);
    declare_string_to_lower_case(context, module);
    declare_string_trim(context, module);
    declare_string_split(context, module);
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

fn declare_ruyi_bool_to_string<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
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
    builder.build_conditional_branch(value, true_bb, false_bb);
    builder.position_at_end(true_bb);
    let fmt_true = builder.build_global_string_ptr("true\n", "fmt_bool_true");
    builder.build_call(
        printf,
        &[fmt_true.as_pointer_value().into()],
        "print_bool_true",
    );
    builder.build_unconditional_branch(merge_bb);
    builder.position_at_end(false_bb);
    let fmt_false = builder.build_global_string_ptr("false\n", "fmt_bool_false");
    builder.build_call(
        printf,
        &[fmt_false.as_pointer_value().into()],
        "print_bool_false",
    );
    builder.build_unconditional_branch(merge_bb);
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
    let sixteen = i64_ty.const_int(16, false);
    let elem_byte_offset = builder.build_int_mul(i, eight, "elem_byte_offset");
    let elem_offset_with_header =
        builder.build_int_add(sixteen, elem_byte_offset, "elem_offset_hdr");
    let elem_offset_i32 =
        builder.build_int_cast(elem_offset_with_header, i32_ty, "elem_offset_i32");
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

fn declare_ruyi_bigint_eq<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
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
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_int_value();
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

// ============================================================
// __builtin_array_* declarations
// ============================================================

fn declare_builtin_array_create<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[], false);
    module.add_function("__builtin_array_create", fn_type, None);
}

fn declare_builtin_array_get<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let i64_ty = context.i64_type();
    let fn_type = i64_ty.fn_type(&[i8_ptr.into(), i64_ty.into()], false);
    module.add_function("__builtin_array_get", fn_type, None);
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
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_int_value()
}

fn declare_builtin_array_set<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let i64_ty = context.i64_type();
    let fn_type = context
        .void_type()
        .fn_type(&[i8_ptr.into(), i64_ty.into(), i64_ty.into()], false);
    module.add_function("__builtin_array_set", fn_type, None);
}

fn declare_builtin_array_push<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let i64_ty = context.i64_type();
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into(), i64_ty.into()], false);
    module.add_function("__builtin_array_push", fn_type, None);
}

fn declare_builtin_array_pop<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let i64_ty = context.i64_type();
    let fn_type = i64_ty.fn_type(&[i8_ptr.into()], false);
    module.add_function("__builtin_array_pop", fn_type, None);
}

fn declare_builtin_array_length<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let i64_ty = context.i64_type();
    let fn_type = i64_ty.fn_type(&[i8_ptr.into()], false);
    module.add_function("__builtin_array_length", fn_type, None);
}

// ============================================================
// __builtin_map_* declarations
// ============================================================

fn declare_builtin_map_create<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[], false);
    module.add_function("__builtin_map_create", fn_type, None);
}

fn declare_builtin_map_get<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
    module.add_function("__builtin_map_get", fn_type, None);
}

fn declare_builtin_map_set<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = context
        .void_type()
        .fn_type(&[i8_ptr.into(), i8_ptr.into(), i8_ptr.into()], false);
    module.add_function("__builtin_map_set", fn_type, None);
}

fn declare_builtin_map_delete<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = context
        .void_type()
        .fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
    module.add_function("__builtin_map_delete", fn_type, None);
}

fn declare_builtin_map_has<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let i1_ty = context.bool_type();
    let fn_type = i1_ty.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
    module.add_function("__builtin_map_has", fn_type, None);
}

fn declare_builtin_map_keys<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into()], false);
    module.add_function("__builtin_map_keys", fn_type, None);
}

fn declare_builtin_map_values<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into()], false);
    module.add_function("__builtin_map_values", fn_type, None);
}

// ============================================================
// __builtin_set_* declarations
// ============================================================

fn declare_builtin_set_create<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[], false);
    module.add_function("__builtin_set_create", fn_type, None);
}

fn declare_builtin_set_add<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = context
        .void_type()
        .fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
    module.add_function("__builtin_set_add", fn_type, None);
}

fn declare_builtin_set_delete<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let i1_ty = context.bool_type();
    let fn_type = i1_ty.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
    module.add_function("__builtin_set_delete", fn_type, None);
}

fn declare_builtin_set_has<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let i1_ty = context.bool_type();
    let fn_type = i1_ty.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
    module.add_function("__builtin_set_has", fn_type, None);
}

// ============================================================
// __string_* declarations
// ============================================================

fn declare_string_join<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
    module.add_function("__string_join", fn_type, None);
}

fn declare_string_from_char_code<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let i64_ty = context.i64_type();
    let fn_type = i8_ptr.fn_type(&[i64_ty.into()], false);
    module.add_function("__string_from_char_code", fn_type, None);
}

fn declare_string_from_char_codes<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into()], false);
    module.add_function("__string_from_char_codes", fn_type, None);
}

fn declare_string_replace_all<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into(), i8_ptr.into(), i8_ptr.into()], false);
    module.add_function("__string_replace_all", fn_type, None);
}

fn declare_string_length<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let i64_ty = context.i64_type();
    let fn_type = i64_ty.fn_type(&[i8_ptr.into()], false);
    module.add_function("__string_length", fn_type, None);
}

fn declare_string_contains<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let i1_ty = context.bool_type();
    let fn_type = i1_ty.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
    module.add_function("__string_contains", fn_type, None);
}

fn declare_string_starts_with<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let i1_ty = context.bool_type();
    let fn_type = i1_ty.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
    module.add_function("__string_starts_with", fn_type, None);
}

fn declare_string_ends_with<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let i1_ty = context.bool_type();
    let fn_type = i1_ty.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
    module.add_function("__string_ends_with", fn_type, None);
}

fn declare_string_index_of<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let i64_ty = context.i64_type();
    let fn_type = i64_ty.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
    module.add_function("__string_index_of", fn_type, None);
}

fn declare_string_last_index_of<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let i64_ty = context.i64_type();
    let fn_type = i64_ty.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
    module.add_function("__string_last_index_of", fn_type, None);
}

fn declare_string_char_at<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let i64_ty = context.i64_type();
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into(), i64_ty.into()], false);
    module.add_function("__string_char_at", fn_type, None);
}

fn declare_string_char_code_at<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let i64_ty = context.i64_type();
    let fn_type = i64_ty.fn_type(&[i8_ptr.into(), i64_ty.into()], false);
    module.add_function("__string_char_code_at", fn_type, None);
}

fn declare_string_repeat<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let i64_ty = context.i64_type();
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into(), i64_ty.into()], false);
    module.add_function("__string_repeat", fn_type, None);
}

fn declare_string_substring<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let i64_ty = context.i64_type();
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into(), i64_ty.into(), i64_ty.into()], false);
    module.add_function("__string_substring", fn_type, None);
}

fn declare_string_to_upper_case<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into()], false);
    module.add_function("__string_to_upper_case", fn_type, None);
}

fn declare_string_to_lower_case<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into()], false);
    module.add_function("__string_to_lower_case", fn_type, None);
}

fn declare_string_trim<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into()], false);
    module.add_function("__string_trim", fn_type, None);
}

fn declare_string_split<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
    module.add_function("__string_split", fn_type, None);
}
