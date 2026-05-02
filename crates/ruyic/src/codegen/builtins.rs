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
    declare_ruyi_throw(context, module);
    declare_ruyi_begin_catch(context, module);
    declare_ruyi_end_catch(context, module);
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
