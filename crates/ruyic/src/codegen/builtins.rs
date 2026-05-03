/**
 * Built-in function implementations for Ruyi code generation.
 *
 * Provides LLVM IR for runtime functions like print, gc_alloc, etc.
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */

use inkwell::module::Module;
use inkwell::context::Context;
use inkwell::values::BasicValueEnum;

/// Declare built-in runtime functions in the LLVM module.
pub fn declare_builtins<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    declare_printf(context, module);
    declare_gc_alloc(context, module);
    declare_gc_collect(context, module);
    declare_gc_add_root(context, module);
    declare_gc_remove_root(context, module);
    declare_gc_write_barrier(context, module);
    declare_ruyi_throw(context, module);
    declare_ruyi_begin_catch(context, module);
    declare_ruyi_end_catch(context, module);
    declare_ruyi_async_poll(context, module);
    declare_ruyi_spawn(context, module);
    declare_ruyi_wake_task(context, module);
    declare_ruyi_run_scheduler(context, module);
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
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    value: BasicValueEnum<'ctx>,
) {
    let printf = module.get_function("printf").expect("printf not declared");

    match value {
        BasicValueEnum::IntValue(v) => {
            let fmt = builder.build_global_string_ptr("%ld\n", "fmt_int");
            builder.build_call(printf, &[fmt.as_pointer_value().into(), v.into()], "print_int");
        }
        BasicValueEnum::FloatValue(v) => {
            let fmt = builder.build_global_string_ptr("%f\n", "fmt_float");
            builder.build_call(printf, &[fmt.as_pointer_value().into(), v.into()], "print_float");
        }
        BasicValueEnum::PointerValue(v) => {
            let fmt = builder.build_global_string_ptr("%s\n", "fmt_str");
            builder.build_call(printf, &[fmt.as_pointer_value().into(), v.into()], "print_str");
        }
        _ => {
            let fmt = builder.build_global_string_ptr("<unknown>\n", "fmt_unknown");
            builder.build_call(printf, &[fmt.as_pointer_value().into()], "print_unknown");
        }
    }
}

/// Build a call to `ruyi_gc_alloc`.
pub fn build_gc_alloc<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    size: inkwell::values::IntValue<'ctx>,
) -> inkwell::values::PointerValue<'ctx> {
    let alloc_fn = module.get_function("ruyi_gc_alloc").expect("ruyi_gc_alloc not declared");
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
        Type::Int | Type::Float | Type::Bool | Type::Null | Type::Void | Type::Never | Type::Error => false,
        Type::Nullable(inner) => is_gc_managed(inner),
        _ => true,
    }
}
