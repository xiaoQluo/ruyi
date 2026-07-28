/**
 * Statement code generation for Ruyi.
 *
 * Lowers Ruyi AST statements to LLVM IR instructions.
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */
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
            // EX-C2 修复：检查是否在 try 块内，如果是则先执行 finally
            if let Some(try_ctx) = ctx.current_try() {
                if let Some(finally_bb) = try_ctx.finally_bb {
                    ctx.pending_break_target = Some(target_bb);
                    ctx.builder().build_unconditional_branch(finally_bb).unwrap();
                    return Ok(());
                }
            }
            ctx.builder().build_unconditional_branch(target_bb).unwrap();
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
            // EX-C2 修复：检查是否在 try 块内，如果是则先执行 finally
            if let Some(try_ctx) = ctx.current_try() {
                if let Some(finally_bb) = try_ctx.finally_bb {
                    ctx.pending_continue_target = Some(target_bb);
                    ctx.builder().build_unconditional_branch(finally_bb).unwrap();
                    return Ok(());
                }
            }
            ctx.builder().build_unconditional_branch(target_bb).unwrap();
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
        .build_conditional_branch(cond_val, then_bb, else_bb).unwrap();

    ctx.builder().position_at_end(then_bb);
    compile_stmt(ctx, then_branch)?;
    if ctx
        .builder()
        .get_insert_block()
        .unwrap()
        .get_terminator()
        .is_none()
    {
        ctx.builder().build_unconditional_branch(merge_bb).unwrap();
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
        ctx.builder().build_unconditional_branch(merge_bb).unwrap();
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

    ctx.builder().build_unconditional_branch(cond_bb).unwrap();

    ctx.builder().position_at_end(cond_bb);
    let cond_result = compile_expr(ctx, condition)?;
    let cond_val = match cond_result.value {
        BasicValueEnum::IntValue(v) => v,
        _ => return Err("Condition must be boolean".to_string()),
    };
    ctx.builder()
        .build_conditional_branch(cond_val, body_bb, end_bb).unwrap();

    ctx.builder().position_at_end(body_bb);
    compile_stmt(ctx, body)?;
    if ctx
        .builder()
        .get_insert_block()
        .unwrap()
        .get_terminator()
        .is_none()
    {
        ctx.builder().build_unconditional_branch(cond_bb).unwrap();
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

    ctx.builder().build_unconditional_branch(cond_bb).unwrap();

    ctx.builder().position_at_end(cond_bb);
    if let Some(cond) = condition {
        let cond_result = compile_expr(ctx, cond)?;
        let cond_val = match cond_result.value {
            BasicValueEnum::IntValue(v) => v,
            _ => return Err("Condition must be boolean".to_string()),
        };
        ctx.builder()
            .build_conditional_branch(cond_val, body_bb, end_bb).unwrap();
    } else {
        ctx.builder().build_unconditional_branch(body_bb).unwrap();
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
        ctx.builder().build_unconditional_branch(update_bb).unwrap();
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
        ctx.builder().build_unconditional_branch(cond_bb).unwrap();
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
    let i8_ptr = ctx.context.ptr_type(Default::default());
    let i64_ty = ctx.context.i64_type();
    let i64_ptr_ty = ctx.context.ptr_type(Default::default());

    let iter_result = compile_expr(ctx, iterable)?;

    // for-in on Array: iterate over integer indices
    if let Type::Array(_) = &iter_result.ty {
        let array_ptr = iter_result.value.into_pointer_value();

        let len_ptr = ctx
            .builder()
            .build_bit_cast(array_ptr, i64_ptr_ty, "len_ptr").unwrap()
            .into_pointer_value();
        let len = ctx.builder().build_load(i64_ty, len_ptr, "len").unwrap().into_int_value();

        let idx_ptr = ctx.builder().build_alloca(i64_ty, "for_in_idx").unwrap();
        ctx.builder()
            .build_store(idx_ptr, i64_ty.const_int(0, false)).unwrap();

        // for-in 的循环变量是整数索引
        let var_ptr = ctx.builder().build_alloca(i64_ty, variable).unwrap();
        let old_var = ctx
            .variables
            .insert(variable.to_string(), (var_ptr, Type::Int));

        let cond_bb = ctx.context.append_basic_block(func, "for_in_cond");
        let body_bb = ctx.context.append_basic_block(func, "for_in_body");
        let end_bb = ctx.context.append_basic_block(func, "for_in_end");

        ctx.builder().build_unconditional_branch(cond_bb).unwrap();

        ctx.builder().position_at_end(cond_bb);
        let idx = ctx.builder().build_load(i64_ty, idx_ptr, "idx").unwrap().into_int_value();
        let cond =
            ctx.builder()
                .build_int_compare(inkwell::IntPredicate::SLT, idx, len, "for_in_cond").unwrap();
        ctx.builder()
            .build_conditional_branch(cond, body_bb, end_bb).unwrap();

        let label = ctx.pending_loop_label.take();
        ctx.push_loop(end_bb, cond_bb, label);

        ctx.builder().position_at_end(body_bb);
        let idx = ctx.builder().build_load(i64_ty, idx_ptr, "idx").unwrap().into_int_value();
        let one = i64_ty.const_int(1, false);
        // 将当前索引存入循环变量
        ctx.builder().build_store(var_ptr, idx).unwrap();

        compile_stmt(ctx, body)?;

        if ctx
            .builder()
            .get_insert_block()
            .unwrap()
            .get_terminator()
            .is_none()
        {
            let next_idx = ctx.builder().build_int_add(idx, one, "next_idx").unwrap();
            ctx.builder().build_store(idx_ptr, next_idx).unwrap();
            ctx.builder().build_unconditional_branch(cond_bb).unwrap();
        }

        ctx.builder().position_at_end(end_bb);
        ctx.pop_loop();

        if let Some(old) = old_var {
            ctx.define_variable(variable.to_string(), old);
        } else {
            ctx.remove_variable(variable);
        }

        return Ok(());
    }

    if let crate::typechecker::types::Type::Object(fields) = &iter_result.ty {
        let var_ptr = ctx.builder().build_alloca(i8_ptr, variable).unwrap();
        let old_var = ctx
            .variables
            .insert(variable.to_string(), (var_ptr, Type::String));
        for f in fields {
            let s = ctx.builder().build_global_string_ptr(&f.name, "obj_key").unwrap();
            ctx.builder().build_store(var_ptr, s.as_pointer_value()).unwrap();
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
        .unwrap()
        .try_as_basic_value()
        .unwrap_basic()
        .into_pointer_value();

    let len_ptr = ctx
        .builder()
        .build_bit_cast(keys_arr, i64_ptr_ty, "len_ptr").unwrap()
        .into_pointer_value();
    let len = ctx.builder().build_load(i64_ty, len_ptr, "len").unwrap().into_int_value();

    let idx_ptr = ctx.builder().build_alloca(i64_ty, "for_in_idx").unwrap();
    ctx.builder()
        .build_store(idx_ptr, i64_ty.const_int(0, false)).unwrap();

    let var_ptr = ctx.builder().build_alloca(i8_ptr, variable).unwrap();
    let old_var = ctx
        .variables
        .insert(variable.to_string(), (var_ptr, Type::String));

    let cond_bb = ctx.context.append_basic_block(func, "for_in_cond");
    let body_bb = ctx.context.append_basic_block(func, "for_in_body");
    let end_bb = ctx.context.append_basic_block(func, "for_in_end");

    ctx.builder().build_unconditional_branch(cond_bb).unwrap();

    ctx.builder().position_at_end(cond_bb);
    let idx = ctx.builder().build_load(i64_ty, idx_ptr, "idx").unwrap().into_int_value();
    let cond = ctx
        .builder()
        .build_int_compare(inkwell::IntPredicate::SLT, idx, len, "for_in_cond").unwrap();
    ctx.builder()
        .build_conditional_branch(cond, body_bb, end_bb).unwrap();

    let label = ctx.pending_loop_label.take();
    ctx.push_loop(end_bb, cond_bb, label);

    ctx.builder().position_at_end(body_bb);
    let idx = ctx.builder().build_load(i64_ty, idx_ptr, "idx").unwrap().into_int_value();
    let one = i64_ty.const_int(1, false);
    let elem_offset = ctx
        .builder()
        .build_int_mul(idx, i64_ty.const_int(8, false), "elem_offset").unwrap();
    let data_start = i64_ty.const_int(16, false);
    let elem_offset_with_header =
        ctx.builder()
            .build_int_add(data_start, elem_offset, "elem_offset_hdr").unwrap();
    let elem_offset_i32 = ctx.builder().build_int_cast(
        elem_offset_with_header,
        ctx.context.i32_type(),
        "elem_offset_i32",
    ).unwrap();
    let elem_ptr = unsafe {
        ctx.builder()
            .build_gep(ctx.context.i8_type(), keys_arr, &[elem_offset_i32], "elem_ptr")
            .unwrap()
    };
    let elem_i64_ptr = ctx
        .builder()
        .build_bit_cast(elem_ptr, i64_ptr_ty, "elem_i64_ptr").unwrap()
        .into_pointer_value();
    let key_val = ctx.builder().build_load(ctx.context.ptr_type(Default::default()), elem_i64_ptr, "key_val").unwrap();
    ctx.builder().build_store(var_ptr, key_val).unwrap();

    compile_stmt(ctx, body)?;

    if ctx
        .builder()
        .get_insert_block()
        .unwrap()
        .get_terminator()
        .is_none()
    {
        let next_idx = ctx.builder().build_int_add(idx, one, "next_idx").unwrap();
        ctx.builder().build_store(idx_ptr, next_idx).unwrap();
        ctx.builder().build_unconditional_branch(cond_bb).unwrap();
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
    let i8_ptr = ctx.context.ptr_type(Default::default());
    let i64_ty = ctx.context.i64_type();
    let i64_ptr_ty = ctx.context.ptr_type(Default::default());

    let iter_result = compile_expr(ctx, iterable)?;

    match &iter_result.ty {
        Type::Array(_) => {
            let array_ptr = iter_result.value.into_pointer_value();

            let len_ptr = ctx
                .builder()
                .build_bit_cast(array_ptr, i64_ptr_ty, "len_ptr").unwrap()
                .into_pointer_value();
            let len = ctx.builder().build_load(i64_ty, len_ptr, "len").unwrap().into_int_value();

            let idx_ptr = ctx.builder().build_alloca(i64_ty, "for_of_idx").unwrap();
            let var_ptr = ctx.builder().build_alloca(i64_ty, variable).unwrap();
            ctx.builder()
                .build_store(idx_ptr, i64_ty.const_int(0, false)).unwrap();
            let old_var = ctx
                .variables
                .insert(variable.to_string(), (var_ptr, Type::Int));

            let cond_bb = ctx.context.append_basic_block(func, "for_of_cond");
            let body_bb = ctx.context.append_basic_block(func, "for_of_body");
            let end_bb = ctx.context.append_basic_block(func, "for_of_end");

            ctx.builder().build_unconditional_branch(cond_bb).unwrap();

            ctx.builder().position_at_end(cond_bb);
            let idx = ctx.builder().build_load(i64_ty, idx_ptr, "idx").unwrap().into_int_value();
            let cond = ctx.builder().build_int_compare(
                inkwell::IntPredicate::SLT,
                idx,
                len,
                "for_of_cond",
            ).unwrap();
            ctx.builder()
                .build_conditional_branch(cond, body_bb, end_bb).unwrap();

            let label = ctx.pending_loop_label.take();
            ctx.push_loop(end_bb, cond_bb, label);

            ctx.builder().position_at_end(body_bb);
            let idx = ctx.builder().build_load(i64_ty, idx_ptr, "idx").unwrap().into_int_value();
            let one = i64_ty.const_int(1, false);
            let elem_offset =
                ctx.builder()
                    .build_int_mul(idx, i64_ty.const_int(8, false), "elem_offset").unwrap();
            let data_start = i64_ty.const_int(16, false);
            let elem_offset_with_header =
                ctx.builder()
                    .build_int_add(data_start, elem_offset, "elem_offset_hdr").unwrap();
            let elem_offset_i32 = ctx.builder().build_int_cast(
                elem_offset_with_header,
                ctx.context.i32_type(),
                "elem_offset_i32",
            ).unwrap();
            let elem_ptr = unsafe {
                ctx.builder()
                    .build_gep(ctx.context.i8_type(), array_ptr, &[elem_offset_i32], "elem_ptr")
                    .unwrap()
            };
            let elem_i64_ptr = ctx
                .builder()
                .build_bit_cast(elem_ptr, i64_ptr_ty, "elem_i64_ptr").unwrap()
                .into_pointer_value();
            let elem_val = ctx.builder().build_load(i64_ty, elem_i64_ptr, "elem_val").unwrap();
            ctx.builder().build_store(var_ptr, elem_val).unwrap();

            compile_stmt(ctx, body)?;

            if ctx
                .builder()
                .get_insert_block()
                .unwrap()
                .get_terminator()
                .is_none()
            {
                let next_idx = ctx.builder().build_int_add(idx, one, "next_idx").unwrap();
                ctx.builder().build_store(idx_ptr, next_idx).unwrap();
                ctx.builder().build_unconditional_branch(cond_bb).unwrap();
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
        Type::String => {
            // for-of on String: iterate over each character
            let str_ptr = iter_result.value.into_pointer_value();

            // Call __string_length(str) to get character count
            let str_len_fn = ctx
                .module
                .get_function("__string_length")
                .expect("__string_length not declared");
            let len = ctx
                .builder()
                .build_call(str_len_fn, &[str_ptr.into()], "str_len")
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic()
                .into_int_value();

            let idx_ptr = ctx.builder().build_alloca(i64_ty, "for_of_str_idx").unwrap();
            ctx.builder()
                .build_store(idx_ptr, i64_ty.const_int(0, false)).unwrap();

            // Loop variable is a single-char string (i8*)
            let var_ptr = ctx.builder().build_alloca(i8_ptr, variable).unwrap();
            let old_var = ctx
                .variables
                .insert(variable.to_string(), (var_ptr, Type::String));

            let cond_bb = ctx.context.append_basic_block(func, "for_of_str_cond");
            let body_bb = ctx.context.append_basic_block(func, "for_of_str_body");
            let end_bb = ctx.context.append_basic_block(func, "for_of_str_end");

            ctx.builder().build_unconditional_branch(cond_bb).unwrap();

            ctx.builder().position_at_end(cond_bb);
            let idx = ctx.builder().build_load(i64_ty, idx_ptr, "idx").unwrap().into_int_value();
            let cond = ctx.builder().build_int_compare(
                inkwell::IntPredicate::SLT,
                idx,
                len,
                "for_of_str_cond",
            ).unwrap();
            ctx.builder()
                .build_conditional_branch(cond, body_bb, end_bb).unwrap();

            let label = ctx.pending_loop_label.take();
            ctx.push_loop(end_bb, cond_bb, label);

            ctx.builder().position_at_end(body_bb);
            let idx = ctx.builder().build_load(i64_ty, idx_ptr, "idx").unwrap().into_int_value();
            let one = i64_ty.const_int(1, false);

            // Call __string_char_at(str, idx) to get single-char string
            let char_at_fn = ctx
                .module
                .get_function("__string_char_at")
                .expect("__string_char_at not declared");
            let ch = ctx
                .builder()
                .build_call(char_at_fn, &[str_ptr.into(), idx.into()], "char_at")
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic()
                .into_pointer_value();
            ctx.builder().build_store(var_ptr, ch).unwrap();

            compile_stmt(ctx, body)?;

            if ctx
                .builder()
                .get_insert_block()
                .unwrap()
                .get_terminator()
                .is_none()
            {
                let next_idx = ctx.builder().build_int_add(idx, one, "next_idx").unwrap();
                ctx.builder().build_store(idx_ptr, next_idx).unwrap();
                ctx.builder().build_unconditional_branch(cond_bb).unwrap();
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
            let temp_ptr = ctx.builder().build_alloca(i8_ptr, temp_name).unwrap();
            ctx.builder().build_store(temp_ptr, iterable_ptr).unwrap();
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
            let iter_ptr = ctx.builder().build_alloca(i8_ptr, iter_name).unwrap();
            ctx.builder().build_store(iter_ptr, iter_obj_result.value).unwrap();
            let old_iter = ctx
                .variables
                .insert(iter_name.to_string(), (iter_ptr, Type::Dynamic));

            let cond_bb = ctx.context.append_basic_block(func, "for_of_cond");
            let body_bb = ctx.context.append_basic_block(func, "for_of_body");
            let end_bb = ctx.context.append_basic_block(func, "for_of_end");

            ctx.builder().build_unconditional_branch(cond_bb).unwrap();

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
            let next_int = ctx.builder().build_ptr_to_int(next_ptr, i64_ty, "next_int").unwrap();
            let is_null = ctx.builder().build_int_compare(
                inkwell::IntPredicate::EQ,
                next_int,
                i64_ty.const_int(0, false),
                "is_null",
            ).unwrap();
            ctx.builder()
                .build_conditional_branch(is_null, end_bb, body_bb).unwrap();

            let label = ctx.pending_loop_label.take();
            ctx.push_loop(end_bb, cond_bb, label);

            ctx.builder().position_at_end(body_bb);
            let var_ptr = ctx.builder().build_alloca(i8_ptr, variable).unwrap();
            ctx.builder().build_store(var_ptr, next_result.value).unwrap();
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
                ctx.builder().build_unconditional_branch(cond_bb).unwrap();
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
            ctx.builder().build_store(result_ptr, result.value).unwrap();
        }
        ctx.builder().build_unconditional_branch(return_bb).unwrap();
        return Ok(());
    }

    // EX-C1 修复：检查是否在 try 块内，如果是则先执行 finally
    if let Some(try_ctx) = ctx.current_try() {
        if let Some(finally_bb) = try_ctx.finally_bb {
            // 将返回值存入 pending slot（懒分配）
            if let Some(e) = expr {
                let result = compile_expr(ctx, e)?;
                if ctx.pending_return_value.is_none() {
                    let alloca = ctx
                        .builder()
                        .build_alloca(result.value.get_type(), "pending_ret_val").unwrap();
                    ctx.pending_return_value = Some(alloca);
                }
                if let Some(ret_ptr) = ctx.pending_return_value {
                    ctx.builder().build_store(ret_ptr, result.value).unwrap();
                }
            }
            // 设置 pending return 标志
            if let Some(flag_ptr) = ctx.pending_return_flag {
                ctx.builder()
                    .build_store(flag_ptr, ctx.context.bool_type().const_int(1, false)).unwrap();
            }
            // 跳转到 finally 块
            ctx.builder().build_unconditional_branch(finally_bb).unwrap();
            // 创建不可达块以保持 builder 位置
            if let Some(func) = ctx.current_function() {
                let unreachable_bb = ctx.context.append_basic_block(func, "return.unreachable");
                ctx.builder().position_at_end(unreachable_bb);
            }
            return Ok(());
        }
    }

    ctx.emit_gc_root_removals();
    match expr {
        Some(e) => {
            let result = compile_expr(ctx, e)?;
            // Cast return value to match the function's declared return type.
            // This is needed when returning function pointers (compiled as
            // concrete function types) where the declared return type is
            // `fn(...)` which compiles to `i8*`.
            let return_value = if let Some(expected_ty) = ctx.current_return_type() {
                use super::types::ruyi_type_to_llvm;
                let expected_llvm_ty = ruyi_type_to_llvm(ctx.context, expected_ty);
                let actual_llvm_ty = result.value.get_type();
                if expected_llvm_ty != actual_llvm_ty {
                    // Cast return value to match expected type
                    use inkwell::values::BasicValueEnum;
                    if actual_llvm_ty.is_pointer_type() && expected_llvm_ty.is_pointer_type() {
                        // Pointer to pointer: bitcast
                        ctx.builder()
                            .build_bit_cast(result.value, expected_llvm_ty, "ret_cast")
                            .unwrap()
                    } else if actual_llvm_ty.is_pointer_type() && expected_llvm_ty.is_int_type() {
                        // Pointer to int (e.g., null to i64 for Nullable<T>): ptrtoint
                        BasicValueEnum::IntValue(ctx.builder().build_ptr_to_int(
                            result.value.into_pointer_value(),
                            expected_llvm_ty.into_int_type(),
                            "ret_ptr_to_int",
                        ).unwrap())
                    } else if actual_llvm_ty.is_int_type() && expected_llvm_ty.is_pointer_type() {
                        // Int to pointer: inttoptr
                        BasicValueEnum::PointerValue(ctx.builder().build_int_to_ptr(
                            result.value.into_int_value(),
                            expected_llvm_ty.into_pointer_type(),
                            "ret_int_to_ptr",
                        ).unwrap())
                    } else if actual_llvm_ty.is_struct_type() && expected_llvm_ty.is_int_type() {
                        // Struct to int (e.g., Dynamic {i64, i8*} → i64):
                        // Extract field 1 (data_ptr) and convert to int.
                        let sv = result.value.into_struct_value();
                        let data_ptr = ctx
                            .builder()
                            .build_extract_value(sv, 1, "ret_data")
                            .unwrap()
                            .into_pointer_value();
                        BasicValueEnum::IntValue(ctx.builder().build_ptr_to_int(
                            data_ptr,
                            expected_llvm_ty.into_int_type(),
                            "ret_s2i",
                        ).unwrap())
                    } else if actual_llvm_ty.is_struct_type() && expected_llvm_ty.is_pointer_type()
                    {
                        // Struct to pointer (e.g., Dynamic {i64, i8*} → i8*):
                        // Extract field 1 (data_ptr) directly.
                        let sv = result.value.into_struct_value();
                        let data_ptr = ctx
                            .builder()
                            .build_extract_value(sv, 1, "ret_s2p")
                            .unwrap()
                            .into_pointer_value();
                        BasicValueEnum::PointerValue(data_ptr)
                    } else {
                        result.value
                    }
                } else {
                    result.value
                }
            } else {
                result.value
            };
            // Second-pass: ensure the value matches the LLVM function's actual
            // return type. This handles generic type erasure where the Ruyi type
            // (e.g. Nullable(Dynamic) → {i64, i8*}) differs from the erased LLVM
            // return type (e.g. i64).
            let return_value = if let Some(func) = ctx.current_function() {
                let fn_ret_ty = func.get_type().get_return_type();
                if let Some(fn_ret_ty) = fn_ret_ty {
                    let val_ty = return_value.get_type();
                    if fn_ret_ty != val_ty {
                        use inkwell::values::BasicValueEnum;
                        if val_ty.is_pointer_type() && fn_ret_ty.is_int_type() {
                            BasicValueEnum::IntValue(ctx.builder().build_ptr_to_int(
                                return_value.into_pointer_value(),
                                fn_ret_ty.into_int_type(),
                                "ret_fn_ptr2int",
                            ).unwrap())
                        } else if val_ty.is_struct_type() && fn_ret_ty.is_int_type() {
                            let sv = return_value.into_struct_value();
                            let data_ptr = ctx
                                .builder()
                                .build_extract_value(sv, 1, "ret_fn_data")
                                .unwrap()
                                .into_pointer_value();
                            BasicValueEnum::IntValue(ctx.builder().build_ptr_to_int(
                                data_ptr,
                                fn_ret_ty.into_int_type(),
                                "ret_fn_s2i",
                            ).unwrap())
                        } else if val_ty.is_int_type() && fn_ret_ty.is_pointer_type() {
                            BasicValueEnum::PointerValue(ctx.builder().build_int_to_ptr(
                                return_value.into_int_value(),
                                fn_ret_ty.into_pointer_type(),
                                "ret_fn_int2ptr",
                            ).unwrap())
                        } else if val_ty.is_pointer_type() && fn_ret_ty.is_pointer_type() {
                            ctx.builder()
                                .build_bit_cast(return_value, fn_ret_ty, "ret_fn_p2p")
                                .unwrap()
                        } else if val_ty.is_int_type() && fn_ret_ty.is_int_type() {
                            let val_int = return_value.into_int_value();
                            let src_bits = val_int.get_type().get_bit_width();
                            let dst_bits = fn_ret_ty.into_int_type().get_bit_width();
                            if src_bits > dst_bits {
                                BasicValueEnum::IntValue(ctx.builder().build_int_truncate(
                                    val_int,
                                    fn_ret_ty.into_int_type(),
                                    "ret_fn_trunc",
                                ).unwrap())
                            } else {
                                BasicValueEnum::IntValue(ctx.builder().build_int_z_extend(
                                    val_int,
                                    fn_ret_ty.into_int_type(),
                                    "ret_fn_zext",
                                ).unwrap())
                            }
                        } else if val_ty.is_int_type() && fn_ret_ty.is_struct_type() {
                            // Int to Dynamic struct: box as {type_tag=1, inttoptr(value)}
                            let dyn_st = fn_ret_ty.into_struct_type();
                            let mut ds = dyn_st.const_zero();
                            let type_tag = ctx.context.i64_type().const_int(1, false);
                            ds = ctx
                                .builder()
                                .build_insert_value(ds, type_tag, 0, "box_type_tag")
                                .unwrap()
                                .into_struct_value();
                            let data_ptr = ctx.builder().build_int_to_ptr(
                                return_value.into_int_value(),
                                ctx.context.ptr_type(Default::default()),
                                "box_int_data",
                            ).unwrap();
                            ds = ctx
                                .builder()
                                .build_insert_value(ds, data_ptr, 1, "box_data")
                                .unwrap()
                                .into_struct_value();
                            BasicValueEnum::StructValue(ds)
                        } else if val_ty.is_pointer_type() && fn_ret_ty.is_struct_type() {
                            // Pointer to Dynamic struct: box as {0, ptr}
                            let dyn_st = fn_ret_ty.into_struct_type();
                            let mut ds = dyn_st.const_zero();
                            let casted = ctx.builder().build_bit_cast(
                                return_value,
                                ctx.context.ptr_type(Default::default()),
                                "box_data_ptr",
                            ).unwrap();
                            ds = ctx
                                .builder()
                                .build_insert_value(ds, casted, 1, "box_data")
                                .unwrap()
                                .into_struct_value();
                            BasicValueEnum::StructValue(ds)
                        } else if val_ty.is_struct_type() && fn_ret_ty.is_pointer_type() {
                            // Dynamic struct → i8*：提取 data_ptr 字段作为返回值
                            // （泛型擦除场景：函数返回 T 擦除为 i8*，但实际值是 Dynamic struct）
                            let sv = return_value.into_struct_value();
                            let data_ptr = ctx
                                .builder()
                                .build_extract_value(sv, 1, "ret_fn_s2p")
                                .unwrap()
                                .into_pointer_value();
                            BasicValueEnum::PointerValue(data_ptr)
                        } else {
                            return_value
                        }
                    } else {
                        return_value
                    }
                } else {
                    // LLVM function returns void — ignore the expression value
                    ctx.builder().build_return(None).unwrap();
                    return Ok(());
                }
            } else {
                return_value
            };
            ctx.builder().build_return(Some(&return_value)).unwrap();
            Ok(())
        }
        None => {
            ctx.builder().build_return(None).unwrap();
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
                || name.chars().next().is_some_and(|c| c.is_uppercase());
            if !is_class {
                let exc_result = compile_expr(ctx, expr)?;
                let exc_ptr = match exc_result.value {
                    BasicValueEnum::PointerValue(v) => v,
                    _ => return Err("throw expression must evaluate to a pointer".to_string()),
                };
                emit_throw_call(ctx, exc_ptr, None)?;
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
            emit_throw_call(ctx, exc_ptr, None)?;
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
                let str_ptr = ctx.builder().build_global_string_ptr(s, "throw_msg").unwrap();
                let exc_ptr = str_ptr.as_pointer_value();
                emit_throw_call(ctx, exc_ptr, throw_class.as_deref())?;
            }
            _ => {
                let class_name =
                    throw_class.ok_or("throw requires a class name for non-literal arguments")?;
                let args_cloned: Vec<crate::parser::ast::Argument> =
                    args.iter().map(|a| (*a).clone()).collect();
                let exc_result = super::expr::compile_new(
                    ctx,
                    &Expr::Identifier(class_name.clone()),
                    &args_cloned,
                )?;
                let exc_ptr = match exc_result.value {
                    BasicValueEnum::PointerValue(v) => v,
                    _ => return Err("throw expression must evaluate to a pointer".to_string()),
                };
                emit_throw_call(ctx, exc_ptr, Some(class_name.as_str()))?;
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
    let i8_ptr = ctx.context.ptr_type(Default::default());
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

    let exception_ptr = ctx.builder().build_alloca(i8_ptr, "exc_ptr").unwrap();
    ctx.builder()
        .build_store(exception_ptr, i8_ptr.const_null()).unwrap();

    let clear_fn = ctx
        .module
        .get_function("ruyi_clear_pending_exception")
        .expect("ruyi_clear_pending_exception not declared");
    ctx.builder().build_call(clear_fn, &[], "clear_exc").unwrap();

    ctx.builder().build_unconditional_branch(try_body_bb).unwrap();
    ctx.builder().position_at_end(try_body_bb);

    // EX-C1/C2 修复：如果有 finally 块，设置 pending control flow 支持
    let old_pending_return_flag = ctx.pending_return_flag;
    let old_pending_return_value = ctx.pending_return_value;
    let old_pending_break_target = ctx.pending_break_target;
    let old_pending_continue_target = ctx.pending_continue_target;

    if finally_bb.is_some() {
        let bool_ty = ctx.context.bool_type();
        let flag_alloca = ctx.builder().build_alloca(bool_ty, "pending_ret_flag").unwrap();
        ctx.builder()
            .build_store(flag_alloca, bool_ty.const_int(0, false)).unwrap();
        ctx.pending_return_flag = Some(flag_alloca);
        // pending_return_value 在 compile_return 中懒分配
        ctx.pending_return_value = None;
        ctx.pending_break_target = None;
        ctx.pending_continue_target = None;
    }

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
            ctx.builder().build_unconditional_branch(fb).unwrap();
        } else {
            ctx.builder().build_unconditional_branch(merge_bb).unwrap();
        }
    }

    ctx.pop_try();

    // ── T4: LLVM landing-pad generation (for invoke-based exception handling) ──
    let landing_pad_val;
    {
        let lp_gen = LandingPadGenerator::new(ctx.context, ctx.module, ctx.builder());

        // Create per-catch handler blocks; EX-H1: map type annotations to real type IDs
        let mut catch_handlers: Vec<(
            ruyi_exception::TryTypeId,
            inkwell::basic_block::BasicBlock<'ctx>,
        )> = Vec::new();
        for (i, catch_clause) in catch.iter().enumerate() {
            let handler_bb = ctx
                .context
                .append_basic_block(func, &format!("try.catch.{}", i));
            let type_id = catch_clause
                .ty
                .as_ref()
                .map(catch_type_to_type_id)
                .unwrap_or(0u32); // no type annotation = catch-all
            catch_handlers.push((type_id, handler_bb));
        }

        let catch_type_ids: Vec<ruyi_exception::TryTypeId> =
            catch_handlers.iter().map(|(id, _)| *id).collect();
        let has_cleanup = finally.is_some();

        ctx.builder().position_at_end(landing_pad_bb);
        landing_pad_val = lp_gen.build_landing_pad(&catch_type_ids, has_cleanup, "landingpad");

        // Extract exception pointer and store it for catch blocks to access
        let exc_ptr = lp_gen.extract_exception_ptr(landing_pad_val);
        ctx.builder().build_store(exception_ptr, exc_ptr).unwrap();

        // Dispatch from landing-pad to first catch handler (catch-all mode).
        // Must be called while builder is still positioned inside landing_pad_bb
        // so the branch is emitted from the correct block.
        lp_gen.build_catch_dispatch(landing_pad_val, &catch_handlers, finally_bb, resume_bb);

        // Forward old catch_bb (used by compile_throw/build_exception_check)
        // to the first handler block so both old and new paths reach the same code
        if let Some(cb) = catch_bb {
            ctx.builder().position_at_end(cb);
            ctx.builder()
                .build_unconditional_branch(catch_handlers[0].1).unwrap();
        }

        // Compile per-clause catch handlers
        for (i, catch_clause) in catch.iter().enumerate() {
            let handler_bb = catch_handlers[i].1;
            ctx.builder().position_at_end(handler_bb);

            build_ruyi_clear_pending_exception(ctx.builder(), ctx.module);

            let exc_val = ctx
                .builder()
                .build_load(i8_ptr, exception_ptr, "exc_val").unwrap()
                .into_pointer_value();
            if let Some(pattern) = &catch_clause.pattern {
                if let crate::parser::ast::Pattern::Identifier(name) = pattern {
                    let local_ptr = ctx.builder().build_alloca(i8_ptr, name).unwrap();
                    ctx.builder().build_store(local_ptr, exc_val).unwrap();
                    let var_ty = catch_clause
                        .ty
                        .as_ref()
                        .map(Type::from_annotation)
                        .unwrap_or(Type::Dynamic);
                    ctx.define_variable(name.clone(), (local_ptr, var_ty));
                }
            }

            compile_block(ctx, &catch_clause.body)?;

            let catch_end = ctx.builder().get_insert_block().unwrap();
            if catch_end.get_terminator().is_none() {
                if let Some(fb) = finally_bb {
                    ctx.builder().build_unconditional_branch(fb).unwrap();
                } else {
                    ctx.builder().build_unconditional_branch(merge_bb).unwrap();
                }
            }
        }
    }
    // _try_guard drops here, popping try_frame_stack

    // Uncaught exception: build resume block
    {
        let lp_gen = LandingPadGenerator::new(ctx.context, ctx.module, ctx.builder());
        ctx.builder().position_at_end(resume_bb);
        lp_gen.build_resume(landing_pad_val);
    }

    if let Some(finally_stmts) = finally {
        let fb = finally_bb.unwrap();
        ctx.builder().position_at_end(fb);

        compile_block(ctx, finally_stmts)?;

        let finally_end = ctx.builder().get_insert_block().unwrap();
        if finally_end.get_terminator().is_none() {
            // EX-C1/C2 修复：检查 pending control flow
            let mut next_bb = if catch.is_empty() {
                // 无 catch：检查是否有未捕获异常需要传播
                let exc_val = ctx
                    .builder()
                    .build_load(i8_ptr, exception_ptr, "exc_val").unwrap()
                    .into_pointer_value();
                let exc_int = ctx.builder().build_ptr_to_int(exc_val, i64_ty, "exc_int").unwrap();
                let is_null = ctx.builder().build_int_compare(
                    inkwell::IntPredicate::EQ,
                    exc_int,
                    i64_ty.const_int(0, false),
                    "is_null",
                ).unwrap();
                let pb = propagate_bb.unwrap();
                let normal_bb = ctx.context.append_basic_block(func, "finally.normal");
                ctx.builder()
                    .build_conditional_branch(is_null, normal_bb, pb).unwrap();

                ctx.builder().position_at_end(pb);
                let exc_val2 = ctx
                    .builder()
                    .build_load(i8_ptr, exception_ptr, "exc_val2").unwrap()
                    .into_pointer_value();
                let throw_fn = ctx.module.get_function("ruyi_rethrow").unwrap_or_else(|| {
                    let i8_ptr = ctx.context.ptr_type(Default::default());
                    let fn_type = ctx.context.void_type().fn_type(&[i8_ptr.into()], false);
                    ctx.module.add_function("ruyi_rethrow", fn_type, None)
                });
                ctx.builder()
                    .build_call(throw_fn, &[exc_val2.into()], "rethrow").unwrap();
                ctx.emit_gc_root_removals();
                ctx.builder().build_return(None).unwrap();

                ctx.builder().position_at_end(normal_bb);
                merge_bb
            } else {
                merge_bb
            };

            // 检查 pending return
            if let Some(flag_ptr) = ctx.pending_return_flag {
                let flag_val = ctx
                    .builder()
                    .build_load(ctx.context.bool_type(), flag_ptr, "ret_flag_val").unwrap()
                    .into_int_value();
                let ret_bb = ctx.context.append_basic_block(func, "finally.return");
                let no_ret_bb = ctx.context.append_basic_block(func, "finally.no_return");
                ctx.builder()
                    .build_conditional_branch(flag_val, ret_bb, no_ret_bb).unwrap();

                ctx.builder().position_at_end(ret_bb);
                ctx.emit_gc_root_removals();
                if let Some(ret_ptr) = ctx.pending_return_value {
                    let ret_ty = func.get_type().get_return_type().unwrap();
                    let ret_val = ctx.builder().build_load(ret_ty, ret_ptr, "pending_ret_val").unwrap();
                    ctx.builder().build_return(Some(&ret_val)).unwrap();
                } else {
                    ctx.builder().build_return(None).unwrap();
                }

                ctx.builder().position_at_end(no_ret_bb);
                next_bb = merge_bb;
            }

            // 检查 pending break
            if let Some(break_target) = ctx.pending_break_target {
                let break_bb = ctx.context.append_basic_block(func, "finally.break");
                let no_break_bb = ctx.context.append_basic_block(func, "finally.no_break");
                // 复用 return flag 来标记 break（break 和 return 不会同时发生）
                let has_break = if let Some(flag_ptr) = ctx.pending_return_flag {
                    ctx.builder()
                        .build_load(ctx.context.bool_type(), flag_ptr, "break_flag_val").unwrap()
                        .into_int_value()
                } else {
                    ctx.context.bool_type().const_int(0, false)
                };
                ctx.builder()
                    .build_conditional_branch(has_break, break_bb, no_break_bb).unwrap();

                ctx.builder().position_at_end(break_bb);
                ctx.builder().build_unconditional_branch(break_target).unwrap();

                ctx.builder().position_at_end(no_break_bb);
            }

            // 检查 pending continue
            if let Some(continue_target) = ctx.pending_continue_target {
                let cont_bb = ctx.context.append_basic_block(func, "finally.continue");
                let no_cont_bb = ctx.context.append_basic_block(func, "finally.no_continue");
                let has_cont = if let Some(flag_ptr) = ctx.pending_return_flag {
                    ctx.builder()
                        .build_load(ctx.context.bool_type(), flag_ptr, "cont_flag_val").unwrap()
                        .into_int_value()
                } else {
                    ctx.context.bool_type().const_int(0, false)
                };
                ctx.builder()
                    .build_conditional_branch(has_cont, cont_bb, no_cont_bb).unwrap();

                ctx.builder().position_at_end(cont_bb);
                ctx.builder().build_unconditional_branch(continue_target).unwrap();

                ctx.builder().position_at_end(no_cont_bb);
            }

            // 正常流程：跳转到 merge
            let end_bb = ctx.builder().get_insert_block().unwrap();
            if end_bb.get_terminator().is_none() {
                ctx.builder().build_unconditional_branch(next_bb).unwrap();
            }
        }
    }

    // 恢复旧的 pending 状态
    ctx.pending_return_flag = old_pending_return_flag;
    ctx.pending_return_value = old_pending_return_value;
    ctx.pending_break_target = old_pending_break_target;
    ctx.pending_continue_target = old_pending_continue_target;

    ctx.builder().position_at_end(merge_bb);
    Ok(())
}

fn emit_throw_call<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    exc_ptr: inkwell::values::PointerValue<'ctx>,
    class_name: Option<&str>,
) -> Result<(), String> {
    if let Some(name) = class_name {
        // EX-H3: typed throw — pass class name + message to runtime
        let typed_throw_fn = ctx
            .module
            .get_function("ruyi_throw_typed")
            .unwrap_or_else(|| {
                let i8_ptr = ctx.context.ptr_type(Default::default());
                let fn_type = ctx
                    .context
                    .void_type()
                    .fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
                ctx.module.add_function("ruyi_throw_typed", fn_type, None)
            });
        let name_ptr = ctx.builder().build_global_string_ptr(name, "throw_class").unwrap();
        ctx.builder().build_call(
            typed_throw_fn,
            &[name_ptr.as_pointer_value().into(), exc_ptr.into()],
            "throw_typed",
        ).unwrap();
    } else {
        let throw_fn = ctx
            .module
            .get_function("ruyi_throw")
            .expect("ruyi_throw not declared");
        ctx.builder()
            .build_call(throw_fn, &[exc_ptr.into()], "throw").unwrap();
    }
    emit_throw_branch(ctx, exc_ptr)
}

/// Helper: emit the branch/unreachable after the throw call.
fn emit_throw_branch<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    exc_ptr: inkwell::values::PointerValue<'ctx>,
) -> Result<(), String> {
    if let Some(try_ctx) = ctx.current_try() {
        ctx.builder().build_store(try_ctx.exception_ptr, exc_ptr).unwrap();
        if let Some(catch_bb) = try_ctx.catch_bb {
            ctx.builder().build_unconditional_branch(catch_bb).unwrap();
        } else if let Some(finally_bb) = try_ctx.finally_bb {
            ctx.builder().build_unconditional_branch(finally_bb).unwrap();
        } else {
            ctx.builder().build_unconditional_branch(try_ctx.merge_bb).unwrap();
        }
        if let Some(func) = ctx.current_function() {
            let unreachable_bb = ctx.context.append_basic_block(func, "throw.unreachable");
            ctx.builder().position_at_end(unreachable_bb);
            ctx.builder().build_unreachable().unwrap();
        }
    } else {
        ctx.builder().build_unreachable().unwrap();
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
    let pending = build_ruyi_get_pending_exception(ctx.builder(), ctx.module);

    let i64_ty = ctx.context.i64_type();
    let pending_int = ctx
        .builder()
        .build_ptr_to_int(pending, i64_ty, "pending_int").unwrap();
    let is_null = ctx.builder().build_int_compare(
        inkwell::IntPredicate::EQ,
        pending_int,
        i64_ty.const_int(0, false),
        "no_exc",
    ).unwrap();

    let try_ctx = ctx.current_try().unwrap();
    let continue_bb = ctx.context.append_basic_block(func, "after_exc_check");
    let store_exc_bb = ctx.context.append_basic_block(func, "store_exc");

    let dest_bb = try_ctx
        .catch_bb
        .or(try_ctx.finally_bb)
        .unwrap_or(try_ctx.merge_bb);

    ctx.builder()
        .build_conditional_branch(is_null, continue_bb, store_exc_bb).unwrap();

    ctx.builder().position_at_end(store_exc_bb);
    ctx.builder().build_store(try_ctx.exception_ptr, pending).unwrap();
    build_ruyi_clear_pending_exception(ctx.builder(), ctx.module);
    ctx.builder().build_unconditional_branch(dest_bb).unwrap();

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
        .build_conditional_branch(is_match, then_bb, else_bb).unwrap();

    ctx.builder().position_at_end(then_bb);
    bind_pattern_in_codegen(ctx, pattern, &val)?;
    compile_stmt(ctx, then_branch)?;
    if let Some(bb) = ctx.builder().get_insert_block() {
        if bb.get_terminator().is_none() {
            ctx.builder().build_unconditional_branch(merge_bb).unwrap();
        }
    }

    ctx.builder().position_at_end(else_bb);
    if let Some(else_stmt) = else_branch {
        compile_stmt(ctx, else_stmt)?;
    }
    if let Some(bb) = ctx.builder().get_insert_block() {
        if bb.get_terminator().is_none() {
            ctx.builder().build_unconditional_branch(merge_bb).unwrap();
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

    ctx.builder().build_unconditional_branch(header_bb).unwrap();

    ctx.builder().position_at_end(header_bb);
    let val = compile_expr(ctx, value)?;
    let val_llvm_ty = super::types::ruyi_type_to_llvm(ctx.context, &val.ty);
    let val_ptr = ctx.builder().build_alloca(
        val_llvm_ty,
        "while_let_val",
    ).unwrap();
    ctx.builder().build_store(val_ptr, val.value).unwrap();

    let is_match = pattern_is_matching(ctx, pattern, &val)?;
    ctx.builder()
        .build_conditional_branch(is_match, body_bb, exit_bb).unwrap();

    ctx.builder().position_at_end(body_bb);
    let loaded_val = ctx.builder().build_load(val_llvm_ty, val_ptr, "while_let_loaded").unwrap();
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
            ctx.builder().build_unconditional_branch(header_bb).unwrap();
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
                        let ptr_int = ctx.builder().build_ptr_to_int(p, i64_ty, "ptr_int").unwrap();
                        ctx.builder().build_int_compare(
                            inkwell::IntPredicate::NE,
                            ptr_int,
                            i64_ty.const_int(0, false),
                            "is_non_null",
                        ).unwrap()
                    }
                    BasicValueEnum::IntValue(v) => ctx.builder().build_int_compare(
                        inkwell::IntPredicate::NE,
                        v,
                        ctx.context.i64_type().const_all_ones(),
                        "is_non_null_int",
                    ).unwrap(),
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
            ).unwrap();
            Ok(cmp)
        }
        P::Object(_) | P::Array(_) => Ok(ctx.context.bool_type().const_int(1, false)),
        P::As(inner, _) => pattern_is_matching(ctx, inner, val),
        P::Or(patterns) => {
            let mut result = ctx.context.bool_type().const_int(0, false);
            for p in patterns {
                let m = pattern_is_matching(ctx, p, val)?;
                result = ctx.builder().build_or(result, m, "or_match").unwrap();
            }
            Ok(result)
        }
        P::Rest(_) => Ok(ctx.context.bool_type().const_int(1, false)),
    }
}

pub(super) fn bind_pattern_in_codegen<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    pattern: &crate::parser::ast::Pattern,
    val: &ExprResult<'ctx>,
) -> Result<(), String> {
    use crate::parser::ast::{ObjectPatternField, Pattern as P};
    match pattern {
        P::Identifier(name) => {
            let llvm_ty = super::types::ruyi_type_to_llvm(ctx.context, &val.ty);
            let actual_ty = val.value.get_type();
            // Box the value when the alloca type (from Ruyi type inference)
            // differs from the actual LLVM value type. This handles generic
            // type erasure where the Ruyi type is Dynamic ({i64, i8*}) but
            // the runtime value is i64 or a pointer.
            let store_val = if llvm_ty != actual_ty {
                use inkwell::values::BasicValueEnum;
                if llvm_ty.is_struct_type() && actual_ty.is_int_type() {
                    // Int → Dynamic struct: box as {value, inttoptr(1)}
                    let dyn_st = llvm_ty.into_struct_type();
                    let mut ds = dyn_st.const_zero();
                    let type_tag = ctx.context.i64_type().const_int(1, false);
                    ds = ctx
                        .builder()
                        .build_insert_value(ds, type_tag, 0, "box_type_tag")
                        .unwrap()
                        .into_struct_value();
                    let data = ctx.builder().build_int_to_ptr(
                        val.value.into_int_value(),
                        ctx.context.ptr_type(Default::default()),
                        "box_int_data",
                    ).unwrap();
                    ds = ctx
                        .builder()
                        .build_insert_value(ds, data, 1, "box_data")
                        .unwrap()
                        .into_struct_value();
                    BasicValueEnum::StructValue(ds)
                } else if llvm_ty.is_struct_type() && actual_ty.is_pointer_type() {
                    // Pointer → Dynamic struct: box as {0, ptr}
                    let dyn_st = llvm_ty.into_struct_type();
                    let mut ds = dyn_st.const_zero();
                    let casted = ctx.builder().build_bit_cast(
                        val.value,
                        ctx.context.ptr_type(Default::default()),
                        "box_data_ptr",
                    ).unwrap();
                    ds = ctx
                        .builder()
                        .build_insert_value(ds, casted, 1, "box_data")
                        .unwrap()
                        .into_struct_value();
                    BasicValueEnum::StructValue(ds)
                } else if llvm_ty.is_int_type() && actual_ty.is_struct_type() {
                    // Dynamic struct → int: extract data ptr and convert
                    let sv = val.value.into_struct_value();
                    let data_ptr = ctx
                        .builder()
                        .build_extract_value(sv, 1, "unbox_data")
                        .unwrap()
                        .into_pointer_value();
                    BasicValueEnum::IntValue(ctx.builder().build_ptr_to_int(
                        data_ptr,
                        llvm_ty.into_int_type(),
                        "unbox_s2i",
                    ).unwrap())
                } else {
                    val.value
                }
            } else {
                val.value
            };
            let ptr = ctx.builder().build_alloca(llvm_ty, name).unwrap();
            ctx.builder().build_store(ptr, store_val).unwrap();
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
                    let struct_type = *ctx
                        .class_struct_types
                        .get(class_name)
                        .ok_or_else(|| format!("No struct type for class: {}", class_name))?;

                    let struct_ptr = ctx.builder().build_pointer_cast(
                        obj_ptr,
                        ctx.context.ptr_type(Default::default()),
                        "obj_struct_cast",
                    ).unwrap();

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

                                let field_llvm_ty = super::types::ruyi_type_to_llvm(ctx.context, field_ty);
                                let field_ptr = unsafe {
                                    ctx.builder().build_gep(
                                        struct_type,
                                        struct_ptr,
                                        &[
                                            i32_ty.const_int(0, false),
                                            i32_ty.const_int(field_index as u64, false),
                                        ],
                                        &format!("{}_ptr", key),
                                    ).unwrap()
                                };
                                let field_val = ctx.builder().build_load(field_llvm_ty, field_ptr, key).unwrap();
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

                                let llvm_ty =
                                    super::types::ruyi_type_to_llvm(ctx.context, field_ty);
                                let field_ptr = unsafe {
                                    ctx.builder().build_gep(
                                        struct_type,
                                        struct_ptr,
                                        &[
                                            i32_ty.const_int(0, false),
                                            i32_ty.const_int(field_index as u64, false),
                                        ],
                                        &format!("{}_ptr", name),
                                    ).unwrap()
                                };
                                let field_val = ctx.builder().build_load(llvm_ty, field_ptr, name).unwrap();
                                let ptr = ctx.builder().build_alloca(llvm_ty, name).unwrap();
                                ctx.builder().build_store(ptr, field_val).unwrap();
                                ctx.define_variable(name.clone(), (ptr, field_ty.clone()));
                            }
                            ObjectPatternField::ShorthandDefault(name, default_expr) => {
                                // 字段存在时从对象加载，不存在时使用默认值
                                if let Some(field_index) =
                                    class_fields.iter().position(|(n, _)| n == name)
                                {
                                    let (_, field_ty) = &class_fields[field_index];
                                    let llvm_ty =
                                        super::types::ruyi_type_to_llvm(ctx.context, field_ty);
                                    let field_ptr = unsafe {
                                        ctx.builder().build_gep(
                                            struct_type,
                                            struct_ptr,
                                            &[
                                                i32_ty.const_int(0, false),
                                                i32_ty.const_int(field_index as u64, false),
                                            ],
                                            &format!("{}_ptr", name),
                                        ).unwrap()
                                    };
                                    let field_val = ctx.builder().build_load(llvm_ty, field_ptr, name).unwrap();
                                    let ptr = ctx.builder().build_alloca(llvm_ty, name).unwrap();
                                    ctx.builder().build_store(ptr, field_val).unwrap();
                                    ctx.define_variable(name.clone(), (ptr, field_ty.clone()));
                                } else {
                                    let default_result =
                                        super::expr::compile_expr(ctx, default_expr)?;
                                    let llvm_ty = super::types::ruyi_type_to_llvm(
                                        ctx.context,
                                        &default_result.ty,
                                    );
                                    let ptr = ctx.builder().build_alloca(llvm_ty, name).unwrap();
                                    ctx.builder().build_store(ptr, default_result.value).unwrap();
                                    ctx.define_variable(
                                        name.clone(),
                                        (ptr, default_result.ty.clone()),
                                    );
                                }
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
                                let field_llvm_ty = super::types::ruyi_type_to_llvm(ctx.context, field_ty);
                                let field_ptr = unsafe {
                                    ctx.builder().build_gep(
                                        ctx.context.i8_type(),
                                        obj_ptr,
                                        &[offset],
                                        &format!("{}_ptr", key),
                                    ).unwrap()
                                };
                                let typed_ptr = ctx.builder().build_bit_cast(
                                    field_ptr,
                                    ctx.context.ptr_type(Default::default()),
                                    &format!("{}_typed_ptr", key),
                                ).unwrap();
                                let field_val = ctx
                                    .builder()
                                    .build_load(field_llvm_ty, typed_ptr.into_pointer_value(), key).unwrap();
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
                                let llvm_ty =
                                    super::types::ruyi_type_to_llvm(ctx.context, field_ty);
                                let field_ptr = unsafe {
                                    ctx.builder().build_gep(
                                        ctx.context.i8_type(),
                                        obj_ptr,
                                        &[offset],
                                        &format!("{}_ptr", name),
                                    ).unwrap()
                                };
                                let typed_ptr = ctx.builder().build_bit_cast(
                                    field_ptr,
                                    ctx.context.ptr_type(Default::default()),
                                    &format!("{}_typed_ptr", name),
                                ).unwrap();
                                let field_val = ctx
                                    .builder()
                                    .build_load(llvm_ty, typed_ptr.into_pointer_value(), name).unwrap();
                                let ptr = ctx.builder().build_alloca(llvm_ty, name).unwrap();
                                ctx.builder().build_store(ptr, field_val).unwrap();
                                ctx.define_variable(name.clone(), (ptr, field_ty.clone()));
                            }
                            ObjectPatternField::ShorthandDefault(name, default_expr) => {
                                // 字段存在时从对象加载，不存在时使用默认值
                                if let Some(field_index) =
                                    type_fields.iter().position(|f| f.name == *name)
                                {
                                    let field_ty = &type_fields[field_index].ty;
                                    let offset = i32_ty.const_int((field_index * 8) as u64, false);
                                    let llvm_ty =
                                        super::types::ruyi_type_to_llvm(ctx.context, field_ty);
                                    let field_ptr = unsafe {
                                        ctx.builder().build_gep(
                                            ctx.context.i8_type(),
                                            obj_ptr,
                                            &[offset],
                                            &format!("{}_ptr", name),
                                        ).unwrap()
                                    };
                                    let typed_ptr = ctx.builder().build_bit_cast(
                                        field_ptr,
                                        ctx.context.ptr_type(Default::default()),
                                        &format!("{}_typed_ptr", name),
                                    ).unwrap();
                                    let field_val = ctx
                                        .builder()
                                        .build_load(llvm_ty, typed_ptr.into_pointer_value(), name).unwrap();
                                    let ptr = ctx.builder().build_alloca(llvm_ty, name).unwrap();
                                    ctx.builder().build_store(ptr, field_val).unwrap();
                                    ctx.define_variable(name.clone(), (ptr, field_ty.clone()));
                                } else {
                                    let default_result =
                                        super::expr::compile_expr(ctx, default_expr)?;
                                    let llvm_ty = super::types::ruyi_type_to_llvm(
                                        ctx.context,
                                        &default_result.ty,
                                    );
                                    let ptr = ctx.builder().build_alloca(llvm_ty, name).unwrap();
                                    ctx.builder().build_store(ptr, default_result.value).unwrap();
                                    ctx.define_variable(
                                        name.clone(),
                                        (ptr, default_result.ty.clone()),
                                    );
                                }
                            }
                            ObjectPatternField::Rest(_) => {}
                        }
                    }
                }
                _ => {
                    // 回退方案：Dynamic 或其他类型，将对象视为平坦的 i64 槽位数组
                    // 每个字段按顺序占 8 字节，类型为 Dynamic
                    let i32_ty = ctx.context.i32_type();
                    let i64_ty = ctx.context.i64_type();
                    let obj_ptr_raw = match val.value {
                        BasicValueEnum::PointerValue(p) => p,
                        _ => return Err("Object pattern requires pointer".to_string()),
                    };
                    let mut field_idx = 0u64;
                    for field in fields {
                        match field {
                            ObjectPatternField::Property {
                                key,
                                pattern: inner,
                            } => {
                                let offset = i32_ty.const_int(field_idx * 8, false);
                                let field_ptr = unsafe {
                                    ctx.builder().build_gep(
                                        ctx.context.i8_type(),
                                        obj_ptr_raw,
                                        &[offset],
                                        &format!("{}_ptr", key),
                                    ).unwrap()
                                };
                                let typed_ptr = ctx
                                    .builder()
                                    .build_bit_cast(
                                        field_ptr,
                                        ctx.context.ptr_type(Default::default()),
                                        &format!("{}_typed_ptr", key),
                                    ).unwrap()
                                    .into_pointer_value();
                                let field_val = ctx.builder().build_load(i64_ty, typed_ptr, key).unwrap();
                                let field_result = super::expr::ExprResult {
                                    value: field_val,
                                    ty: Type::Dynamic,
                                };
                                bind_pattern_in_codegen(ctx, inner, &field_result)?;
                                field_idx += 1;
                            }
                            ObjectPatternField::Shorthand(name)
                            | ObjectPatternField::ShorthandDefault(name, _) => {
                                let offset = i32_ty.const_int(field_idx * 8, false);
                                let field_ptr = unsafe {
                                    ctx.builder().build_gep(
                                        ctx.context.i8_type(),
                                        obj_ptr_raw,
                                        &[offset],
                                        &format!("{}_ptr", name),
                                    ).unwrap()
                                };
                                let typed_ptr = ctx
                                    .builder()
                                    .build_bit_cast(
                                        field_ptr,
                                        ctx.context.ptr_type(Default::default()),
                                        &format!("{}_typed_ptr", name),
                                    ).unwrap()
                                    .into_pointer_value();
                                let field_val = ctx.builder().build_load(i64_ty, typed_ptr, name).unwrap();
                                let llvm_ty =
                                    super::types::ruyi_type_to_llvm(ctx.context, &Type::Dynamic);
                                let ptr = ctx.builder().build_alloca(llvm_ty, name).unwrap();
                                ctx.builder().build_store(ptr, field_val).unwrap();
                                ctx.define_variable(name.clone(), (ptr, Type::Dynamic));
                                field_idx += 1;
                            }
                            ObjectPatternField::Rest(_) => {}
                        }
                    }
                }
            }
        }
        P::Array(elements) => {
            super::patterns::bind_array_pattern(ctx, elements, val)?;
        }
        P::As(inner, alias) => {
            bind_pattern_in_codegen(ctx, inner, val)?;
            let llvm_ty = super::types::ruyi_type_to_llvm(ctx.context, &val.ty);
            let ptr = ctx.builder().build_alloca(llvm_ty, alias).unwrap();
            ctx.builder().build_store(ptr, val.value).unwrap();
            ctx.define_variable(alias.clone(), (ptr, val.ty.clone()));
        }
        P::Or(patterns) => {
            if let Some(first) = patterns.first() {
                bind_pattern_in_codegen(ctx, first, val)?;
            }
        }
        P::Rest(name) => {
            let llvm_ty = super::types::ruyi_type_to_llvm(ctx.context, &val.ty);
            let ptr = ctx.builder().build_alloca(llvm_ty, name).unwrap();
            ctx.builder().build_store(ptr, val.value).unwrap();
            ctx.define_variable(name.clone(), (ptr, val.ty.clone()));
        }
        P::Wildcard | P::Literal(_) => {}
    }
    Ok(())
}

/// EX-H1: Map catch clause type annotation to builtin type ID for LLVM
/// landing-pad selector comparison.
///
/// Returns `0` (catch-all) when the annotation is not a recognized
/// built-in exception type.
///
/// @author Ruyi Team
/// @date 2026-07-26
fn catch_type_to_type_id(ty: &crate::parser::ast::TypeAnnotation) -> ruyi_exception::TryTypeId {
    use crate::parser::ast::TypeAnnotation;
    let name = match ty {
        TypeAnnotation::Identifier(n) => n.as_str(),
        TypeAnnotation::Builtin(n) => n.as_str(),
        _ => return 0, // unrecognized → catch-all
    };
    match name {
        "Error" => ruyi_runtime::exception::builtin_type_ids::ERROR as u32,
        "TypeError" => ruyi_runtime::exception::builtin_type_ids::TYPE_ERROR as u32,
        "RangeError" => ruyi_runtime::exception::builtin_type_ids::RANGE_ERROR as u32,
        "RuntimeError" => ruyi_runtime::exception::builtin_type_ids::RUNTIME_ERROR as u32,
        "LogicError" => ruyi_runtime::exception::builtin_type_ids::LOGIC_ERROR as u32,
        "AssertionError" => ruyi_runtime::exception::builtin_type_ids::ASSERTION_ERROR as u32,
        "ArgumentError" => ruyi_runtime::exception::builtin_type_ids::ARGUMENT_ERROR as u32,
        "NullError" => ruyi_runtime::exception::builtin_type_ids::NULL_ERROR as u32,
        "ArithmeticError" => ruyi_runtime::exception::builtin_type_ids::ARITHMETIC_ERROR as u32,
        "IteratorError" => ruyi_runtime::exception::builtin_type_ids::ITERATOR_ERROR as u32,
        "ParseError" => ruyi_runtime::exception::builtin_type_ids::PARSE_ERROR as u32,
        "NullAssertionError" => {
            ruyi_runtime::exception::builtin_type_ids::NULL_ASSERTION_ERROR as u32
        }
        "IOError" => ruyi_runtime::exception::builtin_type_ids::IO_ERROR as u32,
        _ => 0, // unknown type → catch-all
    }
}
