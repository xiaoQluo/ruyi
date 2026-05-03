/**
 * Statement code generation for Ruyi.
 *
 * Lowers Ruyi AST statements to LLVM IR instructions.
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */

use inkwell::values::BasicValueEnum;

use crate::parser::ast::{ForInit, MatchArm, Pattern, Statement};
use super::expr::{compile_expr, ExprResult};
use super::generator::CodegenContext;
use super::types::ruyi_type_to_llvm;
use crate::typechecker::types::Type;

/// Compile a statement.
pub fn compile_stmt<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    stmt: &Statement,
) -> Result<(), String> {
    match stmt {
        Statement::Expression(expr) => {
            compile_expr(ctx, expr)?;
            Ok(())
        }
        Statement::Block(stmts) => compile_block(ctx, stmts),
        Statement::If { condition, then_branch, else_branch } => {
            compile_if(ctx, condition, then_branch, else_branch.as_deref())
        }
        Statement::While { condition, body } => compile_while(ctx, condition, body),
        Statement::Return(expr) => compile_return(ctx, expr.as_deref()),
        Statement::Break(label) => compile_break(ctx, label.as_ref()),
        Statement::Continue(label) => compile_continue(ctx, label.as_ref()),
        Statement::Declaration(decl) => super::decl::compile_declaration(ctx, decl),
        Statement::Empty => Ok(()),
        Statement::For { init, condition, update, body } => {
            compile_for(ctx, init, condition, update, body)
        }
        Statement::ForIn { .. } => Err("for-in loops are not yet supported in codegen".to_string()),
        Statement::ForOf { .. } => Err("for-of loops are not yet supported in codegen".to_string()),
        Statement::Match { value, arms } => compile_match_stmt(ctx, value, arms),
        Statement::Try { body, catch, finally } => compile_try(ctx, body, catch, finally),
        Statement::Throw(expr) => compile_throw(ctx, expr),
        Statement::IfLet { .. } => Err("if-let is not yet supported in codegen".to_string()),
        Statement::WhileLet { .. } => Err("while-let is not yet supported in codegen".to_string()),
    }
}

/// Compile a block of statements.
pub fn compile_block<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    stmts: &[Statement],
) -> Result<(), String> {
    for stmt in stmts {
        compile_stmt(ctx, stmt)?;
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

    // Then branch
    ctx.builder.position_at_end(then_bb);
    compile_stmt(ctx, then_branch)?;
    if ctx.builder.get_insert_block().unwrap().get_terminator().is_none() {
        ctx.builder.build_unconditional_branch(merge_bb);
    }

    // Else branch
    ctx.builder.position_at_end(else_bb);
    if let Some(else_stmt) = else_branch {
        compile_stmt(ctx, else_stmt)?;
    }
    if ctx.builder.get_insert_block().unwrap().get_terminator().is_none() {
        ctx.builder.build_unconditional_branch(merge_bb);
    }

    // Merge block
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

    // Push loop context for break/continue
    ctx.loop_stack.push((end_bb, cond_bb));

    ctx.builder.build_unconditional_branch(cond_bb);

    // Condition block
    ctx.builder.position_at_end(cond_bb);
    let cond_result = compile_expr(ctx, condition)?;
    let cond_val = match cond_result.value {
        BasicValueEnum::IntValue(v) => v,
        _ => return Err("Condition must be boolean".to_string()),
    };
    ctx.builder.build_conditional_branch(cond_val, body_bb, end_bb);

    // Body block
    ctx.builder.position_at_end(body_bb);
    compile_stmt(ctx, body)?;
    if ctx.builder.get_insert_block().unwrap().get_terminator().is_none() {
        ctx.builder.build_unconditional_branch(cond_bb);
    }

    // End block
    ctx.builder.position_at_end(end_bb);

    ctx.loop_stack.pop();

    Ok(())
}

fn compile_for<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    init: &Option<ForInit>,
    condition: &Option<Box<crate::parser::ast::Expr>>,
    update: &Option<Box<crate::parser::ast::Expr>>,
    body: &Statement,
) -> Result<(), String> {
    let func = ctx.current_function.ok_or("No current function")?;

    let init_bb = ctx.context.append_basic_block(func, "for_init");
    let cond_bb = ctx.context.append_basic_block(func, "for_cond");
    let body_bb = ctx.context.append_basic_block(func, "for_body");
    let update_bb = ctx.context.append_basic_block(func, "for_update");
    let end_bb = ctx.context.append_basic_block(func, "for_end");

    ctx.loop_stack.push((end_bb, update_bb));

    ctx.builder.build_unconditional_branch(init_bb);

    ctx.builder.position_at_end(init_bb);
    if let Some(init_expr) = init {
        match init_expr {
            ForInit::VarDecl(decl) => {
                super::decl::compile_declaration(ctx, decl)?;
            }
            ForInit::Expr(expr) => {
                compile_expr(ctx, expr)?;
            }
        }
    }
    ctx.builder.build_unconditional_branch(cond_bb);

    ctx.builder.position_at_end(cond_bb);
    if let Some(cond_expr) = condition {
        let cond_result = compile_expr(ctx, cond_expr)?;
        let cond_val = match cond_result.value {
            BasicValueEnum::IntValue(v) => v,
            _ => return Err("Condition must be boolean".to_string()),
        };
        ctx.builder.build_conditional_branch(cond_val, body_bb, end_bb);
    } else {
        ctx.builder.build_unconditional_branch(body_bb);
    }

    ctx.builder.position_at_end(body_bb);
    compile_stmt(ctx, body)?;
    if ctx.builder.get_insert_block().unwrap().get_terminator().is_none() {
        ctx.builder.build_unconditional_branch(update_bb);
    }

    ctx.builder.position_at_end(update_bb);
    if let Some(update_expr) = update {
        compile_expr(ctx, update_expr)?;
    }
    ctx.builder.build_unconditional_branch(cond_bb);

    ctx.builder.position_at_end(end_bb);

    ctx.loop_stack.pop();

    Ok(())
}

fn compile_return<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    expr: Option<&crate::parser::ast::Expr>,
) -> Result<(), String> {
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

fn compile_break<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    _label: Option<&String>,
) -> Result<(), String> {
    let end_bb = ctx.loop_stack.last()
        .ok_or("break outside of loop")?
        .0;
    ctx.builder.build_unconditional_branch(end_bb);
    Ok(())
}

fn compile_continue<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    _label: Option<&String>,
) -> Result<(), String> {
    let cond_bb = ctx.loop_stack.last()
        .ok_or("continue outside of loop")?
        .1;
    ctx.builder.build_unconditional_branch(cond_bb);
    Ok(())
}

fn compile_match_stmt<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    value: &crate::parser::ast::Expr,
    arms: &[MatchArm],
) -> Result<(), String> {
    if arms.is_empty() {
        return Ok(());
    }

    let scrutinee = compile_expr(ctx, value)?;
    let func = ctx.current_function.ok_or("No current function")?;

    let merge_bb = ctx.context.append_basic_block(func, "match_merge");

    let mut check_bbs = Vec::new();
    let mut body_bbs = Vec::new();
    for i in 0..arms.len() {
        check_bbs.push(ctx.context.append_basic_block(func, &format!("match_check_{}", i)));
        body_bbs.push(ctx.context.append_basic_block(func, &format!("match_body_{}", i)));
    }

    ctx.builder.build_unconditional_branch(check_bbs[0]);

    for (i, arm) in arms.iter().enumerate() {
        ctx.builder.position_at_end(check_bbs[i]);
        let saved_vars = ctx.variables.clone();
        let cond_val = ctx.context.bool_type().const_int(1, false);

        let next_bb = if i + 1 < arms.len() {
            check_bbs[i + 1]
        } else {
            merge_bb
        };
        ctx.builder.build_conditional_branch(cond_val, body_bbs[i], next_bb);

        ctx.builder.position_at_end(body_bbs[i]);
        for stmt in &arm.body {
            compile_stmt(ctx, stmt)?;
        }
        ctx.variables = saved_vars;
        if ctx.builder.get_insert_block().unwrap().get_terminator().is_none() {
            ctx.builder.build_unconditional_branch(merge_bb);
        }
    }

    ctx.builder.position_at_end(merge_bb);
    Ok(())
}

fn compile_try<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    body: &[Statement],
    catch: &Option<crate::parser::ast::CatchClause>,
    finally: &Option<Vec<Statement>>,
) -> Result<(), String> {
    let func = ctx.current_function.ok_or("No current function")?;
    let i8_ptr = ctx.context.i8_type().ptr_type(Default::default());
    let i32_ty = ctx.context.i32_type();
    let lpad_ty = ctx.context.struct_type(&[i8_ptr.into(), i32_ty.into()], false);

    let try_bb = ctx.context.append_basic_block(func, "try_body");
    let catch_lpad_bb = ctx.context.append_basic_block(func, "catch_lpad");
    let catch_body_bb = if catch.is_some() {
        Some(ctx.context.append_basic_block(func, "catch_body"))
    } else {
        None
    };
    let finally_bb = if finally.is_some() {
        Some(ctx.context.append_basic_block(func, "finally"))
    } else {
        None
    };
    let resume_bb = ctx.context.append_basic_block(func, "resume");
    let merge_bb = ctx.context.append_basic_block(func, "try_merge");

    let exc_ptr_alloca = ctx.builder.build_alloca(i8_ptr, "exc_ptr");
    let lpad_alloca = ctx.builder.build_alloca(lpad_ty, "lpad_val");
    ctx.builder.build_store(exc_ptr_alloca, i8_ptr.const_null());

    ctx.builder.build_unconditional_branch(try_bb);

    ctx.builder.position_at_end(try_bb);
    ctx.exception_stack.push(catch_lpad_bb);
    for stmt in body {
        compile_stmt(ctx, stmt)?;
    }
    ctx.exception_stack.pop();
    if ctx.builder.get_insert_block().unwrap().get_terminator().is_none() {
        if let Some(fb) = finally_bb {
            ctx.builder.build_unconditional_branch(fb);
        } else {
            ctx.builder.build_unconditional_branch(merge_bb);
        }
    }

    ctx.builder.position_at_end(catch_lpad_bb);
    let personality_ty = i32_ty.fn_type(&[], false);
    let personality = ctx.module.get_function("__gxx_personality_v0")
        .unwrap_or_else(|| ctx.module.add_function("__gxx_personality_v0", personality_ty, None));

    let null_clause = BasicValueEnum::PointerValue(i8_ptr.const_null());
    let lpad_val = ctx.builder.build_landing_pad(
        lpad_ty,
        personality,
        &[null_clause],
        finally.is_some(),
        "lpad",
    );
    ctx.builder.build_store(lpad_alloca, lpad_val);

    let exc_ptr = ctx.builder
        .build_extract_value(lpad_val.into_struct_value(), 0, "exc.ptr")
        .unwrap()
        .into_pointer_value();
    ctx.builder.build_store(exc_ptr_alloca, exc_ptr);

    if catch.is_some() {
        ctx.builder.build_unconditional_branch(catch_body_bb.unwrap());
    } else if let Some(fb) = finally_bb {
        ctx.builder.build_unconditional_branch(fb);
    } else {
        ctx.builder.build_unconditional_branch(resume_bb);
    }

    if let Some(ref c) = catch {
        let cbb = catch_body_bb.unwrap();
        ctx.builder.position_at_end(cbb);

        let exc_loaded = ctx.builder.build_load(exc_ptr_alloca, "exc_loaded");
        let begin_catch_fn = ctx.module.get_function("ruyi_begin_catch")
            .ok_or("ruyi_begin_catch not declared")?;
        let exc_obj = ctx.builder.build_call(
            begin_catch_fn,
            &[exc_loaded.into_pointer_value().into()],
            "exc_obj",
        ).try_as_basic_value().left().unwrap().into_pointer_value();

        if let Some(ref pattern) = c.pattern {
            if let crate::parser::ast::Pattern::Identifier(name) = pattern {
                let var_ptr = ctx.builder.build_alloca(i8_ptr, name);
                ctx.builder.build_store(var_ptr, exc_obj);
                ctx.variables.insert(name.clone(), (var_ptr, Type::Named("Error".to_string())));
            }
        }

        for stmt in &c.body {
            compile_stmt(ctx, stmt)?;
        }

        let end_catch_fn = ctx.module.get_function("ruyi_end_catch")
            .ok_or("ruyi_end_catch not declared")?;
        ctx.builder.build_call(end_catch_fn, &[], "end_catch");

        ctx.builder.build_store(exc_ptr_alloca, i8_ptr.const_null());

        if ctx.builder.get_insert_block().unwrap().get_terminator().is_none() {
            if let Some(fb) = finally_bb {
                ctx.builder.build_unconditional_branch(fb);
            } else {
                ctx.builder.build_unconditional_branch(merge_bb);
            }
        }
    }

    if let Some(ref finally_stmts) = finally {
        let fb = finally_bb.unwrap();
        ctx.builder.position_at_end(fb);

        for stmt in finally_stmts {
            compile_stmt(ctx, stmt)?;
        }

        let exc_loaded = ctx.builder.build_load(exc_ptr_alloca, "exc_loaded");
        let is_null = ctx.builder.build_int_compare(
            inkwell::IntPredicate::EQ,
            exc_loaded.into_pointer_value(),
            i8_ptr.const_null(),
            "exc_is_null",
        );
        ctx.builder.build_conditional_branch(is_null, merge_bb, resume_bb);
    }

    ctx.builder.position_at_end(resume_bb);
    let lpad_loaded = ctx.builder.build_load(lpad_alloca, "lpad_loaded");
    let lpad_gen2 = ruyi_runtime::LandingPadGenerator::new(ctx.context, ctx.module, &ctx.builder);
    lpad_gen2.build_resume(lpad_loaded);

    ctx.builder.position_at_end(merge_bb);
    Ok(())
}

fn compile_throw<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    expr: &crate::parser::ast::Expr,
) -> Result<(), String> {
    let exc_result = compile_expr(ctx, expr)?;
    let exc_ptr = match exc_result.value {
        BasicValueEnum::PointerValue(p) => p,
        _ => return Err("throw expression must evaluate to a pointer".to_string()),
    };

    let throw_fn = ctx.module.get_function("ruyi_throw")
        .ok_or("ruyi_throw not declared")?;

    if let Some(catch_bb) = ctx.exception_stack.last().copied() {
        let func = ctx.current_function.ok_or("No current function")?;
        let unreachable_bb = ctx.context.append_basic_block(func, "throw_unreachable");
        ctx.builder.build_invoke(throw_fn, &[exc_ptr.into()], unreachable_bb, catch_bb, "throw");
        ctx.builder.position_at_end(unreachable_bb);
        ctx.builder.build_unreachable();
    } else {
        ctx.builder.build_call(throw_fn, &[exc_ptr.into()], "throw");
        ctx.builder.build_unreachable();
    }
    Ok(())
}
