/**
 * Expression code generation for Ruyi.
 *
 * Lowers Ruyi AST expressions to LLVM IR instructions.
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */

use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::BasicValueEnum;
use inkwell::IntPredicate;
use inkwell::FloatPredicate;

use crate::parser::ast::{ArrayElement, BinaryOp, Expr, MatchArm, Pattern, Statement, TemplatePart, UnaryOp, MemberProperty, Argument, ObjectProperty, PropertyName};
use crate::typechecker::types::{Type, ObjectField};
use super::builtins::{build_bigint_from_str, build_int_to_string, build_float_to_string};
use super::types::ruyi_type_to_llvm;
use super::generator::CodegenContext;

/// Result of compiling an expression.
pub struct ExprResult<'ctx> {
    pub value: BasicValueEnum<'ctx>,
    pub ty: Type,
}

impl<'ctx> ExprResult<'ctx> {
    pub fn new(value: BasicValueEnum<'ctx>, ty: Type) -> Self {
        Self { value, ty }
    }
}

/// Compile an expression into LLVM IR.
pub fn compile_expr<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    expr: &Expr,
) -> Result<ExprResult<'ctx>, String> {
    match expr {
        Expr::IntLiteral(n) => compile_int_literal(ctx, *n),
        Expr::FloatLiteral(n) => compile_float_literal(ctx, *n),
        Expr::BooleanLiteral(b) => compile_bool_literal(ctx, *b),
        Expr::StringLiteral(s) => compile_string_literal(ctx, s),
        Expr::BigIntLiteral(n) => compile_bigint_literal(ctx, n),
        Expr::NullLiteral => compile_null_literal(ctx),
        Expr::Identifier(name) => compile_identifier(ctx, name),
        Expr::Binary { op, left, right } => compile_binary(ctx, op, left, right),
        Expr::Unary { op, operand } => compile_unary(ctx, op, operand),
        Expr::Call { callee, args } => compile_call(ctx, callee, args),
        Expr::OptionalCall { .. } => Err("optional call not yet supported in codegen".to_string()),
        Expr::Assignment { left, op, right } => compile_assignment(ctx, left, op, right),
        Expr::Conditional { condition, then_branch, else_branch } => {
            compile_conditional(ctx, condition, then_branch, else_branch)
        }
        Expr::Grouping(inner) => compile_expr(ctx, inner),
        Expr::Await(inner) => super::async_codegen::compile_await(ctx, inner),
        Expr::SelfExpr => compile_self(ctx),
        Expr::Member { object, property, .. } => compile_member(ctx, object, property),
        Expr::New { callee, args } => compile_new(ctx, callee, args),
        Expr::TemplateLiteral(parts) => compile_template_literal(ctx, parts),
        Expr::ObjectLiteral(props) => compile_object_literal(ctx, props),
        Expr::ArrayLiteral(elements) => compile_array_literal(ctx, elements),
        Expr::Match { .. } => Err("match expression not yet supported in codegen".to_string()),
        Expr::Block(_) => Err("block expression not yet supported in codegen".to_string()),
        _ => Err(format!("Unsupported expression: {:?}", expr)),
    }
}

fn compile_int_literal<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
    n: i64,
) -> Result<ExprResult<'ctx>, String> {
    let val = ctx.context.i64_type().const_int(n as u64, true);
    Ok(ExprResult::new(BasicValueEnum::IntValue(val), Type::Int))
}

fn compile_float_literal<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
    n: f64,
) -> Result<ExprResult<'ctx>, String> {
    let val = ctx.context.f64_type().const_float(n);
    Ok(ExprResult::new(BasicValueEnum::FloatValue(val), Type::Float))
}

fn compile_bool_literal<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
    b: bool,
) -> Result<ExprResult<'ctx>, String> {
    let val = ctx.context.bool_type().const_int(b as u64, false);
    Ok(ExprResult::new(BasicValueEnum::IntValue(val), Type::Bool))
}

fn compile_string_literal<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    s: &str,
) -> Result<ExprResult<'ctx>, String> {
    let global = ctx.builder.build_global_string_ptr(s, "str_lit");
    Ok(ExprResult::new(
        BasicValueEnum::PointerValue(global.as_pointer_value()),
        Type::String,
    ))
}

fn compile_bigint_literal<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    n: &str,
) -> Result<ExprResult<'ctx>, String> {
    let global = ctx.builder.build_global_string_ptr(n, "bigint_lit");
    let str_ptr = BasicValueEnum::PointerValue(global.as_pointer_value());
    let bigint_ptr = super::builtins::build_bigint_from_str(&ctx.builder, ctx.module, str_ptr);
    Ok(ExprResult::new(bigint_ptr, Type::BigInt))
}

fn compile_array_literal<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    elements: &[crate::parser::ast::ArrayElement],
) -> Result<ExprResult<'ctx>, String> {
    let mut exprs: Vec<&Expr> = Vec::new();
    for elem in elements {
        match elem {
            ArrayElement::Expr(e) => exprs.push(e),
            ArrayElement::Spread(_) | ArrayElement::Elision => {}
        }
    }

    let count = exprs.len() as i64;
    let count_val = ctx.context.i64_type().const_int(count as u64, false);
    let arr_ptr = super::builtins::build_array_alloc(&ctx.builder, &ctx.module, count_val);

    for (idx, expr) in exprs.iter().enumerate() {
        let result = compile_expr(ctx, expr)?;
        let index_val = ctx.context.i64_type().const_int(idx as u64, false);
        let value_ptr = if result.value.is_pointer_value() {
            result.value.into_pointer_value()
        } else {
            let llvm_ty = ruyi_type_to_llvm(ctx.context, &result.ty);
            let alloca = ctx.builder.build_alloca(llvm_ty, "arr_elem");
            ctx.builder.build_store(alloca, result.value);
            alloca
        };
        let i8_ptr = ctx.context.i8_type().ptr_type(Default::default());
        let void_ptr = ctx.builder.build_bitcast(value_ptr, i8_ptr, "arr_elem_void").into_pointer_value();
        super::builtins::build_array_set(&ctx.builder, &ctx.module, arr_ptr, index_val, void_ptr);
    }

    Ok(ExprResult::new(
        BasicValueEnum::PointerValue(arr_ptr),
        Type::Array(Box::new(Type::Dynamic)),
    ))
}

/// Compile a template literal expression into a chain of string concatenations.
///
/// `Hello ${name}` becomes `"Hello " + name`
/// `${a} + ${b} = ${a+b}` becomes `a + (" + " + (b + " = " + (a + b + "")))`
///
/// Each template part is compiled and concatenated left-to-right.
fn compile_template_literal<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    parts: &[TemplatePart],
) -> Result<ExprResult<'ctx>, String> {
    // Empty template: `` → empty string
    if parts.is_empty() {
        let empty = ctx.builder.build_global_string_ptr("", "empty_template");
        return Ok(ExprResult::new(
            BasicValueEnum::PointerValue(empty.as_pointer_value()),
            Type::String,
        ));
    }

    // Start with first part
    let mut result = compile_template_part(ctx, &parts[0])?;

    // Chain concatenations with remaining parts
    for part in &parts[1..] {
        let part_result = compile_template_part(ctx, part)?;
        let concatenated = super::builtins::build_string_concat(
            &ctx.builder,
            ctx.module,
            result.value,
            part_result.value,
        );
        result = ExprResult::new(concatenated, Type::String);
    }

    Ok(result)
}

/// Compile a single template part to a string value.
fn compile_template_part<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    part: &TemplatePart,
) -> Result<ExprResult<'ctx>, String> {
    match part {
        TemplatePart::String(s) => compile_string_literal(ctx, s),
        TemplatePart::Expr(expr) => {
            let expr_result = compile_expr(ctx, expr)?;
            // Convert the expression result to string
            match expr_result.value {
                BasicValueEnum::PointerValue(ptr) => {
                    // Already a string pointer
                    Ok(ExprResult::new(BasicValueEnum::PointerValue(ptr), Type::String))
                }
                BasicValueEnum::IntValue(v) => {
                    let str_val = build_int_to_string(&ctx.builder, ctx.module, v);
                    Ok(ExprResult::new(str_val, Type::String))
                }
                BasicValueEnum::FloatValue(v) => {
                    let str_val = build_float_to_string(&ctx.builder, ctx.module, v);
                    Ok(ExprResult::new(str_val, Type::String))
                }
                _ => Err("Cannot convert type to string in template literal".to_string()),
            }
        }
    }
}

fn compile_null_literal<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
) -> Result<ExprResult<'ctx>, String> {
    let null_ptr = ctx.context.i8_type().ptr_type(Default::default()).const_null();
    Ok(ExprResult::new(BasicValueEnum::PointerValue(null_ptr), Type::Null))
}

fn compile_object_literal<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    properties: &[ObjectProperty],
) -> Result<ExprResult<'ctx>, String> {
    let mut fields: Vec<(String, ExprResult<'ctx>)> = Vec::new();

    for prop in properties {
        match prop {
            ObjectProperty::Property { key, value } => {
                let key_name = match key {
                    PropertyName::Ident(s) => s.clone(),
                    PropertyName::String(s) => s.clone(),
                    PropertyName::Number(n) => format!("{}", n),
                    PropertyName::Computed(_) => {
                        return Err("Computed property names not yet supported in codegen".to_string());
                    }
                };
                let value_result = compile_expr(ctx, value)?;
                fields.push((key_name, value_result));
            }
            ObjectProperty::Shorthand(name) => {
                match ctx.variables.get(name) {
                    Some((ptr, ty)) => {
                        let val = ctx.builder.build_load(*ptr, name);
                        fields.push((name.clone(), ExprResult::new(val, ty.clone())));
                    }
                    None => return Err(format!("Undefined variable in object shorthand: {}", name)),
                }
            }
            ObjectProperty::Spread(_) => {
                return Err("Object spread not yet supported in codegen".to_string());
            }
            ObjectProperty::ComputedProperty { .. } => {
                return Err("Computed property names not yet supported in codegen".to_string());
            }
        }
    }

    let field_count = fields.len() as i64;
    let field_count_val = ctx.context.i64_type().const_int(field_count as u64, false);
    let obj_ptr = super::builtins::build_object_alloc(&ctx.builder, &ctx.module, field_count_val);

    for (idx, (field_name, field_result)) in fields.iter().enumerate() {
        let offset = ctx.context.i64_type().const_int((8 + idx * 8) as u64, false);
        let field_addr = unsafe {
            ctx.builder.build_gep(
                obj_ptr,
                &[offset],
                &format!("obj_field_{}_addr", field_name),
            )
        };

        let llvm_ty = ruyi_type_to_llvm(ctx.context, &field_result.ty);
        let typed_ptr = ctx.builder.build_bitcast(
            field_addr,
            llvm_ty.ptr_type(Default::default()),
            &format!("obj_field_{}_ptr", field_name),
        ).into_pointer_value();

        ctx.builder.build_store(typed_ptr, field_result.value);
    }

    let object_fields: Vec<ObjectField> = fields
        .into_iter()
        .map(|(name, result)| ObjectField {
            name,
            ty: result.ty,
            optional: false,
        })
        .collect();

    Ok(ExprResult::new(
        BasicValueEnum::PointerValue(obj_ptr),
        Type::Object(object_fields),
    ))
}

fn compile_identifier<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
    name: &str,
) -> Result<ExprResult<'ctx>, String> {
    match ctx.variables.get(name) {
        Some((ptr, ty)) => {
            let val = ctx.builder.build_load(*ptr, name);
            Ok(ExprResult::new(val, ty.clone()))
        }
        None => {
            // Check if it's a function
            if let Some(func) = ctx.module.get_function(name) {
                Ok(ExprResult::new(
                    BasicValueEnum::PointerValue(func.as_global_value().as_pointer_value()),
                    Type::Function {
                        params: vec![],
                        return_type: Box::new(Type::Dynamic),
                    },
                ))
            } else {
                Err(format!("Undefined variable: {}", name))
            }
        }
    }
}

fn compile_binary<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    op: &BinaryOp,
    left: &Expr,
    right: &Expr,
) -> Result<ExprResult<'ctx>, String> {
    let left_result = compile_expr(ctx, left)?;
    let right_result = compile_expr(ctx, right)?;

    match op {
        BinaryOp::Plus => compile_add(ctx, &left_result, &right_result),
        BinaryOp::Minus => compile_sub(ctx, &left_result, &right_result),
        BinaryOp::Star => compile_mul(ctx, &left_result, &right_result),
        BinaryOp::Slash => compile_div(ctx, &left_result, &right_result),
        BinaryOp::Percent => compile_rem(ctx, &left_result, &right_result),
        BinaryOp::StrictEquals | BinaryOp::Equals => compile_eq(ctx, &left_result, &right_result),
        BinaryOp::StrictNotEquals | BinaryOp::NotEquals => compile_ne(ctx, &left_result, &right_result),
        BinaryOp::Less => compile_lt(ctx, &left_result, &right_result),
        BinaryOp::Greater => compile_gt(ctx, &left_result, &right_result),
        BinaryOp::LessEq => compile_le(ctx, &left_result, &right_result),
        BinaryOp::GreaterEq => compile_ge(ctx, &left_result, &right_result),
        BinaryOp::And => compile_and(ctx, &left_result, &right_result),
        BinaryOp::Or => compile_or(ctx, &left_result, &right_result),
        BinaryOp::Amp => compile_bitwise_and(ctx, &left_result, &right_result),
        BinaryOp::Pipe => compile_bitwise_or(ctx, &left_result, &right_result),
        BinaryOp::Caret => compile_bitwise_xor(ctx, &left_result, &right_result),
        BinaryOp::Shl => compile_shl(ctx, &left_result, &right_result),
        BinaryOp::Shr => compile_shr(ctx, &left_result, &right_result),
        _ => Err(format!("Unsupported binary operator: {:?}", op)),
    }
}

fn compile_add<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.ty, &right.ty) {
        (Type::String, Type::String) => {
            let res = super::builtins::build_string_concat(
                &ctx.builder,
                &ctx.module,
                left.value,
                right.value,
            );
            Ok(ExprResult::new(res, Type::String))
        }
        (Type::String, Type::Int) => {
            let num_str = super::builtins::build_int_to_string(
                &ctx.builder,
                &ctx.module,
                right.value.into_int_value(),
            );
            let res = super::builtins::build_string_concat(
                &ctx.builder,
                &ctx.module,
                left.value,
                num_str,
            );
            Ok(ExprResult::new(res, Type::String))
        }
        (Type::Int, Type::String) => {
            let num_str = super::builtins::build_int_to_string(
                &ctx.builder,
                &ctx.module,
                left.value.into_int_value(),
            );
            let res = super::builtins::build_string_concat(
                &ctx.builder,
                &ctx.module,
                num_str,
                right.value,
            );
            Ok(ExprResult::new(res, Type::String))
        }
        (Type::String, Type::Float) => {
            let num_str = super::builtins::build_float_to_string(
                &ctx.builder,
                &ctx.module,
                right.value.into_float_value(),
            );
            let res = super::builtins::build_string_concat(
                &ctx.builder,
                &ctx.module,
                left.value,
                num_str,
            );
            Ok(ExprResult::new(res, Type::String))
        }
        (Type::Float, Type::String) => {
            let num_str = super::builtins::build_float_to_string(
                &ctx.builder,
                &ctx.module,
                left.value.into_float_value(),
            );
            let res = super::builtins::build_string_concat(
                &ctx.builder,
                &ctx.module,
                num_str,
                right.value,
            );
            Ok(ExprResult::new(res, Type::String))
        }
        _ => match (&left.value, &right.value) {
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                let res = ctx.builder.build_int_add(*l, *r, "add");
                Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
            }
            (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
                let res = ctx.builder.build_float_add(*l, *r, "fadd");
                Ok(ExprResult::new(BasicValueEnum::FloatValue(res), Type::Float))
            }
            (BasicValueEnum::IntValue(l), BasicValueEnum::FloatValue(r)) => {
                let l_f = ctx.builder.build_signed_int_to_float(*l, ctx.context.f64_type(), "itof");
                let res = ctx.builder.build_float_add(l_f, *r, "fadd");
                Ok(ExprResult::new(BasicValueEnum::FloatValue(res), Type::Float))
            }
            (BasicValueEnum::FloatValue(l), BasicValueEnum::IntValue(r)) => {
                let r_f = ctx.builder.build_signed_int_to_float(*r, ctx.context.f64_type(), "itof");
                let res = ctx.builder.build_float_add(*l, r_f, "fadd");
                Ok(ExprResult::new(BasicValueEnum::FloatValue(res), Type::Float))
            }
            _ => Err("Invalid operands for +".to_string()),
        },
    }
}

fn compile_sub<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder.build_int_sub(*l, *r, "sub");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
            let res = ctx.builder.build_float_sub(*l, *r, "fsub");
            Ok(ExprResult::new(BasicValueEnum::FloatValue(res), Type::Float))
        }
        _ => Err("Invalid operands for -".to_string()),
    }
}

fn compile_mul<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder.build_int_mul(*l, *r, "mul");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
            let res = ctx.builder.build_float_mul(*l, *r, "fmul");
            Ok(ExprResult::new(BasicValueEnum::FloatValue(res), Type::Float))
        }
        _ => Err("Invalid operands for *".to_string()),
    }
}

fn compile_div<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder.build_int_signed_div(*l, *r, "sdiv");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
            let res = ctx.builder.build_float_div(*l, *r, "fdiv");
            Ok(ExprResult::new(BasicValueEnum::FloatValue(res), Type::Float))
        }
        _ => Err("Invalid operands for /".to_string()),
    }
}

fn compile_rem<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder.build_int_signed_rem(*l, *r, "srem");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
        }
        _ => Err("Invalid operands for %".to_string()),
    }
}

pub(super) fn compile_eq<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder.build_int_compare(IntPredicate::EQ, *l, *r, "eq");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
            let res = ctx.builder.build_float_compare(FloatPredicate::OEQ, *l, *r, "feq");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        _ => Err("Invalid operands for ===".to_string()),
    }
}

fn compile_ne<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder.build_int_compare(IntPredicate::NE, *l, *r, "ne");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
            let res = ctx.builder.build_float_compare(FloatPredicate::ONE, *l, *r, "fne");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        _ => Err("Invalid operands for !==".to_string()),
    }
}

fn compile_lt<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder.build_int_compare(IntPredicate::SLT, *l, *r, "lt");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
            let res = ctx.builder.build_float_compare(FloatPredicate::OLT, *l, *r, "flt");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        _ => Err("Invalid operands for <".to_string()),
    }
}

fn compile_gt<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder.build_int_compare(IntPredicate::SGT, *l, *r, "gt");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
            let res = ctx.builder.build_float_compare(FloatPredicate::OGT, *l, *r, "fgt");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        _ => Err("Invalid operands for >".to_string()),
    }
}

fn compile_le<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder.build_int_compare(IntPredicate::SLE, *l, *r, "le");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
            let res = ctx.builder.build_float_compare(FloatPredicate::OLE, *l, *r, "fle");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        _ => Err("Invalid operands for <=".to_string()),
    }
}

fn compile_ge<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder.build_int_compare(IntPredicate::SGE, *l, *r, "ge");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
            let res = ctx.builder.build_float_compare(FloatPredicate::OGE, *l, *r, "fge");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        _ => Err("Invalid operands for >=".to_string()),
    }
}

fn compile_and<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder.build_and(*l, *r, "and");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        _ => Err("Invalid operands for &&".to_string()),
    }
}

fn compile_or<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder.build_or(*l, *r, "or");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        _ => Err("Invalid operands for ||".to_string()),
    }
}

fn compile_bitwise_and<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder.build_and(*l, *r, "band");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
        }
        _ => Err("Invalid operands for &".to_string()),
    }
}

fn compile_bitwise_or<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder.build_or(*l, *r, "bor");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
        }
        _ => Err("Invalid operands for |".to_string()),
    }
}

fn compile_bitwise_xor<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder.build_xor(*l, *r, "bxor");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
        }
        _ => Err("Invalid operands for ^".to_string()),
    }
}

fn compile_shl<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder.build_left_shift(*l, *r, "shl");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
        }
        _ => Err("Invalid operands for <<".to_string()),
    }
}

fn compile_shr<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder.build_right_shift(*l, *r, true, "shr");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
        }
        _ => Err("Invalid operands for >>".to_string()),
    }
}

fn compile_unary<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    op: &UnaryOp,
    operand: &Expr,
) -> Result<ExprResult<'ctx>, String> {
    let operand_result = compile_expr(ctx, operand)?;

    match op {
        UnaryOp::Minus => match operand_result.value {
            BasicValueEnum::IntValue(v) => {
                let zero = ctx.context.i64_type().const_int(0, false);
                let res = ctx.builder.build_int_sub(zero, v, "neg");
                Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
            }
            BasicValueEnum::FloatValue(v) => {
                let zero = ctx.context.f64_type().const_float(0.0);
                let res = ctx.builder.build_float_sub(zero, v, "fneg");
                Ok(ExprResult::new(BasicValueEnum::FloatValue(res), Type::Float))
            }
            _ => Err("Invalid operand for unary -".to_string()),
        },
        UnaryOp::Not => match operand_result.value {
            BasicValueEnum::IntValue(v) => {
                let one = ctx.context.bool_type().const_int(1, false);
                let res = ctx.builder.build_xor(v, one, "not");
                Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
            }
            _ => Err("Invalid operand for !".to_string()),
        },
        UnaryOp::Tilde => match operand_result.value {
            BasicValueEnum::IntValue(v) => {
                let minus_one = ctx.context.i64_type().const_int(u64::MAX, true);
                let res = ctx.builder.build_xor(v, minus_one, "bitnot");
                Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
            }
            _ => Err("Invalid operand for ~".to_string()),
        },
        _ => Err(format!("Unsupported unary operator: {:?}", op)),
    }
}

fn compile_self<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
) -> Result<ExprResult<'ctx>, String> {
    match ctx.variables.get("self") {
        Some((ptr, ty)) => {
            let val = ctx.builder.build_load(*ptr, "self");
            Ok(ExprResult::new(val, ty.clone()))
        }
        None => Err("self used outside of method".to_string()),
    }
}

fn get_class_name_from_expr<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
    object: &Expr,
    obj_ty: &Type,
) -> Result<String, String> {
    let base_ty = obj_ty.unwrap_nullable();
    match base_ty {
        Type::Named(name) => Ok(name.clone()),
        _ => {
            if let Expr::Identifier(name) = object {
                if name == "self" {
                    if let Some((_, ty)) = ctx.variables.get("self") {
                        match ty.unwrap_nullable() {
                            Type::Named(n) => Ok(n.clone()),
                            _ => Err("self is not a class instance".to_string()),
                        }
                    } else {
                        Err("self not found".to_string())
                    }
                } else {
                    Err(format!("Member access on non-class type: {:?}", obj_ty))
                }
            } else {
                Err(format!("Member access on non-class type: {:?}", obj_ty))
            }
        }
    }
}

fn compile_member<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    object: &Expr,
    property: &MemberProperty,
) -> Result<ExprResult<'ctx>, String> {
    let obj_result = compile_expr(ctx, object)?;

    match property {
        MemberProperty::Ident(field_name) => {
            let fields: Vec<(String, Type)> = match &obj_result.ty {
                Type::Named(class_name) => {
                    ctx.class_fields.get(class_name)
                        .ok_or_else(|| format!("Class not found: {}", class_name))?
                        .clone()
                }
                Type::Object(obj_fields) => {
                    obj_fields.iter().map(|f| (f.name.clone(), f.ty.clone())).collect()
                }
                _ => return Err(format!("Member access on non-object type: {:?}", obj_result.ty)),
            };

            let (field_idx, field_ty) = fields.iter().enumerate()
                .find(|(_, (name, _))| name == field_name)
                .map(|(idx, (_, ty))| (idx, ty.clone()))
                .ok_or_else(|| format!("Field {} not found in object", field_name))?;

            let obj_ptr = obj_result.value.into_pointer_value();
            let offset = ctx.context.i64_type().const_int((8 + field_idx * 8) as u64, false);
            let field_addr = unsafe {
                ctx.builder.build_gep(
                    obj_ptr,
                    &[offset],
                    &format!("field_{}_addr", field_name),
                )
            };

            let llvm_ty = ruyi_type_to_llvm(ctx.context, &field_ty);
            let typed_ptr = ctx.builder.build_bitcast(
                field_addr,
                llvm_ty.ptr_type(Default::default()),
                &format!("field_{}_ptr", field_name),
            ).into_pointer_value();

            let val = ctx.builder.build_load(typed_ptr, &format!("field_{}", field_name));
            Ok(ExprResult::new(val, field_ty))
        }
        MemberProperty::Expr(index_expr) => {
            let elem_ty = get_array_elem_type(&obj_result.ty);
            let index_result = compile_expr(ctx, index_expr)?;
            let array_ptr = obj_result.value.into_pointer_value();
            let index_val = match index_result.value {
                BasicValueEnum::IntValue(v) => v,
                _ => return Err("Array index must be an integer".to_string()),
            };

            let elem_ptr = super::builtins::build_array_get(&ctx.builder, &ctx.module, array_ptr, index_val);
            let val = unbox_value(ctx, elem_ptr, &elem_ty);
            Ok(ExprResult::new(val, elem_ty))
        }
    }
}

fn compile_direct_field_access<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    obj_result: &ExprResult<'ctx>,
    field_idx: usize,
    field_ty: &Type,
    field_name: &str,
) -> Result<ExprResult<'ctx>, String> {
    let obj_ptr = obj_result.value.into_pointer_value();
    let offset = ctx.context.i64_type().const_int((8 + field_idx * 8) as u64, false);
    let field_addr = unsafe {
        ctx.builder.build_gep(
            obj_ptr,
            &[offset],
            &format!("field_{}_addr", field_name),
        )
    };

    let llvm_ty = ruyi_type_to_llvm(ctx.context, field_ty);
    let typed_ptr = ctx.builder.build_bitcast(
        field_addr,
        llvm_ty.ptr_type(Default::default()),
        &format!("field_{}_ptr", field_name),
    ).into_pointer_value();

    let val = ctx.builder.build_load(typed_ptr, &format!("field_{}", field_name));
    Ok(ExprResult::new(val, field_ty.clone()))
}

fn compile_optional_field_access<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    obj_result: &ExprResult<'ctx>,
    field_idx: usize,
    field_ty: &Type,
    field_name: &str,
) -> Result<ExprResult<'ctx>, String> {
    let obj_ptr = match obj_result.value {
        BasicValueEnum::PointerValue(p) => p,
        _ => return Err("Optional chaining requires an object reference".to_string()),
    };

    let func = ctx.current_function.ok_or("No current function")?;
    let null_bb = ctx.context.append_basic_block(func, "opt_null");
    let access_bb = ctx.context.append_basic_block(func, "opt_access");
    let merge_bb = ctx.context.append_basic_block(func, "opt_merge");

    let null_ptr = obj_ptr.get_type().const_null();
    let is_null = ctx.builder.build_int_compare(
        IntPredicate::EQ,
        obj_ptr,
        null_ptr,
        "is_null",
    );
    ctx.builder.build_conditional_branch(is_null, null_bb, access_bb);

    ctx.builder.position_at_end(null_bb);
    let null_val = build_null_value(ctx, field_ty);
    ctx.builder.build_unconditional_branch(merge_bb);
    let null_bb_end = ctx.builder.get_insert_block().unwrap();

    ctx.builder.position_at_end(access_bb);
    let access_result = compile_direct_field_access(ctx, obj_result, field_idx, field_ty, field_name)?;
    ctx.builder.build_unconditional_branch(merge_bb);
    let access_bb_end = ctx.builder.get_insert_block().unwrap();

    ctx.builder.position_at_end(merge_bb);
    let phi_ty = ruyi_type_to_llvm(ctx.context, field_ty);
    let phi = ctx.builder.build_phi(phi_ty, "opt_phi");
    phi.add_incoming(&[(&null_val, null_bb_end), (&access_result.value, access_bb_end)]);

    let result_ty = field_ty.clone().make_nullable();
    Ok(ExprResult::new(phi.as_basic_value(), result_ty))
}

fn compile_direct_array_access<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    obj_result: &ExprResult<'ctx>,
    index_expr: &Expr,
) -> Result<ExprResult<'ctx>, String> {
    let elem_ty = get_array_elem_type(obj_result.ty.unwrap_nullable());
    let index_result = compile_expr(ctx, index_expr)?;
    let array_ptr = obj_result.value.into_pointer_value();
    let index_val = match index_result.value {
        BasicValueEnum::IntValue(v) => v,
        _ => return Err("Array index must be an integer".to_string()),
    };

    let elem_ptr = super::builtins::build_array_get(&ctx.builder, &ctx.module, array_ptr, index_val);
    let val = unbox_value(ctx, elem_ptr, &elem_ty);
    Ok(ExprResult::new(val, elem_ty))
}

fn compile_optional_array_access<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    obj_result: &ExprResult<'ctx>,
    index_expr: &Expr,
) -> Result<ExprResult<'ctx>, String> {
    let elem_ty = get_array_elem_type(obj_result.ty.unwrap_nullable());
    let array_ptr = match obj_result.value {
        BasicValueEnum::PointerValue(p) => p,
        _ => return Err("Optional chaining requires an array reference".to_string()),
    };

    let func = ctx.current_function.ok_or("No current function")?;
    let null_bb = ctx.context.append_basic_block(func, "opt_null");
    let access_bb = ctx.context.append_basic_block(func, "opt_access");
    let merge_bb = ctx.context.append_basic_block(func, "opt_merge");

    let null_ptr = array_ptr.get_type().const_null();
    let is_null = ctx.builder.build_int_compare(
        IntPredicate::EQ,
        array_ptr,
        null_ptr,
        "is_null",
    );
    ctx.builder.build_conditional_branch(is_null, null_bb, access_bb);

    ctx.builder.position_at_end(null_bb);
    let null_val = build_null_value(ctx, &elem_ty);
    ctx.builder.build_unconditional_branch(merge_bb);
    let null_bb_end = ctx.builder.get_insert_block().unwrap();

    ctx.builder.position_at_end(access_bb);
    let index_result = compile_expr(ctx, index_expr)?;
    let index_val = match index_result.value {
        BasicValueEnum::IntValue(v) => v,
        _ => return Err("Array index must be an integer".to_string()),
    };
    let elem_ptr = super::builtins::build_array_get(&ctx.builder, &ctx.module, array_ptr, index_val);
    let access_val = unbox_value(ctx, elem_ptr, &elem_ty);
    ctx.builder.build_unconditional_branch(merge_bb);
    let access_bb_end = ctx.builder.get_insert_block().unwrap();

    ctx.builder.position_at_end(merge_bb);
    let phi_ty = ruyi_type_to_llvm(ctx.context, &elem_ty);
    let phi = ctx.builder.build_phi(phi_ty, "opt_phi");
    phi.add_incoming(&[(&null_val, null_bb_end), (&access_val, access_bb_end)]);

    let result_ty = elem_ty.make_nullable();
    Ok(ExprResult::new(phi.as_basic_value(), result_ty))
}

fn build_null_value<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
    ty: &Type,
) -> BasicValueEnum<'ctx> {
    match ruyi_type_to_llvm(ctx.context, ty) {
        BasicTypeEnum::IntType(t) => BasicValueEnum::IntValue(t.const_int(0, false)),
        BasicTypeEnum::FloatType(t) => BasicValueEnum::FloatValue(t.const_float(0.0)),
        BasicTypeEnum::PointerType(t) => BasicValueEnum::PointerValue(t.const_null()),
        BasicTypeEnum::StructType(t) => BasicValueEnum::StructValue(t.const_zero()),
        BasicTypeEnum::ArrayType(t) => BasicValueEnum::ArrayValue(t.const_zero()),
        BasicTypeEnum::VectorType(t) => BasicValueEnum::VectorValue(t.const_zero()),
    }
}

fn build_null_value_for_basic_ty<'ctx>(
    basic_ty: BasicTypeEnum<'ctx>,
) -> BasicValueEnum<'ctx> {
    match basic_ty {
        BasicTypeEnum::IntType(t) => BasicValueEnum::IntValue(t.const_int(0, false)),
        BasicTypeEnum::FloatType(t) => BasicValueEnum::FloatValue(t.const_float(0.0)),
        BasicTypeEnum::PointerType(t) => BasicValueEnum::PointerValue(t.const_null()),
        BasicTypeEnum::StructType(t) => BasicValueEnum::StructValue(t.const_zero()),
        BasicTypeEnum::ArrayType(t) => BasicValueEnum::ArrayValue(t.const_zero()),
        BasicTypeEnum::VectorType(t) => BasicValueEnum::VectorValue(t.const_zero()),
    }
}

fn compile_optional_call<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    callee: &Expr,
    args: &[crate::parser::ast::Argument],
) -> Result<ExprResult<'ctx>, String> {
    let callee_result = compile_expr(ctx, callee)?;

    let callee_ptr = match callee_result.value {
        BasicValueEnum::PointerValue(p) => p,
        _ => return Err("Optional call requires a callable reference".to_string()),
    };

    let func = ctx.current_function.ok_or("No current function")?;
    let null_bb = ctx.context.append_basic_block(func, "opt_call_null");
    let call_bb = ctx.context.append_basic_block(func, "opt_call_call");
    let merge_bb = ctx.context.append_basic_block(func, "opt_call_merge");

    let null_ptr = callee_ptr.get_type().const_null();
    let is_null = ctx.builder.build_int_compare(
        IntPredicate::EQ,
        callee_ptr,
        null_ptr,
        "opt_call_is_null",
    );
    ctx.builder.build_conditional_branch(is_null, null_bb, call_bb);

    ctx.builder.position_at_end(null_bb);
    let null_val = build_null_value(ctx, &Type::Null);
    ctx.builder.build_unconditional_branch(merge_bb);
    let null_bb_end = ctx.builder.get_insert_block().unwrap();

    ctx.builder.position_at_end(call_bb);
    let call_result = compile_call(ctx, callee, args)?;
    ctx.builder.build_unconditional_branch(merge_bb);
    let call_bb_end = ctx.builder.get_insert_block().unwrap();

    ctx.builder.position_at_end(merge_bb);
    let phi_ty = ruyi_type_to_llvm(ctx.context, &call_result.ty);
    let phi = ctx.builder.build_phi(phi_ty, "opt_call_phi");
    phi.add_incoming(&[(&null_val, null_bb_end), (&call_result.value, call_bb_end)]);

    let result_ty = call_result.ty.make_nullable();
    Ok(ExprResult::new(phi.as_basic_value(), result_ty))
}

fn compile_new<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    callee: &Expr,
    args: &[Argument],
) -> Result<ExprResult<'ctx>, String> {
    let class_name = match callee {
        Expr::Identifier(n) => n.clone(),
        _ => return Err("Complex new expressions not yet supported".to_string()),
    };

    let fields = ctx.class_fields.get(&class_name)
        .ok_or_else(|| format!("Class not found: {}", class_name))?;

    let field_count = fields.len() as i64;
    let field_count_val = ctx.context.i64_type().const_int(field_count as u64, false);
    let obj_ptr = super::builtins::build_object_alloc(&ctx.builder, &ctx.module, field_count_val);

    let ctor_name = format!("{}_new", class_name);
    if let Some(ctor_fn) = ctx.module.get_function(&ctor_name) {
        let mut arg_values = vec![obj_ptr.into()];
        for arg in args {
            match arg {
                Argument::Expr(e) => {
                    let result = compile_expr(ctx, e)?;
                    arg_values.push(result.value.into());
                }
                _ => return Err("Spread arguments not yet supported".to_string()),
            }
        }
        ctx.builder.build_call(ctor_fn, &arg_values, "ctor_call");
    }

    Ok(ExprResult::new(
        BasicValueEnum::PointerValue(obj_ptr),
        Type::Named(class_name),
    ))
}

fn compile_call<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    callee: &Expr,
    args: &[crate::parser::ast::Argument],
) -> Result<ExprResult<'ctx>, String> {
    match callee {
        Expr::Identifier(name) => {
            if name == "print" {
                if args.len() == 1 {
                    match &args[0] {
                        crate::parser::ast::Argument::Expr(e) => {
                            let result = compile_expr(ctx, e)?;
                            super::builtins::build_print(&ctx.builder, &ctx.module, result.value);
                            return Ok(ExprResult::new(
                                BasicValueEnum::IntValue(ctx.context.i64_type().const_int(0, false)),
                                Type::Void,
                            ));
                        }
                        _ => return Err("Invalid print argument".to_string()),
                    }
                } else {
                    return Err("print expects exactly 1 argument".to_string());
                }
            }

            if name == "Error" {
                return compile_error_ctor(ctx, args);
            }

            let func = ctx.module.get_function(name)
                .ok_or_else(|| format!("Function not found: {}", name))?;

            let mut arg_values = Vec::new();
            for arg in args {
                match arg {
                    crate::parser::ast::Argument::Expr(e) => {
                        let result = compile_expr(ctx, e)?;
                        arg_values.push(result.value);
                    }
                    _ => return Err("Spread arguments not yet supported".to_string()),
                }
            }

            let call_site = build_call_or_invoke(ctx, func, &arg_values, "call");
            let value = call_site.try_as_basic_value().left();

            let ret_ty = Type::Dynamic;
            match value {
                Some(v) => Ok(ExprResult::new(v, ret_ty)),
                None => Ok(ExprResult::new(
                    BasicValueEnum::IntValue(ctx.context.i64_type().const_int(0, false)),
                    Type::Void,
                )),
            }
        }
        Expr::Member { object, property: MemberProperty::Ident(method_name), .. } => {
            match object.as_ref() {
                Expr::Identifier(class_name) if ctx.class_fields.contains_key(class_name) => {
                    let mangled_name = format!("{}_{}", class_name, method_name);

                    if method_name == "new" {
                        let fields = ctx.class_fields.get(class_name)
                            .ok_or_else(|| format!("Class not found: {}", class_name))?;
                        let field_count = fields.len() as i64;
                        let field_count_val = ctx.context.i64_type().const_int(field_count as u64, false);
                        let obj_ptr = super::builtins::build_object_alloc(&ctx.builder, &ctx.module, field_count_val);

                        if let Some(ctor_fn) = ctx.module.get_function(&mangled_name) {
                            let mut arg_values = vec![BasicValueEnum::PointerValue(obj_ptr)];
                            for arg in args {
                                match arg {
                                    crate::parser::ast::Argument::Expr(e) => {
                                        let result = compile_expr(ctx, e)?;
                                        arg_values.push(result.value);
                                    }
                                    _ => return Err("Spread arguments not yet supported".to_string()),
                                }
                            }
                            build_call_or_invoke(ctx, ctor_fn, &arg_values, "ctor_call");
                        }

                        return Ok(ExprResult::new(
                            BasicValueEnum::PointerValue(obj_ptr),
                            Type::Named(class_name.clone()),
                        ));
                    }

                    let func = ctx.module.get_function(&mangled_name)
                        .ok_or_else(|| format!("Method not found: {}", mangled_name))?;

                    let mut arg_values = Vec::new();
                    for arg in args {
                        match arg {
                            crate::parser::ast::Argument::Expr(e) => {
                                let result = compile_expr(ctx, e)?;
                                arg_values.push(result.value);
                            }
                            _ => return Err("Spread arguments not yet supported".to_string()),
                        }
                    }

                    let call_site = build_call_or_invoke(ctx, func, &arg_values, "call");
                    let value = call_site.try_as_basic_value().left();
                    let ret_ty = Type::Dynamic;
                    match value {
                        Some(v) => Ok(ExprResult::new(v, ret_ty)),
                        None => Ok(ExprResult::new(
                            BasicValueEnum::IntValue(ctx.context.i64_type().const_int(0, false)),
                            Type::Void,
                        )),
                    }
                }
                _ => {
                    let obj_result = compile_expr(ctx, object)?;
                    let class_name = match &obj_result.ty {
                        Type::Named(name) => name.clone(),
                        _ => return Err(format!("Method call on non-class type: {:?}", obj_result.ty)),
                    };

                    let mangled_name = format!("{}_{}", class_name, method_name);
                    let func = ctx.module.get_function(&mangled_name)
                        .ok_or_else(|| format!("Method not found: {}", mangled_name))?;

                    let mut arg_values = vec![obj_result.value];
                    for arg in args {
                        match arg {
                            crate::parser::ast::Argument::Expr(e) => {
                                let result = compile_expr(ctx, e)?;
                                arg_values.push(result.value);
                            }
                            _ => return Err("Spread arguments not yet supported".to_string()),
                        }
                    }

                    let call_site = build_call_or_invoke(ctx, func, &arg_values, "call");
                    let value = call_site.try_as_basic_value().left();
                    let ret_ty = Type::Dynamic;
                    match value {
                        Some(v) => Ok(ExprResult::new(v, ret_ty)),
                        None => Ok(ExprResult::new(
                            BasicValueEnum::IntValue(ctx.context.i64_type().const_int(0, false)),
                            Type::Void,
                        )),
                    }
                }
            }
        }
        _ => Err("Indirect calls not yet supported".to_string()),
    }
}

/// Build a `call` or `invoke` instruction depending on whether we are
/// inside a `try` region.
fn build_call_or_invoke<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    func: inkwell::values::FunctionValue<'ctx>,
    args: &[inkwell::values::BasicValueEnum<'ctx>],
    name: &str,
) -> inkwell::values::CallSiteValue<'ctx> {
    if let Some(catch_bb) = ctx.exception_stack.last().copied() {
        let func_val = ctx.current_function.unwrap();
        let normal_bb = ctx.context.append_basic_block(func_val, &format!("{}_normal", name));
        let call_site = ctx.builder.build_invoke(func, args, normal_bb, catch_bb, name);
        ctx.builder.position_at_end(normal_bb);
        call_site
    } else {
        let metadata_args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> =
            args.iter().map(|a| (*a).into()).collect();
        ctx.builder.build_call(func, &metadata_args, name)
    }
}

/// Compile `Error("message")` into an on-stack exception object.
fn compile_error_ctor<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    args: &[crate::parser::ast::Argument],
) -> Result<ExprResult<'ctx>, String> {
    let msg_ptr = if args.len() == 1 {
        match &args[0] {
            crate::parser::ast::Argument::Expr(e) => {
                let result = compile_expr(ctx, e)?;
                match result.value {
                    BasicValueEnum::PointerValue(p) => p,
                    _ => return Err("Error message must be a string".to_string()),
                }
            }
            _ => return Err("Invalid Error argument".to_string()),
        }
    } else {
        return Err("Error expects exactly 1 argument".to_string());
    };

    let i64_ty = ctx.context.i64_type();
    let i8_ptr = ctx.context.i8_type().ptr_type(Default::default());

    // ExceptionObject layout: { type_tag: i64, message: *i8, trace_len: i64, trace: *i8 }
    let exc_type = ctx.context.struct_type(&[
        i64_ty.into(),
        i8_ptr.into(),
        i64_ty.into(),
        i8_ptr.into(),
    ], false);
    let exc_ptr = ctx.builder.build_alloca(exc_type, "exc");

    // type_tag = 1 (Error)
    let type_tag_ptr = unsafe {
        ctx.builder.build_in_bounds_gep(
            exc_ptr,
            &[ctx.context.i32_type().const_int(0, false), ctx.context.i32_type().const_int(0, false)],
            "exc.type_tag",
        )
    };
    ctx.builder.build_store(type_tag_ptr, i64_ty.const_int(1, false));

    // message
    let msg_field_ptr = unsafe {
        ctx.builder.build_in_bounds_gep(
            exc_ptr,
            &[ctx.context.i32_type().const_int(0, false), ctx.context.i32_type().const_int(1, false)],
            "exc.message",
        )
    };
    ctx.builder.build_store(msg_field_ptr, msg_ptr);

    // stack_trace_len = 0
    let trace_len_ptr = unsafe {
        ctx.builder.build_in_bounds_gep(
            exc_ptr,
            &[ctx.context.i32_type().const_int(0, false), ctx.context.i32_type().const_int(2, false)],
            "exc.trace_len",
        )
    };
    ctx.builder.build_store(trace_len_ptr, i64_ty.const_int(0, false));

    // stack_trace = null
    let trace_ptr_field = unsafe {
        ctx.builder.build_in_bounds_gep(
            exc_ptr,
            &[ctx.context.i32_type().const_int(0, false), ctx.context.i32_type().const_int(3, false)],
            "exc.trace",
        )
    };
    ctx.builder.build_store(trace_ptr_field, i8_ptr.const_null());

    let exc_void_ptr = ctx.builder.build_bitcast(exc_ptr, i8_ptr, "exc_void").into_pointer_value();
    Ok(ExprResult::new(BasicValueEnum::PointerValue(exc_void_ptr), Type::Named("Error".to_string())))
}

fn compile_assignment<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    left: &Expr,
    op: &crate::parser::ast::AssignOp,
    right: &Expr,
) -> Result<ExprResult<'ctx>, String> {
    let right_result = compile_expr(ctx, right)?;

    match op {
        crate::parser::ast::AssignOp::Assign => {
            match left {
                Expr::Identifier(name) => {
                    if let Some((ptr, _)) = ctx.variables.get(name) {
                        ctx.builder.build_store(*ptr, right_result.value);
                        Ok(right_result)
                    } else {
                        Err(format!("Undefined variable: {}", name))
                    }
                }
                Expr::Member { object, property: MemberProperty::Ident(field_name), .. } => {
                    let obj_result = compile_expr(ctx, object)?;

                    let fields: Vec<(String, Type)> = match &obj_result.ty {
                        Type::Named(class_name) => {
                            ctx.class_fields.get(class_name)
                                .ok_or_else(|| format!("Class not found: {}", class_name))?
                                .clone()
                        }
                        Type::Object(obj_fields) => {
                            obj_fields.iter().map(|f| (f.name.clone(), f.ty.clone())).collect()
                        }
                        _ => return Err(format!("Assignment on non-object type: {:?}", obj_result.ty)),
                    };

                    let (field_idx, field_ty) = fields.iter().enumerate()
                        .find(|(_, (name, _))| name == field_name)
                        .map(|(idx, (_, ty))| (idx, ty.clone()))
                        .ok_or_else(|| format!("Field {} not found in object", field_name))?;

                    let obj_ptr = obj_result.value.into_pointer_value();
                    let offset = ctx.context.i64_type().const_int((8 + field_idx * 8) as u64, false);
                    let field_addr = unsafe {
                        ctx.builder.build_gep(
                            obj_ptr,
                            &[offset],
                            &format!("field_{}_addr", field_name),
                        )
                    };

                    let llvm_ty = ruyi_type_to_llvm(ctx.context, &field_ty);
                    let typed_ptr = ctx.builder.build_bitcast(
                        field_addr,
                        llvm_ty.ptr_type(Default::default()),
                        &format!("field_{}_ptr", field_name),
                    ).into_pointer_value();

                    ctx.builder.build_store(typed_ptr, right_result.value);
                    Ok(right_result)
                }
                _ => Err("Complex assignments not yet supported".to_string()),
            }
        }
        _ => Err(format!("Compound assignment not yet supported: {:?}", op)),
    }
}

fn compile_conditional<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    condition: &Expr,
    then_branch: &Expr,
    else_branch: &Expr,
) -> Result<ExprResult<'ctx>, String> {
    let cond_result = compile_expr(ctx, condition)?;
    let cond_val = match cond_result.value {
        BasicValueEnum::IntValue(v) => v,
        _ => return Err("Condition must be boolean".to_string()),
    };

    let func = ctx.current_function.ok_or("No current function")?;

    let then_bb = ctx.context.append_basic_block(func, "then");
    let else_bb = ctx.context.append_basic_block(func, "else");
    let merge_bb = ctx.context.append_basic_block(func, "merge");

    ctx.builder.build_conditional_branch(cond_val, then_bb, else_bb);

    // Then branch
    ctx.builder.position_at_end(then_bb);
    let then_result = compile_expr(ctx, then_branch)?;
    ctx.builder.build_unconditional_branch(merge_bb);
    let then_bb_end = ctx.builder.get_insert_block().unwrap();

    // Else branch
    ctx.builder.position_at_end(else_bb);
    let else_result = compile_expr(ctx, else_branch)?;
    ctx.builder.build_unconditional_branch(merge_bb);
    let else_bb_end = ctx.builder.get_insert_block().unwrap();

    // Merge
    ctx.builder.position_at_end(merge_bb);

    let phi_ty = ruyi_type_to_llvm(ctx.context, &then_result.ty);
    let phi = ctx.builder.build_phi(phi_ty, "cond_phi");
    phi.add_incoming(&[(&then_result.value, then_bb_end), (&else_result.value, else_bb_end)]);

    let result_ty = then_result.ty.least_upper_bound(&else_result.ty);
    Ok(ExprResult::new(phi.as_basic_value(), result_ty))
}

pub(super) fn compile_pattern_condition<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    scrutinee: &ExprResult<'ctx>,
    pattern: &Pattern,
    guard: &Option<Box<Expr>>,
) -> Result<ExprResult<'ctx>, String> {
    let pattern_cond = match pattern {
        Pattern::Wildcard => {
            let true_val = ctx.context.bool_type().const_int(1, false);
            ExprResult::new(BasicValueEnum::IntValue(true_val), Type::Bool)
        }
        Pattern::Identifier(name) => {
            let llvm_ty = ruyi_type_to_llvm(ctx.context, &scrutinee.ty);
            let ptr = ctx.builder.build_alloca(llvm_ty, name);
            ctx.builder.build_store(ptr, scrutinee.value);
            ctx.variables.insert(name.clone(), (ptr, scrutinee.ty.clone()));
            let true_val = ctx.context.bool_type().const_int(1, false);
            ExprResult::new(BasicValueEnum::IntValue(true_val), Type::Bool)
        }
        Pattern::Literal(lit_expr) => {
            let lit_result = compile_expr(ctx, lit_expr)?;
            compile_eq(ctx, scrutinee, &lit_result)?
        }
        Pattern::As(inner, alias) => {
            let llvm_ty = ruyi_type_to_llvm(ctx.context, &scrutinee.ty);
            let ptr = ctx.builder.build_alloca(llvm_ty, alias);
            ctx.builder.build_store(ptr, scrutinee.value);
            ctx.variables.insert(alias.clone(), (ptr, scrutinee.ty.clone()));
            compile_pattern_condition(ctx, scrutinee, inner, &None)?
        }
        Pattern::Or(patterns) => {
            let mut cond: Option<ExprResult<'ctx>> = None;
            for pat in patterns {
                let pat_cond = compile_pattern_condition(ctx, scrutinee, pat, &None)?;
                cond = match cond {
                    None => Some(pat_cond),
                    Some(c) => {
                        match (&c.value, &pat_cond.value) {
                            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                                let res = ctx.builder.build_or(*l, *r, "or_pat");
                                Some(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
                            }
                            _ => return Err("Pattern OR must be boolean".to_string()),
                        }
                    }
                };
            }
            cond.ok_or("Empty OR pattern".to_string())?
        }
        _ => {
            return Err(format!("Unsupported pattern in codegen: {:?}", pattern));
        }
    };

    match guard {
        Some(guard_expr) => {
            let guard_result = compile_expr(ctx, guard_expr)?;
            match (&pattern_cond.value, &guard_result.value) {
                (BasicValueEnum::IntValue(p), BasicValueEnum::IntValue(g)) => {
                    let res = ctx.builder.build_and(*p, *g, "match_guard");
                    Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
                }
                _ => Err("Guard must be boolean".to_string()),
            }
        }
        None => Ok(pattern_cond),
    }
}

fn compile_match_expr<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    value: &Expr,
    arms: &[MatchArm],
) -> Result<ExprResult<'ctx>, String> {
    if arms.is_empty() {
        let void_val = ctx.context.i64_type().const_int(0, false);
        return Ok(ExprResult::new(BasicValueEnum::IntValue(void_val), Type::Void));
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

    let mut phi_incoming = Vec::new();
    let mut result_ty = Type::Void;

    for (i, arm) in arms.iter().enumerate() {
        ctx.builder.position_at_end(check_bbs[i]);
        let saved_vars = ctx.variables.clone();
        let cond = compile_pattern_condition(ctx, &scrutinee, &arm.pattern, &arm.guard)?;
        let cond_val = match cond.value {
            BasicValueEnum::IntValue(v) => v,
            _ => return Err("Match condition must be boolean".to_string()),
        };

        let next_bb = if i + 1 < arms.len() {
            check_bbs[i + 1]
        } else {
            merge_bb
        };
        ctx.builder.build_conditional_branch(cond_val, body_bbs[i], next_bb);

        ctx.builder.position_at_end(body_bbs[i]);
        let arm_result = compile_block_expr(ctx, &arm.body)?;
        let body_end = ctx.builder.get_insert_block().unwrap();
        if body_end.get_terminator().is_none() {
            ctx.builder.build_unconditional_branch(merge_bb);
        }
        phi_incoming.push((arm_result.value, body_end));
        ctx.variables = saved_vars;
        if i == 0 {
            result_ty = arm_result.ty;
        } else {
            result_ty = result_ty.least_upper_bound(&arm_result.ty);
        }
    }

    ctx.builder.position_at_end(merge_bb);

    let phi_ty = ruyi_type_to_llvm(ctx.context, &result_ty);
    let phi = ctx.builder.build_phi(phi_ty, "match_phi");
    for (val, bb) in &phi_incoming {
        phi.add_incoming(&[(val, *bb)]);
    }

    Ok(ExprResult::new(phi.as_basic_value(), result_ty))
}

fn compile_block_expr<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    stmts: &[Statement],
) -> Result<ExprResult<'ctx>, String> {
    if stmts.is_empty() {
        let void_val = ctx.context.i64_type().const_int(0, false);
        return Ok(ExprResult::new(BasicValueEnum::IntValue(void_val), Type::Void));
    }

    for (i, stmt) in stmts.iter().enumerate() {
        if i == stmts.len() - 1 {
            match stmt {
                Statement::Expression(expr) => {
                    return compile_expr(ctx, expr);
                }
                _ => {
                    super::stmt::compile_stmt(ctx, stmt)?;
                    let void_val = ctx.context.i64_type().const_int(0, false);
                    return Ok(ExprResult::new(BasicValueEnum::IntValue(void_val), Type::Void));
                }
            }
        } else {
            super::stmt::compile_stmt(ctx, stmt)?;
        }
    }

    unreachable!()
}

fn get_array_elem_type(ty: &Type) -> Type {
    match ty {
        Type::Array(elem) => *elem.clone(),
        _ => Type::Dynamic,
    }
}

fn unbox_value<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    ptr: inkwell::values::PointerValue<'ctx>,
    ty: &Type,
) -> BasicValueEnum<'ctx> {
    let llvm_ty = ruyi_type_to_llvm(ctx.context, ty);
    let typed_ptr = ctx.builder.build_bitcast(
        ptr,
        llvm_ty.ptr_type(Default::default()),
        "unbox_ptr",
    ).into_pointer_value();
    ctx.builder.build_load(typed_ptr, "unbox_val")
}
