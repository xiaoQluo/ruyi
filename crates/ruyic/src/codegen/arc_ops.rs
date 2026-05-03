/**
 * ARC code generation helpers for Ruyi.
 *
 * Provides LLVM IR generation for retain/release calls and
 * ARC object allocation.
 *
 * @author Ruyi Team
 * @date 2026-05-02
 */
use inkwell::values::PointerValue;

use crate::codegen::generator::CodegenContext;

/// Emit a call to `ruyi_arc_retain` for the given pointer.
///
/// # Safety
///
/// The pointer must be a valid ARC object payload.
pub fn emit_arc_retain<'ctx>(ctx: &mut CodegenContext<'ctx, '_>, ptr: PointerValue<'ctx>) {
    let fn_name = "ruyi_arc_retain";
    let func = ctx.module.get_function(fn_name).unwrap_or_else(|| {
        let void_ty = ctx.context.void_type();
        let param_ty = ctx.context.i8_type().ptr_type(Default::default());
        let fn_ty = void_ty.fn_type(&[param_ty.into()], false);
        ctx.module.add_function(fn_name, fn_ty, None)
    });
    ctx.builder.build_call(func, &[ptr.into()], "arc_retain");
}

/// Emit a call to `ruyi_arc_release` for the given pointer.
///
/// # Safety
///
/// The pointer must be a valid ARC object payload.
pub fn emit_arc_release<'ctx>(ctx: &mut CodegenContext<'ctx, '_>, ptr: PointerValue<'ctx>) {
    let fn_name = "ruyi_arc_release";
    let func = ctx.module.get_function(fn_name).unwrap_or_else(|| {
        let void_ty = ctx.context.void_type();
        let param_ty = ctx.context.i8_type().ptr_type(Default::default());
        let fn_ty = void_ty.fn_type(&[param_ty.into()], false);
        ctx.module.add_function(fn_name, fn_ty, None)
    });
    ctx.builder.build_call(func, &[ptr.into()], "arc_release");
}

/// Emit a call to `ruyi_arc_alloc`.
///
/// Returns the payload pointer for the newly allocated ARC object.
pub fn emit_arc_alloc<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    size: inkwell::values::IntValue<'ctx>,
    type_info: PointerValue<'ctx>,
) -> PointerValue<'ctx> {
    let fn_name = "ruyi_arc_alloc";
    let func = ctx.module.get_function(fn_name).unwrap_or_else(|| {
        let ptr_ty = ctx.context.i8_type().ptr_type(Default::default());
        let i64_ty = ctx.context.i64_type();
        let fn_ty = ptr_ty.fn_type(&[i64_ty.into(), ptr_ty.into()], false);
        ctx.module.add_function(fn_name, fn_ty, None)
    });
    let call = ctx
        .builder
        .build_call(func, &[size.into(), type_info.into()], "arc_alloc");
    call.try_as_basic_value()
        .left()
        .unwrap()
        .into_pointer_value()
}

/// Emit balanced retain/release around a block of code.
///
/// Retains the pointer at entry and releases it at all exit paths.
/// This is a simplified version that only handles the immediate block.
pub fn emit_arc_balanced<'ctx, F>(
    ctx: &mut CodegenContext<'ctx, '_>,
    ptr: PointerValue<'ctx>,
    body: F,
) where
    F: FnOnce(&mut CodegenContext<'ctx, '_>),
{
    emit_arc_retain(ctx, ptr);
    body(ctx);
    emit_arc_release(ctx, ptr);
}
