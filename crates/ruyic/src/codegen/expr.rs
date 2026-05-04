/**
 * Expression code generation for Ruyi.
 *
 * Lowers Ruyi AST expressions to LLVM IR instructions.
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */
use inkwell::values::{BasicValue, BasicValueEnum};
use inkwell::FloatPredicate;
use inkwell::IntPredicate;

use super::generator::CodegenContext;
use super::types::ruyi_type_to_llvm;
use crate::parser::ast::{BinaryOp, Expr, UnaryOp};
use crate::typechecker::types::Type;

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
        Expr::NullLiteral => compile_null_literal(ctx),
        Expr::Identifier(name) => compile_identifier(ctx, name),
        Expr::SelfExpr => compile_identifier(ctx, "self"),
        Expr::Binary { op, left, right } => compile_binary(ctx, op, left, right),
        Expr::Unary { op, operand } => compile_unary(ctx, op, operand),
        Expr::Call { callee, args } => compile_call(ctx, callee, args),
        Expr::Assignment { left, op, right } => compile_assignment(ctx, left, op, right),
        Expr::Conditional {
            condition,
            then_branch,
            else_branch,
        } => compile_conditional(ctx, condition, then_branch, else_branch),
        Expr::Grouping(inner) => compile_expr(ctx, inner),
        Expr::Await(inner) => super::async_codegen::compile_await(ctx, inner),
        Expr::Function {
            name,
            type_params,
            params,
            return_type,
            body,
            is_async,
        } => {
            if let Some(func_name) = name {
                let decl = crate::parser::ast::Declaration::Function {
                    name: func_name.clone(),
                    type_params: type_params.clone(),
                    params: params.clone(),
                    return_type: return_type.clone(),
                    body: body.clone(),
                    is_async: *is_async,
                };
                super::decl::compile_declaration(ctx, &decl)?;
                if let Some(func) = ctx.module.get_function(func_name) {
                    Ok(ExprResult::new(
                        BasicValueEnum::PointerValue(func.as_global_value().as_pointer_value()),
                        Type::Function {
                            params: vec![],
                            return_type: Box::new(Type::Dynamic),
                        },
                    ))
                } else {
                    Err(format!(
                        "Function not found after compilation: {}",
                        func_name
                    ))
                }
            } else {
                Err("Anonymous functions not yet supported".to_string())
            }
        }
        Expr::ArrayLiteral(elements) => compile_array_literal(ctx, elements),
        Expr::ObjectLiteral(properties) => compile_object_literal(ctx, properties),
        Expr::New { callee, args } => compile_new(ctx, callee, args),
        Expr::Member { object, property, .. } => compile_member_access(ctx, object, property),
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
    Ok(ExprResult::new(
        BasicValueEnum::FloatValue(val),
        Type::Float,
    ))
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

fn compile_null_literal<'ctx>(ctx: &CodegenContext<'ctx, '_>) -> Result<ExprResult<'ctx>, String> {
    let null_ptr = ctx
        .context
        .i8_type()
        .ptr_type(Default::default())
        .const_null();
    Ok(ExprResult::new(
        BasicValueEnum::PointerValue(null_ptr),
        Type::Null,
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

fn compile_member_access<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    object: &Expr,
    property: &crate::parser::ast::MemberProperty,
) -> Result<ExprResult<'ctx>, String> {
    let field_name = match property {
        crate::parser::ast::MemberProperty::Ident(n) => n.clone(),
        _ => return Err("Only simple field access supported".to_string()),
    };

    let (var_ptr, class_name, field_ty) = match object {
        Expr::Identifier(name) => {
            let (ptr, ty) = ctx
                .variables
                .get(name)
                .ok_or_else(|| format!("Undefined variable: {}", name))?;
            let class_name = match ty {
                Type::Named(n) => n.clone(),
                _ => return Err(format!("Cannot access field on type: {:?}", ty)),
            };
            let fields = ctx
                .class_fields
                .get(&class_name)
                .ok_or_else(|| format!("Unknown class: {}", class_name))?;
            let field_ty = fields
                .iter()
                .find(|(n, _)| n == &field_name)
                .map(|(_, ty)| ty.clone())
                .ok_or_else(|| format!("Unknown field: {} in class {}", field_name, class_name))?;
            (*ptr, class_name, field_ty)
        }
        Expr::SelfExpr => {
            let (ptr, ty) = ctx
                .variables
                .get("self")
                .ok_or_else(|| "self not in scope".to_string())?;
            let class_name = match ty {
                Type::Named(n) => n.clone(),
                _ => return Err(format!("Cannot access field on type: {:?}", ty)),
            };
            let fields = ctx
                .class_fields
                .get(&class_name)
                .ok_or_else(|| format!("Unknown class: {}", class_name))?;
            let field_ty = fields
                .iter()
                .find(|(n, _)| n == &field_name)
                .map(|(_, ty)| ty.clone())
                .ok_or_else(|| format!("Unknown field: {} in class {}", field_name, class_name))?;
            (*ptr, class_name, field_ty)
        }
        _ => return Err("Member access only supported on identifiers".to_string()),
    };

    let obj_ptr = ctx.builder.build_load(var_ptr, "obj").into_pointer_value();

    let struct_type = ctx
        .class_struct_types
        .get(&class_name)
        .ok_or_else(|| format!("No struct type for class: {}", class_name))?;

    let struct_ptr = unsafe {
        ctx.builder.build_pointer_cast(
            obj_ptr,
            struct_type.ptr_type(Default::default()),
            &format!("{}_cast", class_name),
        )
    };

    let fields = ctx.class_fields.get(&class_name).unwrap();
    let field_index = fields.iter().position(|(n, _)| n == &field_name).unwrap();

    let i32_ty = ctx.context.i32_type();
    let field_ptr = unsafe {
        ctx.builder.build_gep(
            struct_ptr,
            &[
                i32_ty.const_int(0, false),
                i32_ty.const_int(field_index as u64, false),
            ],
            &format!("{}_ptr", field_name),
        )
    };

    let llvm_ty = ruyi_type_to_llvm(ctx.context, &field_ty);
    let value = unsafe { ctx.builder.build_load(field_ptr, &field_name) };
    Ok(ExprResult::new(value, field_ty))
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
        BinaryOp::StrictNotEquals | BinaryOp::NotEquals => {
            compile_ne(ctx, &left_result, &right_result)
        }
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
    if left.ty == Type::String && right.ty == Type::String {
        let l = left.value.into_pointer_value();
        let r = right.value.into_pointer_value();
        let concat_fn = ctx
            .module
            .get_function("ruyi_str_concat")
            .expect("ruyi_str_concat not declared");
        let res = ctx
            .builder
            .build_call(concat_fn, &[l.into(), r.into()], "str_concat")
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();
        return Ok(ExprResult::new(
            BasicValueEnum::PointerValue(res),
            Type::String,
        ));
    }
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder.build_int_add(*l, *r, "add");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
            let res = ctx.builder.build_float_add(*l, *r, "fadd");
            Ok(ExprResult::new(
                BasicValueEnum::FloatValue(res),
                Type::Float,
            ))
        }
        (BasicValueEnum::IntValue(l), BasicValueEnum::FloatValue(r)) => {
            let l_f = ctx
                .builder
                .build_signed_int_to_float(*l, ctx.context.f64_type(), "itof");
            let res = ctx.builder.build_float_add(l_f, *r, "fadd");
            Ok(ExprResult::new(
                BasicValueEnum::FloatValue(res),
                Type::Float,
            ))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::IntValue(r)) => {
            let r_f = ctx
                .builder
                .build_signed_int_to_float(*r, ctx.context.f64_type(), "itof");
            let res = ctx.builder.build_float_add(*l, r_f, "fadd");
            Ok(ExprResult::new(
                BasicValueEnum::FloatValue(res),
                Type::Float,
            ))
        }
        _ => Err("Invalid operands for +".to_string()),
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
            Ok(ExprResult::new(
                BasicValueEnum::FloatValue(res),
                Type::Float,
            ))
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
            Ok(ExprResult::new(
                BasicValueEnum::FloatValue(res),
                Type::Float,
            ))
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
            Ok(ExprResult::new(
                BasicValueEnum::FloatValue(res),
                Type::Float,
            ))
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

fn compile_eq<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx
                .builder
                .build_int_compare(IntPredicate::EQ, *l, *r, "eq");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
            let res = ctx
                .builder
                .build_float_compare(FloatPredicate::OEQ, *l, *r, "feq");
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
            let res = ctx
                .builder
                .build_int_compare(IntPredicate::NE, *l, *r, "ne");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
            let res = ctx
                .builder
                .build_float_compare(FloatPredicate::ONE, *l, *r, "fne");
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
            let res = ctx
                .builder
                .build_int_compare(IntPredicate::SLT, *l, *r, "lt");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
            let res = ctx
                .builder
                .build_float_compare(FloatPredicate::OLT, *l, *r, "flt");
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
            let res = ctx
                .builder
                .build_int_compare(IntPredicate::SGT, *l, *r, "gt");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
            let res = ctx
                .builder
                .build_float_compare(FloatPredicate::OGT, *l, *r, "fgt");
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
            let res = ctx
                .builder
                .build_int_compare(IntPredicate::SLE, *l, *r, "le");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
            let res = ctx
                .builder
                .build_float_compare(FloatPredicate::OLE, *l, *r, "fle");
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
            let res = ctx
                .builder
                .build_int_compare(IntPredicate::SGE, *l, *r, "ge");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
            let res = ctx
                .builder
                .build_float_compare(FloatPredicate::OGE, *l, *r, "fge");
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
                Ok(ExprResult::new(
                    BasicValueEnum::FloatValue(res),
                    Type::Float,
                ))
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
        UnaryOp::Await => super::async_codegen::compile_await(ctx, operand),
        _ => Err(format!("Unsupported unary operator: {:?}", op)),
    }
}

fn compile_call<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    callee: &Expr,
    args: &[crate::parser::ast::Argument],
) -> Result<ExprResult<'ctx>, String> {
    let (name, self_arg) = match callee {
        Expr::Identifier(n) => (n.clone(), None),
        Expr::Member { object, property, .. } => {
            let method_name = match property {
                crate::parser::ast::MemberProperty::Ident(n) => n.clone(),
                _ => return Err("Only simple method calls supported".to_string()),
            };
            if method_name == "new" {
                let class_name = match object.as_ref() {
                    Expr::Identifier(n) => n.clone(),
                    _ => return Err("new() must be called on a class name".to_string()),
                };
                return compile_new(ctx, object.as_ref(), args);
            }
            let (obj_ptr, class_name) = match object.as_ref() {
                Expr::Identifier(var_name) => {
                    let (ptr, ty) = ctx
                        .variables
                        .get(var_name)
                        .ok_or_else(|| format!("Undefined variable: {}", var_name))?;
                    let class_name = match ty {
                        Type::Named(n) => n.clone(),
                        _ => return Err(format!("Cannot call method on type: {:?}", ty)),
                    };
                    (*ptr, class_name)
                }
                Expr::SelfExpr => {
                    let (ptr, ty) = ctx
                        .variables
                        .get("self")
                        .ok_or_else(|| "self not in scope".to_string())?;
                    let class_name = match ty {
                        Type::Named(n) => n.clone(),
                        _ => return Err(format!("Cannot call method on type: {:?}", ty)),
                    };
                    (*ptr, class_name)
                }
                _ => return Err("Method calls only supported on identifiers".to_string()),
            };
            let func_name = format!("{}_{}", class_name, method_name);
            (func_name, Some(obj_ptr))
        }
        _ => return Err("Indirect calls not yet supported".to_string()),
    };

    if name == "print" {
        if args.len() == 1 {
            match &args[0] {
                crate::parser::ast::Argument::Expr(e) => {
                    let result = compile_expr(ctx, e)?;
                    let func = ctx
                        .current_function
                        .ok_or("print requires a function context")?;
                    super::builtins::build_print(
                        &ctx.context,
                        &ctx.builder,
                        &ctx.module,
                        result.value,
                        &result.ty,
                        func,
                    );
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

    if name == "spawn" {
        if args.len() == 1 {
            match &args[0] {
                crate::parser::ast::Argument::Expr(e) => {
                    let result = compile_expr(ctx, e)?;
                    let future_ptr = result.value.into_pointer_value();
                    let task_handle =
                        super::builtins::build_ruyi_spawn(&ctx.builder, &ctx.module, future_ptr);
                    return Ok(ExprResult::new(
                        BasicValueEnum::PointerValue(task_handle),
                        Type::Dynamic,
                    ));
                }
                _ => return Err("Invalid spawn argument".to_string()),
            }
        } else {
            return Err("spawn expects exactly 1 argument".to_string());
        }
    }

    let func = ctx
        .module
        .get_function(&name)
        .ok_or_else(|| format!("Function not found: {}", name))?;

    let mut arg_values = Vec::new();
    if let Some(self_ptr) = self_arg {
        let obj_ptr = ctx.builder.build_load(self_ptr, "obj").into_pointer_value();
        arg_values.push(obj_ptr.into());
    }
    for arg in args {
        match arg {
            crate::parser::ast::Argument::Expr(e) => {
                let result = compile_expr(ctx, e)?;
                arg_values.push(result.value.into());
            }
            _ => return Err("Spread arguments not yet supported".to_string()),
        }
    }

    let call_site = ctx.builder.build_call(func, &arg_values, "call");
    let value = call_site.try_as_basic_value().left();

    let is_async = ctx.module.get_function(&format!("{}$poll", name)).is_some();

    let ret_ty = if is_async {
        Type::Future(Box::new(Type::Int))
    } else {
        Type::Dynamic
    };

    match value {
        Some(v) => Ok(ExprResult::new(v, ret_ty)),
        None => Ok(ExprResult::new(
            BasicValueEnum::IntValue(ctx.context.i64_type().const_int(0, false)),
            Type::Void,
        )),
    }
}

fn compile_assignment<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    left: &Expr,
    op: &crate::parser::ast::AssignOp,
    right: &Expr,
) -> Result<ExprResult<'ctx>, String> {
    let right_result = compile_expr(ctx, right)?;

    match op {
        crate::parser::ast::AssignOp::Assign => {}
        _ => return Err(format!("Compound assignment not yet supported: {:?}", op)),
    }

    match left {
        Expr::Identifier(name) => {
            if let Some((ptr, _)) = ctx.variables.get(name) {
                ctx.builder.build_store(*ptr, right_result.value);
                Ok(right_result)
            } else {
                Err(format!("Undefined variable: {}", name))
            }
        }
        Expr::Member { object, property, .. } => {
            let field_name = match property {
                crate::parser::ast::MemberProperty::Ident(n) => n.clone(),
                _ => return Err("Only simple field assignments supported".to_string()),
            };

            let (var_ptr, class_name) = match object.as_ref() {
                Expr::Identifier(name) => {
                    let (ptr, ty) = ctx
                        .variables
                        .get(name)
                        .ok_or_else(|| format!("Undefined variable: {}", name))?;
                    let class_name = match ty {
                        Type::Named(n) => n.clone(),
                        _ => return Err(format!("Cannot access field on type: {:?}", ty)),
                    };
                    (*ptr, class_name)
                }
                Expr::SelfExpr => {
                    let (ptr, ty) = ctx
                        .variables
                        .get("self")
                        .ok_or_else(|| "self not in scope".to_string())?;
                    let class_name = match ty {
                        Type::Named(n) => n.clone(),
                        _ => return Err(format!("Cannot access field on type: {:?}", ty)),
                    };
                    (*ptr, class_name)
                }
                _ => return Err("Member assignment only supported on identifiers".to_string()),
            };

            let obj_ptr = ctx.builder.build_load(var_ptr, "obj").into_pointer_value();

            let struct_type = ctx
                .class_struct_types
                .get(&class_name)
                .ok_or_else(|| format!("No struct type for class: {}", class_name))?;

            let struct_ptr = unsafe {
                ctx.builder.build_pointer_cast(
                    obj_ptr,
                    struct_type.ptr_type(Default::default()),
                    &format!("{}_cast", class_name),
                )
            };

            let fields = ctx
                .class_fields
                .get(&class_name)
                .ok_or_else(|| format!("Unknown class: {}", class_name))?;

            let field_index = fields
                .iter()
                .position(|(n, _)| n == &field_name)
                .ok_or_else(|| format!("Unknown field: {} in class {}", field_name, class_name))?;

            let i32_ty = ctx.context.i32_type();
            let field_ptr = unsafe {
                ctx.builder.build_gep(
                    struct_ptr,
                    &[
                        i32_ty.const_int(0, false),
                        i32_ty.const_int(field_index as u64, false),
                    ],
                    &format!("{}_ptr", field_name),
                )
            };

            ctx.builder.build_store(field_ptr, right_result.value);
            Ok(right_result)
        }
        _ => Err("Complex assignments not yet supported".to_string()),
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

    ctx.builder
        .build_conditional_branch(cond_val, then_bb, else_bb);

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
    phi.add_incoming(&[
        (&then_result.value, then_bb_end),
        (&else_result.value, else_bb_end),
    ]);

    let result_ty = then_result.ty.least_upper_bound(&else_result.ty);
    Ok(ExprResult::new(phi.as_basic_value(), result_ty))
}

fn compile_array_literal<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    elements: &[crate::parser::ast::ArrayElement],
) -> Result<ExprResult<'ctx>, String> {
    let len = elements.len() as u64;
    let total_size = ctx.context.i64_type().const_int(len * 8 + 8, false);
    let ptr = super::builtins::build_gc_alloc(&ctx.builder, &ctx.module, total_size);

    let len_ptr = ctx
        .builder
        .build_bitcast(
            ptr,
            ctx.context.i64_type().ptr_type(Default::default()),
            "len_ptr",
        )
        .into_pointer_value();
    ctx.builder
        .build_store(len_ptr, ctx.context.i64_type().const_int(len, false));

    for (i, elem) in elements.iter().enumerate() {
        match elem {
            crate::parser::ast::ArrayElement::Expr(e) => {
                let val = compile_expr(ctx, e)?;
                let offset = ctx.context.i32_type().const_int((8 + i * 8) as u64, false);
                let elem_ptr = unsafe { ctx.builder.build_gep(ptr, &[offset], "elem_ptr") };
                let i64_ptr = ctx
                    .builder
                    .build_bitcast(
                        elem_ptr,
                        ctx.context.i64_type().ptr_type(Default::default()),
                        "elem_i64_ptr",
                    )
                    .into_pointer_value();

                let stored_val = match val.value {
                    BasicValueEnum::IntValue(v) => v.as_basic_value_enum(),
                    BasicValueEnum::FloatValue(v) => ctx
                        .builder
                        .build_bitcast(v, ctx.context.i64_type(), "f_to_i")
                        .as_basic_value_enum(),
                    BasicValueEnum::PointerValue(v) => v.as_basic_value_enum(),
                    _ => val.value,
                };
                ctx.builder.build_store(i64_ptr, stored_val);

                if super::builtins::is_gc_managed(&val.ty) {
                    if let BasicValueEnum::PointerValue(pv) = val.value {
                        super::builtins::build_gc_write_barrier(&ctx.builder, &ctx.module, ptr, pv);
                    }
                }
            }
            _ => return Err("Unsupported array element".to_string()),
        }
    }

    Ok(ExprResult::new(
        BasicValueEnum::PointerValue(ptr),
        Type::Array(Box::new(Type::Dynamic)),
    ))
}

fn compile_object_literal<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    properties: &[crate::parser::ast::ObjectProperty],
) -> Result<ExprResult<'ctx>, String> {
    let len = properties.len() as u64;
    let total_size = ctx.context.i64_type().const_int(len * 8, false);
    let ptr = super::builtins::build_gc_alloc(&ctx.builder, &ctx.module, total_size);

    for (i, prop) in properties.iter().enumerate() {
        match prop {
            crate::parser::ast::ObjectProperty::Property { value, .. } => {
                let val = compile_expr(ctx, value)?;
                let offset = ctx.context.i32_type().const_int((i * 8) as u64, false);
                let field_ptr = unsafe { ctx.builder.build_gep(ptr, &[offset], "field_ptr") };
                let i64_ptr = ctx
                    .builder
                    .build_bitcast(
                        field_ptr,
                        ctx.context.i64_type().ptr_type(Default::default()),
                        "field_i64_ptr",
                    )
                    .into_pointer_value();

                let stored_val = match val.value {
                    BasicValueEnum::IntValue(v) => v.as_basic_value_enum(),
                    BasicValueEnum::FloatValue(v) => ctx
                        .builder
                        .build_bitcast(v, ctx.context.i64_type(), "f_to_i")
                        .as_basic_value_enum(),
                    BasicValueEnum::PointerValue(v) => v.as_basic_value_enum(),
                    _ => val.value,
                };
                ctx.builder.build_store(i64_ptr, stored_val);

                if super::builtins::is_gc_managed(&val.ty) {
                    if let BasicValueEnum::PointerValue(pv) = val.value {
                        super::builtins::build_gc_write_barrier(&ctx.builder, &ctx.module, ptr, pv);
                    }
                }
            }
            _ => return Err("Unsupported object property".to_string()),
        }
    }

    Ok(ExprResult::new(
        BasicValueEnum::PointerValue(ptr),
        Type::Object(vec![]),
    ))
}

fn compile_new<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    callee: &crate::parser::ast::Expr,
    args: &[crate::parser::ast::Argument],
) -> Result<ExprResult<'ctx>, String> {
    let class_name = match callee {
        crate::parser::ast::Expr::Identifier(name) => name.clone(),
        _ => return Err("Complex new expressions not yet supported".to_string()),
    };

    let total_size = ctx.context.i64_type().const_int(64, false);
    let ptr = super::builtins::build_gc_alloc(&ctx.builder, &ctx.module, total_size);

    let ctor_name = format!("{}_new", class_name);
    if let Some(ctor) = ctx.module.get_function(&ctor_name) {
        let mut arg_values = vec![ptr.into()];
        for arg in args {
            match arg {
                crate::parser::ast::Argument::Expr(e) => {
                    let result = compile_expr(ctx, e)?;
                    arg_values.push(result.value.into());
                }
                _ => return Err("Spread arguments not yet supported".to_string()),
            }
        }
        ctx.builder.build_call(ctor, &arg_values, "ctor_call");
    }

    Ok(ExprResult::new(
        BasicValueEnum::PointerValue(ptr),
        Type::Named(class_name),
    ))
}
