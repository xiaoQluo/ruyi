/**
 * Statement code generation for Ruyi.
 *
 * Lowers Ruyi AST statements to LLVM IR instructions.
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */

use inkwell::types::BasicTypeEnum;
use inkwell::values::BasicValueEnum;

use crate::parser::ast::{Expr, Statement};
use crate::typechecker::types::Type;
use super::builtins::{build_ruyi_clear_pending_exception, build_ruyi_get_pending_exception};
use super::expr::compile_expr;
use super::generator::{CodegenContext, TryContext};

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
        Statement::If { condition, then_branch, else_branch } => {
            compile_if(ctx, condition, then_branch, else_branch.as_deref())
        }
        Statement::While { condition, body } => compile_while(ctx, condition, body),
        Statement::Return(expr) => compile_return(ctx, expr.as_deref()),
        Statement::Declaration(decl) => super::decl::compile_declaration(ctx, decl),
        Statement::Throw(expr) => compile_throw(ctx, expr),
        Statement::Try { body, catch, finally } => compile_try(ctx, body, catch.as_ref(), finally.as_deref()),
        Statement::Empty => Ok(()),
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

    ctx.builder.build_conditional_branch(cond_val, then_bb, else_bb);

    ctx.builder.position_at_end(then_bb);
    compile_stmt(ctx, then_branch)?;
    if ctx.builder.get_insert_block().unwrap().get_terminator().is_none() {
        ctx.builder.build_unconditional_branch(merge_bb);
    }

    ctx.builder.position_at_end(else_bb);
    if let Some(else_stmt) = else_branch {
        compile_stmt(ctx, else_stmt)?;
    }
    if ctx.builder.get_insert_block().unwrap().get_terminator().is_none() {
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
    ctx.builder.build_conditional_branch(cond_val, body_bb, end_bb);

    ctx.builder.position_at_end(body_bb);
    compile_stmt(ctx, body)?;
    if ctx.builder.get_insert_block().unwrap().get_terminator().is_none() {
        ctx.builder.build_unconditional_branch(cond_bb);
    }

    ctx.builder.position_at_end(end_bb);

    ctx.loop_stack.pop();

    Ok(())
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

fn compile_throw<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    expr: &Expr,
) -> Result<(), String> {
    let exc_result = compile_expr(ctx, expr)?;
    let exc_ptr = match exc_result.value {
        BasicValueEnum::PointerValue(v) => v,
        _ => return Err("throw expression must evaluate to a pointer".to_string()),
    };

    let throw_fn = ctx.module.get_function("ruyi_throw").expect("ruyi_throw not declared");
    ctx.builder.build_call(throw_fn, &[exc_ptr.into()], "throw");

    if let Some(try_ctx) = ctx.try_stack.last() {
        ctx.builder.build_store(try_ctx.exception_ptr, exc_ptr);
        if let Some(catch_bb) = try_ctx.catch_bb {
            ctx.builder.build_unconditional_branch(catch_bb);
        } else if let Some(finally_bb) = try_ctx.finally_bb {
            ctx.builder.build_unconditional_branch(finally_bb);
        } else {
            ctx.builder.build_unconditional_branch(try_ctx.merge_bb);
        }
    } else {
        ctx.emit_gc_root_removals();
        if let Some(func) = ctx.current_function {
            let fn_type = func.get_type();
            let ret_ty = fn_type.get_return_type();
            match ret_ty {
                None => {
                    ctx.builder.build_return(None);
                }
                Some(ty) => {
                    match ty {
                        BasicTypeEnum::IntType(t) => {
                            let zero = t.const_int(0, false);
                            ctx.builder.build_return(Some(&BasicValueEnum::IntValue(zero)));
                        }
                        BasicTypeEnum::FloatType(t) => {
                            let zero = t.const_float(0.0);
                            ctx.builder.build_return(Some(&BasicValueEnum::FloatValue(zero)));
                        }
                        BasicTypeEnum::PointerType(t) => {
                            let null = t.const_null();
                            ctx.builder.build_return(Some(&BasicValueEnum::PointerValue(null)));
                        }
                        _ => {
                            ctx.builder.build_return(None);
                        }
                    }
                }
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
    catch: Option<&crate::parser::ast::CatchClause>,
    finally: Option<&[Statement]>,
) -> Result<(), String> {
    let func = ctx.current_function.ok_or("No current function")?;
    let i8_ptr = ctx.context.i8_type().ptr_type(Default::default());
    let i64_ty = ctx.context.i64_type();

    let try_body_bb = ctx.context.append_basic_block(func, "try_body");
    let merge_bb = ctx.context.append_basic_block(func, "try_merge");

    let catch_bb = catch.map(|_| ctx.context.append_basic_block(func, "try_catch"));
    let finally_bb = finally.map(|_| ctx.context.append_basic_block(func, "try_finally"));
    let propagate_bb = if finally.is_some() && catch.is_none() {
        Some(ctx.context.append_basic_block(func, "try_propagate"))
    } else {
        None
    };

    let exception_ptr = ctx.builder.build_alloca(i8_ptr, "exc_ptr");
    ctx.builder.build_store(exception_ptr, i8_ptr.const_null());

    let clear_fn = ctx.module.get_function("ruyi_clear_pending_exception")
        .expect("ruyi_clear_pending_exception not declared");
    ctx.builder.build_call(clear_fn, &[], "clear_exc");

    ctx.builder.build_unconditional_branch(try_body_bb);
    ctx.builder.position_at_end(try_body_bb);

    let try_ctx = TryContext {
        exception_ptr,
        catch_bb,
        finally_bb,
        merge_bb,
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

    if let Some(catch_clause) = catch {
        let cb = catch_bb.unwrap();
        ctx.builder.position_at_end(cb);

        build_ruyi_clear_pending_exception(&ctx.builder, &ctx.module);

        let exc_val = ctx.builder.build_load(exception_ptr, "exc_val").into_pointer_value();
        if let Some(pattern) = &catch_clause.pattern {
            match pattern {
                crate::parser::ast::Pattern::Identifier(name) => {
                    let local_ptr = ctx.builder.build_alloca(i8_ptr, name);
                    ctx.builder.build_store(local_ptr, exc_val);
                    ctx.variables.insert(name.clone(), (local_ptr, Type::String));
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
            if catch.is_none() {
                let exc_val = ctx.builder.build_load(exception_ptr, "exc_val").into_pointer_value();
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
                let exc_val2 = ctx.builder.build_load(exception_ptr, "exc_val2").into_pointer_value();
                let throw_fn = ctx.module.get_function("ruyi_throw").expect("ruyi_throw not declared");
                ctx.builder.build_call(throw_fn, &[exc_val2.into()], "rethrow");
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

fn build_exception_check<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
) -> Result<(), String> {
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

    let dest_bb = try_ctx.catch_bb
        .or(try_ctx.finally_bb)
        .unwrap_or(try_ctx.merge_bb);

    ctx.builder.build_conditional_branch(is_null, continue_bb, store_exc_bb);

    ctx.builder.position_at_end(store_exc_bb);
    ctx.builder.build_store(try_ctx.exception_ptr, pending);
    build_ruyi_clear_pending_exception(&ctx.builder, &ctx.module);
    ctx.builder.build_unconditional_branch(dest_bb);

    ctx.builder.position_at_end(continue_bb);
    Ok(())
}
