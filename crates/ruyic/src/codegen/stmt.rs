/**
 * Statement code generation for Ruyi.
 *
 * Lowers Ruyi AST statements to LLVM IR instructions.
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicValue, BasicValueEnum};

use super::builtins::{build_ruyi_clear_pending_exception, build_ruyi_get_pending_exception};
use super::expr::compile_expr;
use super::generator::{CodegenContext, TryContext};
use crate::parser::ast::{Expr, Statement};
use crate::typechecker::types::Type;

pub fn compile_stmt<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    stmt: &Statement,
) -> Result<(), String> {
    match stmt {
        Statement::Expression(expr) => {
            compile_expr(ctx, expr)?;
            build_exception_check(ctx)?;
            Ok(())
        }
        Statement::Block(stmts) => compile_block(ctx, stmts),
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => compile_if(ctx, condition, then_branch, else_branch.as_deref()),
        Statement::While { condition, body } => compile_while(ctx, condition, body),
        Statement::For {
            init,
            condition,
            update,
            body,
        } => compile_for(
            ctx,
            init.as_ref(),
            condition.as_deref(),
            update.as_deref(),
            body,
        ),
        Statement::ForIn {
            variable,
            iterable,
            body,
        } => compile_for_in(ctx, variable, iterable, body),
        Statement::ForOf {
            variable,
            iterable,
            body,
            ..
        } => compile_for_of(ctx, variable, iterable, body),
        Statement::Return(expr) => compile_return(ctx, expr.as_deref()),
        Statement::Declaration(decl) => super::decl::compile_declaration(ctx, decl),
        Statement::Throw(expr) => compile_throw(ctx, expr),
        Statement::Try {
            body,
            catch,
            finally,
        } => compile_try(ctx, body, catch, finally.as_deref()),
        Statement::Empty => Ok(()),
        Statement::Yield(_) => {
            // Generators not yet fully implemented — yield is a no-op for now
            Ok(())
        }
        Statement::Break(_) => {
            let (end_bb, _) = ctx
                .loop_stack
                .last()
                .ok_or("BreakOutsideLoop: break statement must be inside a loop")?;
            ctx.builder.build_unconditional_branch(*end_bb);
            Ok(())
        }
        Statement::Continue(_) => {
            let (_, cond_bb) = ctx
                .loop_stack
                .last()
                .ok_or("ContinueOutsideLoop: continue statement must be inside a loop")?;
            ctx.builder.build_unconditional_branch(*cond_bb);
            Ok(())
        }
        Statement::Match { value, arms } => super::patterns::compile_match_stmt(ctx, value, arms),
        Statement::Labeled { body, .. } => compile_stmt(ctx, body),
        _ => Err(format!("Unsupported statement: {:?}", stmt)),
    }
}

pub fn compile_block<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    stmts: &[Statement],
) -> Result<(), String> {
    for stmt in stmts {
        compile_stmt(ctx, stmt)?;
        if let Some(bb) = ctx.builder.get_insert_block() {
            if bb.get_terminator().is_some() {
                break;
            }
        }
    }
    Ok(())
}

fn compile_if<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    condition: &crate::parser::ast::Expr,
    then_branch: &Statement,
    else_branch: Option<&Statement>,
) -> Result<(), String> {
    let cond_result = compile_expr(ctx, condition)?;
    let cond_val = match cond_result.value {
        BasicValueEnum::IntValue(v) => v,
        _ => return Err("Condition must be boolean".to_string()),
    };

    let func = ctx.current_function.ok_or("No current function")?;

    let then_bb = ctx.context.append_basic_block(func, "if_then");
    let else_bb = ctx.context.append_basic_block(func, "if_else");
    let merge_bb = ctx.context.append_basic_block(func, "if_merge");

    ctx.builder
        .build_conditional_branch(cond_val, then_bb, else_bb);

    ctx.builder.position_at_end(then_bb);
    compile_stmt(ctx, then_branch)?;
    if ctx
        .builder
        .get_insert_block()
        .unwrap()
        .get_terminator()
        .is_none()
    {
        ctx.builder.build_unconditional_branch(merge_bb);
    }

    ctx.builder.position_at_end(else_bb);
    if let Some(else_stmt) = else_branch {
        compile_stmt(ctx, else_stmt)?;
    }
    if ctx
        .builder
        .get_insert_block()
        .unwrap()
        .get_terminator()
        .is_none()
    {
        ctx.builder.build_unconditional_branch(merge_bb);
    }

    ctx.builder.position_at_end(merge_bb);

    Ok(())
}

fn compile_while<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    condition: &crate::parser::ast::Expr,
    body: &Statement,
) -> Result<(), String> {
    let func = ctx.current_function.ok_or("No current function")?;

    let cond_bb = ctx.context.append_basic_block(func, "while_cond");
    let body_bb = ctx.context.append_basic_block(func, "while_body");
    let end_bb = ctx.context.append_basic_block(func, "while_end");

    ctx.loop_stack.push((end_bb, cond_bb));

    ctx.builder.build_unconditional_branch(cond_bb);

    ctx.builder.position_at_end(cond_bb);
    let cond_result = compile_expr(ctx, condition)?;
    let cond_val = match cond_result.value {
        BasicValueEnum::IntValue(v) => v,
        _ => return Err("Condition must be boolean".to_string()),
    };
    ctx.builder
        .build_conditional_branch(cond_val, body_bb, end_bb);

    ctx.builder.position_at_end(body_bb);
    compile_stmt(ctx, body)?;
    if ctx
        .builder
        .get_insert_block()
        .unwrap()
        .get_terminator()
        .is_none()
    {
        ctx.builder.build_unconditional_branch(cond_bb);
    }

    ctx.builder.position_at_end(end_bb);

    ctx.loop_stack.pop();

    Ok(())
}

fn compile_for<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    init: Option<&crate::parser::ast::ForInit>,
    condition: Option<&crate::parser::ast::Expr>,
    update: Option<&crate::parser::ast::Expr>,
    body: &Statement,
) -> Result<(), String> {
    let func = ctx.current_function.ok_or("No current function")?;

    let mut prev_vars = std::collections::HashMap::new();
    let mut declared_names = Vec::new();
    if let Some(crate::parser::ast::ForInit::VarDecl(decl)) = init {
        if let crate::parser::ast::Declaration::Let(bindings)
        | crate::parser::ast::Declaration::Const(bindings) = decl
        {
            for binding in bindings {
                if let crate::parser::ast::Pattern::Identifier(name) = &binding.pattern {
                    declared_names.push(name.clone());
                    if let Some(old) = ctx.variables.get(name).cloned() {
                        prev_vars.insert(name.clone(), old);
                    }
                }
            }
        }
        super::decl::compile_declaration(ctx, decl)?;
    } else if let Some(crate::parser::ast::ForInit::Expr(expr)) = init {
        compile_expr(ctx, expr)?;
    }

    let cond_bb = ctx.context.append_basic_block(func, "for_cond");
    let body_bb = ctx.context.append_basic_block(func, "for_body");
    let update_bb = ctx.context.append_basic_block(func, "for_update");
    let end_bb = ctx.context.append_basic_block(func, "for_end");

    ctx.builder.build_unconditional_branch(cond_bb);

    ctx.builder.position_at_end(cond_bb);
    if let Some(cond) = condition {
        let cond_result = compile_expr(ctx, cond)?;
        let cond_val = match cond_result.value {
            BasicValueEnum::IntValue(v) => v,
            _ => return Err("Condition must be boolean".to_string()),
        };
        ctx.builder
            .build_conditional_branch(cond_val, body_bb, end_bb);
    } else {
        ctx.builder.build_unconditional_branch(body_bb);
    }

    ctx.loop_stack.push((end_bb, update_bb));

    ctx.builder.position_at_end(body_bb);
    compile_stmt(ctx, body)?;
    if ctx
        .builder
        .get_insert_block()
        .unwrap()
        .get_terminator()
        .is_none()
    {
        ctx.builder.build_unconditional_branch(update_bb);
    }

    ctx.builder.position_at_end(update_bb);
    if let Some(upd) = update {
        compile_expr(ctx, upd)?;
    }
    if ctx
        .builder
        .get_insert_block()
        .unwrap()
        .get_terminator()
        .is_none()
    {
        ctx.builder.build_unconditional_branch(cond_bb);
    }

    ctx.builder.position_at_end(end_bb);
    ctx.loop_stack.pop();

    for name in declared_names {
        if let Some(old) = prev_vars.remove(&name) {
            ctx.variables.insert(name, old);
        } else {
            ctx.variables.remove(&name);
        }
    }

    Ok(())
}

fn compile_for_in<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    variable: &str,
    iterable: &crate::parser::ast::Expr,
    body: &Statement,
) -> Result<(), String> {
    let func = ctx.current_function.ok_or("No current function")?;
    let i8_ptr = ctx.context.i8_type().ptr_type(Default::default());
    let i64_ty = ctx.context.i64_type();
    let i64_ptr_ty = i64_ty.ptr_type(Default::default());

    let iter_result = compile_expr(ctx, iterable)?;
    let obj_ptr = iter_result.value.into_pointer_value();

    let keys_fn = ctx
        .module
        .get_function("ruyi_obj_keys")
        .expect("ruyi_obj_keys not declared");
    let keys_arr = ctx
        .builder
        .build_call(keys_fn, &[obj_ptr.into()], "keys_arr")
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_pointer_value();

    let len_ptr = ctx
        .builder
        .build_bitcast(keys_arr, i64_ptr_ty, "len_ptr")
        .into_pointer_value();
    let len = ctx.builder.build_load(len_ptr, "len").into_int_value();

    let idx_ptr = ctx.builder.build_alloca(i64_ty, "for_in_idx");
    ctx.builder.build_store(idx_ptr, i64_ty.const_int(0, false));

    let var_ptr = ctx.builder.build_alloca(i8_ptr, variable);
    let old_var = ctx
        .variables
        .insert(variable.to_string(), (var_ptr, Type::String));

    let cond_bb = ctx.context.append_basic_block(func, "for_in_cond");
    let body_bb = ctx.context.append_basic_block(func, "for_in_body");
    let end_bb = ctx.context.append_basic_block(func, "for_in_end");

    ctx.builder.build_unconditional_branch(cond_bb);

    ctx.builder.position_at_end(cond_bb);
    let idx = ctx.builder.build_load(idx_ptr, "idx").into_int_value();
    let cond = ctx
        .builder
        .build_int_compare(inkwell::IntPredicate::SLT, idx, len, "for_in_cond");
    ctx.builder.build_conditional_branch(cond, body_bb, end_bb);

    ctx.loop_stack.push((end_bb, cond_bb));

    ctx.builder.position_at_end(body_bb);
    let idx = ctx.builder.build_load(idx_ptr, "idx").into_int_value();
    let one = i64_ty.const_int(1, false);
    let elem_idx = ctx.builder.build_int_add(idx, one, "elem_idx");
    let elem_offset =
        ctx.builder
            .build_int_mul(elem_idx, i64_ty.const_int(8, false), "elem_offset");
    let elem_offset_i32 =
        ctx.builder
            .build_int_cast(elem_offset, ctx.context.i32_type(), "elem_offset_i32");
    let elem_ptr = unsafe {
        ctx.builder
            .build_gep(keys_arr, &[elem_offset_i32], "elem_ptr")
    };
    let elem_i64_ptr = ctx
        .builder
        .build_bitcast(elem_ptr, i64_ptr_ty, "elem_i64_ptr")
        .into_pointer_value();
    let key_val = ctx.builder.build_load(elem_i64_ptr, "key_val");
    ctx.builder.build_store(var_ptr, key_val);

    compile_stmt(ctx, body)?;

    if ctx
        .builder
        .get_insert_block()
        .unwrap()
        .get_terminator()
        .is_none()
    {
        let next_idx = ctx.builder.build_int_add(idx, one, "next_idx");
        ctx.builder.build_store(idx_ptr, next_idx);
        ctx.builder.build_unconditional_branch(cond_bb);
    }

    ctx.builder.position_at_end(end_bb);
    ctx.loop_stack.pop();

    if let Some(old) = old_var {
        ctx.variables.insert(variable.to_string(), old);
    } else {
        ctx.variables.remove(variable);
    }

    Ok(())
}

fn compile_for_of<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    variable: &str,
    iterable: &crate::parser::ast::Expr,
    body: &Statement,
) -> Result<(), String> {
    let func = ctx.current_function.ok_or("No current function")?;
    let i8_ptr = ctx.context.i8_type().ptr_type(Default::default());
    let i64_ty = ctx.context.i64_type();
    let i64_ptr_ty = i64_ty.ptr_type(Default::default());

    let iter_result = compile_expr(ctx, iterable)?;

    match &iter_result.ty {
        Type::Array(_) => {
            let array_ptr = iter_result.value.into_pointer_value();

            let len_ptr = ctx
                .builder
                .build_bitcast(array_ptr, i64_ptr_ty, "len_ptr")
                .into_pointer_value();
            let len = ctx.builder.build_load(len_ptr, "len").into_int_value();

            let idx_ptr = ctx.builder.build_alloca(i64_ty, "for_of_idx");
            let var_ptr = ctx.builder.build_alloca(i64_ty, variable);
            ctx.builder.build_store(idx_ptr, i64_ty.const_int(0, false));
            let old_var = ctx
                .variables
                .insert(variable.to_string(), (var_ptr, Type::Int));

            let cond_bb = ctx.context.append_basic_block(func, "for_of_cond");
            let body_bb = ctx.context.append_basic_block(func, "for_of_body");
            let end_bb = ctx.context.append_basic_block(func, "for_of_end");

            ctx.builder.build_unconditional_branch(cond_bb);

            ctx.builder.position_at_end(cond_bb);
            let idx = ctx.builder.build_load(idx_ptr, "idx").into_int_value();
            let cond =
                ctx.builder
                    .build_int_compare(inkwell::IntPredicate::SLT, idx, len, "for_of_cond");
            ctx.builder.build_conditional_branch(cond, body_bb, end_bb);

            ctx.loop_stack.push((end_bb, cond_bb));

            ctx.builder.position_at_end(body_bb);
            let idx = ctx.builder.build_load(idx_ptr, "idx").into_int_value();
            let one = i64_ty.const_int(1, false);
            let elem_idx = ctx.builder.build_int_add(idx, one, "elem_idx");
            let elem_offset =
                ctx.builder
                    .build_int_mul(elem_idx, i64_ty.const_int(8, false), "elem_offset");
            let elem_offset_i32 =
                ctx.builder
                    .build_int_cast(elem_offset, ctx.context.i32_type(), "elem_offset_i32");
            let elem_ptr = unsafe {
                ctx.builder
                    .build_gep(array_ptr, &[elem_offset_i32], "elem_ptr")
            };
            let elem_i64_ptr = ctx
                .builder
                .build_bitcast(elem_ptr, i64_ptr_ty, "elem_i64_ptr")
                .into_pointer_value();
            let elem_val = ctx.builder.build_load(elem_i64_ptr, "elem_val");
            ctx.builder.build_store(var_ptr, elem_val);

            compile_stmt(ctx, body)?;

            if ctx
                .builder
                .get_insert_block()
                .unwrap()
                .get_terminator()
                .is_none()
            {
                let next_idx = ctx.builder.build_int_add(idx, one, "next_idx");
                ctx.builder.build_store(idx_ptr, next_idx);
                ctx.builder.build_unconditional_branch(cond_bb);
            }

            ctx.builder.position_at_end(end_bb);
            ctx.loop_stack.pop();

            if let Some(old) = old_var {
                ctx.variables.insert(variable.to_string(), old);
            } else {
                ctx.variables.remove(variable);
            }

            Ok(())
        }
        _ => {
            let iterable_ptr = iter_result.value.into_pointer_value();

            let temp_name = "__for_of_iterable";
            let temp_ptr = ctx.builder.build_alloca(i8_ptr, temp_name);
            ctx.builder.build_store(temp_ptr, iterable_ptr);
            let old_temp = ctx
                .variables
                .insert(temp_name.to_string(), (temp_ptr, iter_result.ty.clone()));

            let iter_call = crate::parser::ast::Expr::Call {
                callee: Box::new(crate::parser::ast::Expr::Member {
                    object: Box::new(crate::parser::ast::Expr::Identifier(temp_name.to_string())),
                    property: crate::parser::ast::MemberProperty::Ident("iter".to_string()),
                    optional: false,
                }),
                args: vec![],
            };
            let iter_obj_result = compile_expr(ctx, &iter_call)?;

            let iter_name = "__for_of_iterator";
            let iter_ptr = ctx.builder.build_alloca(i8_ptr, iter_name);
            ctx.builder.build_store(iter_ptr, iter_obj_result.value);
            let old_iter = ctx
                .variables
                .insert(iter_name.to_string(), (iter_ptr, Type::Dynamic));

            let cond_bb = ctx.context.append_basic_block(func, "for_of_cond");
            let body_bb = ctx.context.append_basic_block(func, "for_of_body");
            let end_bb = ctx.context.append_basic_block(func, "for_of_end");

            ctx.builder.build_unconditional_branch(cond_bb);

            ctx.builder.position_at_end(cond_bb);
            let next_call = crate::parser::ast::Expr::Call {
                callee: Box::new(crate::parser::ast::Expr::Member {
                    object: Box::new(crate::parser::ast::Expr::Identifier(iter_name.to_string())),
                    property: crate::parser::ast::MemberProperty::Ident("next".to_string()),
                    optional: false,
                }),
                args: vec![],
            };
            let next_result = compile_expr(ctx, &next_call)?;
            let next_ptr = next_result.value.into_pointer_value();
            let next_int = ctx.builder.build_ptr_to_int(next_ptr, i64_ty, "next_int");
            let is_null = ctx.builder.build_int_compare(
                inkwell::IntPredicate::EQ,
                next_int,
                i64_ty.const_int(0, false),
                "is_null",
            );
            ctx.builder
                .build_conditional_branch(is_null, end_bb, body_bb);

            ctx.loop_stack.push((end_bb, cond_bb));

            ctx.builder.position_at_end(body_bb);
            let var_ptr = ctx.builder.build_alloca(i8_ptr, variable);
            ctx.builder.build_store(var_ptr, next_result.value);
            let old_var = ctx
                .variables
                .insert(variable.to_string(), (var_ptr, Type::Dynamic));

            compile_stmt(ctx, body)?;

            if ctx
                .builder
                .get_insert_block()
                .unwrap()
                .get_terminator()
                .is_none()
            {
                ctx.builder.build_unconditional_branch(cond_bb);
            }

            ctx.builder.position_at_end(end_bb);
            ctx.loop_stack.pop();

            if let Some(old) = old_var {
                ctx.variables.insert(variable.to_string(), old);
            } else {
                ctx.variables.remove(variable);
            }
            if let Some(old) = old_iter {
                ctx.variables.insert(iter_name.to_string(), old);
            } else {
                ctx.variables.remove(iter_name);
            }
            if let Some(old) = old_temp {
                ctx.variables.insert(temp_name.to_string(), old);
            } else {
                ctx.variables.remove(temp_name);
            }

            Ok(())
        }
    }
}

fn compile_return<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    expr: Option<&crate::parser::ast::Expr>,
) -> Result<(), String> {
    if let (Some(result_ptr), Some(return_bb)) = (ctx.async_result_ptr, ctx.async_return_bb) {
        if let Some(e) = expr {
            let result = compile_expr(ctx, e)?;
            ctx.builder.build_store(result_ptr, result.value);
        }
        ctx.builder.build_unconditional_branch(return_bb);
        return Ok(());
    }

    ctx.emit_gc_root_removals();
    match expr {
        Some(e) => {
            let result = compile_expr(ctx, e)?;
            ctx.builder.build_return(Some(&result.value));
            Ok(())
        }
        None => {
            ctx.builder.build_return(None);
            Ok(())
        }
    }
}

fn compile_throw<'ctx>(ctx: &mut CodegenContext<'ctx, '_>, expr: &Expr) -> Result<(), String> {
    let exc_result = compile_expr(ctx, expr)?;
    let exc_ptr = match exc_result.value {
        BasicValueEnum::PointerValue(v) => v,
        _ => return Err("throw expression must evaluate to a pointer".to_string()),
    };

    let throw_fn = ctx
        .module
        .get_function("ruyi_throw")
        .expect("ruyi_throw not declared");

    if let Some(try_ctx) = ctx.try_stack.last() {
        if let Some(lpad_bb) = try_ctx.landing_pad_bb {
            let func = ctx.current_function.ok_or("No current function")?;
            let unreachable_bb = ctx.context.append_basic_block(func, "throw_unreachable");
            ctx.builder.build_invoke(
                throw_fn,
                &[exc_ptr.into()],
                unreachable_bb,
                lpad_bb,
                "throw",
            );
            ctx.builder.position_at_end(unreachable_bb);
            ctx.builder.build_unreachable();
        } else {
            ctx.builder.build_call(throw_fn, &[exc_ptr.into()], "throw");
            ctx.builder.build_store(try_ctx.exception_ptr, exc_ptr);
            if let Some(catch_bb) = try_ctx.catch_bb {
                ctx.builder.build_unconditional_branch(catch_bb);
            } else if let Some(finally_bb) = try_ctx.finally_bb {
                ctx.builder.build_unconditional_branch(finally_bb);
            } else {
                ctx.builder.build_unconditional_branch(try_ctx.merge_bb);
            }
        }
    } else {
        ctx.builder.build_call(throw_fn, &[exc_ptr.into()], "throw");
        ctx.emit_gc_root_removals();
        if let Some(func) = ctx.current_function {
            let fn_type = func.get_type();
            let ret_ty = fn_type.get_return_type();
            match ret_ty {
                None => {
                    ctx.builder.build_return(None);
                }
                Some(ty) => match ty {
                    BasicTypeEnum::IntType(t) => {
                        let zero = t.const_int(0, false);
                        ctx.builder
                            .build_return(Some(&BasicValueEnum::IntValue(zero)));
                    }
                    BasicTypeEnum::FloatType(t) => {
                        let zero = t.const_float(0.0);
                        ctx.builder
                            .build_return(Some(&BasicValueEnum::FloatValue(zero)));
                    }
                    BasicTypeEnum::PointerType(t) => {
                        let null = t.const_null();
                        ctx.builder
                            .build_return(Some(&BasicValueEnum::PointerValue(null)));
                    }
                    _ => {
                        ctx.builder.build_return(None);
                    }
                },
            }
        } else {
            return Err("throw outside function".to_string());
        }
    }

    Ok(())
}

fn compile_try<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    body: &[Statement],
    catch: &[crate::parser::ast::CatchClause],
    finally: Option<&[Statement]>,
) -> Result<(), String> {
    let func = ctx.current_function.ok_or("No current function")?;
    let i8_ptr = ctx.context.i8_type().ptr_type(Default::default());
    let i64_ty = ctx.context.i64_type();

    let try_body_bb = ctx.context.append_basic_block(func, "try_body");
    let merge_bb = ctx.context.append_basic_block(func, "try_merge");

    let catch_bb = if !catch.is_empty() {
        Some(ctx.context.append_basic_block(func, "try_catch"))
    } else {
        None
    };
    let finally_bb = finally.map(|_| ctx.context.append_basic_block(func, "try_finally"));
    let propagate_bb = if finally.is_some() && catch.is_empty() {
        Some(ctx.context.append_basic_block(func, "try_propagate"))
    } else {
        None
    };
    let landing_pad_bb = if !catch.is_empty() || finally.is_some() {
        Some(ctx.context.append_basic_block(func, "try_lpad"))
    } else {
        None
    };

    let exception_ptr = ctx.builder.build_alloca(i8_ptr, "exc_ptr");
    ctx.builder.build_store(exception_ptr, i8_ptr.const_null());

    let clear_fn = ctx
        .module
        .get_function("ruyi_clear_pending_exception")
        .expect("ruyi_clear_pending_exception not declared");
    ctx.builder.build_call(clear_fn, &[], "clear_exc");

    ctx.builder.build_unconditional_branch(try_body_bb);
    ctx.builder.position_at_end(try_body_bb);

    let try_ctx = TryContext {
        exception_ptr,
        catch_bb,
        finally_bb,
        merge_bb,
        landing_pad_bb,
    };
    ctx.try_stack.push(try_ctx);

    compile_block(ctx, body)?;

    let body_end = ctx.builder.get_insert_block().unwrap();
    if body_end.get_terminator().is_none() {
        if let Some(fb) = finally_bb {
            ctx.builder.build_unconditional_branch(fb);
        } else {
            ctx.builder.build_unconditional_branch(merge_bb);
        }
    }

    ctx.try_stack.pop();

    if let Some(lpad_bb) = landing_pad_bb {
        ctx.builder.position_at_end(lpad_bb);
        let i32_ty = ctx.context.i32_type();
        let lpad_ty = ctx
            .context
            .struct_type(&[i8_ptr.into(), i32_ty.into()], false);
        let personality_ty = i32_ty.fn_type(&[], false);
        let personality = ctx
            .module
            .get_function("__gxx_personality_v0")
            .unwrap_or_else(|| {
                ctx.module
                    .add_function("__gxx_personality_v0", personality_ty, None)
            });
        let null_clause = i8_ptr.const_null().as_basic_value_enum();
        let lpad = ctx.builder.build_landing_pad(
            lpad_ty,
            personality,
            &[null_clause],
            finally.is_some(),
            "lpad",
        );
        let exc_val = ctx
            .builder
            .build_extract_value(lpad.into_struct_value(), 0, "exc_val")
            .unwrap()
            .into_pointer_value();
        ctx.builder.build_store(exception_ptr, exc_val);
        if let Some(cb) = catch_bb {
            ctx.builder.build_unconditional_branch(cb);
        } else if let Some(fb) = finally_bb {
            ctx.builder.build_unconditional_branch(fb);
        } else {
            ctx.builder.build_unconditional_branch(merge_bb);
        }
    }

    for catch_clause in catch {
        let cb = catch_bb.unwrap();
        ctx.builder.position_at_end(cb);

        build_ruyi_clear_pending_exception(&ctx.builder, &ctx.module);

        let exc_val = ctx
            .builder
            .build_load(exception_ptr, "exc_val")
            .into_pointer_value();
        if let Some(pattern) = &catch_clause.pattern {
            match pattern {
                crate::parser::ast::Pattern::Identifier(name) => {
                    let local_ptr = ctx.builder.build_alloca(i8_ptr, name);
                    ctx.builder.build_store(local_ptr, exc_val);
                    ctx.variables
                        .insert(name.clone(), (local_ptr, Type::String));
                }
                _ => {}
            }
        }

        compile_block(ctx, &catch_clause.body)?;

        let catch_end = ctx.builder.get_insert_block().unwrap();
        if catch_end.get_terminator().is_none() {
            if let Some(fb) = finally_bb {
                ctx.builder.build_unconditional_branch(fb);
            } else {
                ctx.builder.build_unconditional_branch(merge_bb);
            }
        }
    }

    if let Some(finally_stmts) = finally {
        let fb = finally_bb.unwrap();
        ctx.builder.position_at_end(fb);

        compile_block(ctx, finally_stmts)?;

        let finally_end = ctx.builder.get_insert_block().unwrap();
        if finally_end.get_terminator().is_none() {
            if catch.is_empty() {
                let exc_val = ctx
                    .builder
                    .build_load(exception_ptr, "exc_val")
                    .into_pointer_value();
                let exc_int = ctx.builder.build_ptr_to_int(exc_val, i64_ty, "exc_int");
                let is_null = ctx.builder.build_int_compare(
                    inkwell::IntPredicate::EQ,
                    exc_int,
                    i64_ty.const_int(0, false),
                    "is_null",
                );

                let pb = propagate_bb.unwrap();
                ctx.builder.build_conditional_branch(is_null, merge_bb, pb);

                ctx.builder.position_at_end(pb);
                let exc_val2 = ctx
                    .builder
                    .build_load(exception_ptr, "exc_val2")
                    .into_pointer_value();
                let throw_fn = ctx
                    .module
                    .get_function("ruyi_throw")
                    .expect("ruyi_throw not declared");
                ctx.builder
                    .build_call(throw_fn, &[exc_val2.into()], "rethrow");
                ctx.emit_gc_root_removals();
                ctx.builder.build_return(None);
            } else {
                ctx.builder.build_unconditional_branch(merge_bb);
            }
        }
    }

    ctx.builder.position_at_end(merge_bb);
    Ok(())
}

fn build_exception_check<'ctx>(ctx: &mut CodegenContext<'ctx, '_>) -> Result<(), String> {
    if ctx.try_stack.is_empty() {
        return Ok(());
    }

    let func = ctx.current_function.unwrap();
    let pending = build_ruyi_get_pending_exception(&ctx.builder, &ctx.module);

    let i64_ty = ctx.context.i64_type();
    let pending_int = ctx.builder.build_ptr_to_int(pending, i64_ty, "pending_int");
    let is_null = ctx.builder.build_int_compare(
        inkwell::IntPredicate::EQ,
        pending_int,
        i64_ty.const_int(0, false),
        "no_exc",
    );

    let try_ctx = ctx.try_stack.last().unwrap();
    let continue_bb = ctx.context.append_basic_block(func, "after_exc_check");
    let store_exc_bb = ctx.context.append_basic_block(func, "store_exc");

    let dest_bb = try_ctx
        .catch_bb
        .or(try_ctx.finally_bb)
        .unwrap_or(try_ctx.merge_bb);

    ctx.builder
        .build_conditional_branch(is_null, continue_bb, store_exc_bb);

    ctx.builder.position_at_end(store_exc_bb);
    ctx.builder.build_store(try_ctx.exception_ptr, pending);
    build_ruyi_clear_pending_exception(&ctx.builder, &ctx.module);
    ctx.builder.build_unconditional_branch(dest_bb);

    ctx.builder.position_at_end(continue_bb);
    Ok(())
}
