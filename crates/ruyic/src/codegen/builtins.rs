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
    declare_ruyi_string_concat(context, module);
    declare_ruyi_int_to_string(context, module);
    declare_ruyi_float_to_string(context, module);
    declare_ruyi_array_alloc(context, module);
    declare_ruyi_array_length(context, module);
    declare_ruyi_array_get(context, module);
    declare_ruyi_array_set(context, module);
    declare_ruyi_array_push(context, module);
    declare_ruyi_array_pop(context, module);
    declare_ruyi_object_alloc(context, module);
    declare_ruyi_bigint_from_str(context, module);
    declare_ruyi_member_access(context, module);
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

fn declare_ruyi_string_concat<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
    module.add_function("ruyi_string_concat", fn_type, None);
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

fn declare_ruyi_array_alloc<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i64_ty = context.i64_type();
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i64_ty.into()], false);
    module.add_function("ruyi_array_alloc", fn_type, None);
}

fn declare_ruyi_object_alloc<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i64_ty = context.i64_type();
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i64_ty.into()], false);
    module.add_function("ruyi_object_alloc", fn_type, None);
}

fn declare_ruyi_bigint_from_str<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into()], false);
    module.add_function("ruyi_bigint_from_str", fn_type, None);
}

fn declare_ruyi_member_access<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i64_ty = context.i64_type();
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into(), i64_ty.into()], false);
    module.add_function("ruyi_member_access", fn_type, None);
}

fn declare_ruyi_array_length<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i64_ty = context.i64_type();
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i64_ty.fn_type(&[i8_ptr.into()], false);
    module.add_function("ruyi_array_length", fn_type, None);
}

fn declare_ruyi_array_get<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i64_ty = context.i64_type();
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into(), i64_ty.into()], false);
    module.add_function("ruyi_array_get", fn_type, None);
}

fn declare_ruyi_array_set<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i64_ty = context.i64_type();
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let void_ty = context.void_type();
    let fn_type = void_ty.fn_type(&[i8_ptr.into(), i64_ty.into(), i8_ptr.into()], false);
    module.add_function("ruyi_array_set", fn_type, None);
}

fn declare_ruyi_array_push<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i64_ty = context.i64_type();
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
    module.add_function("ruyi_array_push", fn_type, None);
}

fn declare_ruyi_array_pop<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into()], false);
    module.add_function("ruyi_array_pop", fn_type, None);
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

/// Build a call to `ruyi_string_concat`.
pub fn build_string_concat<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    lhs: BasicValueEnum<'ctx>,
    rhs: BasicValueEnum<'ctx>,
) -> BasicValueEnum<'ctx> {
    let concat_fn = module.get_function("ruyi_string_concat").expect("ruyi_string_concat not declared");
    builder
        .build_call(concat_fn, &[lhs.into(), rhs.into()], "string_concat")
        .try_as_basic_value()
        .left()
        .unwrap()
}

/// Build a call to `ruyi_int_to_string`.
pub fn build_int_to_string<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    val: inkwell::values::IntValue<'ctx>,
) -> BasicValueEnum<'ctx> {
    let fn_val = module.get_function("ruyi_int_to_string").expect("ruyi_int_to_string not declared");
    builder
        .build_call(fn_val, &[val.into()], "int_to_string")
        .try_as_basic_value()
        .left()
        .unwrap()
}

/// Build a call to `ruyi_float_to_string`.
pub fn build_float_to_string<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    val: inkwell::values::FloatValue<'ctx>,
) -> BasicValueEnum<'ctx> {
    let fn_val = module.get_function("ruyi_float_to_string").expect("ruyi_float_to_string not declared");
    builder
        .build_call(fn_val, &[val.into()], "float_to_string")
        .try_as_basic_value()
        .left()
        .unwrap()
}

/// Build a call to `ruyi_array_alloc`.
pub fn build_array_alloc<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    size: inkwell::values::IntValue<'ctx>,
) -> inkwell::values::PointerValue<'ctx> {
    let alloc_fn = module.get_function("ruyi_array_alloc").expect("ruyi_array_alloc not declared");
    builder
        .build_call(alloc_fn, &[size.into()], "array_alloc")
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_pointer_value()
}

/// Build a call to `ruyi_object_alloc`.
pub fn build_object_alloc<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    size: inkwell::values::IntValue<'ctx>,
) -> inkwell::values::PointerValue<'ctx> {
    let alloc_fn = module.get_function("ruyi_object_alloc").expect("ruyi_object_alloc not declared");
    builder
        .build_call(alloc_fn, &[size.into()], "object_alloc")
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_pointer_value()
}

/// Build a call to `ruyi_bigint_from_str`.
pub fn build_bigint_from_str<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    str_ptr: BasicValueEnum<'ctx>,
) -> BasicValueEnum<'ctx> {
    let conv_fn = module.get_function("ruyi_bigint_from_str").expect("ruyi_bigint_from_str not declared");
    builder
        .build_call(conv_fn, &[str_ptr.into()], "bigint_from_str")
        .try_as_basic_value()
        .left()
        .unwrap()
}

/// Build a call to `ruyi_member_access`.
pub fn build_member_access<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    obj: BasicValueEnum<'ctx>,
    offset: inkwell::values::IntValue<'ctx>,
) -> BasicValueEnum<'ctx> {
    let access_fn = module.get_function("ruyi_member_access").expect("ruyi_member_access not declared");
    builder
        .build_call(access_fn, &[obj.into(), offset.into()], "member_access")
        .try_as_basic_value()
        .left()
        .unwrap()
}

/// Build a call to `ruyi_array_length`.
pub fn build_array_length<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    arr: inkwell::values::PointerValue<'ctx>,
) -> inkwell::values::IntValue<'ctx> {
    let fn_val = module.get_function("ruyi_array_length").expect("ruyi_array_length not declared");
    builder
        .build_call(fn_val, &[arr.into()], "array_length")
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_int_value()
}

/// Build a call to `ruyi_array_get`.
pub fn build_array_get<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    arr: inkwell::values::PointerValue<'ctx>,
    index: inkwell::values::IntValue<'ctx>,
) -> inkwell::values::PointerValue<'ctx> {
    let fn_val = module.get_function("ruyi_array_get").expect("ruyi_array_get not declared");
    builder
        .build_call(fn_val, &[arr.into(), index.into()], "array_get")
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_pointer_value()
}

/// Build a call to `ruyi_array_set`.
pub fn build_array_set<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    arr: inkwell::values::PointerValue<'ctx>,
    index: inkwell::values::IntValue<'ctx>,
    value: inkwell::values::PointerValue<'ctx>,
) {
    let fn_val = module.get_function("ruyi_array_set").expect("ruyi_array_set not declared");
    builder.build_call(fn_val, &[arr.into(), index.into(), value.into()], "array_set");
}

/// Build a call to `ruyi_array_push`.
pub fn build_array_push<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    arr: inkwell::values::PointerValue<'ctx>,
    value: inkwell::values::PointerValue<'ctx>,
) -> inkwell::values::PointerValue<'ctx> {
    let fn_val = module.get_function("ruyi_array_push").expect("ruyi_array_push not declared");
    builder
        .build_call(fn_val, &[arr.into(), value.into()], "array_push")
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_pointer_value()
}

/// Build a call to `ruyi_array_pop`.
pub fn build_array_pop<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    module: &Module<'ctx>,
    arr: inkwell::values::PointerValue<'ctx>,
) -> inkwell::values::PointerValue<'ctx> {
    let fn_val = module.get_function("ruyi_array_pop").expect("ruyi_array_pop not declared");
    builder
        .build_call(fn_val, &[arr.into()], "array_pop")
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_pointer_value()
}
