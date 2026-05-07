use inkwell::types::BasicType;
/**
 * Expression code generation for Ruyi.
 *
 * Lowers Ruyi AST expressions to LLVM IR instructions.
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */
use inkwell::values::{BasicMetadataValueEnum, BasicValue, BasicValueEnum};
use inkwell::FloatPredicate;
use inkwell::IntPredicate;

use super::generator::CodegenContext;
use super::types::{function_type_from_ruyi, ruyi_type_to_llvm};
use crate::parser::ast::{
    ArrowBody, BinaryOp, Expr, Param, Pattern, Statement, TemplatePart, UnaryOp,
};
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

/// Infer the type of an expression for arrow-function return-type deduction.
/// Only handles the common literal/identifier/binary cases needed for codegen.
fn infer_expr_type(expr: &Expr, param_types: &std::collections::HashMap<String, Type>) -> Type {
    match expr {
        Expr::IntLiteral(_) => Type::Int,
        Expr::FloatLiteral(_) => Type::Float,
        Expr::BooleanLiteral(_) => Type::Bool,
        Expr::StringLiteral(_) => Type::String,
        Expr::NullLiteral => Type::Null,
        Expr::BigIntLiteral(_) => Type::BigInt,
        Expr::Identifier(name) => param_types.get(name).cloned().unwrap_or(Type::Dynamic),
        Expr::Binary { op, left, right } => {
            let left_ty = infer_expr_type(left, param_types);
            let right_ty = infer_expr_type(right, param_types);
            match op {
                BinaryOp::Plus => {
                    if left_ty == Type::String || right_ty == Type::String {
                        Type::String
                    } else if left_ty == Type::Float || right_ty == Type::Float {
                        Type::Float
                    } else {
                        Type::Int
                    }
                }
                BinaryOp::Minus
                | BinaryOp::Star
                | BinaryOp::Slash
                | BinaryOp::Percent
                | BinaryOp::Power => {
                    if left_ty == Type::Float || right_ty == Type::Float {
                        Type::Float
                    } else {
                        Type::Int
                    }
                }
                BinaryOp::StrictEquals
                | BinaryOp::StrictNotEquals
                | BinaryOp::Less
                | BinaryOp::Greater
                | BinaryOp::LessEq
                | BinaryOp::GreaterEq
                | BinaryOp::And
                | BinaryOp::Or => Type::Bool,
                _ => Type::Dynamic,
            }
        }
        Expr::Unary { op, operand } => match op {
            UnaryOp::Minus => infer_expr_type(operand, param_types),
            UnaryOp::Not => Type::Bool,
            UnaryOp::Tilde => Type::Int,
            _ => Type::Dynamic,
        },
        Expr::Conditional {
            then_branch,
            else_branch,
            ..
        } => {
            let then_ty = infer_expr_type(then_branch, param_types);
            let else_ty = infer_expr_type(else_branch, param_types);
            then_ty.least_upper_bound(&else_ty)
        }
        Expr::Grouping(inner) => infer_expr_type(inner, param_types),
        _ => Type::Dynamic,
    }
}

/// Infer parameter types for an arrow function from its body.
fn infer_arrow_param_types(params: &[Param], body: &ArrowBody) -> Vec<Type> {
    let mut param_map: std::collections::HashMap<String, Type> = std::collections::HashMap::new();
    for param in params {
        if let Pattern::Identifier(name) = &param.pattern {
            param_map.insert(name.clone(), Type::Dynamic);
        }
    }

    match body {
        ArrowBody::Expr(expr) => infer_param_types_expr(expr, &mut param_map),
        ArrowBody::Block(stmts) => {
            for stmt in stmts {
                infer_param_types_stmt(stmt, &mut param_map);
            }
        }
    }

    params
        .iter()
        .map(|p| {
            p.ty.as_ref().map(Type::from_annotation).unwrap_or_else(|| {
                if let Pattern::Identifier(name) = &p.pattern {
                    param_map.get(name).cloned().unwrap_or(Type::Dynamic)
                } else {
                    Type::Dynamic
                }
            })
        })
        .collect()
}

fn infer_param_types_expr(expr: &Expr, param_map: &mut std::collections::HashMap<String, Type>) {
    match expr {
        Expr::Binary { op, left, right } => {
            if matches!(
                op,
                BinaryOp::Plus
                    | BinaryOp::Minus
                    | BinaryOp::Star
                    | BinaryOp::Slash
                    | BinaryOp::Percent
            ) {
                if is_int_literal(left) && is_param_identifier(right) {
                    if let Some(name) = get_identifier_name(right) {
                        param_map.insert(name, Type::Int);
                    }
                }
                if is_int_literal(right) && is_param_identifier(left) {
                    if let Some(name) = get_identifier_name(left) {
                        param_map.insert(name, Type::Int);
                    }
                }
                if is_float_literal(left) && is_param_identifier(right) {
                    if let Some(name) = get_identifier_name(right) {
                        param_map.insert(name, Type::Float);
                    }
                }
                if is_float_literal(right) && is_param_identifier(left) {
                    if let Some(name) = get_identifier_name(left) {
                        param_map.insert(name, Type::Float);
                    }
                }
                if is_param_identifier(left) && is_param_identifier(right) {
                    if let (Some(lname), Some(rname)) =
                        (get_identifier_name(left), get_identifier_name(right))
                    {
                        if lname == rname {
                            if matches!(
                                op,
                                BinaryOp::Star
                                    | BinaryOp::Minus
                                    | BinaryOp::Slash
                                    | BinaryOp::Percent
                            ) {
                                param_map.insert(lname, Type::Int);
                            }
                        }
                    }
                }
            }
            if *op == BinaryOp::Plus {
                if is_string_literal(left) && is_param_identifier(right) {
                    if let Some(name) = get_identifier_name(right) {
                        param_map.insert(name, Type::String);
                    }
                }
                if is_string_literal(right) && is_param_identifier(left) {
                    if let Some(name) = get_identifier_name(left) {
                        param_map.insert(name, Type::String);
                    }
                }
            }
            infer_param_types_expr(left, param_map);
            infer_param_types_expr(right, param_map);
        }
        Expr::Call { callee, args } => {
            if let Expr::Identifier(callee_name) = callee.as_ref() {
                if callee_name == "print" {
                    for arg in args {
                        if let crate::parser::ast::Argument::Expr(arg_expr) = arg {
                            if is_param_identifier(arg_expr) {
                                if let Some(name) = get_identifier_name(arg_expr) {
                                    if param_map.get(&name) == Some(&Type::Dynamic) {
                                        param_map.insert(name, Type::String);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            for arg in args {
                if let crate::parser::ast::Argument::Expr(e) = arg {
                    infer_param_types_expr(e, param_map);
                }
            }
        }
        Expr::Unary { operand, .. } => infer_param_types_expr(operand, param_map),
        Expr::Conditional {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            infer_param_types_expr(condition, param_map);
            infer_param_types_expr(then_branch, param_map);
            infer_param_types_expr(else_branch, param_map);
        }
        Expr::Grouping(inner) => infer_param_types_expr(inner, param_map),
        _ => {}
    }
}

fn infer_param_types_stmt(
    stmt: &Statement,
    param_map: &mut std::collections::HashMap<String, Type>,
) {
    match stmt {
        Statement::Expression(expr) => infer_param_types_expr(expr, param_map),
        Statement::Return(Some(expr)) => infer_param_types_expr(expr, param_map),
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            infer_param_types_expr(condition, param_map);
            infer_param_types_stmt(then_branch, param_map);
            if let Some(else_b) = else_branch {
                infer_param_types_stmt(else_b, param_map);
            }
        }
        Statement::Block(stmts) => {
            for s in stmts {
                infer_param_types_stmt(s, param_map);
            }
        }
        Statement::For {
            body, init, update, ..
        } => {
            if let Some(crate::parser::ast::ForInit::Expr(init_expr)) = init {
                infer_param_types_expr(init_expr, param_map);
            }
            if let Some(update_expr) = update {
                infer_param_types_expr(update_expr, param_map);
            }
            infer_param_types_stmt(body, param_map);
        }
        Statement::ForOf { body, .. } => {
            infer_param_types_stmt(body, param_map);
        }
        Statement::While { condition, body } => {
            infer_param_types_expr(condition, param_map);
            infer_param_types_stmt(body, param_map);
        }
        _ => {}
    }
}

fn is_int_literal(expr: &Expr) -> bool {
    matches!(expr, Expr::IntLiteral(_))
}

fn is_float_literal(expr: &Expr) -> bool {
    matches!(expr, Expr::FloatLiteral(_))
}

fn is_string_literal(expr: &Expr) -> bool {
    matches!(expr, Expr::StringLiteral(_))
}

fn is_param_identifier(expr: &Expr) -> bool {
    matches!(expr, Expr::Identifier(_))
}

fn get_identifier_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Identifier(name) => Some(name.clone()),
        _ => None,
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
        Expr::BigIntLiteral(n) => compile_bigint_literal(ctx, n),
        Expr::TemplateLiteral(parts) => compile_template_literal(ctx, parts),
        Expr::Identifier(name) => {
            // Handle enum variant constructors used as values: None, Err
            if name == "None" || name == "Err" {
                compile_enum_variant(ctx, name, &[], 0)
            } else {
                compile_identifier(ctx, name)
            }
        }
        Expr::SelfExpr => compile_identifier(ctx, "self"),
        Expr::Binary { op, left, right } => compile_binary(ctx, op, left, right),
        Expr::Unary { op, operand } => compile_unary(ctx, op, operand),
        Expr::NullAssert(expr) => compile_expr(ctx, expr),
        Expr::Call { callee, args } => compile_call(ctx, callee, args),
        Expr::Assignment { left, op, right } => compile_assignment(ctx, left, op, right),
        Expr::Conditional {
            condition,
            then_branch,
            else_branch,
        } => compile_conditional(ctx, condition, then_branch, else_branch),
        Expr::If {
            condition,
            then_branch,
            else_branch,
        } => compile_if_expr(ctx, condition, then_branch, else_branch.as_deref()),
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
        Expr::ArrowFunction {
            params,
            return_type,
            body,
            is_async,
        } => {
            if *is_async {
                return Err("Async arrow functions not yet supported".to_string());
            }
            let name = format!("__arrow_{}", ctx.arrow_counter);
            ctx.arrow_counter += 1;

            let body_stmts = match body {
                ArrowBody::Expr(expr) => {
                    vec![Statement::Return(Some(expr.clone()))]
                }
                ArrowBody::Block(stmts) => stmts.clone(),
            };

            let inferred_param_types = infer_arrow_param_types(params, body);

            // Infer return type from body when not annotated
            let inferred_ret_type = return_type
                .as_ref()
                .map(Type::from_annotation)
                .unwrap_or_else(|| {
                    let mut param_types_map = std::collections::HashMap::new();
                    for (param, ty) in params.iter().zip(inferred_param_types.iter()) {
                        if let crate::parser::ast::Pattern::Identifier(param_name) = &param.pattern
                        {
                            param_types_map.insert(param_name.clone(), ty.clone());
                        }
                    }
                    match body {
                        ArrowBody::Expr(expr) => infer_expr_type(expr, &param_types_map),
                        ArrowBody::Block(_) => Type::Void,
                    }
                });

            super::decl::compile_function(
                ctx,
                &name,
                params,
                return_type.as_ref(),
                Some(&inferred_param_types),
                Some(&inferred_ret_type),
                &body_stmts,
            )?;

            if let Some(func) = ctx.module.get_function(&name) {
                Ok(ExprResult::new(
                    BasicValueEnum::PointerValue(func.as_global_value().as_pointer_value()),
                    Type::Function {
                        params: inferred_param_types,
                        return_type: Box::new(inferred_ret_type),
                    },
                ))
            } else {
                Err(format!(
                    "Arrow function not found after compilation: {}",
                    name
                ))
            }
        }
        Expr::ArrayLiteral(elements) => compile_array_literal(ctx, elements),
        Expr::ObjectLiteral(properties) => compile_object_literal(ctx, properties),
        Expr::New { callee, args } => compile_new(ctx, callee, args),
        Expr::Member {
            object,
            property,
            optional,
        } => compile_member_access(ctx, object, property, *optional),
        Expr::Match { value, arms } => compile_match_expr(ctx, value, arms),
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

fn compile_template_literal<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    parts: &[TemplatePart],
) -> Result<ExprResult<'ctx>, String> {
    if parts.is_empty() {
        return compile_string_literal(ctx, "");
    }

    if parts.len() == 1 {
        match &parts[0] {
            TemplatePart::String(s) => return compile_string_literal(ctx, s),
            TemplatePart::Expr(_) => {}
        }
    }

    let concat_fn = ctx
        .module
        .get_function("ruyi_str_concat")
        .expect("ruyi_str_concat not declared");

    let mut result_ptr: Option<inkwell::values::PointerValue<'ctx>> = None;

    for part in parts {
        match part {
            TemplatePart::String(s) => {
                let global = ctx.builder.build_global_string_ptr(s, "tmpl_str");
                let str_ptr = global.as_pointer_value();
                match result_ptr {
                    None => result_ptr = Some(str_ptr),
                    Some(prev) => {
                        let res = ctx
                            .builder
                            .build_call(concat_fn, &[prev.into(), str_ptr.into()], "str_concat")
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();
                        result_ptr = Some(res);
                    }
                }
            }
            TemplatePart::Expr(expr) => {
                let expr_result = compile_expr(ctx, expr)?;
                let expr_ptr = value_to_i8_ptr(ctx, &expr_result.value)?;
                match result_ptr {
                    None => result_ptr = Some(expr_ptr),
                    Some(prev) => {
                        let res = ctx
                            .builder
                            .build_call(concat_fn, &[prev.into(), expr_ptr.into()], "str_concat")
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();
                        result_ptr = Some(res);
                    }
                }
            }
        }
    }

    Ok(ExprResult::new(
        BasicValueEnum::PointerValue(result_ptr.unwrap()),
        Type::String,
    ))
}

fn compile_null_literal<'ctx>(ctx: &CodegenContext<'ctx, '_>) -> Result<ExprResult<'ctx>, String> {
    let is_nullable_int = matches!(ctx.expected_expr_type, Some(Type::Nullable(ref inner)) if **inner == Type::Int)
        || matches!(ctx.current_return_type, Some(Type::Nullable(ref inner)) if **inner == Type::Int);
    if is_nullable_int {
        let sentinel = ctx.context.i64_type().const_all_ones();
        Ok(ExprResult::new(
            BasicValueEnum::IntValue(sentinel),
            Type::Nullable(Box::new(Type::Int)),
        ))
    } else {
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
}

fn compile_bigint_literal<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    n: &str,
) -> Result<ExprResult<'ctx>, String> {
    let global = ctx.builder.build_global_string_ptr(n, "bigint_lit");
    let str_ptr = global.as_pointer_value();
    let bigint_ptr =
        super::builtins::build_ruyi_bigint_from_str(&ctx.builder, &ctx.module, str_ptr)?;
    Ok(ExprResult::new(
        BasicValueEnum::PointerValue(bigint_ptr),
        Type::BigInt,
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
    optional: bool,
) -> Result<ExprResult<'ctx>, String> {
    match property {
        crate::parser::ast::MemberProperty::Expr(key_expr) => {
            let obj_result = compile_expr(ctx, object)?;
            let key_result = compile_expr(ctx, key_expr)?;
            let obj_ptr = value_to_i8_ptr(ctx, &obj_result.value)?;
            let key_ptr = value_to_i8_ptr(ctx, &key_result.value)?;
            let result =
                super::builtins::build_ruyi_obj_get(&ctx.builder, &ctx.module, obj_ptr, key_ptr);
            Ok(ExprResult::new(
                BasicValueEnum::PointerValue(result),
                Type::Dynamic,
            ))
        }
        crate::parser::ast::MemberProperty::Ident(field_name) => {
            if optional {
                compile_optional_member_access(ctx, object, field_name)
            } else {
                compile_simple_member_access(ctx, object, field_name)
            }
        }
    }
}

fn value_to_i8_ptr<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
    value: &BasicValueEnum<'ctx>,
) -> Result<inkwell::values::PointerValue<'ctx>, String> {
    let i8_ptr_ty = ctx.context.i8_type().ptr_type(Default::default());
    match value {
        BasicValueEnum::PointerValue(p) => {
            Ok(ctx.builder.build_pointer_cast(*p, i8_ptr_ty, "cast_i8_ptr"))
        }
        BasicValueEnum::IntValue(v) => {
            Ok(ctx.builder.build_int_to_ptr(*v, i8_ptr_ty, "int_to_ptr"))
        }
        _ => Err("Cannot convert value to i8* for runtime call".to_string()),
    }
}

fn compile_simple_member_access<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    object: &Expr,
    field_name: &str,
) -> Result<ExprResult<'ctx>, String> {
    let (var_ptr, field_ty, field_index) = match object {
        Expr::Identifier(name) => {
            let (ptr, ty) = ctx
                .variables
                .get(name)
                .ok_or_else(|| format!("Undefined variable: {}", name))?;
            match ty {
                Type::Named(class_name) => {
                    let class_name = class_name.clone();
                    let fields = ctx
                        .class_fields
                        .get(&class_name)
                        .ok_or_else(|| format!("Unknown class: {}", class_name))?;
                    let field_ty = fields
                        .iter()
                        .find(|(n, _)| n == field_name)
                        .map(|(_, ty)| ty.clone())
                        .ok_or_else(|| {
                            format!("Unknown field: {} in class {}", field_name, class_name)
                        })?;
                    let field_index = fields.iter().position(|(n, _)| n == field_name).unwrap();
                    (*ptr, field_ty, field_index)
                }
                Type::Array(_) => {
                    let class_name = "Array".to_string();
                    let fields = ctx
                        .class_fields
                        .get(&class_name)
                        .ok_or_else(|| format!("Unknown class: {}", class_name))?;
                    let field_ty = fields
                        .iter()
                        .find(|(n, _)| n == field_name)
                        .map(|(_, ty)| ty.clone())
                        .ok_or_else(|| {
                            format!("Unknown field: {} in class {}", field_name, class_name)
                        })?;
                    let field_index = fields.iter().position(|(n, _)| n == field_name).unwrap();
                    (*ptr, field_ty, field_index)
                }
                Type::Generic { base, .. } => {
                    let class_name = base.clone();
                    let fields = ctx
                        .class_fields
                        .get(&class_name)
                        .ok_or_else(|| format!("Unknown class: {}", class_name))?;
                    let field_ty = fields
                        .iter()
                        .find(|(n, _)| n == field_name)
                        .map(|(_, ty)| ty.clone())
                        .ok_or_else(|| {
                            format!("Unknown field: {} in class {}", field_name, class_name)
                        })?;
                    let field_index = fields.iter().position(|(n, _)| n == field_name).unwrap();
                    (*ptr, field_ty, field_index)
                }
                Type::Object(fields) => {
                    let field = fields
                        .iter()
                        .find(|f| f.name == field_name)
                        .ok_or_else(|| format!("Unknown field: {} in object", field_name))?;
                    let field_index = fields.iter().position(|f| f.name == field_name).unwrap();
                    (*ptr, field.ty.clone(), field_index)
                }
                _ => return Err(format!("Cannot access field on type: {:?}", ty)),
            }
        }
        Expr::SelfExpr => {
            let (ptr, ty) = ctx
                .variables
                .get("self")
                .ok_or_else(|| "self not in scope".to_string())?;
            let class_name = match ty {
                Type::Named(n) => n.clone(),
                Type::Array(_) => "Array".to_string(),
                Type::Generic { base, .. } => base.clone(),
                _ => return Err(format!("Cannot access field on type: {:?}", ty)),
            };
            let fields = ctx
                .class_fields
                .get(&class_name)
                .ok_or_else(|| format!("Unknown class: {}", class_name))?;
            let field_ty = fields
                .iter()
                .find(|(n, _)| n == field_name)
                .map(|(_, ty)| ty.clone())
                .ok_or_else(|| format!("Unknown field: {} in class {}", field_name, class_name))?;
            let field_index = fields.iter().position(|(n, _)| n == field_name).unwrap();
            (*ptr, field_ty, field_index)
        }
        _ => return Err("Member access only supported on identifiers".to_string()),
    };

    let obj_ptr = ctx.builder.build_load(var_ptr, "obj").into_pointer_value();

    let offset = ctx
        .context
        .i32_type()
        .const_int((field_index * 8) as u64, false);
    let field_ptr = unsafe {
        ctx.builder
            .build_gep(obj_ptr, &[offset], &format!("{}_ptr", field_name))
    };

    let value = match field_ty {
        Type::Float => {
            let i64_ptr = ctx
                .builder
                .build_bitcast(
                    field_ptr,
                    ctx.context.i64_type().ptr_type(Default::default()),
                    "field_i64_ptr",
                )
                .into_pointer_value();
            let loaded = ctx.builder.build_load(i64_ptr, field_name).into_int_value();
            let float_val =
                ctx.builder
                    .build_bitcast(loaded, ctx.context.f64_type(), "field_float");
            BasicValueEnum::FloatValue(float_val.into_float_value())
        }
        _ => {
            let llvm_ty = ruyi_type_to_llvm(ctx.context, &field_ty);
            let typed_ptr = ctx
                .builder
                .build_bitcast(
                    field_ptr,
                    llvm_ty.ptr_type(Default::default()),
                    "field_typed_ptr",
                )
                .into_pointer_value();
            ctx.builder.build_load(typed_ptr, field_name)
        }
    };

    Ok(ExprResult::new(value, field_ty))
}

fn compile_optional_member_access<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    object: &Expr,
    field_name: &str,
) -> Result<ExprResult<'ctx>, String> {
    let obj_result = compile_expr(ctx, object)?;
    let obj_ptr = obj_result.value.into_pointer_value();

    let class_name = match &obj_result.ty {
        Type::Named(n) => n.clone(),
        Type::Array(_) => "Array".to_string(),
        Type::Generic { base, .. } => base.clone(),
        Type::Nullable(inner) => match inner.as_ref() {
            Type::Named(n) => n.clone(),
            Type::Array(_) => "Array".to_string(),
            Type::Generic { base, .. } => base.clone(),
            _ => return Err("Optional chaining only supported on class instances".to_string()),
        },
        _ => return Err("Optional chaining only supported on class instances".to_string()),
    };

    let fields = ctx
        .class_fields
        .get(&class_name)
        .ok_or_else(|| format!("Unknown class: {}", class_name))?;
    let field_ty = fields
        .iter()
        .find(|(n, _)| n == field_name)
        .map(|(_, ty)| ty.clone())
        .ok_or_else(|| format!("Unknown field: {} in class {}", field_name, class_name))?;

    let func = ctx.current_function.ok_or("No current function")?;
    let i64_ty = ctx.context.i64_type();
    let obj_int = ctx.builder.build_ptr_to_int(obj_ptr, i64_ty, "obj_int");
    let is_null = ctx.builder.build_int_compare(
        inkwell::IntPredicate::EQ,
        obj_int,
        i64_ty.const_int(0, false),
        "is_null",
    );

    let non_null_bb = ctx.context.append_basic_block(func, "opt_non_null");
    let null_bb = ctx.context.append_basic_block(func, "opt_null");
    let merge_bb = ctx.context.append_basic_block(func, "opt_merge");

    ctx.builder
        .build_conditional_branch(is_null, null_bb, non_null_bb);

    ctx.builder.position_at_end(null_bb);
    let llvm_ty = ruyi_type_to_llvm(ctx.context, &field_ty);
    let null_val = build_zero_value(llvm_ty);
    ctx.builder.build_unconditional_branch(merge_bb);

    ctx.builder.position_at_end(non_null_bb);
    let struct_type = ctx
        .class_struct_types
        .get(&class_name)
        .ok_or_else(|| format!("No struct type for class: {}", class_name))?;
    let struct_ptr = ctx.builder.build_pointer_cast(
        obj_ptr,
        struct_type.ptr_type(Default::default()),
        &format!("{}_cast", class_name),
    );
    let field_index = fields.iter().position(|(n, _)| n == field_name).unwrap();
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
    let value = ctx.builder.build_load(field_ptr, field_name);
    ctx.builder.build_unconditional_branch(merge_bb);
    let non_null_bb_end = ctx.builder.get_insert_block().unwrap();

    ctx.builder.position_at_end(merge_bb);
    let phi = ctx.builder.build_phi(llvm_ty, "opt_phi");
    phi.add_incoming(&[(&null_val, null_bb), (&value, non_null_bb_end)]);

    let result_ty = field_ty.make_nullable();
    Ok(ExprResult::new(phi.as_basic_value(), result_ty))
}

fn build_zero_value<'ctx>(ty: inkwell::types::BasicTypeEnum<'ctx>) -> BasicValueEnum<'ctx> {
    match ty {
        inkwell::types::BasicTypeEnum::IntType(t) => {
            BasicValueEnum::IntValue(t.const_int(0, false))
        }
        inkwell::types::BasicTypeEnum::FloatType(t) => {
            BasicValueEnum::FloatValue(t.const_float(0.0))
        }
        inkwell::types::BasicTypeEnum::PointerType(t) => {
            BasicValueEnum::PointerValue(t.const_null())
        }
        inkwell::types::BasicTypeEnum::StructType(t) => BasicValueEnum::StructValue(t.const_zero()),
        inkwell::types::BasicTypeEnum::ArrayType(t) => BasicValueEnum::ArrayValue(t.const_zero()),
        inkwell::types::BasicTypeEnum::VectorType(t) => BasicValueEnum::VectorValue(t.const_zero()),
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
        BinaryOp::Nullish => compile_nullish(ctx, left, right, &left_result, &right_result),
        BinaryOp::Power => compile_power(ctx, &left_result, &right_result),
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

    // String + Int: convert int to string, then concat
    if left.ty == Type::String && right.ty == Type::Int {
        let l = left.value.into_pointer_value();
        let r_int = right.value.into_int_value();
        let int_to_str_fn = ctx
            .module
            .get_function("ruyi_int_to_string")
            .expect("ruyi_int_to_string not declared");
        let r_str = ctx
            .builder
            .build_call(int_to_str_fn, &[r_int.into()], "int_to_str")
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();
        let concat_fn = ctx
            .module
            .get_function("ruyi_str_concat")
            .expect("ruyi_str_concat not declared");
        let res = ctx
            .builder
            .build_call(concat_fn, &[l.into(), r_str.into()], "str_concat")
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();
        return Ok(ExprResult::new(
            BasicValueEnum::PointerValue(res),
            Type::String,
        ));
    }

    // String + Float: convert float to string, then concat
    if left.ty == Type::String && right.ty == Type::Float {
        let l = left.value.into_pointer_value();
        let r_float = right.value.into_float_value();
        let float_to_str_fn = ctx
            .module
            .get_function("ruyi_float_to_string")
            .expect("ruyi_float_to_string not declared");
        let r_str = ctx
            .builder
            .build_call(float_to_str_fn, &[r_float.into()], "float_to_str")
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();
        let concat_fn = ctx
            .module
            .get_function("ruyi_str_concat")
            .expect("ruyi_str_concat not declared");
        let res = ctx
            .builder
            .build_call(concat_fn, &[l.into(), r_str.into()], "str_concat")
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();
        return Ok(ExprResult::new(
            BasicValueEnum::PointerValue(res),
            Type::String,
        ));
    }

    // Int + String: convert int to string, then concat
    if left.ty == Type::Int && right.ty == Type::String {
        let l_int = left.value.into_int_value();
        let r = right.value.into_pointer_value();
        let int_to_str_fn = ctx
            .module
            .get_function("ruyi_int_to_string")
            .expect("ruyi_int_to_string not declared");
        let l_str = ctx
            .builder
            .build_call(int_to_str_fn, &[l_int.into()], "int_to_str")
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();
        let concat_fn = ctx
            .module
            .get_function("ruyi_str_concat")
            .expect("ruyi_str_concat not declared");
        let res = ctx
            .builder
            .build_call(concat_fn, &[l_str.into(), r.into()], "str_concat")
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();
        return Ok(ExprResult::new(
            BasicValueEnum::PointerValue(res),
            Type::String,
        ));
    }

    // Float + String: convert float to string, then concat
    if left.ty == Type::Float && right.ty == Type::String {
        let l_float = left.value.into_float_value();
        let r = right.value.into_pointer_value();
        let float_to_str_fn = ctx
            .module
            .get_function("ruyi_float_to_string")
            .expect("ruyi_float_to_string not declared");
        let l_str = ctx
            .builder
            .build_call(float_to_str_fn, &[l_float.into()], "float_to_str")
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();
        let concat_fn = ctx
            .module
            .get_function("ruyi_str_concat")
            .expect("ruyi_str_concat not declared");
        let res = ctx
            .builder
            .build_call(concat_fn, &[l_str.into(), r.into()], "str_concat")
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

fn compile_power<'ctx>(
    ctx: &CodegenContext<'ctx, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    let pow_fn = ctx.module.get_function("pow").expect("pow not declared");

    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let l_f = ctx
                .builder
                .build_signed_int_to_float(*l, ctx.context.f64_type(), "itof");
            let r_f = ctx
                .builder
                .build_signed_int_to_float(*r, ctx.context.f64_type(), "itof");
            let res = ctx
                .builder
                .build_call(pow_fn, &[l_f.into(), r_f.into()], "pow")
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_float_value();
            let res_int =
                ctx.builder
                    .build_float_to_signed_int(res, ctx.context.i64_type(), "ftoi");
            Ok(ExprResult::new(
                BasicValueEnum::IntValue(res_int),
                Type::Int,
            ))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
            let res = ctx
                .builder
                .build_call(pow_fn, &[(*l).into(), (*r).into()], "pow")
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_float_value();
            Ok(ExprResult::new(
                BasicValueEnum::FloatValue(res),
                Type::Float,
            ))
        }
        (BasicValueEnum::IntValue(l), BasicValueEnum::FloatValue(r)) => {
            let l_f = ctx
                .builder
                .build_signed_int_to_float(*l, ctx.context.f64_type(), "itof");
            let res = ctx
                .builder
                .build_call(pow_fn, &[l_f.into(), (*r).into()], "pow")
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_float_value();
            Ok(ExprResult::new(
                BasicValueEnum::FloatValue(res),
                Type::Float,
            ))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::IntValue(r)) => {
            let r_f = ctx
                .builder
                .build_signed_int_to_float(*r, ctx.context.f64_type(), "itof");
            let res = ctx
                .builder
                .build_call(pow_fn, &[(*l).into(), r_f.into()], "pow")
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_float_value();
            Ok(ExprResult::new(
                BasicValueEnum::FloatValue(res),
                Type::Float,
            ))
        }
        _ => Err("Invalid operands for **".to_string()),
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
        (BasicValueEnum::PointerValue(l), BasicValueEnum::PointerValue(r))
            if matches!(&left.ty, Type::Generic { .. })
                && matches!(&right.ty, Type::Generic { .. }) =>
        {
            let l_struct = ctx.builder.build_load(*l, "l_enum_loaded");
            let r_struct = ctx.builder.build_load(*r, "r_enum_loaded");
            let l_struct = match l_struct {
                BasicValueEnum::StructValue(s) => s,
                _ => return Err(format!("Enum pointer loaded to {:?}, not struct", l_struct)),
            };
            let r_struct = match r_struct {
                BasicValueEnum::StructValue(s) => s,
                _ => return Err(format!("Enum pointer loaded to {:?}, not struct", r_struct)),
            };
            let l_tag = ctx
                .builder
                .build_extract_value(l_struct, 0, "l_tag")
                .unwrap()
                .into_int_value();
            let r_tag = ctx
                .builder
                .build_extract_value(r_struct, 0, "r_tag")
                .unwrap()
                .into_int_value();
            let tag_eq = ctx
                .builder
                .build_int_compare(IntPredicate::EQ, l_tag, r_tag, "tag_eq");
            let l_val = ctx
                .builder
                .build_extract_value(l_struct, 1, "l_val")
                .unwrap()
                .into_pointer_value();
            let r_val = ctx
                .builder
                .build_extract_value(r_struct, 1, "r_val")
                .unwrap()
                .into_pointer_value();
            let li = ctx
                .builder
                .build_ptr_to_int(l_val, ctx.context.i64_type(), "l_val_int");
            let ri = ctx
                .builder
                .build_ptr_to_int(r_val, ctx.context.i64_type(), "r_val_int");
            let val_eq = ctx
                .builder
                .build_int_compare(IntPredicate::EQ, li, ri, "val_eq");
            let result = ctx.builder.build_and(tag_eq, val_eq, "enum_eq");
            Ok(ExprResult::new(
                BasicValueEnum::IntValue(result),
                Type::Bool,
            ))
        }
        (BasicValueEnum::PointerValue(l), BasicValueEnum::StructValue(r))
            if matches!(&left.ty, Type::Generic { .. })
                && matches!(&right.ty, Type::Generic { .. }) =>
        {
            let i8_ty = ctx.context.i8_type();
            let i8_ptr_ty = i8_ty.ptr_type(Default::default());
            let option_struct = ctx
                .context
                .struct_type(&[i8_ty.into(), i8_ptr_ty.into()], false);
            let l_struct_ptr = ctx
                .builder
                .build_bitcast(
                    *l,
                    option_struct.ptr_type(Default::default()),
                    "l_struct_ptr",
                )
                .into_pointer_value();
            let l_struct = ctx.builder.build_load(l_struct_ptr, "l_enum_loaded");
            let l_struct = match l_struct {
                BasicValueEnum::StructValue(s) => s,
                _ => return Err(format!("Enum pointer loaded to {:?}, not struct", l_struct)),
            };
            let l_tag = ctx
                .builder
                .build_extract_value(l_struct, 0, "l_tag")
                .unwrap()
                .into_int_value();
            let r_tag = ctx
                .builder
                .build_extract_value(*r, 0, "r_tag")
                .unwrap()
                .into_int_value();
            let tag_eq = ctx
                .builder
                .build_int_compare(IntPredicate::EQ, l_tag, r_tag, "tag_eq");
            let l_val = ctx
                .builder
                .build_extract_value(l_struct, 1, "l_val")
                .unwrap()
                .into_pointer_value();
            let r_val = ctx
                .builder
                .build_extract_value(*r, 1, "r_val")
                .unwrap()
                .into_pointer_value();
            let li = ctx
                .builder
                .build_ptr_to_int(l_val, ctx.context.i64_type(), "l_val_int");
            let ri = ctx
                .builder
                .build_ptr_to_int(r_val, ctx.context.i64_type(), "r_val_int");
            let val_eq = ctx
                .builder
                .build_int_compare(IntPredicate::EQ, li, ri, "val_eq");
            let result = ctx.builder.build_and(tag_eq, val_eq, "enum_eq");
            Ok(ExprResult::new(
                BasicValueEnum::IntValue(result),
                Type::Bool,
            ))
        }
        (BasicValueEnum::StructValue(l), BasicValueEnum::PointerValue(r))
            if matches!(&left.ty, Type::Generic { .. })
                && matches!(&right.ty, Type::Generic { .. }) =>
        {
            let r_struct = ctx.builder.build_load(*r, "r_enum_loaded");
            let r_struct = match r_struct {
                BasicValueEnum::StructValue(s) => s,
                _ => return Err(format!("Enum pointer loaded to {:?}, not struct", r_struct)),
            };
            let l_tag = ctx
                .builder
                .build_extract_value(*l, 0, "l_tag")
                .unwrap()
                .into_int_value();
            let r_tag = ctx
                .builder
                .build_extract_value(r_struct, 0, "r_tag")
                .unwrap()
                .into_int_value();
            let tag_eq = ctx
                .builder
                .build_int_compare(IntPredicate::EQ, l_tag, r_tag, "tag_eq");
            let l_val = ctx
                .builder
                .build_extract_value(*l, 1, "l_val")
                .unwrap()
                .into_pointer_value();
            let r_val = ctx
                .builder
                .build_extract_value(r_struct, 1, "r_val")
                .unwrap()
                .into_pointer_value();
            let li = ctx
                .builder
                .build_ptr_to_int(l_val, ctx.context.i64_type(), "l_val_int");
            let ri = ctx
                .builder
                .build_ptr_to_int(r_val, ctx.context.i64_type(), "r_val_int");
            let val_eq = ctx
                .builder
                .build_int_compare(IntPredicate::EQ, li, ri, "val_eq");
            let result = ctx.builder.build_and(tag_eq, val_eq, "enum_eq");
            Ok(ExprResult::new(
                BasicValueEnum::IntValue(result),
                Type::Bool,
            ))
        }
        (BasicValueEnum::PointerValue(l), BasicValueEnum::PointerValue(r)) => {
            let l_int = ctx
                .builder
                .build_ptr_to_int(*l, ctx.context.i64_type(), "l_ptr_int");
            let r_int = ctx
                .builder
                .build_ptr_to_int(*r, ctx.context.i64_type(), "r_ptr_int");
            let res = ctx
                .builder
                .build_int_compare(IntPredicate::EQ, l_int, r_int, "ptr_eq");
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        (BasicValueEnum::StructValue(l), BasicValueEnum::StructValue(r)) => {
            // Compare struct values by comparing each field
            let struct_ty = l.get_type();
            let count = struct_ty.count_fields();
            let mut result = ctx.context.bool_type().const_int(1, false);
            for i in 0..count {
                let l_field = ctx
                    .builder
                    .build_extract_value(*l, i, &format!("l_field_{}", i))
                    .unwrap();
                let r_field = ctx
                    .builder
                    .build_extract_value(*r, i, &format!("r_field_{}", i))
                    .unwrap();
                let field_eq = match (l_field, r_field) {
                    (BasicValueEnum::IntValue(lv), BasicValueEnum::IntValue(rv)) => ctx
                        .builder
                        .build_int_compare(IntPredicate::EQ, lv, rv, &format!("field_eq_{}", i)),
                    (BasicValueEnum::PointerValue(lp), BasicValueEnum::PointerValue(rp)) => {
                        let li = ctx.builder.build_ptr_to_int(
                            lp,
                            ctx.context.i64_type(),
                            &format!("l_ptr_int_{}", i),
                        );
                        let ri = ctx.builder.build_ptr_to_int(
                            rp,
                            ctx.context.i64_type(),
                            &format!("r_ptr_int_{}", i),
                        );
                        ctx.builder.build_int_compare(
                            IntPredicate::EQ,
                            li,
                            ri,
                            &format!("field_eq_{}", i),
                        )
                    }
                    _ => ctx.context.bool_type().const_int(0, false),
                };
                result = ctx
                    .builder
                    .build_and(result, field_eq, &format!("and_{}", i));
            }
            Ok(ExprResult::new(
                BasicValueEnum::IntValue(result),
                Type::Bool,
            ))
        }
        _ => Err(format!(
            "Invalid operands for ===: left={:?}({:?}), right={:?}({:?})",
            left.value, left.ty, right.value, right.ty
        )),
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
        (BasicValueEnum::PointerValue(l), BasicValueEnum::PointerValue(r)) => {
            let l_int = ctx
                .builder
                .build_ptr_to_int(*l, ctx.context.i64_type(), "l_ptr_int");
            let r_int = ctx
                .builder
                .build_ptr_to_int(*r, ctx.context.i64_type(), "r_ptr_int");
            let res = ctx
                .builder
                .build_int_compare(IntPredicate::NE, l_int, r_int, "ptr_ne");
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

fn compile_nullish<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    left_expr: &Expr,
    right_expr: &Expr,
    _left: &ExprResult<'ctx>,
    _right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    let left_result = compile_expr(ctx, left_expr)?;

    if let BasicValueEnum::PointerValue(ptr) = left_result.value {
        let is_null = ctx
            .builder
            .build_ptr_to_int(ptr, ctx.context.i64_type(), "ptr_to_int");
        let zero = ctx.context.i64_type().const_int(0, false);
        let cond = ctx
            .builder
            .build_int_compare(IntPredicate::EQ, is_null, zero, "is_null");

        let current_bb = ctx.builder.get_insert_block().unwrap();
        let func = current_bb.get_parent().unwrap();
        let then_bb = ctx.context.append_basic_block(func, "nullish_then");
        let else_bb = ctx.context.append_basic_block(func, "nullish_else");
        let merge_bb = ctx.context.append_basic_block(func, "nullish_merge");

        ctx.builder.build_conditional_branch(cond, then_bb, else_bb);

        ctx.builder.position_at_end(then_bb);
        let right_result = compile_expr(ctx, right_expr)?;
        ctx.builder.build_unconditional_branch(merge_bb);
        let then_bb_end = ctx.builder.get_insert_block().unwrap();

        ctx.builder.position_at_end(else_bb);
        ctx.builder.build_unconditional_branch(merge_bb);

        ctx.builder.position_at_end(merge_bb);
        let phi = ctx
            .builder
            .build_phi(left_result.value.get_type(), "nullish_phi");
        phi.add_incoming(&[
            (&right_result.value, then_bb_end),
            (&left_result.value, else_bb),
        ]);

        return Ok(ExprResult::new(phi.as_basic_value(), right_result.ty));
    }

    Ok(left_result)
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
        Expr::Identifier(n) => {
            if ctx.class_struct_types.contains_key(n) {
                let class_expr = Expr::Identifier(n.clone());
                return compile_new(ctx, &class_expr, args);
            }
            // Handle enum variant constructors: Some(value), None, Ok(value), Err(value)
            if n == "Some" || n == "Ok" {
                return compile_enum_variant(ctx, n, args, 1);
            }
            if n == "None" || n == "Err" {
                return compile_enum_variant(ctx, n, args, 0);
            }
            (n.clone(), None)
        }
        Expr::Member {
            object, property, ..
        } => {
            let method_name = match property {
                crate::parser::ast::MemberProperty::Ident(n) => n.clone(),
                _ => return Err("Only simple method calls supported".to_string()),
            };
            if method_name == "new" {
                match object.as_ref() {
                    Expr::Identifier(_n) => {
                        return compile_new(ctx, object.as_ref(), args);
                    }
                    Expr::Super => {
                        return compile_super_new(ctx, args);
                    }
                    _ => return Err("new() must be called on a class name or super".to_string()),
                }
            }
            let (obj_ptr, class_name) = match object.as_ref() {
                Expr::Identifier(var_name) => {
                    if let Some((ptr, ty)) = ctx.variables.get(var_name) {
                        let class_name = match ty {
                            Type::Named(n) => n.clone(),
                            Type::Array(_) => "Array".to_string(),
                            Type::Generic { base, .. } => base.clone(),
                            Type::Int => "Int".to_string(),
                            Type::Float => "Float".to_string(),
                            Type::Bool => "Bool".to_string(),
                            _ => return Err(format!("Cannot call method on type: {:?}", ty)),
                        };
                        (Some(*ptr), class_name)
                    } else if ctx.class_struct_types.contains_key(var_name) {
                        (None, var_name.clone())
                    } else {
                        return Err(format!("Undefined variable: {}", var_name));
                    }
                }
                Expr::SelfExpr => {
                    let (ptr, ty) = ctx
                        .variables
                        .get("self")
                        .ok_or_else(|| "self not in scope".to_string())?;
                    let class_name = match ty {
                        Type::Named(n) => n.clone(),
                        Type::Array(_) => "Array".to_string(),
                        Type::Generic { base, .. } => base.clone(),
                        Type::Int => "Int".to_string(),
                        Type::Float => "Float".to_string(),
                        Type::Bool => "Bool".to_string(),
                        _ => return Err(format!("Cannot call method on type: {:?}", ty)),
                    };
                    (Some(*ptr), class_name)
                }
                Expr::Member {
                    object: inner_obj,
                    property: inner_prop,
                    ..
                } => {
                    let inner_field = match inner_prop {
                        crate::parser::ast::MemberProperty::Ident(n) => n.clone(),
                        _ => return Err("Only simple field access supported".to_string()),
                    };
                    // Get field pointer and type from inner member access
                    let (inner_var_ptr, inner_class_name, field_ty, field_index) = match inner_obj
                        .as_ref()
                    {
                        Expr::SelfExpr => {
                            let (ptr, ty) = ctx
                                .variables
                                .get("self")
                                .ok_or_else(|| "self not in scope".to_string())?;
                            let class_name = match ty {
                                Type::Named(n) => n.clone(),
                                Type::Array(_) => "Array".to_string(),
                                Type::Generic { base, .. } => base.clone(),
                                _ => {
                                    return Err(format!(
                                        "Cannot access field on self type: {:?}",
                                        ty
                                    ))
                                }
                            };
                            let fields = ctx
                                .class_fields
                                .get(&class_name)
                                .ok_or_else(|| format!("Unknown class: {}", class_name))?;
                            let idx = fields
                                .iter()
                                .position(|(n, _)| n == &inner_field)
                                .ok_or_else(|| {
                                    format!(
                                        "Unknown field: {} in class {}",
                                        inner_field, class_name
                                    )
                                })?;
                            let fty = fields[idx].1.clone();
                            (*ptr, class_name, fty, idx)
                        }
                        Expr::Identifier(name) => {
                            let (ptr, ty) = ctx
                                .variables
                                .get(name.as_str())
                                .ok_or_else(|| format!("Undefined variable: {}", name))?;
                            let class_name = match ty {
                                Type::Named(n) => n.clone(),
                                Type::Array(_) => "Array".to_string(),
                                Type::Generic { base, .. } => base.clone(),
                                _ => return Err(format!("Cannot access field on type: {:?}", ty)),
                            };
                            let fields = ctx
                                .class_fields
                                .get(&class_name)
                                .ok_or_else(|| format!("Unknown class: {}", class_name))?;
                            let idx = fields
                                .iter()
                                .position(|(n, _)| n == &inner_field)
                                .ok_or_else(|| {
                                    format!(
                                        "Unknown field: {} in class {}",
                                        inner_field, class_name
                                    )
                                })?;
                            let fty = fields[idx].1.clone();
                            (*ptr, class_name, fty, idx)
                        }
                        _ => return Err("Nested member access not yet supported".to_string()),
                    };
                    let obj_ptr = ctx
                        .builder
                        .build_load(inner_var_ptr, "obj")
                        .into_pointer_value();
                    let offset = ctx
                        .context
                        .i32_type()
                        .const_int((field_index * 8) as u64, false);
                    let field_ptr = unsafe {
                        ctx.builder
                            .build_gep(obj_ptr, &[offset], &format!("{}_ptr", inner_field))
                    };
                    let class_name = match field_ty {
                        Type::Named(ref n) => n.clone(),
                        Type::Array(_) => "Array".to_string(),
                        Type::Generic { ref base, .. } => base.clone(),
                        _ => {
                            return Err(format!("Cannot call method on field type: {:?}", field_ty))
                        }
                    };
                    (Some(field_ptr), class_name)
                }
                _ => return Err("Method calls only supported on identifiers".to_string()),
            };
            let func_name = format!("{}_{}", class_name, method_name);
            let func_name = if ctx.module.get_function(&func_name).is_some() {
                func_name
            } else {
                // Handle primitive type methods: Int.toString -> ruyi_int_to_string
                if class_name == "Int" && method_name == "toString" {
                    "ruyi_int_to_string".to_string()
                } else if class_name == "Float" && method_name == "toString" {
                    "ruyi_float_to_string".to_string()
                } else if class_name == "Bool" && method_name == "toString" {
                    "ruyi_bool_to_string".to_string()
                } else {
                    // Trait impl pattern: {method}_{trait}_for_{type}
                    // Also try: {method}_for_{type} for simpler cases
                    let suffix = format!("_for_{}", class_name);
                    let prefix = format!("{}_", method_name);
                    let mut found = None;
                    for func in ctx.module.get_functions() {
                        let fname = func.get_name().to_string_lossy().to_string();
                        if fname.starts_with(&prefix) && fname.ends_with(&suffix) {
                            found = Some(fname);
                            break;
                        }
                    }
                    found.unwrap_or(func_name)
                }
            };
            (func_name, obj_ptr)
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

    // Check if name refers to a local variable holding a function pointer
    if self_arg.is_none() {
        if let Some((ptr, ty)) = ctx.variables.get(&name).map(|(p, t)| (*p, t.clone())) {
            if let Type::Function {
                params: fn_params,
                return_type: fn_ret,
            } = ty
            {
                let func_ptr_val = ctx.builder.build_load(ptr, "func_ptr");
                let func_ptr = func_ptr_val.into_pointer_value();

                let mut arg_values = Vec::new();
                for arg in args {
                    match arg {
                        crate::parser::ast::Argument::Expr(e) => {
                            let result = compile_expr(ctx, e)?;
                            arg_values.push(result.value.into());
                        }
                        _ => return Err("Spread arguments not yet supported".to_string()),
                    }
                }

                let fn_type = function_type_from_ruyi(ctx.context, &fn_params, &fn_ret);
                let fn_ptr_type = fn_type.ptr_type(Default::default());
                let casted_ptr = ctx
                    .builder
                    .build_bitcast(func_ptr, fn_ptr_type, "fn_cast")
                    .into_pointer_value();
                let callable: inkwell::values::CallableValue<'ctx> = casted_ptr
                    .try_into()
                    .map_err(|_| "Failed to create callable from function pointer".to_string())?;

                let call_site = ctx.builder.build_call(callable, &arg_values, "call");
                let value = call_site.try_as_basic_value().left();

                return match value {
                    Some(v) => Ok(ExprResult::new(v, *fn_ret)),
                    None => Ok(ExprResult::new(
                        BasicValueEnum::IntValue(ctx.context.i64_type().const_int(0, false)),
                        Type::Void,
                    )),
                };
            }
        }
    }

    let func = ctx
        .module
        .get_function(&name)
        .ok_or_else(|| format!("Function not found: {}", name))?;

    let mut arg_values: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> = Vec::new();
    if let Some(self_ptr) = self_arg {
        if name.starts_with("ruyi_") {
            let loaded = ctx.builder.build_load(self_ptr, "obj");
            arg_values.push(loaded.into());
        } else {
            let loaded = ctx.builder.build_load(self_ptr, "obj");
            let obj_ptr = match loaded {
                BasicValueEnum::PointerValue(p) => p,
                BasicValueEnum::IntValue(i) => ctx.builder.build_int_to_ptr(
                    i,
                    ctx.context.i8_type().ptr_type(Default::default()),
                    "obj_ptr",
                ),
                _ => return Err(format!("Cannot convert {:?} to object pointer", loaded)),
            };
            arg_values.push(obj_ptr.into());
        }
    }
    // Get the function's LLVM parameter types for type-aware conversion
    let func_param_types: Vec<_> = func.get_type().get_param_types();

    let self_arg_offset = if self_arg.is_some() { 1 } else { 0 };

    for (arg_idx, arg) in args.iter().enumerate() {
        match arg {
            crate::parser::ast::Argument::Expr(e) => {
                let result = compile_expr(ctx, e)?;
                let func_param_idx = self_arg_offset + arg_idx;
                let expected_ty = func_param_types.get(func_param_idx);
                let i8_ptr = ctx.context.i8_type().ptr_type(Default::default());

                // Only convert to i8* if the function actually expects i8*
                let adjusted_value = if expected_ty == Some(&i8_ptr.into()) {
                    match result.value {
                        BasicValueEnum::PointerValue(pv) => {
                            if pv.get_type() != i8_ptr {
                                ctx.builder.build_bitcast(pv, i8_ptr, "ptr_cast").into()
                            } else {
                                pv.into()
                            }
                        }
                        BasicValueEnum::IntValue(iv) => ctx
                            .builder
                            .build_int_to_ptr(iv, i8_ptr, "int_to_ptr")
                            .into(),
                        BasicValueEnum::FloatValue(fv) => {
                            let i64_val =
                                ctx.builder
                                    .build_bitcast(fv, ctx.context.i64_type(), "f_to_i");
                            ctx.builder
                                .build_int_to_ptr(i64_val.into_int_value(), i8_ptr, "int_to_ptr")
                                .into()
                        }
                        other => other.into(),
                    }
                } else {
                    result.value.into()
                };
                arg_values.push(adjusted_value);
            }
            _ => return Err("Spread arguments not yet supported".to_string()),
        }
    }

    // Handle rest parameters: package extra arguments into an Array
    let rest_info = ctx.rest_params.get(&name).cloned();
    if let Some((rest_idx, elem_ty)) = rest_info {
        if arg_values.len() > rest_idx {
            let rest_args: Vec<_> = arg_values.drain(rest_idx..).collect();
            let array_ptr = compile_rest_args_to_array(ctx, &rest_args, &elem_ty)?;
            arg_values.push(array_ptr.into());
        }
    }

    let call_site = ctx.builder.build_call(func, &arg_values, "call");
    let value = call_site.try_as_basic_value().left();

    let is_async = ctx.module.get_function(&format!("{}$poll", name)).is_some();

    let ret_ty = if is_async {
        Type::Future(Box::new(Type::Int))
    } else if let Some(Type::Function { return_type, .. }) = ctx.function_types.get(&name) {
        *return_type.clone()
    } else if name == "ruyi_int_to_string"
        || name == "ruyi_float_to_string"
        || name == "ruyi_bool_to_string"
    {
        Type::String
    } else if name.starts_with("__builtin_array_") && name.ends_with("length") {
        Type::Int
    } else if name.starts_with("__builtin_array_") {
        Type::Dynamic
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
        Expr::Member {
            object, property, ..
        } => {
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
                        Type::Array(_) => "Array".to_string(),
                        Type::Generic { base, .. } => base.clone(),
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
                        Type::Array(_) => "Array".to_string(),
                        Type::Generic { base, .. } => base.clone(),
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

            let struct_ptr = ctx.builder.build_pointer_cast(
                obj_ptr,
                struct_type.ptr_type(Default::default()),
                &format!("{}_cast", class_name),
            );

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

    ctx.builder.position_at_end(then_bb);
    let then_result = compile_expr(ctx, then_branch)?;
    ctx.builder.build_unconditional_branch(merge_bb);
    let then_bb_end = ctx.builder.get_insert_block().unwrap();

    ctx.builder.position_at_end(else_bb);
    let else_result = compile_expr(ctx, else_branch)?;
    ctx.builder.build_unconditional_branch(merge_bb);
    let else_bb_end = ctx.builder.get_insert_block().unwrap();

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

fn compile_if_expr<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    condition: &Expr,
    then_branch: &Expr,
    else_branch: Option<&Expr>,
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

    ctx.builder.position_at_end(then_bb);
    let then_result = compile_expr(ctx, then_branch)?;
    ctx.builder.build_unconditional_branch(merge_bb);
    let then_bb_end = ctx.builder.get_insert_block().unwrap();

    ctx.builder.position_at_end(else_bb);
    let else_result = if let Some(else_expr) = else_branch {
        compile_expr(ctx, else_expr)?
    } else {
        ExprResult::new(
            BasicValueEnum::IntValue(ctx.context.i64_type().const_int(0, false)),
            Type::Void,
        )
    };
    ctx.builder.build_unconditional_branch(merge_bb);
    let else_bb_end = ctx.builder.get_insert_block().unwrap();

    ctx.builder.position_at_end(merge_bb);

    let phi_ty = ruyi_type_to_llvm(ctx.context, &then_result.ty);
    let phi = ctx.builder.build_phi(phi_ty, "if_phi");
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
    let cap = if len == 0 { 4 } else { len };
    let total_size = ctx.context.i64_type().const_int(16 + cap * 8, false);
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

    let cap_ptr = unsafe {
        ctx.builder.build_gep(
            ptr,
            &[ctx.context.i32_type().const_int(8, false)],
            "cap_ptr",
        )
    };
    let cap_i64_ptr = ctx
        .builder
        .build_bitcast(
            cap_ptr,
            ctx.context.i64_type().ptr_type(Default::default()),
            "cap_i64_ptr",
        )
        .into_pointer_value();
    ctx.builder
        .build_store(cap_i64_ptr, ctx.context.i64_type().const_int(cap, false));

    for (i, elem) in elements.iter().enumerate() {
        match elem {
            crate::parser::ast::ArrayElement::Expr(e) => {
                let val = compile_expr(ctx, e)?;
                let offset = ctx.context.i32_type().const_int((16 + i * 8) as u64, false);
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

fn compile_rest_args_to_array<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    rest_args: &[inkwell::values::BasicMetadataValueEnum<'ctx>],
    _elem_ty: &Type,
) -> Result<inkwell::values::PointerValue<'ctx>, String> {
    let len = rest_args.len() as u64;
    let cap = if len == 0 { 4 } else { len };
    let total_size = ctx.context.i64_type().const_int(16 + cap * 8, false);
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

    let cap_ptr = unsafe {
        ctx.builder.build_gep(
            ptr,
            &[ctx.context.i32_type().const_int(8, false)],
            "cap_ptr",
        )
    };
    let cap_i64_ptr = ctx
        .builder
        .build_bitcast(
            cap_ptr,
            ctx.context.i64_type().ptr_type(Default::default()),
            "cap_i64_ptr",
        )
        .into_pointer_value();
    ctx.builder
        .build_store(cap_i64_ptr, ctx.context.i64_type().const_int(cap, false));

    for (i, val) in rest_args.iter().enumerate() {
        let offset = ctx.context.i32_type().const_int((16 + i * 8) as u64, false);
        let elem_ptr = unsafe { ctx.builder.build_gep(ptr, &[offset], "elem_ptr") };
        let i64_ptr = ctx
            .builder
            .build_bitcast(
                elem_ptr,
                ctx.context.i64_type().ptr_type(Default::default()),
                "elem_i64_ptr",
            )
            .into_pointer_value();

        let stored_val = match val {
            inkwell::values::BasicMetadataValueEnum::IntValue(v) => v.as_basic_value_enum(),
            inkwell::values::BasicMetadataValueEnum::FloatValue(v) => ctx
                .builder
                .build_bitcast(*v, ctx.context.i64_type(), "f_to_i")
                .as_basic_value_enum(),
            inkwell::values::BasicMetadataValueEnum::PointerValue(v) => v.as_basic_value_enum(),
            _ => continue,
        };
        ctx.builder.build_store(i64_ptr, stored_val);
    }

    Ok(ptr)
}

fn compile_object_literal<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    properties: &[crate::parser::ast::ObjectProperty],
) -> Result<ExprResult<'ctx>, String> {
    let len = properties.len() as u64;
    let total_size = ctx.context.i64_type().const_int(len * 8, false);
    let ptr = super::builtins::build_gc_alloc(&ctx.builder, &ctx.module, total_size);

    let mut fields = Vec::new();
    for (i, prop) in properties.iter().enumerate() {
        match prop {
            crate::parser::ast::ObjectProperty::Property { key, value } => {
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

                let name = match key {
                    crate::parser::ast::PropertyName::Ident(n) => n.clone(),
                    crate::parser::ast::PropertyName::String(n) => n.clone(),
                    crate::parser::ast::PropertyName::Number(n) => format!("{}", n),
                    crate::parser::ast::PropertyName::Computed(_) => format!("[computed]"),
                };
                fields.push(crate::typechecker::types::ObjectField {
                    name,
                    ty: val.ty,
                    optional: false,
                });
            }
            crate::parser::ast::ObjectProperty::Shorthand(name) => {
                let val = compile_expr(ctx, &Expr::Identifier(name.clone()))?;
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
                ctx.builder.build_store(i64_ptr, val.value);
                fields.push(crate::typechecker::types::ObjectField {
                    name: name.clone(),
                    ty: val.ty,
                    optional: false,
                });
            }
            _ => return Err("Unsupported object property".to_string()),
        }
    }

    Ok(ExprResult::new(
        BasicValueEnum::PointerValue(ptr),
        Type::Object(fields),
    ))
}

pub(crate) fn compile_new<'ctx>(
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

fn compile_super_new<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    args: &[crate::parser::ast::Argument],
) -> Result<ExprResult<'ctx>, String> {
    let func = ctx
        .current_function
        .ok_or("super.new() can only be called within a method")?;
    let func_name = func.get_name().to_string_lossy().to_string();

    let current_class = func_name
        .split('_')
        .next()
        .ok_or("Cannot determine current class from function name")?;

    let parent_class = ctx
        .class_extends
        .get(current_class)
        .ok_or_else(|| format!("Class '{}' has no parent class", current_class))?
        .clone();

    let (self_ptr, self_ty) = ctx
        .variables
        .get("self")
        .ok_or_else(|| "self not in scope".to_string())?;
    let self_ptr_copy = *self_ptr;
    let self_ty_copy = self_ty.clone();

    let ctor_name = format!("{}_new", parent_class);
    if let Some(ctor) = ctx.module.get_function(&ctor_name) {
        let self_loaded = ctx.builder.build_load(self_ptr_copy, "super_self");
        let mut arg_values = vec![self_loaded.into()];
        for arg in args {
            match arg {
                crate::parser::ast::Argument::Expr(e) => {
                    let result = compile_expr(ctx, e)?;
                    arg_values.push(result.value.into());
                }
                _ => return Err("Spread arguments not yet supported".to_string()),
            }
        }
        ctx.builder.build_call(ctor, &arg_values, "super_ctor_call");
    }

    Ok(ExprResult::new(
        BasicValueEnum::PointerValue(self_ptr_copy),
        self_ty_copy,
    ))
}

/// Compile enum variant constructors: Some(value), None, Ok(value), Err(value)
/// Layout: { tag: i8, value: i8* } where tag 0 = None/Err, tag 1 = Some/Ok
fn compile_enum_variant<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    variant: &str,
    args: &[crate::parser::ast::Argument],
    tag: u64,
) -> Result<ExprResult<'ctx>, String> {
    let i8_ty = ctx.context.i8_type();
    let i8_ptr_ty = i8_ty.ptr_type(Default::default());
    let option_struct = ctx
        .enum_struct_types
        .entry("Option".to_string())
        .or_insert_with(|| {
            ctx.context
                .struct_type(&[i8_ty.into(), i8_ptr_ty.into()], false)
        })
        .clone();
    let ptr = ctx.builder.build_alloca(option_struct, "enum_variant");

    let tag_ptr = ctx.builder.build_struct_gep(ptr, 0, "tag_ptr").unwrap();
    ctx.builder
        .build_store(tag_ptr, i8_ty.const_int(tag, false));

    let value_ptr = ctx.builder.build_struct_gep(ptr, 1, "value_ptr").unwrap();
    if tag == 1 && !args.is_empty() {
        if let crate::parser::ast::Argument::Expr(e) = &args[0] {
            let result = compile_expr(ctx, e)?;
            let casted = match result.value {
                BasicValueEnum::PointerValue(p) => {
                    ctx.builder.build_bitcast(p, i8_ptr_ty, "value_cast")
                }
                BasicValueEnum::IntValue(i) => BasicValueEnum::PointerValue(
                    ctx.builder.build_int_to_ptr(i, i8_ptr_ty, "value_cast"),
                ),
                BasicValueEnum::FloatValue(f) => {
                    ctx.builder.build_bitcast(f, i8_ptr_ty, "value_cast")
                }
                _ => return Err("Unsupported enum variant value type".to_string()),
            };
            let ptr_val = match casted {
                BasicValueEnum::PointerValue(p) => p,
                _ => return Err("Enum variant value must be a pointer".to_string()),
            };
            ctx.builder.build_store(value_ptr, ptr_val);
        }
    } else {
        ctx.builder.build_store(value_ptr, i8_ptr_ty.const_null());
    }

    let loaded = ctx.builder.build_load(ptr, "enum_loaded");
    Ok(ExprResult::new(
        loaded,
        Type::Generic {
            base: "Option".to_string(),
            args: vec![Type::Dynamic],
        },
    ))
}

fn compile_match_expr<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    value: &crate::parser::ast::Expr,
    arms: &[crate::parser::ast::MatchArm],
) -> Result<ExprResult<'ctx>, String> {
    use crate::parser::ast::{Pattern, Statement};
    let func = ctx.current_function.ok_or("No current function")?;
    let scrutinee = compile_expr(ctx, value)?;

    let arm_bbs: Vec<_> = (0..arms.len())
        .map(|i| {
            ctx.context
                .append_basic_block(func, &format!("match_expr_arm_{}", i))
        })
        .collect();
    let merge_bb = ctx.context.append_basic_block(func, "match_expr_merge");

    let result_ty = Type::Dynamic;
    let llvm_ty = super::types::ruyi_type_to_llvm(ctx.context, &result_ty);
    let result_ptr = ctx.builder.build_alloca(llvm_ty, "match_result");

    ctx.builder.build_unconditional_branch(arm_bbs[0]);

    for (i, arm) in arms.iter().enumerate() {
        ctx.builder.position_at_end(arm_bbs[i]);

        super::patterns::bind_pattern(ctx, &arm.pattern, &scrutinee)?;

        if let Some(guard) = &arm.guard {
            let guard_val = compile_expr(ctx, guard)?;
            let body_bb = ctx
                .context
                .append_basic_block(func, &format!("match_expr_body_{}", i));
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

        let body_len = arm.body.len();
        for (j, stmt) in arm.body.iter().enumerate() {
            compile_stmt_for_match(ctx, stmt, j == body_len - 1, result_ptr, llvm_ty)?;
            if let Some(bb) = ctx.builder.get_insert_block() {
                if bb.get_terminator().is_some() {
                    break;
                }
            }
        }

        if let Some(bb) = ctx.builder.get_insert_block() {
            if bb.get_terminator().is_none() {
                let undef: BasicValueEnum<'ctx> = match llvm_ty {
                    inkwell::types::BasicTypeEnum::IntType(t) => t.get_undef().into(),
                    inkwell::types::BasicTypeEnum::FloatType(t) => t.get_undef().into(),
                    inkwell::types::BasicTypeEnum::PointerType(t) => t.get_undef().into(),
                    inkwell::types::BasicTypeEnum::StructType(t) => t.get_undef().into(),
                    inkwell::types::BasicTypeEnum::ArrayType(t) => t.get_undef().into(),
                    inkwell::types::BasicTypeEnum::VectorType(t) => t.get_undef().into(),
                };
                ctx.builder.build_store(result_ptr, undef);
                ctx.builder.build_unconditional_branch(merge_bb);
            }
        }
    }

    ctx.builder.position_at_end(merge_bb);
    let loaded = ctx.builder.build_load(result_ptr, "match_result_final");
    Ok(ExprResult::new(loaded, result_ty))
}

fn compile_stmt_for_match<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_>,
    stmt: &Statement,
    is_last: bool,
    result_ptr: inkwell::values::PointerValue<'ctx>,
    llvm_ty: inkwell::types::BasicTypeEnum<'ctx>,
) -> Result<(), String> {
    use crate::parser::ast::Statement;
    match stmt {
        Statement::Expression(expr) => {
            let result = compile_expr(ctx, expr)?;
            if is_last {
                ctx.builder.build_store(result_ptr, result.value);
            }
            Ok(())
        }
        _ => super::stmt::compile_stmt(ctx, stmt),
    }
}
