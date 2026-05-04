/**
 * Pattern matching code generation for Ruyi.
 *
 * Generates LLVM IR for match expressions with literal patterns,
 * wildcards, identifier bindings, and nullable patterns.
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */
use inkwell::values::BasicValueEnum;
use inkwell::IntPredicate;

use crate::codegen::expr::{compile_expr, ExprResult};
use crate::codegen::generator::CodegenContext;
use crate::codegen::stmt::compile_block;
use crate::parser::ast::{Expr, MatchArm, Pattern};
use crate::typechecker::types::Type;

/// Compiles a match statement to LLVM IR.
///
/// Generates a dispatch structure (switch, br, or icmp chain) based on
/// the scrutinee type, then compiles each arm body in its own basic block.
/// All arms converge at a merge block.
pub fn compile_match_stmt<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    value: &Expr,
    arms: &[MatchArm],
) -> Result<(), String> {
    let scrutinee = compile_expr(ctx, value)?;
    let func = ctx.current_function.ok_or("No current function")?;
    let merge_bb = ctx.context.append_basic_block(func, "match_merge");

    // Build dispatch and arm blocks based on scrutinee type
    match scrutinee.ty {
        Type::Int | Type::BigInt => compile_int_match(ctx, &scrutinee, arms, merge_bb),
        Type::Bool => compile_bool_match(ctx, &scrutinee, arms, merge_bb),
        Type::String => compile_string_match(ctx, &scrutinee, arms, merge_bb),
        Type::Nullable(_) | Type::Null => {
            compile_nullable_match(ctx, &scrutinee, arms, merge_bb)
        }
        _ => compile_generic_match(ctx, &scrutinee, arms, merge_bb),
    }
}

/// Binds a pattern variable by allocating storage and storing the value.
///
/// For `Identifier(name)`, creates a stack slot and stores the scrutinee.
/// For `Wildcard`, no-op. For other patterns, currently no-op.
fn bind_pattern<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    pattern: &Pattern,
    scrutinee: &ExprResult<'ctx>,
) -> Result<(), String> {
    match pattern {
        Pattern::Identifier(name) => {
            let llvm_ty = super::types::ruyi_type_to_llvm(ctx.context, &scrutinee.ty);
            let ptr = ctx.builder.build_alloca(llvm_ty, name);
            ctx.builder.build_store(ptr, scrutinee.value);
            ctx.variables.insert(name.clone(), (ptr, scrutinee.ty.clone()));
            Ok(())
        }
        Pattern::Wildcard => Ok(()),
        Pattern::As(inner, alias) => {
            bind_pattern(ctx, inner, scrutinee)?;
            let llvm_ty = super::types::ruyi_type_to_llvm(ctx.context, &scrutinee.ty);
            let ptr = ctx.builder.build_alloca(llvm_ty, alias);
            ctx.builder.build_store(ptr, scrutinee.value);
            ctx.variables.insert(alias.clone(), (ptr, scrutinee.ty.clone()));
            Ok(())
        }
        Pattern::Literal(_) => Ok(()),
        _ => {
            // Object/Array/Rest/Or patterns are not yet supported in match codegen
            Ok(())
        }
    }
}

/// Compile arm bodies after dispatch has been set up.
///
/// Positions the builder at each arm block, binds the pattern,
/// compiles the arm statements, and adds a branch to merge if
/// the arm doesn't already terminate.
fn compile_arm_bodies<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    arms: &[MatchArm],
    arm_bbs: &[inkwell::basic_block::BasicBlock<'ctx>],
    merge_bb: inkwell::basic_block::BasicBlock<'ctx>,
    scrutinee: &ExprResult<'ctx>,
) -> Result<(), String> {
    for (i, arm) in arms.iter().enumerate() {
        ctx.builder.position_at_end(arm_bbs[i]);
        bind_pattern(ctx, &arm.pattern, scrutinee)?;
        compile_block(ctx, &arm.body)?;
        if let Some(bb) = ctx.builder.get_insert_block() {
            if bb.get_terminator().is_none() {
                ctx.builder.build_unconditional_branch(merge_bb);
            }
        }
    }
    ctx.builder.position_at_end(merge_bb);
    Ok(())
}

/// Integer match using LLVM `switch`.
///
/// Literal int patterns become switch cases. The first wildcard or
/// identifier arm serves as the default. If there is no default, an
/// unreachable trap block is emitted.
fn compile_int_match<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    scrutinee: &ExprResult<'ctx>,
    arms: &[MatchArm],
    merge_bb: inkwell::basic_block::BasicBlock<'ctx>,
) -> Result<(), String> {
    let func = ctx.current_function.ok_or("No current function")?;

    let mut arm_bbs = Vec::with_capacity(arms.len());
    let mut cases: Vec<(inkwell::values::IntValue<'ctx>, inkwell::basic_block::BasicBlock<'ctx>)> =
        Vec::new();
    let mut default_bb: Option<inkwell::basic_block::BasicBlock<'ctx>> = None;

    for (i, arm) in arms.iter().enumerate() {
        let bb = ctx.context.append_basic_block(func, &format!("int_arm_{}", i));
        arm_bbs.push(bb);
        match &arm.pattern {
            Pattern::Literal(lit) => {
                if let Expr::IntLiteral(n) = lit.as_ref() {
                    cases.push((
                        ctx.context.i64_type().const_int(*n as u64, true),
                        bb,
                    ));
                }
            }
            Pattern::Wildcard | Pattern::Identifier(_) => {
                if default_bb.is_none() {
                    default_bb = Some(bb);
                }
            }
            _ => {}
        }
    }

    let default_block = default_bb.unwrap_or_else(|| {
        let trap = ctx.context.append_basic_block(func, "match_trap");
        ctx.builder.position_at_end(trap);
        ctx.builder.build_unreachable();
        trap
    });

    let scrutinee_int = match scrutinee.value {
        BasicValueEnum::IntValue(v) => v,
        _ => return Err("Int match requires integer scrutinee".to_string()),
    };

    ctx.builder.build_switch(scrutinee_int, default_block, &cases);

    compile_arm_bodies(ctx, arms, &arm_bbs, merge_bb, scrutinee)
}

/// Boolean match using conditional branch.
///
/// `true` and `false` literal arms get direct branches. Wildcard or
/// identifier arms act as the default for the missing branch.
fn compile_bool_match<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    scrutinee: &ExprResult<'ctx>,
    arms: &[MatchArm],
    merge_bb: inkwell::basic_block::BasicBlock<'ctx>,
) -> Result<(), String> {
    let func = ctx.current_function.ok_or("No current function")?;

    let mut arm_bbs = Vec::with_capacity(arms.len());
    let mut true_bb: Option<inkwell::basic_block::BasicBlock<'ctx>> = None;
    let mut false_bb: Option<inkwell::basic_block::BasicBlock<'ctx>> = None;
    let mut default_bb: Option<inkwell::basic_block::BasicBlock<'ctx>> = None;

    for (i, arm) in arms.iter().enumerate() {
        let bb = ctx.context.append_basic_block(func, &format!("bool_arm_{}", i));
        arm_bbs.push(bb);
        match &arm.pattern {
            Pattern::Literal(lit) => {
                if let Expr::BooleanLiteral(b) = lit.as_ref() {
                    if *b {
                        true_bb = Some(bb);
                    } else {
                        false_bb = Some(bb);
                    }
                }
            }
            Pattern::Wildcard | Pattern::Identifier(_) => {
                if default_bb.is_none() {
                    default_bb = Some(bb);
                }
            }
            _ => {}
        }
    }

    let true_target = true_bb.or(default_bb).unwrap_or(merge_bb);
    let false_target = false_bb.or(default_bb).unwrap_or(merge_bb);

    let cond_val = match scrutinee.value {
        BasicValueEnum::IntValue(v) => v,
        _ => return Err("Bool match requires boolean scrutinee".to_string()),
    };

    ctx.builder
        .build_conditional_branch(cond_val, true_target, false_target);

    compile_arm_bodies(ctx, arms, &arm_bbs, merge_bb, scrutinee)
}

/// String match using pointer comparison (minimum viable).
///
/// Compares string pointers with `strcmp` if available; falls back to
/// pointer equality for string literals. A chain of conditional
/// branches is generated.
fn compile_string_match<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    scrutinee: &ExprResult<'ctx>,
    arms: &[MatchArm],
    merge_bb: inkwell::basic_block::BasicBlock<'ctx>,
) -> Result<(), String> {
    let func = ctx.current_function.ok_or("No current function")?;

    let mut arm_bbs = Vec::with_capacity(arms.len());
    let mut literal_arms: Vec<(String, inkwell::basic_block::BasicBlock<'ctx>)> = Vec::new();
    let mut default_bb: Option<inkwell::basic_block::BasicBlock<'ctx>> = None;

    for (i, arm) in arms.iter().enumerate() {
        let bb = ctx.context.append_basic_block(func, &format!("str_arm_{}", i));
        arm_bbs.push(bb);
        match &arm.pattern {
            Pattern::Literal(lit) => {
                if let Expr::StringLiteral(s) = lit.as_ref() {
                    literal_arms.push((s.clone(), bb));
                }
            }
            Pattern::Wildcard | Pattern::Identifier(_) => {
                if default_bb.is_none() {
                    default_bb = Some(bb);
                }
            }
            _ => {}
        }
    }

    let scrutinee_ptr = match scrutinee.value {
        BasicValueEnum::PointerValue(v) => v,
        _ => return Err("String match requires string scrutinee".to_string()),
    };

    // Build a chain of strcmp calls for each literal arm
    let i32_ty = ctx.context.i32_type();
    let strcmp_fn = ctx
        .module
        .get_function("strcmp")
        .unwrap_or_else(|| {
            let i8_ptr = ctx.context.i8_type().ptr_type(Default::default());
            let fn_type = i32_ty.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
            ctx.module.add_function("strcmp", fn_type, None)
        });

    let mut check_bbs: Vec<inkwell::basic_block::BasicBlock<'ctx>> = Vec::new();
    for (i, (s, target_bb)) in literal_arms.iter().enumerate() {
        let check_bb = if i == 0 {
            ctx.builder.get_insert_block().unwrap()
        } else {
            ctx.context.append_basic_block(func, &format!("str_check_{}", i))
        };
        if i > 0 {
            check_bbs.push(check_bb);
        }

        ctx.builder.position_at_end(check_bb);
        let lit_global = ctx.builder.build_global_string_ptr(s, "match_str_lit");
        let cmp_result = ctx
            .builder
            .build_call(
                strcmp_fn,
                &[scrutinee_ptr.into(), lit_global.as_pointer_value().into()],
                "str_cmp",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();
        let is_equal = ctx.builder.build_int_compare(
            IntPredicate::EQ,
            cmp_result,
            i32_ty.const_int(0, false),
            "str_eq",
        );

        let next_bb = if i + 1 < literal_arms.len() {
            ctx.context.append_basic_block(func, &format!("str_check_{}", i + 1))
        } else {
            default_bb.unwrap_or(merge_bb)
        };

        ctx.builder.build_conditional_branch(is_equal, *target_bb, next_bb);
    }

    compile_arm_bodies(ctx, arms, &arm_bbs, merge_bb, scrutinee)
}

/// Nullable match: null check + branch.
///
/// A `null` literal arm matches the null case; identifier/wildcard
/// arms match the non-null value. Pointer types are checked with
/// ptr-to-int; integer types use 0 as a sentinel.
fn compile_nullable_match<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    scrutinee: &ExprResult<'ctx>,
    arms: &[MatchArm],
    merge_bb: inkwell::basic_block::BasicBlock<'ctx>,
) -> Result<(), String> {
    let func = ctx.current_function.ok_or("No current function")?;

    let mut arm_bbs = Vec::with_capacity(arms.len());
    let mut null_bb: Option<inkwell::basic_block::BasicBlock<'ctx>> = None;
    let mut value_bb: Option<inkwell::basic_block::BasicBlock<'ctx>> = None;
    let mut default_bb: Option<inkwell::basic_block::BasicBlock<'ctx>> = None;

    for (i, arm) in arms.iter().enumerate() {
        let bb = ctx.context.append_basic_block(func, &format!("null_arm_{}", i));
        arm_bbs.push(bb);
        match &arm.pattern {
            Pattern::Literal(lit) => {
                if matches!(lit.as_ref(), Expr::NullLiteral) {
                    null_bb = Some(bb);
                }
            }
            Pattern::Identifier(_) => {
                if value_bb.is_none() {
                    value_bb = Some(bb);
                }
                if default_bb.is_none() {
                    default_bb = Some(bb);
                }
            }
            Pattern::Wildcard => {
                if default_bb.is_none() {
                    default_bb = Some(bb);
                }
            }
            _ => {}
        }
    }

    // Generate null check
    let is_null = match scrutinee.value {
        BasicValueEnum::PointerValue(p) => {
            let i64_ty = ctx.context.i64_type();
            let ptr_int = ctx.builder.build_ptr_to_int(p, i64_ty, "ptr_int");
            ctx.builder.build_int_compare(
                IntPredicate::EQ,
                ptr_int,
                i64_ty.const_int(0, false),
                "is_null",
            )
        }
        BasicValueEnum::IntValue(v) => {
            // For nullable primitives (erased to inner type), use 0 sentinel
            ctx.builder.build_int_compare(
                IntPredicate::EQ,
                v,
                ctx.context.i64_type().const_int(0, false),
                "is_null_int",
            )
        }
        _ => {
            return Err(
                "Nullable match requires pointer or integer scrutinee".to_string(),
            )
        }
    };

    let null_target = null_bb.or(default_bb).unwrap_or(merge_bb);
    let value_target = value_bb.or(default_bb).unwrap_or(merge_bb);

    ctx.builder
        .build_conditional_branch(is_null, null_target, value_target);

    compile_arm_bodies(ctx, arms, &arm_bbs, merge_bb, scrutinee)
}

/// Generic fallback match for unsupported types.
///
/// Evaluates each arm in order with no optimisation. The first
/// wildcard or identifier arm always matches.
fn compile_generic_match<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    scrutinee: &ExprResult<'ctx>,
    arms: &[MatchArm],
    merge_bb: inkwell::basic_block::BasicBlock<'ctx>,
) -> Result<(), String> {
    let func = ctx.current_function.ok_or("No current function")?;

    let mut arm_bbs = Vec::with_capacity(arms.len());
    for (i, _) in arms.iter().enumerate() {
        let bb = ctx.context.append_basic_block(func, &format!("match_arm_{}", i));
        arm_bbs.push(bb);
    }

    // Just jump to the first arm that is a wildcard or identifier
    let mut target_bb = merge_bb;
    for (i, arm) in arms.iter().enumerate().rev() {
        if matches!(arm.pattern, Pattern::Wildcard | Pattern::Identifier(_)) {
            target_bb = arm_bbs[i];
        }
    }

    ctx.builder.build_unconditional_branch(target_bb);

    compile_arm_bodies(ctx, arms, &arm_bbs, merge_bb, scrutinee)
}
