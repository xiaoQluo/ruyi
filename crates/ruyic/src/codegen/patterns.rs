use inkwell::types::BasicType;
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
        Type::Nullable(_) | Type::Null => compile_nullable_match(ctx, &scrutinee, arms, merge_bb),
        Type::Array(_) => compile_array_match(ctx, &scrutinee, arms, merge_bb),
        _ => compile_generic_match(ctx, &scrutinee, arms, merge_bb),
    }
}

/// Binds a pattern variable by allocating storage and storing the value.
///
/// For `Identifier(name)`, creates a stack slot and stores the scrutinee.
/// For `Wildcard`, no-op. For other patterns, currently no-op.
pub fn bind_pattern<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    pattern: &Pattern,
    scrutinee: &ExprResult<'ctx>,
) -> Result<(), String> {
    match pattern {
        Pattern::Identifier(name) => {
            let llvm_ty = super::types::ruyi_type_to_llvm(ctx.context, &scrutinee.ty);
            let ptr = ctx.builder.build_alloca(llvm_ty, name);
            ctx.builder.build_store(ptr, scrutinee.value);
            ctx.variables
                .insert(name.clone(), (ptr, scrutinee.ty.clone()));
            Ok(())
        }
        Pattern::Wildcard => Ok(()),
        Pattern::As(inner, alias) => {
            bind_pattern(ctx, inner, scrutinee)?;
            let llvm_ty = super::types::ruyi_type_to_llvm(ctx.context, &scrutinee.ty);
            let ptr = ctx.builder.build_alloca(llvm_ty, alias);
            ctx.builder.build_store(ptr, scrutinee.value);
            ctx.variables
                .insert(alias.clone(), (ptr, scrutinee.ty.clone()));
            Ok(())
        }
        Pattern::Literal(_) => Ok(()),
        Pattern::Array(elements) => bind_array_pattern(ctx, elements, scrutinee),
        Pattern::Object(obj_fields) => bind_object_pattern(ctx, obj_fields, scrutinee),
        Pattern::Rest(_) | Pattern::Or(_) => Ok(()),
    }
}

pub fn bind_array_pattern<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    elements: &[crate::parser::ast::ArrayPatternElement],
    scrutinee: &ExprResult<'ctx>,
) -> Result<(), String> {
    let array_ptr = match scrutinee.value {
        BasicValueEnum::PointerValue(p) => p,
        _ => return Err("Array pattern requires pointer scrutinee".to_string()),
    };

    let i32_ty = ctx.context.i32_type();
    let i64_ty = ctx.context.i64_type();

    let mut idx = 0;
    for elem in elements {
        match elem {
            crate::parser::ast::ArrayPatternElement::Pattern(p) => {
                let elem_ty = match p {
                    Pattern::Identifier(_) => {
                        if let Type::Array(inner) = &scrutinee.ty {
                            *inner.clone()
                        } else {
                            Type::Dynamic
                        }
                    }
                    _ => Type::Dynamic,
                };
                let llvm_ty = super::types::ruyi_type_to_llvm(ctx.context, &elem_ty);

                let offset = i32_ty.const_int((16 + idx * 8) as u64, false);
                let elem_ptr = unsafe {
                    ctx.builder
                        .build_gep(array_ptr, &[offset], &format!("elem_ptr_{}", idx))
                };
                let typed_ptr = ctx.builder.build_bitcast(
                    elem_ptr,
                    llvm_ty.ptr_type(Default::default()),
                    &format!("elem_typed_ptr_{}", idx),
                );
                let loaded = ctx.builder.build_load(
                    typed_ptr.into_pointer_value(),
                    &format!("loaded_elem_{}", idx),
                );
                let name = match p {
                    Pattern::Identifier(n) => n.clone(),
                    _ => format!("_anon_{}", idx),
                };
                let ptr = ctx.builder.build_alloca(llvm_ty, &name);
                ctx.builder.build_store(ptr, loaded);
                ctx.variables.insert(name, (ptr, elem_ty));

                idx += 1;
            }
            crate::parser::ast::ArrayPatternElement::Rest(_) => {
                break;
            }
            crate::parser::ast::ArrayPatternElement::Elision => {
                idx += 1;
            }
        }
    }
    Ok(())
}

pub fn bind_object_pattern<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    fields: &[crate::parser::ast::ObjectPatternField],
    scrutinee: &ExprResult<'ctx>,
) -> Result<(), String> {
    let obj_ptr = match scrutinee.value {
        BasicValueEnum::PointerValue(p) => p,
        _ => return Err("Object pattern requires pointer scrutinee".to_string()),
    };

    let i32_ty = ctx.context.i32_type();

    if let Type::Named(class_name) = &scrutinee.ty {
        let class_fields: Vec<_> = ctx
            .class_fields
            .get(class_name)
            .ok_or_else(|| format!("Unknown class: {}", class_name))?
            .clone();
        let struct_type = ctx
            .class_struct_types
            .get(class_name)
            .ok_or_else(|| format!("No struct type for class: {}", class_name))?;

        let struct_ptr = ctx.builder.build_pointer_cast(
            obj_ptr,
            struct_type.ptr_type(Default::default()),
            "obj_struct_cast",
        );

        for f in fields {
            match f {
                crate::parser::ast::ObjectPatternField::Property {
                    key,
                    pattern: inner,
                } => {
                    let field_index = class_fields
                        .iter()
                        .position(|(name, _)| name == key)
                        .ok_or_else(|| format!("Unknown field: {}", key))?;
                    let (_, field_ty) = &class_fields[field_index];

                    let field_ptr = unsafe {
                        ctx.builder.build_gep(
                            struct_ptr,
                            &[
                                i32_ty.const_int(0, false),
                                i32_ty.const_int(field_index as u64, false),
                            ],
                            &format!("{}_ptr", key),
                        )
                    };
                    let field_val = ctx.builder.build_load(field_ptr, key);

                    bind_pattern(ctx, inner, &ExprResult::new(field_val, field_ty.clone()))?;
                }
                crate::parser::ast::ObjectPatternField::Shorthand(name) => {
                    let field_index = class_fields
                        .iter()
                        .position(|(n, _)| n == name)
                        .ok_or_else(|| format!("Unknown field: {}", name))?;
                    let (_, field_ty) = &class_fields[field_index];

                    let field_ptr = unsafe {
                        ctx.builder.build_gep(
                            struct_ptr,
                            &[
                                i32_ty.const_int(0, false),
                                i32_ty.const_int(field_index as u64, false),
                            ],
                            &format!("{}_ptr", name),
                        )
                    };
                    let field_val = ctx.builder.build_load(field_ptr, name);

                    let llvm_ty = super::types::ruyi_type_to_llvm(ctx.context, field_ty);
                    let ptr = ctx.builder.build_alloca(llvm_ty, name);
                    ctx.builder.build_store(ptr, field_val);
                    ctx.variables.insert(name.clone(), (ptr, field_ty.clone()));
                }
                _ => {}
            }
        }
    } else if let Type::Object(type_fields) = &scrutinee.ty {
        for f in fields {
            match f {
                crate::parser::ast::ObjectPatternField::Property {
                    key,
                    pattern: inner,
                } => {
                    let field_index = type_fields
                        .iter()
                        .position(|field| field.name == *key)
                        .ok_or_else(|| format!("Unknown field: {}", key))?;
                    let field_ty = &type_fields[field_index].ty;

                    let offset = i32_ty.const_int((field_index * 8) as u64, false);
                    let field_ptr = unsafe {
                        ctx.builder
                            .build_gep(obj_ptr, &[offset], &format!("{}_ptr", key))
                    };
                    let typed_ptr = ctx.builder.build_bitcast(
                        field_ptr,
                        super::types::ruyi_type_to_llvm(ctx.context, field_ty)
                            .ptr_type(Default::default()),
                        &format!("{}_typed_ptr", key),
                    );
                    let field_val = ctx.builder.build_load(typed_ptr.into_pointer_value(), key);

                    bind_pattern(ctx, inner, &ExprResult::new(field_val, field_ty.clone()))?;
                }
                crate::parser::ast::ObjectPatternField::Shorthand(name) => {
                    let field_index = type_fields
                        .iter()
                        .position(|field| field.name == *name)
                        .ok_or_else(|| format!("Unknown field: {}", name))?;
                    let field_ty = &type_fields[field_index].ty;

                    let offset = i32_ty.const_int((field_index * 8) as u64, false);
                    let field_ptr = unsafe {
                        ctx.builder
                            .build_gep(obj_ptr, &[offset], &format!("{}_ptr", name))
                    };
                    let typed_ptr = ctx.builder.build_bitcast(
                        field_ptr,
                        super::types::ruyi_type_to_llvm(ctx.context, field_ty)
                            .ptr_type(Default::default()),
                        &format!("{}_typed_ptr", name),
                    );
                    let field_val = ctx.builder.build_load(typed_ptr.into_pointer_value(), name);

                    let llvm_ty = super::types::ruyi_type_to_llvm(ctx.context, field_ty);
                    let ptr = ctx.builder.build_alloca(llvm_ty, name);
                    ctx.builder.build_store(ptr, field_val);
                    ctx.variables.insert(name.clone(), (ptr, field_ty.clone()));
                }
                _ => {}
            }
        }
    }

    Ok(())
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

/// Integer match using LLVM `switch`, or sequential chain when guards present.
///
/// Literal int patterns become switch cases. The first wildcard or
/// identifier arm serves as the default. If there is no default, an
/// unreachable trap block is emitted.
///
/// When any arm has a guard clause, falls back to sequential comparison
/// chain: each arm checks pattern match, evaluates guard, and branches
/// to body or falls through to the next arm.
fn compile_int_match<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    scrutinee: &ExprResult<'ctx>,
    arms: &[MatchArm],
    merge_bb: inkwell::basic_block::BasicBlock<'ctx>,
) -> Result<(), String> {
    let has_guards = arms.iter().any(|arm| arm.guard.is_some());
    if has_guards {
        compile_int_match_sequential(ctx, scrutinee, arms, merge_bb)
    } else {
        compile_int_match_switch(ctx, scrutinee, arms, merge_bb)
    }
}

/// Switch-based integer match (no guards).
fn compile_int_match_switch<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    scrutinee: &ExprResult<'ctx>,
    arms: &[MatchArm],
    merge_bb: inkwell::basic_block::BasicBlock<'ctx>,
) -> Result<(), String> {
    let func = ctx.current_function.ok_or("No current function")?;

    let mut arm_bbs = Vec::with_capacity(arms.len());
    let mut cases: Vec<(
        inkwell::values::IntValue<'ctx>,
        inkwell::basic_block::BasicBlock<'ctx>,
    )> = Vec::new();
    let mut default_bb: Option<inkwell::basic_block::BasicBlock<'ctx>> = None;

    for (i, arm) in arms.iter().enumerate() {
        let bb = ctx
            .context
            .append_basic_block(func, &format!("int_arm_{}", i));
        arm_bbs.push(bb);
        match &arm.pattern {
            Pattern::Literal(lit) => {
                if let Expr::IntLiteral(n) = lit.as_ref() {
                    cases.push((ctx.context.i64_type().const_int(*n as u64, true), bb));
                }
            }
            Pattern::Or(patterns) => {
                for p in patterns {
                    if let Pattern::Literal(lit) = p {
                        if let Expr::IntLiteral(n) = lit.as_ref() {
                            cases.push((ctx.context.i64_type().const_int(*n as u64, true), bb));
                        }
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

    ctx.builder
        .build_switch(scrutinee_int, default_block, &cases);

    compile_arm_bodies(ctx, arms, &arm_bbs, merge_bb, scrutinee)
}

/// Sequential comparison chain for integer match with guard clauses.
///
/// Each arm checks pattern match, then evaluates guard if present.
/// If guard fails, falls through to the next arm.
fn compile_int_match_sequential<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    scrutinee: &ExprResult<'ctx>,
    arms: &[MatchArm],
    merge_bb: inkwell::basic_block::BasicBlock<'ctx>,
) -> Result<(), String> {
    let func = ctx.current_function.ok_or("No current function")?;
    let i64_ty = ctx.context.i64_type();

    let scrutinee_int = match scrutinee.value {
        BasicValueEnum::IntValue(v) => v,
        _ => return Err("Int match requires integer scrutinee".to_string()),
    };

    let arm_bbs: Vec<_> = (0..arms.len())
        .map(|i| {
            ctx.context
                .append_basic_block(func, &format!("int_seq_arm_{}", i))
        })
        .collect();

    let entry_bb = ctx.context.append_basic_block(func, "int_seq_entry");
    ctx.builder.build_unconditional_branch(entry_bb);

    // Create check blocks for all arms except the last
    let check_bbs: Vec<_> = (0..arms.len().saturating_sub(1))
        .map(|i| {
            ctx.context
                .append_basic_block(func, &format!("int_seq_check_{}", i))
        })
        .collect();

    // Build check chain from last to first (so each check knows its successor)
    for i in (0..arms.len()).rev() {
        let arm = &arms[i];
        let next_bb = if i + 1 < arms.len() {
            check_bbs.get(i).copied().unwrap_or(arm_bbs[i + 1])
        } else {
            merge_bb
        };

        let current_bb = if i == 0 { entry_bb } else { check_bbs[i - 1] };

        ctx.builder.position_at_end(current_bb);

        match &arm.pattern {
            Pattern::Literal(lit) => {
                if let Expr::IntLiteral(n) = lit.as_ref() {
                    let is_match = ctx.builder.build_int_compare(
                        inkwell::IntPredicate::EQ,
                        scrutinee_int,
                        i64_ty.const_int(*n as u64, true),
                        &format!("int_seq_check_{}", i),
                    );
                    ctx.builder
                        .build_conditional_branch(is_match, arm_bbs[i], next_bb);
                } else {
                    ctx.builder.build_unconditional_branch(next_bb);
                }
            }
            Pattern::Or(patterns) => {
                let mut final_match = ctx.context.bool_type().const_int(0, false);
                for p in patterns {
                    if let Pattern::Literal(lit) = p {
                        if let Expr::IntLiteral(n) = lit.as_ref() {
                            let is_eq = ctx.builder.build_int_compare(
                                inkwell::IntPredicate::EQ,
                                scrutinee_int,
                                i64_ty.const_int(*n as u64, true),
                                &format!("int_seq_or_check_{}", i),
                            );
                            final_match = ctx.builder.build_or(
                                final_match,
                                is_eq,
                                &format!("int_seq_or_{}", i),
                            );
                        }
                    }
                }
                ctx.builder
                    .build_conditional_branch(final_match, arm_bbs[i], next_bb);
            }
            Pattern::Wildcard | Pattern::Identifier(_) => {
                ctx.builder.build_unconditional_branch(arm_bbs[i]);
            }
            _ => {
                ctx.builder.build_unconditional_branch(next_bb);
            }
        }
    }

    // Now compile arm bodies
    for (i, arm) in arms.iter().enumerate() {
        ctx.builder.position_at_end(arm_bbs[i]);

        bind_pattern(ctx, &arm.pattern, scrutinee)?;

        if let Some(guard) = &arm.guard {
            let guard_val = compile_expr(ctx, guard)?;
            let body_bb = ctx
                .context
                .append_basic_block(func, &format!("int_seq_body_{}", i));
            let next_bb = if i + 1 < arm_bbs.len() {
                arm_bbs[i + 1]
            } else {
                merge_bb
            };
            ctx.builder.build_conditional_branch(
                guard_val.value.into_int_value(),
                body_bb,
                next_bb,
            );
            ctx.builder.position_at_end(body_bb);
        }

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
        let bb = ctx
            .context
            .append_basic_block(func, &format!("bool_arm_{}", i));
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
        let bb = ctx
            .context
            .append_basic_block(func, &format!("str_arm_{}", i));
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
    let strcmp_fn = ctx.module.get_function("strcmp").unwrap_or_else(|| {
        let i8_ptr = ctx.context.i8_type().ptr_type(Default::default());
        let fn_type = i32_ty.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
        ctx.module.add_function("strcmp", fn_type, None)
    });

    let mut check_bbs: Vec<inkwell::basic_block::BasicBlock<'ctx>> = Vec::new();
    for (i, (s, target_bb)) in literal_arms.iter().enumerate() {
        let check_bb = if i == 0 {
            ctx.builder.get_insert_block().unwrap()
        } else {
            ctx.context
                .append_basic_block(func, &format!("str_check_{}", i))
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
            ctx.context
                .append_basic_block(func, &format!("str_check_{}", i + 1))
        } else {
            default_bb.unwrap_or(merge_bb)
        };

        ctx.builder
            .build_conditional_branch(is_equal, *target_bb, next_bb);
    }

    compile_arm_bodies(ctx, arms, &arm_bbs, merge_bb, scrutinee)
}

/// Nullable match: null check + branch.
///
/// A `null` literal arm matches the null case; identifier/wildcard
/// arms match the non-null value. Pointer types are checked with
/// ptr-to-int; integer types use -1 (all-ones) as a sentinel.
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
        let bb = ctx
            .context
            .append_basic_block(func, &format!("null_arm_{}", i));
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

    let i64_ty = ctx.context.i64_type();

    // Generate null check
    let is_null = match scrutinee.value {
        BasicValueEnum::PointerValue(p) => {
            let ptr_int = ctx.builder.build_ptr_to_int(p, i64_ty, "ptr_int");
            ctx.builder.build_int_compare(
                IntPredicate::EQ,
                ptr_int,
                i64_ty.const_int(0, false),
                "is_null",
            )
        }
        BasicValueEnum::IntValue(v) => {
            // For nullable primitives (erased to inner type), use -1 (all-ones) sentinel
            ctx.builder.build_int_compare(
                IntPredicate::EQ,
                v,
                i64_ty.const_all_ones(),
                "is_null_int",
            )
        }
        _ => return Err("Nullable match requires pointer or integer scrutinee".to_string()),
    };

    let null_target = null_bb.or(default_bb).unwrap_or(merge_bb);
    let value_target = value_bb.or(default_bb).unwrap_or(merge_bb);

    ctx.builder
        .build_conditional_branch(is_null, null_target, value_target);

    compile_arm_bodies(ctx, arms, &arm_bbs, merge_bb, scrutinee)
}

/// Array match using length comparison.
///
/// Each array pattern arm checks the array length. Literal length patterns
/// (e.g., `[]`, `[x]`, `[x, y]`) become length equality checks. Wildcard or
/// identifier arms serve as the default.
fn compile_array_match<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    scrutinee: &ExprResult<'ctx>,
    arms: &[MatchArm],
    merge_bb: inkwell::basic_block::BasicBlock<'ctx>,
) -> Result<(), String> {
    let func = ctx.current_function.ok_or("No current function")?;
    let array_ptr = match scrutinee.value {
        BasicValueEnum::PointerValue(p) => p,
        _ => return Err("Array match requires pointer scrutinee".to_string()),
    };

    let i64_ty = ctx.context.i64_type();

    // Get array length: first 8 bytes of array memory store the length
    let len_ptr = ctx.builder.build_pointer_cast(
        array_ptr,
        i64_ty.ptr_type(Default::default()),
        "array_len_ptr",
    );
    let array_len = ctx.builder.build_load(len_ptr, "array_len");

    let mut arm_bbs = Vec::with_capacity(arms.len());
    let mut length_cases: Vec<(u64, inkwell::basic_block::BasicBlock<'ctx>)> = Vec::new();
    let mut default_bb: Option<inkwell::basic_block::BasicBlock<'ctx>> = None;

    for (i, arm) in arms.iter().enumerate() {
        let bb = ctx
            .context
            .append_basic_block(func, &format!("arr_arm_{}", i));
        arm_bbs.push(bb);

        match &arm.pattern {
            Pattern::Array(elements) => {
                let mut len = 0u64;
                let mut has_rest = false;
                for elem in elements {
                    match elem {
                        crate::parser::ast::ArrayPatternElement::Pattern(_) => len += 1,
                        crate::parser::ast::ArrayPatternElement::Rest(_) => {
                            has_rest = true;
                            break;
                        }
                        crate::parser::ast::ArrayPatternElement::Elision => len += 1,
                    }
                }
                if has_rest {
                    if default_bb.is_none() {
                        default_bb = Some(bb);
                    }
                } else {
                    length_cases.push((len, bb));
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

    let default_block = default_bb.unwrap_or(merge_bb);

    ctx.builder.build_switch(
        array_len.into_int_value(),
        default_block,
        &length_cases
            .iter()
            .map(|(len, bb)| (i64_ty.const_int(*len, false), *bb))
            .collect::<Vec<_>>(),
    );

    compile_arm_bodies(ctx, arms, &arm_bbs, merge_bb, scrutinee)
}

/// Generic fallback match for unsupported types.
///
/// For Named types (classes), generates field-by-field comparison chains.
/// For other types, evaluates each arm in order with no optimisation.
fn compile_generic_match<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    scrutinee: &ExprResult<'ctx>,
    arms: &[MatchArm],
    merge_bb: inkwell::basic_block::BasicBlock<'ctx>,
) -> Result<(), String> {
    let func = ctx.current_function.ok_or("No current function")?;

    // For Named types (classes), try to generate field-based dispatch
    if let Type::Named(class_name) = &scrutinee.ty {
        let fields_opt = ctx.class_fields.get(class_name).cloned();
        let struct_type_opt = ctx.class_struct_types.get(class_name).cloned();
        if let (Some(fields), Some(struct_type)) = (fields_opt, struct_type_opt) {
            return compile_object_match(ctx, scrutinee, arms, merge_bb, &fields, struct_type);
        }
    }

    // Fallback: simple sequential dispatch
    let mut arm_bbs = Vec::with_capacity(arms.len());
    for (i, _) in arms.iter().enumerate() {
        let bb = ctx
            .context
            .append_basic_block(func, &format!("match_arm_{}", i));
        arm_bbs.push(bb);
    }

    let mut target_bb = merge_bb;
    for (i, arm) in arms.iter().enumerate().rev() {
        let is_catch_all = matches!(arm.pattern, Pattern::Wildcard | Pattern::Identifier(_))
            || matches!(&arm.pattern, Pattern::Object(_) if !has_literal_object_checks(&arm.pattern));
        if is_catch_all {
            target_bb = arm_bbs[i];
        }
    }

    ctx.builder.build_unconditional_branch(target_bb);
    compile_arm_bodies(ctx, arms, &arm_bbs, merge_bb, scrutinee)
}

fn has_literal_object_checks(pattern: &Pattern) -> bool {
    match pattern {
        Pattern::Object(fields) => {
            for f in fields {
                if let crate::parser::ast::ObjectPatternField::Property { pattern: inner, .. } = f {
                    if matches!(inner, Pattern::Literal(_)) {
                        return true;
                    }
                }
            }
            false
        }
        _ => false,
    }
}

/// Object match: generates field-by-field comparison chains.
///
/// For each arm, generates code that checks literal patterns against
/// the corresponding object fields. If all checks pass, jumps to that
/// arm's body. Otherwise, falls through to the next arm.
fn compile_object_match<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    scrutinee: &ExprResult<'ctx>,
    arms: &[MatchArm],
    merge_bb: inkwell::basic_block::BasicBlock<'ctx>,
    fields: &[(String, Type)],
    struct_type: inkwell::types::StructType<'ctx>,
) -> Result<(), String> {
    let func = ctx.current_function.ok_or("No current function")?;
    let obj_ptr = match scrutinee.value {
        BasicValueEnum::PointerValue(p) => p,
        _ => return Err("Object match requires pointer scrutinee".to_string()),
    };

    let i64_ty = ctx.context.i64_type();
    let i32_ty = ctx.context.i32_type();

    let mut arm_bbs = Vec::with_capacity(arms.len());
    for (i, _) in arms.iter().enumerate() {
        let bb = ctx
            .context
            .append_basic_block(func, &format!("obj_arm_{}", i));
        arm_bbs.push(bb);
    }

    // Find the first catch-all arm (wildcard, identifier, or object with no literal checks)
    let mut catch_all_idx = None;
    for (i, arm) in arms.iter().enumerate() {
        let is_wildcard_or_id = matches!(arm.pattern, Pattern::Wildcard | Pattern::Identifier(_));
        if is_wildcard_or_id {
            catch_all_idx = Some(i);
            break;
        }
        if let Pattern::Object(_) = &arm.pattern {
            let checks = collect_object_pattern_checks(&arm.pattern, fields)?;
            if checks.is_empty() {
                catch_all_idx = Some(i);
                break;
            }
        }
    }

    // Build check blocks only for arms before the catch-all
    let check_limit = catch_all_idx.unwrap_or(arms.len());
    let mut check_bbs: Vec<Option<inkwell::basic_block::BasicBlock<'ctx>>> =
        Vec::with_capacity(check_limit);
    for arm_idx in 0..check_limit {
        let arm = &arms[arm_idx];
        let checks = collect_object_pattern_checks(&arm.pattern, fields)?;
        if checks.is_empty() {
            check_bbs.push(None);
        } else {
            let check_bb = ctx
                .context
                .append_basic_block(func, &format!("obj_check_{}", arm_idx));
            check_bbs.push(Some(check_bb));
        }
    }

    // Entry: branch to first check or first arm
    let entry_bb = ctx.context.append_basic_block(func, "obj_match_entry");
    ctx.builder.build_unconditional_branch(entry_bb);
    ctx.builder.position_at_end(entry_bb);

    let first_target = check_bbs.iter().find_map(|x| *x).unwrap_or(arm_bbs[0]);
    ctx.builder.build_unconditional_branch(first_target);

    // Build each check block
    for (arm_idx, check_bb_opt) in check_bbs.iter().enumerate() {
        let check_bb = match check_bb_opt {
            Some(bb) => *bb,
            None => continue,
        };

        ctx.builder.position_at_end(check_bb);

        let checks = collect_object_pattern_checks(&arms[arm_idx].pattern, fields)?;
        let next_bb = if arm_idx + 1 < check_bbs.len() {
            check_bbs[arm_idx + 1].unwrap_or(arm_bbs[arm_idx + 1])
        } else if let Some(catch_idx) = catch_all_idx {
            arm_bbs[catch_idx]
        } else {
            merge_bb
        };

        chain_object_checks(
            ctx,
            &checks,
            obj_ptr,
            struct_type,
            arm_bbs[arm_idx],
            next_bb,
            i64_ty,
            i32_ty,
        )?;
    }

    compile_arm_bodies(ctx, arms, &arm_bbs, merge_bb, scrutinee)
}

struct FieldCheck {
    field_index: usize,
    expected_value: i64,
}

fn collect_object_pattern_checks(
    pattern: &Pattern,
    fields: &[(String, Type)],
) -> Result<Vec<FieldCheck>, String> {
    match pattern {
        Pattern::Object(obj_fields) => {
            let mut checks = Vec::new();
            for f in obj_fields {
                match f {
                    crate::parser::ast::ObjectPatternField::Property {
                        key,
                        pattern: inner,
                    } => {
                        if let Pattern::Literal(lit) = inner {
                            if let Expr::IntLiteral(n) = lit.as_ref() {
                                let field_index =
                                    fields
                                        .iter()
                                        .position(|(name, _)| name == key)
                                        .ok_or_else(|| format!("Unknown field: {}", key))?;
                                checks.push(FieldCheck {
                                    field_index,
                                    expected_value: *n,
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(checks)
        }
        _ => Ok(Vec::new()),
    }
}

fn chain_object_checks<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    checks: &[FieldCheck],
    obj_ptr: inkwell::values::PointerValue<'ctx>,
    struct_type: inkwell::types::StructType<'ctx>,
    success_bb: inkwell::basic_block::BasicBlock<'ctx>,
    fail_bb: inkwell::basic_block::BasicBlock<'ctx>,
    i64_ty: inkwell::types::IntType<'ctx>,
    i32_ty: inkwell::types::IntType<'ctx>,
) -> Result<(), String> {
    if checks.is_empty() {
        ctx.builder.build_unconditional_branch(success_bb);
        return Ok(());
    }

    let func = ctx.current_function.ok_or("No current function")?;

    // Cast object pointer to struct type
    let struct_ptr = ctx.builder.build_pointer_cast(
        obj_ptr,
        struct_type.ptr_type(Default::default()),
        "obj_struct_cast",
    );

    for (i, check) in checks.iter().enumerate() {
        let field_ptr = unsafe {
            ctx.builder.build_gep(
                struct_ptr,
                &[
                    i32_ty.const_int(0, false),
                    i32_ty.const_int(check.field_index as u64, false),
                ],
                &format!("field_ptr_{}", i),
            )
        };
        let field_val = ctx
            .builder
            .build_load(field_ptr, &format!("field_val_{}", i));

        let is_match = ctx.builder.build_int_compare(
            IntPredicate::EQ,
            field_val.into_int_value(),
            i64_ty.const_int(check.expected_value as u64, true),
            &format!("check_{}", i),
        );

        if i + 1 < checks.len() {
            let next_check_bb = ctx
                .context
                .append_basic_block(func, &format!("next_check_{}", i));
            ctx.builder
                .build_conditional_branch(is_match, next_check_bb, fail_bb);
            ctx.builder.position_at_end(next_check_bb);
        } else {
            ctx.builder
                .build_conditional_branch(is_match, success_bb, fail_bb);
        }
    }
    Ok(())
}
