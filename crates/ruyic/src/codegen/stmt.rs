/**
 * Statement code generation for Ruyi.
 *
 * Lowers Ruyi AST statements to LLVM IR instructions.
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */
use inkwell::types::BasicType;
use inkwell::values::BasicValueEnum;

use ruyi_exception::landing_pad::LandingPadGenerator;

use super::builtins::{build_ruyi_clear_pending_exception, build_ruyi_get_pending_exception};
use super::expr::{compile_expr, ExprResult};
use super::generator::{CodegenContext, TryContext, TryFrame, TryStackGuard};
use crate::parser::ast::{Expr, Statement};
use crate::typechecker::types::Type;

pub fn compile_stmt<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
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
        Statement::Break(target) => {
            let target_bb = match target {
                None => ctx
                    .loop_stack
                    .last()
                    .map(|(end_bb, _, _)| *end_bb)
                    .ok_or("BreakOutsideLoop: break statement must be inside a loop")?,
                Some(label) => find_loop_target(ctx, label.to_string(), |(end_bb, _, _)| *end_bb)?,
            };
            ctx.builder().build_unconditional_branch(target_bb);
            Ok(())
        }
        Statement::Continue(target) => {
            let target_bb = match target {
                None => ctx
                    .loop_stack
                    .last()
                    .map(|(_, cond_bb, _)| *cond_bb)
                    .ok_or("ContinueOutsideLoop: continue statement must be inside a loop")?,
                Some(label) => {
                    find_loop_target(ctx, label.to_string(), |(_, cond_bb, _)| *cond_bb)?
                }
            };
            ctx.builder().build_unconditional_branch(target_bb);
            Ok(())
        }
        Statement::Match { value, arms } => super::patterns::compile_match_stmt(ctx, value, arms),
        Statement::Labeled { label, body } => match body.as_ref() {
            Statement::For { .. }
            | Statement::ForIn { .. }
            | Statement::ForOf { .. }
            | Statement::While { .. } => {
                ctx.pending_loop_label = Some(label.clone());
                let result = compile_stmt(ctx, body);
                ctx.pending_loop_label = None;
                result
            }
            _ => compile_stmt(ctx, body),
        },
        Statement::IfLet {
            pattern,
            value,
            then_branch,
            else_branch,
        } => compile_if_let(ctx, pattern, value, then_branch, else_branch.as_deref()),
        Statement::WhileLet {
            pattern,
            value,
            body,
        } => compile_while_let(ctx, pattern, value, body),
    }
}

pub fn compile_block<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    stmts: &[Statement],
) -> Result<(), String> {
    for stmt in stmts {
        compile_stmt(ctx, stmt)?;
        if let Some(bb) = ctx.builder().get_insert_block() {
            if bb.get_terminator().is_some() {
                break;
            }
        }
    }
    Ok(())
}

fn compile_if<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    condition: &crate::parser::ast::Expr,
    then_branch: &Statement,
    else_branch: Option<&Statement>,
) -> Result<(), String> {
    let cond_result = compile_expr(ctx, condition)?;
    let cond_val = match cond_result.value {
        BasicValueEnum::IntValue(v) => v,
        _ => return Err("Condition must be boolean".to_string()),
    };

    let func = ctx.current_function().ok_or("No current function")?;

    let then_bb = ctx.context.append_basic_block(func, "if_then");
    let else_bb = ctx.context.append_basic_block(func, "if_else");
    let merge_bb = ctx.context.append_basic_block(func, "if_merge");

    ctx.builder()
        .build_conditional_branch(cond_val, then_bb, else_bb);

    ctx.builder().position_at_end(then_bb);
    compile_stmt(ctx, then_branch)?;
    if ctx
        .builder()
        .get_insert_block()
        .unwrap()
        .get_terminator()
        .is_none()
    {
        ctx.builder().build_unconditional_branch(merge_bb);
    }

    ctx.builder().position_at_end(else_bb);
    if let Some(else_stmt) = else_branch {
        compile_stmt(ctx, else_stmt)?;
    }
    if ctx
        .builder()
        .get_insert_block()
        .unwrap()
        .get_terminator()
        .is_none()
    {
        ctx.builder().build_unconditional_branch(merge_bb);
    }

    ctx.builder().position_at_end(merge_bb);

    Ok(())
}

fn compile_while<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    condition: &crate::parser::ast::Expr,
    body: &Statement,
) -> Result<(), String> {
    let func = ctx.current_function().ok_or("No current function")?;

    let cond_bb = ctx.context.append_basic_block(func, "while_cond");
    let body_bb = ctx.context.append_basic_block(func, "while_body");
    let end_bb = ctx.context.append_basic_block(func, "while_end");

    let label = ctx.pending_loop_label.take();
    ctx.push_loop(end_bb, cond_bb, label);

    ctx.builder().build_unconditional_branch(cond_bb);

    ctx.builder().position_at_end(cond_bb);
    let cond_result = compile_expr(ctx, condition)?;
    let cond_val = match cond_result.value {
        BasicValueEnum::IntValue(v) => v,
        _ => return Err("Condition must be boolean".to_string()),
    };
    ctx.builder()
        .build_conditional_branch(cond_val, body_bb, end_bb);

    ctx.builder().position_at_end(body_bb);
    compile_stmt(ctx, body)?;
    if ctx
        .builder()
        .get_insert_block()
        .unwrap()
        .get_terminator()
        .is_none()
    {
        ctx.builder().build_unconditional_branch(cond_bb);
    }

    ctx.builder().position_at_end(end_bb);

    ctx.pop_loop();

    Ok(())
}

fn compile_for<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    init: Option<&crate::parser::ast::ForInit>,
    condition: Option<&crate::parser::ast::Expr>,
    update: Option<&crate::parser::ast::Expr>,
    body: &Statement,
) -> Result<(), String> {
    let func = ctx.current_function().ok_or("No current function")?;

    let mut prev_vars = std::collections::HashMap::new();
    let mut declared_names = Vec::new();
    if let Some(crate::parser::ast::ForInit::VarDecl(decl)) = init {
        if let crate::parser::ast::Declaration::Let(bindings)
        | crate::parser::ast::Declaration::Const(bindings) = decl
        {
            for binding in bindings {
                if let crate::parser::ast::Pattern::Identifier(name) = &binding.pattern {
                    declared_names.push(name.clone());
                    if let Some(old) = ctx.lookup_variable(name) {
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

    ctx.builder().build_unconditional_branch(cond_bb);

    ctx.builder().position_at_end(cond_bb);
    if let Some(cond) = condition {
        let cond_result = compile_expr(ctx, cond)?;
        let cond_val = match cond_result.value {
            BasicValueEnum::IntValue(v) => v,
            _ => return Err("Condition must be boolean".to_string()),
        };
        ctx.builder()
            .build_conditional_branch(cond_val, body_bb, end_bb);
    } else {
        ctx.builder().build_unconditional_branch(body_bb);
    }

    let label = ctx.pending_loop_label.take();
    ctx.push_loop(end_bb, update_bb, label);

    ctx.builder().position_at_end(body_bb);
    compile_stmt(ctx, body)?;
    if ctx
        .builder()
        .get_insert_block()
        .unwrap()
        .get_terminator()
        .is_none()
    {
        ctx.builder().build_unconditional_branch(update_bb);
    }

    ctx.builder().position_at_end(update_bb);
    if let Some(upd) = update {
        compile_expr(ctx, upd)?;
    }
    if ctx
        .builder()
        .get_insert_block()
        .unwrap()
        .get_terminator()
        .is_none()
    {
        ctx.builder().build_unconditional_branch(cond_bb);
    }

    ctx.builder().position_at_end(end_bb);
    ctx.pop_loop();

    for name in declared_names {
        if let Some(old) = prev_vars.remove(&name) {
            ctx.define_variable(name, old);
        } else {
            ctx.remove_variable(&name);
        }
    }

    Ok(())
}

fn compile_for_in<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    variable: &str,
    iterable: &crate::parser::ast::Expr,
    body: &Statement,
) -> Result<(), String> {
    let func = ctx.current_function().ok_or("No current function")?;
    let i8_ptr = ctx.context.i8_type().ptr_type(Default::default());
    let i64_ty = ctx.context.i64_type();
    let i64_ptr_ty = i64_ty.ptr_type(Default::default());

    let iter_result = compile_expr(ctx, iterable)?;

    if let crate::typechecker::types::Type::Object(fields) = &iter_result.ty {
        let var_ptr = ctx.builder().build_alloca(i8_ptr, variable);
        let old_var = ctx
            .variables
            .insert(variable.to_string(), (var_ptr, Type::String));
        for f in fields {
            let s = ctx.builder().build_global_string_ptr(&f.name, "obj_key");
            ctx.builder().build_store(var_ptr, s.as_pointer_value());
            compile_stmt(ctx, body)?;
        }
        ctx.builder()
            .position_at_end(ctx.builder().get_insert_block().unwrap());
        if let Some(old) = old_var {
            ctx.define_variable(variable.to_string(), old);
        } else {
            ctx.remove_variable(variable);
        }
        return Ok(());
    }

    let obj_ptr = iter_result.value.into_pointer_value();

    let keys_fn = ctx
        .module
        .get_function("ruyi_obj_keys")
        .expect("ruyi_obj_keys not declared");
    let keys_arr = ctx
        .builder()
        .build_call(keys_fn, &[obj_ptr.into()], "keys_arr")
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_pointer_value();

    let len_ptr = ctx
        .builder()
        .build_bitcast(keys_arr, i64_ptr_ty, "len_ptr")
        .into_pointer_value();
    let len = ctx.builder().build_load(len_ptr, "len").into_int_value();

    let idx_ptr = ctx.builder().build_alloca(i64_ty, "for_in_idx");
    ctx.builder()
        .build_store(idx_ptr, i64_ty.const_int(0, false));

    let var_ptr = ctx.builder().build_alloca(i8_ptr, variable);
    let old_var = ctx
        .variables
        .insert(variable.to_string(), (var_ptr, Type::String));

    let cond_bb = ctx.context.append_basic_block(func, "for_in_cond");
    let body_bb = ctx.context.append_basic_block(func, "for_in_body");
    let end_bb = ctx.context.append_basic_block(func, "for_in_end");

    ctx.builder().build_unconditional_branch(cond_bb);

    ctx.builder().position_at_end(cond_bb);
    let idx = ctx.builder().build_load(idx_ptr, "idx").into_int_value();
    let cond = ctx
        .builder()
        .build_int_compare(inkwell::IntPredicate::SLT, idx, len, "for_in_cond");
    ctx.builder()
        .build_conditional_branch(cond, body_bb, end_bb);

    let label = ctx.pending_loop_label.take();
    ctx.push_loop(end_bb, cond_bb, label);

    ctx.builder().position_at_end(body_bb);
    let idx = ctx.builder().build_load(idx_ptr, "idx").into_int_value();
    let one = i64_ty.const_int(1, false);
    let elem_offset = ctx
        .builder()
        .build_int_mul(idx, i64_ty.const_int(8, false), "elem_offset");
    let data_start = i64_ty.const_int(16, false);
    let elem_offset_with_header =
        ctx.builder()
            .build_int_add(data_start, elem_offset, "elem_offset_hdr");
    let elem_offset_i32 = ctx.builder().build_int_cast(
        elem_offset_with_header,
        ctx.context.i32_type(),
        "elem_offset_i32",
    );
    let elem_ptr = unsafe {
        ctx.builder()
            .build_gep(keys_arr, &[elem_offset_i32], "elem_ptr")
    };
    let elem_i64_ptr = ctx
        .builder()
        .build_bitcast(elem_ptr, i64_ptr_ty, "elem_i64_ptr")
        .into_pointer_value();
    let key_val = ctx.builder().build_load(elem_i64_ptr, "key_val");
    ctx.builder().build_store(var_ptr, key_val);

    compile_stmt(ctx, body)?;

    if ctx
        .builder()
        .get_insert_block()
        .unwrap()
        .get_terminator()
        .is_none()
    {
        let next_idx = ctx.builder().build_int_add(idx, one, "next_idx");
        ctx.builder().build_store(idx_ptr, next_idx);
        ctx.builder().build_unconditional_branch(cond_bb);
    }

    ctx.builder().position_at_end(end_bb);
    ctx.pop_loop();

    if let Some(old) = old_var {
        ctx.define_variable(variable.to_string(), old);
    } else {
        ctx.remove_variable(variable);
    }

    Ok(())
}

fn compile_for_of<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    variable: &str,
    iterable: &crate::parser::ast::Expr,
    body: &Statement,
) -> Result<(), String> {
    let func = ctx.current_function().ok_or("No current function")?;
    let i8_ptr = ctx.context.i8_type().ptr_type(Default::default());
    let i64_ty = ctx.context.i64_type();
    let i64_ptr_ty = i64_ty.ptr_type(Default::default());

    let iter_result = compile_expr(ctx, iterable)?;

    match &iter_result.ty {
        Type::Array(_) => {
            let array_ptr = iter_result.value.into_pointer_value();

            let len_ptr = ctx
                .builder()
                .build_bitcast(array_ptr, i64_ptr_ty, "len_ptr")
                .into_pointer_value();
            let len = ctx.builder().build_load(len_ptr, "len").into_int_value();

            let idx_ptr = ctx.builder().build_alloca(i64_ty, "for_of_idx");
            let var_ptr = ctx.builder().build_alloca(i64_ty, variable);
            ctx.builder()
                .build_store(idx_ptr, i64_ty.const_int(0, false));
            let old_var = ctx
                .variables
                .insert(variable.to_string(), (var_ptr, Type::Int));

            let cond_bb = ctx.context.append_basic_block(func, "for_of_cond");
            let body_bb = ctx.context.append_basic_block(func, "for_of_body");
            let end_bb = ctx.context.append_basic_block(func, "for_of_end");

            ctx.builder().build_unconditional_branch(cond_bb);

            ctx.builder().position_at_end(cond_bb);
            let idx = ctx.builder().build_load(idx_ptr, "idx").into_int_value();
            let cond = ctx.builder().build_int_compare(
                inkwell::IntPredicate::SLT,
                idx,
                len,
                "for_of_cond",
            );
            ctx.builder()
                .build_conditional_branch(cond, body_bb, end_bb);

            let label = ctx.pending_loop_label.take();
            ctx.push_loop(end_bb, cond_bb, label);

            ctx.builder().position_at_end(body_bb);
            let idx = ctx.builder().build_load(idx_ptr, "idx").into_int_value();
            let one = i64_ty.const_int(1, false);
            let elem_offset =
                ctx.builder()
                    .build_int_mul(idx, i64_ty.const_int(8, false), "elem_offset");
            let data_start = i64_ty.const_int(16, false);
            let elem_offset_with_header =
                ctx.builder()
                    .build_int_add(data_start, elem_offset, "elem_offset_hdr");
            let elem_offset_i32 = ctx.builder().build_int_cast(
                elem_offset_with_header,
                ctx.context.i32_type(),
                "elem_offset_i32",
            );
            let elem_ptr = unsafe {
                ctx.builder()
                    .build_gep(array_ptr, &[elem_offset_i32], "elem_ptr")
            };
            let elem_i64_ptr = ctx
                .builder()
                .build_bitcast(elem_ptr, i64_ptr_ty, "elem_i64_ptr")
                .into_pointer_value();
            let elem_val = ctx.builder().build_load(elem_i64_ptr, "elem_val");
            ctx.builder().build_store(var_ptr, elem_val);

            compile_stmt(ctx, body)?;

            if ctx
                .builder()
                .get_insert_block()
                .unwrap()
                .get_terminator()
                .is_none()
            {
                let next_idx = ctx.builder().build_int_add(idx, one, "next_idx");
                ctx.builder().build_store(idx_ptr, next_idx);
                ctx.builder().build_unconditional_branch(cond_bb);
            }

            ctx.builder().position_at_end(end_bb);
            ctx.pop_loop();

            if let Some(old) = old_var {
                ctx.define_variable(variable.to_string(), old);
            } else {
                ctx.remove_variable(variable);
            }

            Ok(())
        }
        _ => {
            let iterable_ptr = iter_result.value.into_pointer_value();

            let temp_name = "__for_of_iterable";
            let temp_ptr = ctx.builder().build_alloca(i8_ptr, temp_name);
            ctx.builder().build_store(temp_ptr, iterable_ptr);
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
            let iter_ptr = ctx.builder().build_alloca(i8_ptr, iter_name);
            ctx.builder().build_store(iter_ptr, iter_obj_result.value);
            let old_iter = ctx
                .variables
                .insert(iter_name.to_string(), (iter_ptr, Type::Dynamic));

            let cond_bb = ctx.context.append_basic_block(func, "for_of_cond");
            let body_bb = ctx.context.append_basic_block(func, "for_of_body");
            let end_bb = ctx.context.append_basic_block(func, "for_of_end");

            ctx.builder().build_unconditional_branch(cond_bb);

            ctx.builder().position_at_end(cond_bb);
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
            let next_int = ctx.builder().build_ptr_to_int(next_ptr, i64_ty, "next_int");
            let is_null = ctx.builder().build_int_compare(
                inkwell::IntPredicate::EQ,
                next_int,
                i64_ty.const_int(0, false),
                "is_null",
            );
            ctx.builder()
                .build_conditional_branch(is_null, end_bb, body_bb);

            let label = ctx.pending_loop_label.take();
            ctx.push_loop(end_bb, cond_bb, label);

            ctx.builder().position_at_end(body_bb);
            let var_ptr = ctx.builder().build_alloca(i8_ptr, variable);
            ctx.builder().build_store(var_ptr, next_result.value);
            let old_var = ctx
                .variables
                .insert(variable.to_string(), (var_ptr, Type::Dynamic));

            compile_stmt(ctx, body)?;

            if ctx
                .builder()
                .get_insert_block()
                .unwrap()
                .get_terminator()
                .is_none()
            {
                ctx.builder().build_unconditional_branch(cond_bb);
            }

            ctx.builder().position_at_end(end_bb);
            ctx.pop_loop();

            if let Some(old) = old_var {
                ctx.define_variable(variable.to_string(), old);
            } else {
                ctx.remove_variable(variable);
            }
            if let Some(old) = old_iter {
                ctx.define_variable(iter_name.to_string(), old);
            } else {
                ctx.remove_variable(iter_name);
            }
            if let Some(old) = old_temp {
                ctx.define_variable(temp_name.to_string(), old);
            } else {
                ctx.remove_variable(temp_name);
            }

            Ok(())
        }
    }
}

fn compile_return<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    expr: Option<&crate::parser::ast::Expr>,
) -> Result<(), String> {
    if let (Some(result_ptr), Some(return_bb)) = (ctx.async_result_ptr, ctx.async_return_bb) {
        if let Some(e) = expr {
            let result = compile_expr(ctx, e)?;
            ctx.builder().build_store(result_ptr, result.value);
        }
        ctx.builder().build_unconditional_branch(return_bb);
        return Ok(());
    }

    ctx.emit_gc_root_removals();
    match expr {
        Some(e) => {
            let result = compile_expr(ctx, e)?;
            ctx.builder().build_return(Some(&result.value));
            Ok(())
        }
        None => {
            ctx.builder().build_return(None);
            Ok(())
        }
    }
}

fn compile_throw<'ctx>(ctx: &mut CodegenContext<'ctx, '_, '_>, expr: &Expr) -> Result<(), String> {
    // Handle both `throw ClassName(...)` and `throw new ClassName(...)` patterns.
    // Extract the class name and argument list for downstream processing.
    let (throw_class, args): (Option<String>, Vec<&crate::parser::ast::Argument>) = match expr {
        Expr::New { callee, args } => match callee.as_ref() {
            Expr::Identifier(n) => (Some(n.clone()), args.iter().collect()),
            Expr::Call {
                callee: c,
                args: call_args,
            } => {
                if let Expr::Identifier(n) = c.as_ref() {
                    if args.is_empty() {
                        return compile_throw(
                            ctx,
                            &Expr::Call {
                                callee: Box::new(Expr::Identifier(n.clone())),
                                args: call_args.clone(),
                            },
                        );
                    }
                }
                return Err("throw new: unsupported callee".to_string());
            }
            _ => return Err("throw new: unsupported callee".to_string()),
        },
        Expr::Call { callee, args } => {
            let name = match callee.as_ref() {
                Expr::Identifier(n) => n.clone(),
                Expr::Member {
                    object, property, ..
                } => match (object.as_ref(), property) {
                    (Expr::Identifier(n), crate::parser::ast::MemberProperty::Ident(method))
                        if method == "new" =>
                    {
                        n.clone()
                    }
                    _ => return Err("throw: unsupported callee".to_string()),
                },
                _ => return Err("throw: unsupported callee".to_string()),
            };
            let is_class = ctx.class_struct_types.contains_key(&name)
                || name.chars().next().map_or(false, |c| c.is_uppercase());
            if !is_class {
                let exc_result = compile_expr(ctx, expr)?;
                let exc_ptr = match exc_result.value {
                    BasicValueEnum::PointerValue(v) => v,
                    _ => return Err("throw expression must evaluate to a pointer".to_string()),
                };
                emit_throw_call(ctx, exc_ptr)?;
                return Ok(());
            }
            (Some(name.clone()), args.iter().collect())
        }
        _ => {
            let exc_result = compile_expr(ctx, expr)?;
            let exc_ptr = match exc_result.value {
                BasicValueEnum::PointerValue(v) => v,
                _ => return Err("throw expression must evaluate to a pointer".to_string()),
            };
            emit_throw_call(ctx, exc_ptr)?;
            return Ok(());
        }
    };

    // At this point we have a class name + args. Extract the message string
    // and pass it directly to ruyi_throw (the runtime creates RuyiException from
    // the message, using the class name-derived type_id).
    let msg = args
        .first()
        .ok_or("throw requires at least one argument (message)")?;
    match msg {
        crate::parser::ast::Argument::Expr(e) => match e.as_ref() {
            Expr::StringLiteral(s) => {
                let str_ptr = ctx.builder().build_global_string_ptr(s, "throw_msg");
                let exc_ptr = str_ptr.as_pointer_value();
                emit_throw_call(ctx, exc_ptr)?;
            }
            _ => {
                let class_name =
                    throw_class.ok_or("throw requires a class name for non-literal arguments")?;
                let args_cloned: Vec<crate::parser::ast::Argument> =
                    args.iter().map(|a| (*a).clone()).collect();
                let exc_result =
                    super::expr::compile_new(ctx, &Expr::Identifier(class_name), &args_cloned)?;
                let exc_ptr = match exc_result.value {
                    BasicValueEnum::PointerValue(v) => v,
                    _ => return Err("throw expression must evaluate to a pointer".to_string()),
                };
                emit_throw_call(ctx, exc_ptr)?;
            }
        },
        _ => return Err("throw class: argument must be an expression".to_string()),
    }

    Ok(())
}

fn compile_try<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    body: &[Statement],
    catch: &[crate::parser::ast::CatchClause],
    finally: Option<&[Statement]>,
) -> Result<(), String> {
    let func = ctx.current_function().ok_or("No current function")?;
    let i8_ptr = ctx.context.i8_type().ptr_type(Default::default());
    let i64_ty = ctx.context.i64_type();

    let try_body_bb = ctx.context.append_basic_block(func, "try_body");
    let merge_bb = ctx.context.append_basic_block(func, "try_merge");
    // T4: new landing-pad and resume blocks for LLVM exception handling
    let landing_pad_bb = ctx.context.append_basic_block(func, "try.landingpad");
    let resume_bb = ctx.context.append_basic_block(func, "try.resume");

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

    let exception_ptr = ctx.builder().build_alloca(i8_ptr, "exc_ptr");
    ctx.builder()
        .build_store(exception_ptr, i8_ptr.const_null());

    let clear_fn = ctx
        .module
        .get_function("ruyi_clear_pending_exception")
        .expect("ruyi_clear_pending_exception not declared");
    ctx.builder().build_call(clear_fn, &[], "clear_exc");

    ctx.builder().build_unconditional_branch(try_body_bb);
    ctx.builder().position_at_end(try_body_bb);

    let try_ctx = TryContext {
        exception_ptr,
        catch_bb,
        finally_bb,
        merge_bb,
        landing_pad_bb: Some(landing_pad_bb),
    };
    ctx.push_try(try_ctx);

    let try_frame = TryFrame {
        landing_pad_bb,
        catch_bb,
        finally_bb,
        exception_ptr,
    };
    // SAFETY: ctx outlives the guard; guard is local-scoped and pops on drop
    let _try_guard = unsafe { TryStackGuard::push(ctx, try_frame) };

    compile_block(ctx, body)?;

    let body_end = ctx.builder().get_insert_block().unwrap();
    if body_end.get_terminator().is_none() {
        if let Some(fb) = finally_bb {
            ctx.builder().build_unconditional_branch(fb);
        } else {
            ctx.builder().build_unconditional_branch(merge_bb);
        }
    }

    ctx.pop_try();

    // ── T4: LLVM landing-pad generation (for invoke-based exception handling) ──
    let landing_pad_val;
    {
        let lp_gen = LandingPadGenerator::new(&ctx.context, &ctx.module, ctx.builder());

        // Create per-catch handler blocks; type_id = 0 is catch-all for ruyi exception type
        let mut catch_handlers: Vec<(
            ruyi_exception::TryTypeId,
            inkwell::basic_block::BasicBlock<'ctx>,
        )> = Vec::new();
        for (i, _) in catch.iter().enumerate() {
            let handler_bb = ctx
                .context
                .append_basic_block(func, &format!("try.catch.{}", i));
            catch_handlers.push((0u32, handler_bb));
        }

        let catch_type_ids: Vec<ruyi_exception::TryTypeId> =
            catch_handlers.iter().map(|(id, _)| *id).collect();
        let has_cleanup = finally.is_some();

        ctx.builder().position_at_end(landing_pad_bb);
        landing_pad_val = lp_gen.build_landing_pad(&catch_type_ids, has_cleanup, "landingpad");

        // Extract exception pointer and store it for catch blocks to access
        let exc_ptr = lp_gen.extract_exception_ptr(landing_pad_val);
        ctx.builder().build_store(exception_ptr, exc_ptr);

        // Dispatch from landing-pad to first catch handler (catch-all mode).
        // Must be called while builder is still positioned inside landing_pad_bb
        // so the branch is emitted from the correct block.
        lp_gen.build_catch_dispatch(landing_pad_val, &catch_handlers, finally_bb, resume_bb);

        // Forward old catch_bb (used by compile_throw/build_exception_check)
        // to the first handler block so both old and new paths reach the same code
        if let Some(cb) = catch_bb {
            ctx.builder().position_at_end(cb);
            ctx.builder()
                .build_unconditional_branch(catch_handlers[0].1);
        }

        // Compile per-clause catch handlers
        for (i, catch_clause) in catch.iter().enumerate() {
            let handler_bb = catch_handlers[i].1;
            ctx.builder().position_at_end(handler_bb);

            build_ruyi_clear_pending_exception(ctx.builder(), &ctx.module);

            let exc_val = ctx
                .builder()
                .build_load(exception_ptr, "exc_val")
                .into_pointer_value();
            if let Some(pattern) = &catch_clause.pattern {
                match pattern {
                    crate::parser::ast::Pattern::Identifier(name) => {
                        let local_ptr = ctx.builder().build_alloca(i8_ptr, name);
                        ctx.builder().build_store(local_ptr, exc_val);
                        let var_ty = catch_clause
                            .ty
                            .as_ref()
                            .map(Type::from_annotation)
                            .unwrap_or(Type::Dynamic);
                        ctx.define_variable(name.clone(), (local_ptr, var_ty));
                    }
                    _ => {}
                }
            }

            compile_block(ctx, &catch_clause.body)?;

            let catch_end = ctx.builder().get_insert_block().unwrap();
            if catch_end.get_terminator().is_none() {
                if let Some(fb) = finally_bb {
                    ctx.builder().build_unconditional_branch(fb);
                } else {
                    ctx.builder().build_unconditional_branch(merge_bb);
                }
            }
        }
    }
    // _try_guard drops here, popping try_frame_stack

    // Uncaught exception: build resume block
    {
        let lp_gen = LandingPadGenerator::new(&ctx.context, &ctx.module, ctx.builder());
        ctx.builder().position_at_end(resume_bb);
        lp_gen.build_resume(landing_pad_val);
    }

    if let Some(finally_stmts) = finally {
        let fb = finally_bb.unwrap();
        ctx.builder().position_at_end(fb);

        compile_block(ctx, finally_stmts)?;

        let finally_end = ctx.builder().get_insert_block().unwrap();
        if finally_end.get_terminator().is_none() {
            if catch.is_empty() {
                let exc_val = ctx
                    .builder()
                    .build_load(exception_ptr, "exc_val")
                    .into_pointer_value();
                let exc_int = ctx.builder().build_ptr_to_int(exc_val, i64_ty, "exc_int");
                let is_null = ctx.builder().build_int_compare(
                    inkwell::IntPredicate::EQ,
                    exc_int,
                    i64_ty.const_int(0, false),
                    "is_null",
                );

                let pb = propagate_bb.unwrap();
                ctx.builder()
                    .build_conditional_branch(is_null, merge_bb, pb);

                ctx.builder().position_at_end(pb);
                let exc_val2 = ctx
                    .builder()
                    .build_load(exception_ptr, "exc_val2")
                    .into_pointer_value();
                let throw_fn = ctx
                    .module
                    .get_function("ruyi_throw")
                    .expect("ruyi_throw not declared");
                ctx.builder()
                    .build_call(throw_fn, &[exc_val2.into()], "rethrow");
                ctx.emit_gc_root_removals();
                ctx.builder().build_return(None);
            } else {
                ctx.builder().build_unconditional_branch(merge_bb);
            }
        }
    }

    ctx.builder().position_at_end(merge_bb);
    Ok(())
}

fn emit_throw_call<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    exc_ptr: inkwell::values::PointerValue<'ctx>,
) -> Result<(), String> {
    let throw_fn = ctx
        .module
        .get_function("ruyi_throw")
        .expect("ruyi_throw not declared");

    if let Some(try_ctx) = ctx.current_try() {
        ctx.builder()
            .build_call(throw_fn, &[exc_ptr.into()], "throw");
        ctx.builder().build_store(try_ctx.exception_ptr, exc_ptr);
        if let Some(catch_bb) = try_ctx.catch_bb {
            ctx.builder().build_unconditional_branch(catch_bb);
        } else if let Some(finally_bb) = try_ctx.finally_bb {
            ctx.builder().build_unconditional_branch(finally_bb);
        } else {
            ctx.builder().build_unconditional_branch(try_ctx.merge_bb);
        }
        if let Some(func) = ctx.current_function() {
            let unreachable_bb = ctx.context.append_basic_block(func, "throw.unreachable");
            ctx.builder().position_at_end(unreachable_bb);
            ctx.builder().build_unreachable();
        }
    } else {
        ctx.builder()
            .build_call(throw_fn, &[exc_ptr.into()], "throw");
        ctx.builder().build_unreachable();
    }

    Ok(())
}

/// Look up the loop_stack entry whose label matches `target_label` and
/// extract the requested basic block via `selector`. Walks top-down so
/// the innermost matching label wins (matching JS-style label scoping).
/// Returns E3003 error if no matching label is found.
fn find_loop_target<'ctx, 'm, 'env, F>(
    ctx: &CodegenContext<'ctx, 'm, 'env>,
    target_label: String,
    selector: F,
) -> Result<inkwell::basic_block::BasicBlock<'ctx>, String>
where
    F: Fn(
        &(
            inkwell::basic_block::BasicBlock<'ctx>,
            inkwell::basic_block::BasicBlock<'ctx>,
            Option<String>,
        ),
    ) -> inkwell::basic_block::BasicBlock<'ctx>,
{
    for entry in ctx.loop_stack.iter().rev() {
        if entry.2.as_deref() == Some(target_label.as_str()) {
            return Ok(selector(entry));
        }
    }
    Err(format!(
        "E3003: Undefined label: `break`/`continue` references label `{}` which is not associated with any enclosing loop",
        target_label
    ))
}

fn build_exception_check<'ctx>(ctx: &mut CodegenContext<'ctx, '_, '_>) -> Result<(), String> {
    if ctx.try_stack_is_empty() {
        return Ok(());
    }

    let func = ctx.current_function().unwrap();
    let pending = build_ruyi_get_pending_exception(ctx.builder(), &ctx.module);

    let i64_ty = ctx.context.i64_type();
    let pending_int = ctx
        .builder()
        .build_ptr_to_int(pending, i64_ty, "pending_int");
    let is_null = ctx.builder().build_int_compare(
        inkwell::IntPredicate::EQ,
        pending_int,
        i64_ty.const_int(0, false),
        "no_exc",
    );

    let try_ctx = ctx.current_try().unwrap();
    let continue_bb = ctx.context.append_basic_block(func, "after_exc_check");
    let store_exc_bb = ctx.context.append_basic_block(func, "store_exc");

    let dest_bb = try_ctx
        .catch_bb
        .or(try_ctx.finally_bb)
        .unwrap_or(try_ctx.merge_bb);

    ctx.builder()
        .build_conditional_branch(is_null, continue_bb, store_exc_bb);

    ctx.builder().position_at_end(store_exc_bb);
    ctx.builder().build_store(try_ctx.exception_ptr, pending);
    build_ruyi_clear_pending_exception(ctx.builder(), &ctx.module);
    ctx.builder().build_unconditional_branch(dest_bb);

    ctx.builder().position_at_end(continue_bb);
    Ok(())
}

fn compile_if_let<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    pattern: &crate::parser::ast::Pattern,
    value: &Expr,
    then_branch: &Statement,
    else_branch: Option<&Statement>,
) -> Result<(), String> {
    let func = ctx.current_function().ok_or("No current function")?;
    let val = compile_expr(ctx, value)?;

    let then_bb = ctx.context.append_basic_block(func, "if_let_then");
    let else_bb = ctx.context.append_basic_block(func, "if_let_else");
    let merge_bb = ctx.context.append_basic_block(func, "if_let_merge");

    let is_match = pattern_is_matching(ctx, pattern, &val)?;

    ctx.builder()
        .build_conditional_branch(is_match, then_bb, else_bb);

    ctx.builder().position_at_end(then_bb);
    bind_pattern_in_codegen(ctx, pattern, &val)?;
    compile_stmt(ctx, then_branch)?;
    if let Some(bb) = ctx.builder().get_insert_block() {
        if bb.get_terminator().is_none() {
            ctx.builder().build_unconditional_branch(merge_bb);
        }
    }

    ctx.builder().position_at_end(else_bb);
    if let Some(else_stmt) = else_branch {
        compile_stmt(ctx, else_stmt)?;
    }
    if let Some(bb) = ctx.builder().get_insert_block() {
        if bb.get_terminator().is_none() {
            ctx.builder().build_unconditional_branch(merge_bb);
        }
    }

    ctx.builder().position_at_end(merge_bb);
    Ok(())
}

fn compile_while_let<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    pattern: &crate::parser::ast::Pattern,
    value: &Expr,
    body: &Statement,
) -> Result<(), String> {
    let func = ctx.current_function().ok_or("No current function")?;
    let header_bb = ctx.context.append_basic_block(func, "while_let_header");
    let body_bb = ctx.context.append_basic_block(func, "while_let_body");
    let exit_bb = ctx.context.append_basic_block(func, "while_let_exit");

    ctx.builder().build_unconditional_branch(header_bb);

    ctx.builder().position_at_end(header_bb);
    let val = compile_expr(ctx, value)?;
    let val_ptr = ctx.builder().build_alloca(
        super::types::ruyi_type_to_llvm(ctx.context, &val.ty),
        "while_let_val",
    );
    ctx.builder().build_store(val_ptr, val.value);

    let is_match = pattern_is_matching(ctx, pattern, &val)?;
    ctx.builder()
        .build_conditional_branch(is_match, body_bb, exit_bb);

    ctx.builder().position_at_end(body_bb);
    let loaded_val = ctx.builder().build_load(val_ptr, "while_let_loaded");
    let loaded_result = ExprResult {
        value: loaded_val,
        ty: val.ty.clone(),
    };
    bind_pattern_in_codegen(ctx, pattern, &loaded_result)?;
    let label = ctx.pending_loop_label.take();
    ctx.push_loop(exit_bb, header_bb, label);
    compile_stmt(ctx, body)?;
    ctx.pop_loop();
    if let Some(bb) = ctx.builder().get_insert_block() {
        if bb.get_terminator().is_none() {
            ctx.builder().build_unconditional_branch(header_bb);
        }
    }

    ctx.builder().position_at_end(exit_bb);
    Ok(())
}

fn pattern_is_matching<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    pattern: &crate::parser::ast::Pattern,
    val: &ExprResult<'ctx>,
) -> Result<inkwell::values::IntValue<'ctx>, String> {
    use crate::parser::ast::Pattern as P;
    match pattern {
        P::Wildcard => Ok(ctx.context.bool_type().const_int(1, false)),
        P::Identifier(_) => {
            if matches!(val.ty, Type::Nullable(_) | Type::Null) {
                let is_non_null = match val.value {
                    BasicValueEnum::PointerValue(p) => {
                        let i64_ty = ctx.context.i64_type();
                        let ptr_int = ctx.builder().build_ptr_to_int(p, i64_ty, "ptr_int");
                        ctx.builder().build_int_compare(
                            inkwell::IntPredicate::NE,
                            ptr_int,
                            i64_ty.const_int(0, false),
                            "is_non_null",
                        )
                    }
                    BasicValueEnum::IntValue(v) => ctx.builder().build_int_compare(
                        inkwell::IntPredicate::NE,
                        v,
                        ctx.context.i64_type().const_all_ones(),
                        "is_non_null_int",
                    ),
                    _ => {
                        return Err(
                            "Nullable match requires pointer or integer scrutinee".to_string()
                        )
                    }
                };
                Ok(is_non_null)
            } else {
                Ok(ctx.context.bool_type().const_int(1, false))
            }
        }
        P::Literal(expr) => {
            let lit_val = compile_expr(ctx, expr)?;
            let cmp = ctx.builder().build_int_compare(
                inkwell::IntPredicate::EQ,
                val.value.into_int_value(),
                lit_val.value.into_int_value(),
                "lit_match",
            );
            Ok(cmp)
        }
        P::Object(_) | P::Array(_) => Ok(ctx.context.bool_type().const_int(1, false)),
        P::As(inner, _) => pattern_is_matching(ctx, inner, val),
        P::Or(patterns) => {
            let mut result = ctx.context.bool_type().const_int(0, false);
            for p in patterns {
                let m = pattern_is_matching(ctx, p, val)?;
                result = ctx.builder().build_or(result, m, "or_match");
            }
            Ok(result)
        }
        P::Rest(_) => Ok(ctx.context.bool_type().const_int(1, false)),
    }
}

fn bind_pattern_in_codegen<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    pattern: &crate::parser::ast::Pattern,
    val: &ExprResult<'ctx>,
) -> Result<(), String> {
    use crate::parser::ast::{ObjectPatternField, Pattern as P};
    match pattern {
        P::Identifier(name) => {
            let llvm_ty = super::types::ruyi_type_to_llvm(ctx.context, &val.ty);
            let ptr = ctx.builder().build_alloca(llvm_ty, name);
            ctx.builder().build_store(ptr, val.value);
            ctx.define_variable(name.clone(), (ptr, val.ty.clone()));
        }
        P::Object(fields) => {
            let obj_ptr = match val.value {
                BasicValueEnum::PointerValue(p) => p,
                _ => return Err("Object pattern requires pointer".to_string()),
            };

            let i32_ty = ctx.context.i32_type();
            let _i64_ty = ctx.context.i64_type();

            match &val.ty {
                Type::Named(class_name, _) => {
                    let class_fields: Vec<_> = ctx
                        .class_fields
                        .get(class_name)
                        .ok_or_else(|| format!("Unknown class: {}", class_name))?
                        .clone();
                    let struct_type = ctx
                        .class_struct_types
                        .get(class_name)
                        .ok_or_else(|| format!("No struct type for class: {}", class_name))?;

                    let struct_ptr = ctx.builder().build_pointer_cast(
                        obj_ptr,
                        struct_type.ptr_type(Default::default()),
                        "obj_struct_cast",
                    );

                    for field in fields {
                        match field {
                            ObjectPatternField::Property {
                                key,
                                pattern: inner,
                            } => {
                                let field_index =
                                    class_fields
                                        .iter()
                                        .position(|(n, _)| n == key)
                                        .ok_or_else(|| format!("Unknown field: {}", key))?;
                                let (_, field_ty) = &class_fields[field_index];

                                let field_ptr = unsafe {
                                    ctx.builder().build_gep(
                                        struct_ptr,
                                        &[
                                            i32_ty.const_int(0, false),
                                            i32_ty.const_int(field_index as u64, false),
                                        ],
                                        &format!("{}_ptr", key),
                                    )
                                };
                                let field_val = ctx.builder().build_load(field_ptr, key);
                                let field_result = super::expr::ExprResult {
                                    value: field_val,
                                    ty: field_ty.clone(),
                                };
                                bind_pattern_in_codegen(ctx, inner, &field_result)?;
                            }
                            ObjectPatternField::Shorthand(name) => {
                                let field_index = class_fields
                                    .iter()
                                    .position(|(n, _)| n == name)
                                    .ok_or_else(|| format!("Unknown field: {}", name))?;
                                let (_, field_ty) = &class_fields[field_index];

                                let field_ptr = unsafe {
                                    ctx.builder().build_gep(
                                        struct_ptr,
                                        &[
                                            i32_ty.const_int(0, false),
                                            i32_ty.const_int(field_index as u64, false),
                                        ],
                                        &format!("{}_ptr", name),
                                    )
                                };
                                let field_val = ctx.builder().build_load(field_ptr, name);
                                let llvm_ty =
                                    super::types::ruyi_type_to_llvm(ctx.context, field_ty);
                                let ptr = ctx.builder().build_alloca(llvm_ty, name);
                                ctx.builder().build_store(ptr, field_val);
                                ctx.define_variable(name.clone(), (ptr, field_ty.clone()));
                            }
                            ObjectPatternField::Rest(_) => {}
                        }
                    }
                }
                Type::Object(type_fields) => {
                    for field in fields {
                        match field {
                            ObjectPatternField::Property {
                                key,
                                pattern: inner,
                            } => {
                                let field_index =
                                    type_fields
                                        .iter()
                                        .position(|f| f.name == *key)
                                        .ok_or_else(|| format!("Unknown field: {}", key))?;
                                let field_ty = &type_fields[field_index].ty;

                                let offset = i32_ty.const_int((field_index * 8) as u64, false);
                                let field_ptr = unsafe {
                                    ctx.builder().build_gep(
                                        obj_ptr,
                                        &[offset],
                                        &format!("{}_ptr", key),
                                    )
                                };
                                let typed_ptr = ctx.builder().build_bitcast(
                                    field_ptr,
                                    super::types::ruyi_type_to_llvm(ctx.context, field_ty)
                                        .ptr_type(Default::default()),
                                    &format!("{}_typed_ptr", key),
                                );
                                let field_val = ctx
                                    .builder()
                                    .build_load(typed_ptr.into_pointer_value(), key);
                                let field_result = super::expr::ExprResult {
                                    value: field_val,
                                    ty: field_ty.clone(),
                                };
                                bind_pattern_in_codegen(ctx, inner, &field_result)?;
                            }
                            ObjectPatternField::Shorthand(name) => {
                                let field_index = type_fields
                                    .iter()
                                    .position(|f| f.name == *name)
                                    .ok_or_else(|| format!("Unknown field: {}", name))?;
                                let field_ty = &type_fields[field_index].ty;

                                let offset = i32_ty.const_int((field_index * 8) as u64, false);
                                let field_ptr = unsafe {
                                    ctx.builder().build_gep(
                                        obj_ptr,
                                        &[offset],
                                        &format!("{}_ptr", name),
                                    )
                                };
                                let typed_ptr = ctx.builder().build_bitcast(
                                    field_ptr,
                                    super::types::ruyi_type_to_llvm(ctx.context, field_ty)
                                        .ptr_type(Default::default()),
                                    &format!("{}_typed_ptr", name),
                                );
                                let field_val = ctx
                                    .builder()
                                    .build_load(typed_ptr.into_pointer_value(), name);
                                let llvm_ty =
                                    super::types::ruyi_type_to_llvm(ctx.context, field_ty);
                                let ptr = ctx.builder().build_alloca(llvm_ty, name);
                                ctx.builder().build_store(ptr, field_val);
                                ctx.define_variable(name.clone(), (ptr, field_ty.clone()));
                            }
                            ObjectPatternField::Rest(_) => {}
                        }
                    }
                }
                _ => return Err("Object pattern requires Named or Object type".to_string()),
            }
        }
        P::Array(elements) => {
            super::patterns::bind_array_pattern(ctx, elements, val)?;
        }
        P::As(inner, alias) => {
            bind_pattern_in_codegen(ctx, inner, val)?;
            let llvm_ty = super::types::ruyi_type_to_llvm(ctx.context, &val.ty);
            let ptr = ctx.builder().build_alloca(llvm_ty, alias);
            ctx.builder().build_store(ptr, val.value);
            ctx.define_variable(alias.clone(), (ptr, val.ty.clone()));
        }
        P::Or(patterns) => {
            if let Some(first) = patterns.first() {
                bind_pattern_in_codegen(ctx, first, val)?;
            }
        }
        P::Rest(name) => {
            let llvm_ty = super::types::ruyi_type_to_llvm(ctx.context, &val.ty);
            let ptr = ctx.builder().build_alloca(llvm_ty, name);
            ctx.builder().build_store(ptr, val.value);
            ctx.define_variable(name.clone(), (ptr, val.ty.clone()));
        }
        P::Wildcard | P::Literal(_) => {}
    }
    Ok(())
}
