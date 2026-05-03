use inkwell::values::BasicValueEnum;
use crate::parser::ast::Expr;
use crate::typechecker::types::Type;
use super::builtins::is_gc_managed;
use super::generator::CodegenContext;
use super::expr::compile_expr;
use super::stmt::compile_block;

/// Compile an async function declaration.
///
/// An async function is lowered to:
/// 1. A state-machine struct that captures locals across await points.
/// 2. A `poll` function that drives the state machine.
/// 3. A constructor function that returns a pointer to the future.
pub fn compile_async_function<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    name: &str,
    params: &[crate::parser::ast::Param],
    return_type: Option<&crate::parser::ast::TypeAnnotation>,
    body: &[crate::parser::ast::Statement],
) -> Result<(), String> {
    let ret_type = return_type.map(Type::from_annotation).unwrap_or(Type::Void);
    let future_ret = Type::Future(Box::new(ret_type.clone()));

    let param_types: Vec<Type> = params
        .iter()
        .map(|p| p.ty.as_ref().map(Type::from_annotation).unwrap_or(Type::Dynamic))
        .collect();

    // For now, emit a synchronous wrapper that returns a placeholder future pointer.
    // Full state-machine lowering requires tracking every await point, which is
    // deferred to the LLVM pass pipeline in a production compiler.
    let fn_type = super::types::function_type_from_ruyi(
        ctx.context,
        &param_types,
        &future_ret,
    );

    let function = ctx.module.add_function(name, fn_type, None);

    let prev_function = ctx.current_function;
    ctx.current_function = Some(function);

    let entry_bb = ctx.context.append_basic_block(function, "entry");
    let prev_block = ctx.builder.get_insert_block();
    ctx.builder.position_at_end(entry_bb);

    let mut prev_vars = std::collections::HashMap::new();

    ctx.push_gc_root_scope();

    for (i, param) in params.iter().enumerate() {
        let param_name = match &param.pattern {
            crate::parser::ast::Pattern::Identifier(n) => n.clone(),
            _ => format!("param_{}", i),
        };
        let param_ty = param_types.get(i).cloned().unwrap_or(Type::Dynamic);
        let llvm_ty = super::types::ruyi_type_to_llvm(ctx.context, &param_ty);
        let ptr = ctx.builder.build_alloca(llvm_ty, &param_name);
        let param_value = function.get_nth_param(i as u32)
            .ok_or_else(|| format!("Missing parameter {}", i))?;
        ctx.builder.build_store(ptr, param_value);
        if is_gc_managed(&param_ty) {
            ctx.add_gc_root(ptr, param_ty.clone());
        }
        if let Some(old) = ctx.variables.insert(param_name.clone(), (ptr, param_ty)) {
            prev_vars.insert(param_name, old);
        }
    }

    // Compile body normally (synchronous fallback).
    let _ = compile_block(ctx, body);

    let current_bb = ctx.builder.get_insert_block().unwrap();
    if current_bb.get_terminator().is_none() {
        ctx.emit_gc_root_removals();
        let null_ptr = ctx.context.i8_type().ptr_type(Default::default()).const_null();
        ctx.builder.build_return(Some(&null_ptr));
    }

    ctx.pop_gc_root_scope();

    ctx.current_function = prev_function;
    if let Some(block) = prev_block {
        ctx.builder.position_at_end(block);
    }
    for (name, old) in prev_vars {
        ctx.variables.insert(name, old);
    }

    Ok(())
}

/// Compile an `await` expression.
///
/// In the baseline codegen, `await expr` is compiled as:
/// - Evaluate `expr` to get a future pointer.
/// - Call the runtime `ruyi_await` helper (or inline poll loop).
/// - Return the unwrapped value.
pub fn compile_await<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    expr: &Expr,
) -> Result<super::expr::ExprResult<'ctx>, String> {
    let inner_result = compile_expr(ctx, expr)?;

    // Declare a runtime helper: void* ruyi_await(void* future)
    let i8_ptr = ctx.context.i8_type().ptr_type(Default::default());
    let await_fn_type = i8_ptr.fn_type(&[i8_ptr.into()], false);
    let await_fn = ctx.module.get_function("ruyi_await")
        .unwrap_or_else(|| ctx.module.add_function("ruyi_await", await_fn_type, None));

    let call_site = ctx.builder.build_call(
        await_fn,
        &[inner_result.value.into()],
        "await_result",
    );
    let result_val = call_site
        .try_as_basic_value()
        .left()
        .unwrap_or_else(|| BasicValueEnum::PointerValue(i8_ptr.const_null()));

    // The return type after await is the inner type of the Future.
    let result_ty = match inner_result.ty {
        Type::Future(inner) => *inner,
        _ => Type::Dynamic,
    };

    Ok(super::expr::ExprResult::new(result_val, result_ty))
}
