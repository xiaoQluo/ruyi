/**
 * Statement code generation for Ruyi.
 *
 * Lowers Ruyi AST statements to LLVM IR instructions.
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */

use inkwell::values::BasicValueEnum;

use crate::parser::ast::Statement;
use super::expr::compile_expr;
use super::generator::CodegenContext;

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
        Statement::Declaration(decl) => super::decl::compile_declaration(ctx, decl),
        Statement::Empty => Ok(()),
        _ => Err(format!("Unsupported statement: {:?}", stmt)),
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
