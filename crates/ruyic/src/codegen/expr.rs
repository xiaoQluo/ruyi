use inkwell::types::{BasicMetadataTypeEnum, BasicType};
/**
 * Expression code generation for Ruyi.
 *
 * Lowers Ruyi AST expressions to LLVM IR instructions.
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */
use inkwell::values::{BasicValue, BasicValueEnum, FunctionValue};
use inkwell::FloatPredicate;
use inkwell::IntPredicate;

use ruyi_exception::landing_pad::LandingPadGenerator;

use super::gc_alloc::GcAllocFn;
use super::generator::CodegenContext;
use super::types::{function_type_from_ruyi, is_type_param_name, ruyi_type_to_llvm};
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
///
/// `ctx` supplies what a bare name resolves to outside the local scope —
/// declared function signatures and class templates — because a generic
/// method's type parameter is usually bound through a callback whose body
/// *calls* something (`andThen((x: int) => safeDivide(x + 5, 1))`). Without
/// those lookups the callback infers as `dyn`, the method type parameter
/// stays unbound, and the whole specialization collapses to the erased
/// pointer path.
fn infer_expr_type<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    expr: &Expr,
    param_types: &std::collections::HashMap<String, Type>,
) -> Type {
    match expr {
        Expr::IntLiteral(_) => Type::Int,
        Expr::FloatLiteral(_) => Type::Float,
        Expr::BooleanLiteral(_) => Type::Bool,
        Expr::StringLiteral(_) => Type::String,
        Expr::NullLiteral => Type::Null,
        Expr::BigIntLiteral(_) => Type::BigInt,
        Expr::Identifier(name) => param_types.get(name).cloned().unwrap_or(Type::Dynamic),
        Expr::Binary { op, left, right } => {
            let left_ty = infer_expr_type(ctx, left, param_types);
            let right_ty = infer_expr_type(ctx, right, param_types);
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
            UnaryOp::Minus => infer_expr_type(ctx, operand, param_types),
            UnaryOp::Not => Type::Bool,
            UnaryOp::Tilde => Type::Int,
            _ => Type::Dynamic,
        },
        Expr::Conditional {
            then_branch,
            else_branch,
            ..
        } => {
            let then_ty = infer_expr_type(ctx, then_branch, param_types);
            let else_ty = infer_expr_type(ctx, else_branch, param_types);
            then_ty.least_upper_bound(&else_ty)
        }
        Expr::Grouping(inner) => infer_expr_type(ctx, inner, param_types),
        Expr::ArrayLiteral(elements) => {
            let elem_ty = elements
                .first()
                .map(|el| match el {
                    crate::parser::ast::ArrayElement::Expr(e)
                    | crate::parser::ast::ArrayElement::Spread(e) => {
                        infer_expr_type(ctx, e, param_types)
                    }
                    _ => Type::Dynamic,
                })
                .unwrap_or(Type::Dynamic);
            Type::Array(Box::new(elem_ty))
        }
        Expr::ArrowFunction {
            params,
            return_type,
            body,
            ..
        } => {
            // A generic method's type parameter is usually bound through its
            // callback argument — `fn map<U>(self, f: fn(T) -> U)` takes `U`
            // from the closure's return type — so closures have to carry a
            // usable signature here or `U` stays unbound and the whole
            // specialization falls back to the erased `ptr` path.
            let param_types: Vec<Type> = params
                .iter()
                .map(|p| {
                    p.ty.as_ref()
                        .map(Type::from_annotation)
                        .unwrap_or(Type::Dynamic)
                })
                .collect();
            let mut param_map: std::collections::HashMap<String, Type> =
                std::collections::HashMap::new();
            for (p, ty) in params.iter().zip(param_types.iter()) {
                if let Pattern::Identifier(name) = &p.pattern {
                    param_map.insert(name.clone(), ty.clone());
                }
            }
            let ret = match return_type {
                Some(ann) => Type::from_annotation(ann),
                None => match body {
                    ArrowBody::Expr(expr) => infer_expr_type(ctx, expr, &param_map),
                    ArrowBody::Block(stmts) => infer_block_return_type(ctx, stmts, &param_map),
                },
            };
            Type::Function {
                params: param_types,
                return_type: Box::new(ret),
            }
        }
        Expr::Call { callee, args } => infer_call_type(ctx, callee, args, param_types),
        _ => Type::Dynamic,
    }
}

/// Type of a call expression, resolved from the callee's declaration.
fn infer_call_type<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    callee: &Expr,
    args: &[crate::parser::ast::Argument],
    param_types: &std::collections::HashMap<String, Type>,
) -> Type {
    match callee {
        Expr::Identifier(name) => {
            if name == "Array" && args.len() == 1 {
                if let crate::parser::ast::Argument::Expr(e) = &args[0] {
                    return Type::Array(Box::new(infer_expr_type(ctx, e, param_types)));
                }
            }
            declared_return_type(ctx, name).unwrap_or(Type::Dynamic)
        }
        Expr::Member {
            object, property, ..
        } => match property {
            crate::parser::ast::MemberProperty::Ident(method) => {
                infer_method_call_type(ctx, object, method, args, param_types)
            }
            _ => Type::Dynamic,
        },
        _ => Type::Dynamic,
    }
}

/// Declared return type of a named function already compiled into the
/// module, or `None` when it was never seen.
fn declared_return_type<'ctx>(ctx: &CodegenContext<'ctx, '_, '_>, name: &str) -> Option<Type> {
    match ctx.function_types.get(name)? {
        Type::Function { return_type, .. } => Some(*return_type.clone()),
        _ => None,
    }
}

/// Type of a method call `object.method(args)`.
fn infer_method_call_type<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    object: &Expr,
    method: &str,
    args: &[crate::parser::ast::Argument],
    param_types: &std::collections::HashMap<String, Type>,
) -> Type {
    // A static receiver names a class directly (`Ok.new(5)`), so no local
    // lookup is needed for it.
    if let Expr::Identifier(name) = object {
        if param_types.get(name).is_none() {
            if let Some(ty) = infer_static_method_type(ctx, name, method, args, param_types) {
                return ty;
            }
        }
    }
    let receiver = infer_expr_type(ctx, object, param_types);
    primitive_method_return_type(&receiver, method)
        .or_else(|| class_method_return_type(ctx, &receiver, method))
        .unwrap_or(Type::Dynamic)
}

/// Result type of a call on a primitive receiver, via the same builtin
/// naming codegen uses at the call site.
fn primitive_method_return_type(receiver: &Type, method: &str) -> Option<Type> {
    let name = match receiver {
        Type::String => format!(
            "__string_{}",
            method
                .chars()
                .enumerate()
                .flat_map(|(i, c)| if i > 0 && c.is_ascii_uppercase() {
                    vec!['_', c.to_ascii_lowercase()]
                } else {
                    vec![c.to_ascii_lowercase()]
                })
                .collect::<String>()
        ),
        Type::Int if method == "toString" => "ruyi_int_to_string".to_string(),
        Type::Float if method == "toString" => "ruyi_float_to_string".to_string(),
        Type::Bool if method == "toString" => "ruyi_bool_to_string".to_string(),
        Type::Array(_) => {
            if !matches!(method, "create" | "get" | "set" | "push" | "pop" | "length") {
                return None;
            }
            format!("__builtin_array_{}", method)
        }
        _ => return None,
    };
    if let Some(sig) = super::builtins_table::builtin_ret_sig(&name) {
        return Some(super::builtins_table::sig_to_type(&name, sig));
    }
    match name.as_str() {
        "ruyi_int_to_string" | "ruyi_float_to_string" | "ruyi_bool_to_string" => Some(Type::String),
        _ => None,
    }
}

/// Result type of a call on a user-defined class receiver, looked up from
/// the `{Class}_{method}` signature recorded when the method was compiled.
fn class_method_return_type<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    receiver: &Type,
    method: &str,
) -> Option<Type> {
    let class_name = match receiver {
        Type::Named(name, _) => name.clone(),
        Type::Nullable(inner) => match inner.as_ref() {
            Type::Named(name, _) => name.clone(),
            _ => return None,
        },
        _ => return None,
    };
    declared_return_type(ctx, &format!("{}_{}", class_name, method))
}

/// Result type of a static call on a class (`Ok.new(5)`).
///
/// A union member's constructor yields the *alias*, not the member class —
/// `Ok.new(5)` is a `Result<int, ?>`. The alias's type-parameter names come
/// from the member class itself (`Ok<T, E>`), and the constructor's
/// declared parameter types say which slot each argument lands in
/// (`Ok.new(value: T)`). Slots no argument determines stay `Dynamic`, which
/// makes unification skip them instead of mis-binding.
fn infer_static_method_type<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    class_name: &str,
    method: &str,
    args: &[crate::parser::ast::Argument],
    param_types: &std::collections::HashMap<String, Type>,
) -> Option<Type> {
    if method != "new" {
        return None;
    }
    let (class_params, body) = ctx.generic_classes.get(class_name)?;
    let ctor = body.iter().find_map(|el| match el {
        crate::parser::ast::ClassElement::Method {
            name: crate::parser::ast::PropertyName::Ident(n),
            is_static: false,
            params,
            ..
        } if n == method => Some(params),
        _ => None,
    })?;
    let arg_types: Vec<Type> = args
        .iter()
        .map(|a| match a {
            crate::parser::ast::Argument::Expr(e) | crate::parser::ast::Argument::Spread(e) => {
                infer_expr_type(ctx, e, param_types)
            }
        })
        .collect();

    if let Some(alias) = union_alias_of_member(class_name) {
        let mut alias_args = vec![Type::Dynamic; class_params.len()];
        for (param, ty) in ctor
            .iter()
            .filter(|p| !matches!(&p.pattern, Pattern::Identifier(n) if n == "self"))
            .zip(arg_types)
        {
            if let Some(crate::parser::ast::TypeAnnotation::Identifier(name)) = &param.ty {
                if let Some(slot) = class_params.iter().position(|p| p == name) {
                    alias_args[slot] = ty;
                }
            }
        }
        return Some(Type::Generic {
            base: alias.to_string(),
            args: alias_args,
        });
    }
    declared_return_type(ctx, &format!("{}_{}", class_name, method))
}

/// The union alias a class belongs to — the inverse of
/// [`union_alias_members`] (`Ok` → `Result`).
fn union_alias_of_member(class_name: &str) -> Option<&'static str> {
    ["Result", "Option"].into_iter().find(|alias| {
        union_alias_members(alias).is_some_and(|members| members.contains(&class_name))
    })
}

/// Build a map of currently-known variable types for lightweight AST type
/// inference used when selecting a generic specialization at a call site.
fn build_local_type_map<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
) -> std::collections::HashMap<String, Type> {
    let mut map = std::collections::HashMap::new();
    for (name, (_, ty)) in &ctx.variables {
        map.insert(name.clone(), ty.clone());
    }
    map
}

/// If the callee is a generic function and we can infer concrete type
/// arguments from the AST argument types, return the mangled name of the
/// selected specialization along with its concrete parameter types.
///
/// When the requested specialization is not already recorded by the
/// typechecker (for example, a helper generic called from inside another
/// generic body), this function creates it on demand and compiles its body.
fn resolve_generic_specialization<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    name: &str,
    args: &[crate::parser::ast::Argument],
) -> Option<(String, Vec<Type>)> {
    let tracker = ctx.monomorph_tracker.as_mut()?;
    let generic_def = tracker.get_generic(name)?.clone();
    let declared_fn_ty = ctx.function_types.get(name)?.clone();
    let declared_params = match declared_fn_ty {
        Type::Function { params, .. } => params,
        _ => return None,
    };

    let type_map = build_local_type_map(ctx);
    let mut arg_types: Vec<Type> = Vec::new();
    for arg in args {
        match arg {
            crate::parser::ast::Argument::Expr(e) => {
                arg_types.push(infer_expr_type(ctx, e, &type_map));
            }
            crate::parser::ast::Argument::Spread(e) => {
                let ty = infer_expr_type(ctx, e, &type_map);
                match ty {
                    Type::Array(inner) => arg_types.push(*inner),
                    _ => arg_types.push(Type::Dynamic),
                }
            }
        }
    }

    // Bind type parameters by matching declared parameter types against the
    // inferred argument types. We only need the parameter names that appear
    // in the generic definition.
    let tparam_names: Vec<String> = generic_def
        .type_params
        .iter()
        .map(|p| p.name.clone())
        .collect();
    let mut bindings: std::collections::HashMap<String, Type> = std::collections::HashMap::new();
    for (declared, concrete) in declared_params.iter().zip(arg_types.iter()) {
        super::specialize::bind_type_params_from_type(
            declared,
            concrete,
            &tparam_names,
            &mut bindings,
        );
    }

    // Reject if any type parameter is still unbound; we cannot pick a
    // specialization without a complete set of concrete type arguments.
    let type_args: Vec<Type> = tparam_names
        .iter()
        .map(|n| bindings.get(n).cloned())
        .collect::<Option<Vec<_>>>()?;
    if type_args.iter().any(|t| !super::specialize::is_concrete(t)) {
        return None;
    }

    let mangled_name = crate::typechecker::generics::mangle_name(name, &type_args);
    let tracker = ctx.monomorph_tracker.as_mut()?;
    let spec = if let Some(existing) = tracker.specializations().get(&mangled_name).cloned() {
        existing
    } else {
        tracker.ensure_specialization(name, type_args)?
    };

    // Generate the specialization if it does not already exist in the module.
    if ctx.module.get_function(&mangled_name).is_none() {
        let mono_func = super::monomorph::MonomorphizedFunction::from_specialization(&spec);
        if let Err(e) = super::generator::compile_monomorphized_function(ctx, &mono_func) {
            log::warn!(
                "Failed to compile on-demand specialization {}: {}",
                mangled_name,
                e
            );
            return None;
        }
    }

    match &spec.specialized_type {
        Type::Function { params, .. } => {
            // Nullable<T> signatures specialize too: the monomorphized body
            // substitutes the annotation (`T?` → `int?`), so `return null`
            // emits the concrete sentinel (-1 for ints, null pointer for
            // reference types) and consumers comparing against `null` get
            // the matching value. Falling back to the erased generic here
            // instead lost the sentinel and mis-typed the call result.
            Some((mangled_name, params.clone()))
        }
        _ => None,
    }
}

/// Resolve the mangled name for a generic method call on a concrete
/// generic receiver, appending the method's *own* type arguments to the
/// receiver's class arguments.
///
/// `fn map<U>(self, f: fn(T) -> U): Result<U, E>` needs both: `U` never
/// appears in the receiver's type arguments, so substituting only the
/// class parameters leaves the return type at `Result<U, E>` — which
/// downstream codegen materializes as a pointer and then hands to
/// `printf("%s")`. Returns the mangled name plus the full argument list
/// (class arguments first) for `ensure_method_specialization`, or `None`
/// when a method type parameter stays unbound.
fn resolve_generic_method_target<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    base: &str,
    method_name: &str,
    class_type_args: &[Type],
    args: &[crate::parser::ast::Argument],
) -> Option<(String, Vec<Type>)> {
    let (class_param_names, class_body) = ctx.generic_classes.get(base)?.clone();
    let mut class_subst: std::collections::HashMap<String, Type> = std::collections::HashMap::new();
    for (name, ty) in class_param_names.iter().zip(class_type_args) {
        class_subst.insert(name.clone(), ty.clone());
    }
    let type_map = build_local_type_map(ctx);
    let arg_types: Vec<Type> = args
        .iter()
        .map(|arg| match arg {
            crate::parser::ast::Argument::Expr(e) | crate::parser::ast::Argument::Spread(e) => {
                infer_expr_type(ctx, e, &type_map)
            }
        })
        .collect();
    let method_args = super::specialize::infer_method_type_args(
        &class_body,
        method_name,
        &class_subst,
        &arg_types,
    )?;
    // Only the first `class_param_names.len()` entries are this class's own
    // arguments. A union alias hands every member the alias's arguments
    // (`Option<int>` gives `[int]`), but `None` declares no class parameters
    // at all, so the surplus entry would misalign the specialization key.
    let mut all_args: Vec<Type> = class_type_args
        .iter()
        .take(class_param_names.len())
        .cloned()
        .collect();
    all_args.extend(method_args);
    let mangled =
        crate::typechecker::generics::mangle_name(&format!("{}_{}", base, method_name), &all_args);
    Some((mangled, all_args))
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

    finish_inferred_param_types(params, param_map)
}

/// Parameter type inference for `fn` expressions, whose bodies are
/// statement blocks (no `ArrowBody` wrapper).
fn infer_fn_expr_param_types(params: &[Param], body: &[Statement]) -> Vec<Type> {
    let mut param_map: std::collections::HashMap<String, Type> = std::collections::HashMap::new();
    for param in params {
        if let Pattern::Identifier(name) = &param.pattern {
            param_map.insert(name.clone(), Type::Dynamic);
        }
    }

    for stmt in body {
        infer_param_types_stmt(stmt, &mut param_map);
    }

    finish_inferred_param_types(params, param_map)
}

fn finish_inferred_param_types(
    params: &[Param],
    param_map: std::collections::HashMap<String, Type>,
) -> Vec<Type> {
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
                        if lname == rname
                            && matches!(
                                op,
                                BinaryOp::Star
                                    | BinaryOp::Minus
                                    | BinaryOp::Slash
                                    | BinaryOp::Percent
                            )
                        {
                            param_map.insert(lname, Type::Int);
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

/// Infer the type a block-bodied arrow function returns, from its last
/// expression or its `return` statements.
fn infer_block_return_type<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    stmts: &[Statement],
    param_map: &std::collections::HashMap<String, Type>,
) -> Type {
    let mut returned: Option<Type> = None;
    let mut tail: Option<Type> = None;
    for stmt in stmts {
        match stmt {
            Statement::Return(Some(expr)) => returned = Some(infer_expr_type(ctx, expr, param_map)),
            Statement::Expression(expr) => tail = Some(infer_expr_type(ctx, expr, param_map)),
            _ => {}
        }
    }
    returned.or(tail).unwrap_or(Type::Dynamic)
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

/// Pack a first-class function value `{ fnptr, env }`. When `captures`
/// is given, allocate the environment block on the GC heap and copy the
/// current values of the named slots into it; otherwise `env` is null.
/// The block layout (one 8-byte-aligned slot per capture, in order) must
/// match the load side in `decl::compile_function`.
pub(super) fn wrap_fn_value<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    fn_ptr: Option<inkwell::values::PointerValue<'ctx>>,
    captures: Option<&[(String, Type)]>,
) -> BasicValueEnum<'ctx> {
    let ptr_ty = ctx.context.ptr_type(Default::default());
    let null_ptr = ptr_ty.const_null();
    let fn_value_ty = super::types::fn_value_type(ctx.context);
    let mut s = fn_value_ty.const_zero();

    let env_ptr: inkwell::values::PointerValue<'ctx> = match captures {
        None => null_ptr,
        Some(captures) => {
            let total: u64 = captures
                .iter()
                .map(|(_, ty)| super::capture::capture_slot_size(ty))
                .sum();
            let total_words = ctx.context.i64_type().const_int(total, false);
            let block =
                GcAllocFn::for_mode(ctx.gc_mode).emit(ctx.builder(), ctx.module, total_words);
            let mut offset: u64 = 0;
            for (cap_name, cap_ty) in captures {
                let (slot, _) = ctx.variables.get(cap_name).unwrap_or_else(|| {
                    panic!(
                        "capture '{}' vanished from scope before env materialization",
                        cap_name
                    )
                });
                let llvm_ty = ruyi_type_to_llvm(ctx.context, cap_ty);
                let current = if ctx.closure_shared_names.contains(cap_name) {
                    // Shared mutable capture: copy the GC cell pointer
                    // (not the current value) so the closure and the
                    // enclosing scope read/write one location.
                    ctx.builder().build_load(ptr_ty, *slot, cap_name).unwrap()
                } else {
                    ctx.builder().build_load(llvm_ty, *slot, cap_name).unwrap()
                };
                let off = ctx.context.i32_type().const_int(offset, false);
                let slot_ptr = unsafe {
                    ctx.builder()
                        .build_gep(ctx.context.i8_type(), block, &[off], "cap_slot")
                        .unwrap()
                };
                ctx.builder().build_store(slot_ptr, current).unwrap();
                offset += super::capture::capture_slot_size(cap_ty);
            }
            block
        }
    };

    s = ctx
        .builder()
        .build_insert_value(s, fn_ptr.unwrap_or(null_ptr), 0, "fn_val_ptr")
        .unwrap()
        .into_struct_value();
    s = ctx
        .builder()
        .build_insert_value(s, env_ptr, 1, "fn_val_env")
        .unwrap()
        .into_struct_value();
    s.into()
}

/// Call a first-class function value. The env word selects the ABI at
/// runtime: null → plain signature (named functions, capture-free
/// closures, extern functions used as values), non-null → the signature
/// with the trailing env parameter. Both arms are emitted and merged
/// with a phi so the choice is per-value, not per-site.
#[allow(clippy::too_many_arguments)]
pub(super) fn emit_fn_value_call<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    fn_value: BasicValueEnum<'ctx>,
    fn_params: &[Type],
    fn_ret: &Type,
    arg_values: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>>,
    call_label: &str,
) -> Result<ExprResult<'ctx>, String> {
    let fn_struct = match fn_value {
        BasicValueEnum::StructValue(s) => s,
        other => {
            return Err(format!(
                "Function value is not a {{fnptr, env}} struct: {:?}",
                other
            ))
        }
    };
    let fn_ptr = ctx
        .builder()
        .build_extract_value(fn_struct, 0, "call_fnptr")
        .unwrap()
        .into_pointer_value();
    let env = ctx
        .builder()
        .build_extract_value(fn_struct, 1, "call_env")
        .unwrap()
        .into_pointer_value();

    let plain_fn_ty = super::types::function_type_from_ruyi(ctx.context, fn_params, fn_ret);
    let env_fn_ty = super::types::function_type_with_trailing_env(ctx.context, fn_params, fn_ret);

    let current_fn = ctx
        .builder()
        .get_insert_block()
        .and_then(|bb| bb.get_parent())
        .ok_or("Function value call outside any function")?;
    let env_null = ctx.builder().build_is_null(env, "env_is_null").unwrap();
    let plain_bb = ctx
        .context
        .append_basic_block(current_fn, "fnval_call_plain");
    let env_bb = ctx.context.append_basic_block(current_fn, "fnval_call_env");
    let merge_bb = ctx
        .context
        .append_basic_block(current_fn, "fnval_call_merge");
    ctx.builder()
        .build_conditional_branch(env_null, plain_bb, env_bb)
        .unwrap();

    ctx.builder().position_at_end(plain_bb);
    let call_plain = ctx
        .builder()
        .build_indirect_call(plain_fn_ty, fn_ptr, &arg_values, call_label)
        .unwrap();
    let v_plain = call_plain.try_as_basic_value().basic();
    ctx.builder().build_unconditional_branch(merge_bb).unwrap();

    ctx.builder().position_at_end(env_bb);
    let mut args_env: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> = arg_values.clone();
    args_env.push(env.into());
    let call_env = ctx
        .builder()
        .build_indirect_call(env_fn_ty, fn_ptr, &args_env, call_label)
        .unwrap();
    let v_env = call_env.try_as_basic_value().basic();
    ctx.builder().build_unconditional_branch(merge_bb).unwrap();

    ctx.builder().position_at_end(merge_bb);

    let returns_value = !matches!(fn_ret, Type::Void | Type::Never);
    if returns_value {
        let (vp, ve) = match (v_plain, v_env) {
            (Some(vp), Some(ve)) => (vp, ve),
            _ => {
                return Err(format!(
                    "{}: call arms disagreed on result presence",
                    call_label
                ))
            }
        };
        let ret_llvm = ruyi_type_to_llvm(ctx.context, fn_ret);
        let phi = ctx.builder().build_phi(ret_llvm, "fnval_result").unwrap();
        phi.add_incoming(&[(&vp, plain_bb), (&ve, env_bb)]);
        Ok(ExprResult::new(phi.as_basic_value(), fn_ret.clone()))
    } else {
        Ok(ExprResult::new(
            BasicValueEnum::IntValue(ctx.context.i64_type().const_int(0, false)),
            Type::Void,
        ))
    }
}

/// Compile an expression into LLVM IR.
pub fn compile_expr<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    expr: &Expr,
) -> Result<ExprResult<'ctx>, String> {
    match expr {
        Expr::IntLiteral(n) => {
            // Byte literals: when the expected type is Byte, produce an i8 constant
            if matches!(ctx.expected_expr_type(), Some(Type::Byte)) {
                compile_byte_from_int(ctx, *n)
            } else {
                compile_int_literal(ctx, *n)
            }
        }
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
        Expr::NullAssert(inner) => {
            // EX-H6 修复：运行时检查值是否为 null，如果是则抛出 NullAssertionError
            let result = compile_expr(ctx, inner)?;
            let func = ctx
                .builder()
                .get_insert_block()
                .and_then(|bb| bb.get_parent())
                .ok_or("NullAssert: no current function")?;

            // 判断值是否为 null
            let is_null = match result.value {
                BasicValueEnum::PointerValue(ptr) => {
                    let ptr_int = ctx
                        .builder()
                        .build_ptr_to_int(ptr, ctx.context.i64_type(), "ptr_int")
                        .unwrap();
                    ctx.builder()
                        .build_int_compare(
                            inkwell::IntPredicate::EQ,
                            ptr_int,
                            ctx.context.i64_type().const_int(0, false),
                            "is_null",
                        )
                        .unwrap()
                }
                BasicValueEnum::IntValue(v) if matches!(result.ty, Type::Nullable(_)) => {
                    // Nullable int sentinel check (all ones = null; 0 is a
                    // legitimate int value and must not collide with null)
                    let sentinel = ctx.context.i64_type().const_all_ones();
                    ctx.builder()
                        .build_int_compare(
                            inkwell::IntPredicate::EQ,
                            v,
                            sentinel,
                            "is_null_sentinel",
                        )
                        .unwrap()
                }
                BasicValueEnum::FloatValue(v) if matches!(result.ty, Type::Nullable(_)) => {
                    // Nullable float sentinel check: all-ones bit pattern
                    // (NaN). Must test bits, not float equality.
                    let bits = ctx
                        .builder()
                        .build_bit_cast(v, ctx.context.i64_type(), "f_bits")
                        .unwrap()
                        .into_int_value();
                    ctx.builder()
                        .build_int_compare(
                            inkwell::IntPredicate::EQ,
                            bits,
                            ctx.context.i64_type().const_all_ones(),
                            "is_null_float",
                        )
                        .unwrap()
                }
                _ => {
                    // 非指针/非 nullable 类型不可能是 null，直接返回
                    return Ok(result);
                }
            };

            let null_bb = ctx.context.append_basic_block(func, "null_assert_fail");
            let ok_bb = ctx.context.append_basic_block(func, "null_assert_ok");
            ctx.builder()
                .build_conditional_branch(is_null, null_bb, ok_bb)
                .unwrap();

            // null 分支：抛出 NullAssertionError
            ctx.builder().position_at_end(null_bb);
            let msg = "NullAssertionError: value is null";
            let msg_ptr = ctx
                .builder()
                .build_global_string_ptr(msg, "null_assert_msg")
                .unwrap();
            let throw_fn = ctx
                .module
                .get_function("ruyi_throw")
                .expect("ruyi_throw not declared");
            ctx.builder()
                .build_call(
                    throw_fn,
                    &[msg_ptr.as_pointer_value().into()],
                    "throw_null_assert",
                )
                .unwrap();
            ctx.builder().build_unreachable().unwrap();

            // 正常分支：返回值
            ctx.builder().position_at_end(ok_bb);
            Ok(result)
        }
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
            type_params: _,
            params,
            return_type,
            body,
            is_async,
        } => {
            // A `fn` expression is a closure literal just like an arrow:
            // names it reads from the enclosing scope travel in the
            // environment block. Assignments to captured names are
            // rejected loudly unless the binding is a shared heap cell
            // (closure_shared_names), where mutation is the point.
            let analysis = super::capture::analyze(params, body);
            if let Some(assign_name) = analysis
                .assigned
                .iter()
                .find(|n| ctx.variables.contains_key(*n) && !ctx.closure_shared_names.contains(*n))
            {
                return Err(format!(
                    "Cannot assign to captured variable '{}' inside a closure \
                     (mutable captures are not supported yet)",
                    assign_name
                ));
            }
            let captures: Vec<(String, Type)> = analysis
                .reads
                .iter()
                .filter_map(|n| ctx.variables.get(n).map(|(_, ty)| (n.clone(), ty.clone())))
                .collect();
            let capture_ref: Option<&[(String, Type)]> = if captures.is_empty() {
                None
            } else {
                Some(captures.as_slice())
            };

            // Infer the closure's real signature the same way arrows do,
            // so call sites (Thread.spawn's trampoline, fn-value calls)
            // see actual parameter and return types instead of an empty
            // placeholder.
            let fallback_param_types = infer_fn_expr_param_types(params, body);
            let expected_fn = ctx.expected_expr_type().cloned().and_then(|t| match t {
                Type::Function {
                    params,
                    return_type,
                } => Some((params, *return_type)),
                _ => None,
            });
            let inferred_param_types: Vec<Type> = params
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    p.ty.as_ref()
                        .map(Type::from_annotation)
                        .or_else(|| expected_fn.as_ref().and_then(|(ps, _)| ps.get(i).cloned()))
                        .unwrap_or_else(|| {
                            fallback_param_types
                                .get(i)
                                .cloned()
                                .unwrap_or(Type::Dynamic)
                        })
                })
                .collect();
            let inferred_ret_type = return_type
                .as_ref()
                .map(Type::from_annotation)
                .or_else(|| expected_fn.as_ref().map(|(_, ret)| ret.clone()))
                .unwrap_or(Type::Void);
            let fn_expr_type = Type::Function {
                params: inferred_param_types.clone(),
                return_type: Box::new(inferred_ret_type.clone()),
            };

            let compile_captureless_fn =
                |ctx: &mut CodegenContext<'ctx, '_, '_>, fn_name: &str| -> Result<(), String> {
                    if *is_async {
                        super::async_codegen::compile_async_function(
                            ctx,
                            fn_name,
                            params,
                            return_type.as_ref(),
                            body,
                        )
                    } else {
                        super::decl::compile_function(
                            ctx,
                            fn_name,
                            params,
                            return_type.as_ref(),
                            Some(&inferred_param_types),
                            Some(&inferred_ret_type),
                            body,
                            capture_ref,
                        )
                    }
                };

            if let Some(func_name) = name {
                compile_captureless_fn(ctx, func_name)?;
                if let Some(func) = ctx.module.get_function(func_name) {
                    let fn_ptr = func.as_global_value().as_pointer_value();
                    Ok(ExprResult::new(
                        wrap_fn_value(ctx, Some(fn_ptr), capture_ref),
                        fn_expr_type,
                    ))
                } else {
                    Err(format!(
                        "Function not found after compilation: {}",
                        func_name
                    ))
                }
            } else {
                let anon_name = format!("__anon_{}", ctx.anon_counter);
                ctx.anon_counter += 1;
                compile_captureless_fn(ctx, &anon_name)?;
                if let Some(func) = ctx.module.get_function(&anon_name) {
                    let fn_ptr = func.as_global_value().as_pointer_value();
                    Ok(ExprResult::new(
                        wrap_fn_value(ctx, Some(fn_ptr), capture_ref),
                        fn_expr_type,
                    ))
                } else {
                    Err(format!(
                        "Function not found after compilation: {}",
                        anon_name
                    ))
                }
            }
        }
        Expr::ArrowFunction {
            params,
            return_type,
            body,
            is_async,
        } => {
            if *is_async {
                let async_arrow_name = format!("__async_arrow_{}", ctx.async_arrow_counter);
                ctx.async_arrow_counter += 1;
                let body_stmts = match body {
                    ArrowBody::Expr(expr) => vec![Statement::Return(Some(expr.clone()))],
                    ArrowBody::Block(stmts) => stmts.clone(),
                };
                super::async_codegen::compile_async_function(
                    ctx,
                    &async_arrow_name,
                    params,
                    return_type.as_ref(),
                    &body_stmts,
                )?;
                let func_ptr = ctx
                    .module
                    .get_function(&async_arrow_name)
                    .map(|f| BasicValueEnum::PointerValue(f.as_global_value().as_pointer_value()))
                    .ok_or_else(|| {
                        format!("Async arrow function not found: {}", async_arrow_name)
                    })?;
                return Ok(ExprResult::new(
                    func_ptr,
                    Type::Function {
                        params: vec![],
                        return_type: Box::new(Type::Dynamic),
                    },
                ));
            }
            let name = format!("__arrow_{}", ctx.arrow_counter);
            ctx.arrow_counter += 1;

            let body_stmts = match body {
                ArrowBody::Expr(expr) => {
                    vec![Statement::Return(Some(expr.clone()))]
                }
                ArrowBody::Block(stmts) => stmts.clone(),
            };

            // ── Capture analysis ────────────────────────────────────
            // Names the body reads but does not bind, filtered against
            // the variables visible at the creation site, become the
            // closure's environment. Assignments to captured names are
            // rejected loudly: by-value capture would diverge from the
            // outer binding, and silently corrupting it is worse.
            let analysis = super::capture::analyze(params, &body_stmts);
            if let Some(assign_name) = analysis
                .assigned
                .iter()
                .find(|n| ctx.variables.contains_key(*n) && !ctx.closure_shared_names.contains(*n))
            {
                return Err(format!(
                    "Cannot assign to captured variable '{}' inside a closure \
                     (mutable captures are not supported yet)",
                    assign_name
                ));
            }
            let captures: Vec<(String, Type)> = analysis
                .reads
                .iter()
                .filter_map(|n| ctx.variables.get(n).map(|(_, ty)| (n.clone(), ty.clone())))
                .collect();

            let fallback_param_types = infer_arrow_param_types(params, body);
            let expected_fn = ctx.expected_expr_type().cloned().and_then(|t| match t {
                Type::Function {
                    params,
                    return_type,
                } => Some((params, *return_type)),
                _ => None,
            });
            let inferred_param_types: Vec<Type> = params
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    p.ty.as_ref()
                        .map(Type::from_annotation)
                        .or_else(|| expected_fn.as_ref().and_then(|(ps, _)| ps.get(i).cloned()))
                        .unwrap_or_else(|| {
                            fallback_param_types
                                .get(i)
                                .cloned()
                                .unwrap_or(Type::Dynamic)
                        })
                })
                .collect();

            // Infer return type from body when not annotated; prefer the
            // expected function return type so arrow signatures match the
            // parameter slot they are passed into.
            let inferred_ret_type = return_type
                .as_ref()
                .map(Type::from_annotation)
                .or_else(|| expected_fn.as_ref().map(|(_, ret)| ret.clone()))
                .unwrap_or_else(|| {
                    let mut param_types_map = std::collections::HashMap::new();
                    for (param, ty) in params.iter().zip(inferred_param_types.iter()) {
                        if let crate::parser::ast::Pattern::Identifier(param_name) = &param.pattern
                        {
                            param_types_map.insert(param_name.clone(), ty.clone());
                        }
                    }
                    match body {
                        ArrowBody::Expr(expr) => infer_expr_type(ctx, expr, &param_types_map),
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
                if captures.is_empty() {
                    None
                } else {
                    Some(&captures)
                },
            )?;

            if let Some(func) = ctx.module.get_function(&name) {
                let fn_ptr = func.as_global_value().as_pointer_value();
                let fn_value = wrap_fn_value(
                    ctx,
                    Some(fn_ptr),
                    if captures.is_empty() {
                        None
                    } else {
                        Some(captures.as_slice())
                    },
                );
                Ok(ExprResult::new(
                    fn_value,
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
        Expr::Sequence(exprs) => compile_tuple_literal(ctx, exprs),
        Expr::Block(stmts) => {
            use crate::codegen::stmt::compile_stmt;
            if stmts.is_empty() {
                return Ok(ExprResult::new(
                    BasicValueEnum::IntValue(ctx.context.i64_type().const_int(0, false)),
                    Type::Void,
                ));
            }

            let (init_stmts, last_stmt) = stmts.split_at(stmts.len() - 1);
            for s in init_stmts {
                compile_stmt(ctx, s)?;
            }

            // 最后一条语句：如果是表达式语句，取其值作为块的返回值
            match &last_stmt[0] {
                Statement::Expression(expr) => compile_expr(ctx, expr),
                other => {
                    compile_stmt(ctx, other)?;
                    Ok(ExprResult::new(
                        BasicValueEnum::IntValue(ctx.context.i64_type().const_int(0, false)),
                        Type::Void,
                    ))
                }
            }
        }
        _ => Err(format!("Unsupported expression: {:?}", expr)),
    }
}

fn compile_int_literal<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    n: i64,
) -> Result<ExprResult<'ctx>, String> {
    let val = ctx.context.i64_type().const_int(n as u64, true);
    Ok(ExprResult::new(BasicValueEnum::IntValue(val), Type::Int))
}

fn compile_byte_from_int<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    n: i64,
) -> Result<ExprResult<'ctx>, String> {
    let val = ctx.context.i8_type().const_int((n & 0xFF) as u64, false);
    Ok(ExprResult::new(BasicValueEnum::IntValue(val), Type::Byte))
}

fn compile_float_literal<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    n: f64,
) -> Result<ExprResult<'ctx>, String> {
    let val = ctx.context.f64_type().const_float(n);
    Ok(ExprResult::new(
        BasicValueEnum::FloatValue(val),
        Type::Float,
    ))
}

fn compile_bool_literal<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    b: bool,
) -> Result<ExprResult<'ctx>, String> {
    let val = ctx.context.bool_type().const_int(b as u64, false);
    Ok(ExprResult::new(BasicValueEnum::IntValue(val), Type::Bool))
}

fn compile_string_literal<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    s: &str,
) -> Result<ExprResult<'ctx>, String> {
    let global = ctx.builder().build_global_string_ptr(s, "str_lit").unwrap();
    Ok(ExprResult::new(
        BasicValueEnum::PointerValue(global.as_pointer_value()),
        Type::String,
    ))
}

fn compile_template_literal<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
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
                let global = ctx
                    .builder()
                    .build_global_string_ptr(s, "tmpl_str")
                    .unwrap();
                let str_ptr = global.as_pointer_value();
                match result_ptr {
                    None => result_ptr = Some(str_ptr),
                    Some(prev) => {
                        let res = ctx
                            .builder()
                            .build_call(concat_fn, &[prev.into(), str_ptr.into()], "str_concat")
                            .unwrap()
                            .try_as_basic_value()
                            .unwrap_basic()
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
                            .builder()
                            .build_call(concat_fn, &[prev.into(), expr_ptr.into()], "str_concat")
                            .unwrap()
                            .try_as_basic_value()
                            .unwrap_basic()
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

fn compile_null_literal<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
) -> Result<ExprResult<'ctx>, String> {
    // The expected type (e.g. the declared type of a `let` or a call
    // parameter) is the only reliable source for the sentinel choice. The
    // enclosing return type is merely a fallback for bare `return null;`
    // positions — it must not leak into e.g. a `string?` argument inside
    // a `float?`-returning function, which would produce a NaN sentinel
    // the caller then mis-converts.
    let sentinel_ty = ctx
        .expected_expr_type()
        .or_else(|| ctx.current_return_type());
    let is_nullable_int =
        matches!(sentinel_ty, Some(Type::Nullable(ref inner)) if **inner == Type::Int);
    let is_nullable_float =
        matches!(sentinel_ty, Some(Type::Nullable(ref inner)) if **inner == Type::Float);
    if is_nullable_int {
        // Use all-ones as the null sentinel for Nullable<int>: 0 is a
        // legitimate int value (e.g. the first element of an iterator)
        // and must not be mistaken for null by `if let` / `while let`
        // and null comparisons.
        let sentinel = ctx.context.i64_type().const_all_ones();
        Ok(ExprResult::new(
            BasicValueEnum::IntValue(sentinel),
            Type::Nullable(Box::new(Type::Int)),
        ))
    } else if is_nullable_float {
        // Mirror the Nullable<int> convention: the all-ones bit pattern
        // (a NaN) is the null sentinel for Nullable<float>. Consumers
        // must compare the bit pattern — `NaN == NaN` is false under
        // float semantics.
        let bits = ctx.context.i64_type().const_all_ones();
        let sentinel = ctx
            .builder()
            .build_bit_cast(bits, ctx.context.f64_type(), "null_f64")
            .unwrap()
            .into_float_value();
        Ok(ExprResult::new(
            BasicValueEnum::FloatValue(sentinel),
            Type::Nullable(Box::new(Type::Float)),
        ))
    } else {
        let null_ptr = ctx.context.ptr_type(Default::default()).const_null();
        Ok(ExprResult::new(
            BasicValueEnum::PointerValue(null_ptr),
            Type::Null,
        ))
    }
}

pub fn compile_bigint_literal<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    n: &str,
) -> Result<ExprResult<'ctx>, String> {
    let global = ctx
        .builder()
        .build_global_string_ptr(n, "bigint_lit")
        .unwrap();
    let str_ptr = global.as_pointer_value();
    let bigint_ptr =
        super::builtins::build_ruyi_bigint_from_str(ctx.builder(), ctx.module, str_ptr)?;
    Ok(ExprResult::new(
        BasicValueEnum::PointerValue(bigint_ptr),
        Type::BigInt,
    ))
}

fn compile_identifier<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    name: &str,
) -> Result<ExprResult<'ctx>, String> {
    match ctx.lookup_variable(name) {
        Some((ptr, ty)) => {
            let val = ctx
                .builder()
                .build_load(ruyi_type_to_llvm(ctx.context, &ty), ptr, name)
                .unwrap();
            Ok(ExprResult::new(val, ty))
        }
        None => {
            // Check if it's a function
            if let Some(func) = ctx.module.get_function(name) {
                let fn_ptr = func.as_global_value().as_pointer_value();
                Ok(ExprResult::new(
                    wrap_fn_value(ctx, Some(fn_ptr), None),
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
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    object: &Expr,
    property: &crate::parser::ast::MemberProperty,
    optional: bool,
) -> Result<ExprResult<'ctx>, String> {
    match property {
        crate::parser::ast::MemberProperty::Expr(key_expr) => {
            let obj_result = compile_expr(ctx, object)?;
            // A `T?` receiver only reaches an index expression after a null
            // check proved the value present, so its layout is the inner
            // type's. The rest of this arm already unwraps Nullable for class
            // and object receivers; without it here `args[0]` fell through to
            // the runtime key lookup and segfaulted.
            let mut indexed_ty = obj_result.ty.clone();
            while let Type::Nullable(inner) = indexed_ty {
                indexed_ty = *inner;
            }
            if let Type::Array(elem_ty) = &indexed_ty {
                let arr_ptr = value_to_i8_ptr(ctx, &obj_result.value)?;
                let index_val = match key_expr.as_ref() {
                    crate::parser::ast::Expr::IntLiteral(idx) => {
                        ctx.context.i64_type().const_int(*idx as u64, false)
                    }
                    _ => {
                        let key_result = compile_expr(ctx, key_expr)?;
                        match key_result.value {
                            BasicValueEnum::IntValue(v) => v,
                            BasicValueEnum::FloatValue(v) => ctx
                                .builder()
                                .build_float_to_signed_int(v, ctx.context.i64_type(), "idx_f2i")
                                .unwrap(),
                            _ => return Err("Array index must be an integer".to_string()),
                        }
                    }
                };
                // Dynamic/Function elements are heap-boxed 16-byte structs
                // behind an 8-byte pointer word (see
                // `build_box_struct_to_word`); load the word, then the
                // struct out of the box.
                if matches!(elem_ty.as_ref(), Type::Dynamic | Type::Function { .. }) {
                    let elem_val = super::builtins::build_builtin_array_get(
                        ctx.builder(),
                        ctx.module,
                        arr_ptr,
                        index_val,
                    );
                    let box_ptr = ctx
                        .builder()
                        .build_int_to_ptr(
                            elem_val,
                            ctx.context.ptr_type(Default::default()),
                            "dyn_elem_box",
                        )
                        .unwrap();
                    let dyn_struct_ty = ruyi_type_to_llvm(ctx.context, &Type::Dynamic);
                    let loaded = ctx
                        .builder()
                        .build_load(dyn_struct_ty, box_ptr, "dyn_elem")
                        .unwrap();
                    return Ok(ExprResult::new(loaded, Type::Dynamic));
                }

                let elem_val = super::builtins::build_builtin_array_get(
                    ctx.builder(),
                    ctx.module,
                    arr_ptr,
                    index_val,
                );
                // The runtime stores every element as an i64 universal
                // register; convert the loaded word back to the element
                // type's natural representation so downstream code sees the
                // precise type (e.g. `parts[i]` on Array<string> is a string
                // pointer, not an int).
                return Ok(match elem_ty.as_ref() {
                    Type::Float => {
                        let f = ctx
                            .builder()
                            .build_bit_cast(elem_val, ctx.context.f64_type(), "elem_f64")
                            .unwrap();
                        ExprResult::new(f, Type::Float)
                    }
                    Type::Dynamic | Type::Function { .. } => {
                        unreachable!("handled above")
                    }
                    Type::String | Type::Named(_, _) | Type::Array(_) | Type::Object(_) => {
                        let ptr = ctx
                            .builder()
                            .build_int_to_ptr(
                                elem_val,
                                ctx.context.ptr_type(Default::default()),
                                "elem_ptr",
                            )
                            .unwrap();
                        ExprResult::new(BasicValueEnum::PointerValue(ptr), *elem_ty.clone())
                    }
                    _ => ExprResult::new(BasicValueEnum::IntValue(elem_val), Type::Int),
                });
            }

            if let crate::parser::ast::Expr::StringLiteral(key_str) = key_expr.as_ref() {
                if let Some(class_name) = resolve_class_from_type(&obj_result.ty) {
                    let obj_ptr = value_to_i8_ptr(ctx, &obj_result.value)?;
                    let (field_ptr, field_ty) =
                        class_field_access(ctx, obj_ptr, &class_name, key_str, &obj_result.ty)?;
                    let value = emit_field_load(ctx, field_ptr, &field_ty, key_str);
                    return Ok(ExprResult::new(value, field_ty));
                }
                if let Type::Object(fields) = &obj_result.ty {
                    let field = fields
                        .iter()
                        .find(|f| f.name == key_str.as_str())
                        .ok_or_else(|| format!("Unknown field: {} in object", key_str))?;
                    let obj_ptr = value_to_i8_ptr(ctx, &obj_result.value)?;
                    let offset = ctx.context.i32_type().const_int(
                        (fields.iter().position(|f| f.name == field.name).unwrap() * 8) as u64,
                        false,
                    );
                    let field_ptr = unsafe {
                        ctx.builder()
                            .build_gep(
                                ctx.context.i8_type(),
                                obj_ptr,
                                &[offset],
                                &format!("{}_ptr", field.name),
                            )
                            .unwrap()
                    };
                    let typed_ptr = ctx
                        .builder()
                        .build_pointer_cast(
                            field_ptr,
                            ctx.context.ptr_type(Default::default()),
                            &format!("{}_typed", field.name),
                        )
                        .unwrap();
                    let field_val = ctx
                        .builder()
                        .build_load(
                            ruyi_type_to_llvm(ctx.context, &field.ty),
                            typed_ptr,
                            &field.name,
                        )
                        .unwrap();
                    return Ok(ExprResult::new(field_val, field.ty.clone()));
                }
            }

            let key_result = compile_expr(ctx, key_expr)?;
            let obj_ptr = value_to_i8_ptr(ctx, &obj_result.value)?;
            let key_ptr = value_to_i8_ptr(ctx, &key_result.value)?;

            // Object literals use a flat slot layout ([value_0][value_1]...)
            // without embedded key names, so the runtime ruyi_obj_get
            // (which expects [field_count][key,value pairs]) cannot be
            // used on them. When the shape is statically known, emit a
            // strcmp dispatch chain over the field names and box the
            // matched slot value into a Dynamic result.
            if let Type::Object(fields) = &obj_result.ty {
                if !fields.is_empty() {
                    let func = ctx.current_function.ok_or("No current function")?;
                    let i8_ptr_ty = ctx.context.ptr_type(Default::default());
                    let dyn_llvm_ty = ruyi_type_to_llvm(ctx.context, &Type::Dynamic);

                    // Declare strcmp if not present.
                    let strcmp_fn = match ctx.module.get_function("strcmp") {
                        Some(f) => f,
                        None => {
                            let strcmp_ty = ctx
                                .context
                                .i32_type()
                                .fn_type(&[i8_ptr_ty.into(), i8_ptr_ty.into()], false);
                            ctx.module.add_function("strcmp", strcmp_ty, None)
                        }
                    };

                    let merge_bb = ctx.context.append_basic_block(func, "obj_get_merge");
                    let miss_bb = ctx.context.append_basic_block(func, "obj_get_miss");
                    let mut incoming: Vec<(
                        BasicValueEnum<'ctx>,
                        inkwell::basic_block::BasicBlock<'ctx>,
                    )> = Vec::new();

                    // Chain: entry -> test_0 -> test_1 -> ... -> miss.
                    // All test blocks are created up front (appended at the
                    // function tail) so branch targets exist before use and
                    // names never collide. Blocks must never be inserted
                    // mid-function: inserting a block between the current
                    // block and its pending instructions would make the
                    // builder fall through into the new block and emit the
                    // current block's instructions there, producing invalid
                    // IR with two terminators.
                    let test_bbs: Vec<inkwell::basic_block::BasicBlock<'ctx>> = fields
                        .iter()
                        .enumerate()
                        .map(|(idx, _)| {
                            ctx.context
                                .append_basic_block(func, &format!("obj_get_test_{}", idx))
                        })
                        .collect();

                    let entry_bb = ctx.builder().get_insert_block().unwrap();
                    ctx.builder().position_at_end(entry_bb);
                    ctx.builder()
                        .build_unconditional_branch(test_bbs[0])
                        .unwrap();

                    for (idx, field) in fields.iter().enumerate() {
                        let test_bb = test_bbs[idx];
                        let match_bb = ctx
                            .context
                            .append_basic_block(func, &format!("obj_get_match_{}", idx));
                        let next_test_bb = if idx + 1 < fields.len() {
                            test_bbs[idx + 1]
                        } else {
                            miss_bb
                        };

                        // Test: strcmp(key, "name") == 0 ?
                        ctx.builder().position_at_end(test_bb);
                        let key_name_global = ctx
                            .builder()
                            .build_global_string_ptr(&field.name, "obj_key_name")
                            .unwrap();
                        let cmp = ctx
                            .builder()
                            .build_call(
                                strcmp_fn,
                                &[key_ptr.into(), key_name_global.as_pointer_value().into()],
                                "key_cmp",
                            )
                            .unwrap()
                            .try_as_basic_value()
                            .unwrap_basic()
                            .into_int_value();
                        let eq = ctx
                            .builder()
                            .build_int_compare(
                                inkwell::IntPredicate::EQ,
                                cmp,
                                ctx.context.i32_type().const_int(0, false),
                                "key_eq",
                            )
                            .unwrap();
                        ctx.builder()
                            .build_conditional_branch(eq, match_bb, next_test_bb)
                            .unwrap();

                        // Match: load flat slot and box into Dynamic.
                        ctx.builder().position_at_end(match_bb);
                        let offset = ctx.context.i32_type().const_int((idx * 8) as u64, false);
                        let slot_ptr = unsafe {
                            ctx.builder()
                                .build_gep(
                                    ctx.context.i8_type(),
                                    obj_ptr,
                                    &[offset],
                                    "obj_field_slot",
                                )
                                .unwrap()
                        };
                        let field_llvm_ty = ruyi_type_to_llvm(ctx.context, &field.ty);
                        let raw = ctx
                            .builder()
                            .build_load(field_llvm_ty, slot_ptr, &field.name)
                            .unwrap();
                        let boxed = build_box_dynamic(ctx, raw, &field.ty);
                        ctx.builder().build_unconditional_branch(merge_bb).unwrap();
                        incoming.push((boxed, match_bb));
                    }
                    // Last test misses to the miss block.

                    // Miss: null dynamic {0, null}.
                    ctx.builder().position_at_end(miss_bb);
                    let null_dyn = ruyi_type_to_llvm(ctx.context, &Type::Dynamic)
                        .into_struct_type()
                        .const_zero();
                    ctx.builder().build_unconditional_branch(merge_bb).unwrap();
                    incoming.push((BasicValueEnum::StructValue(null_dyn), miss_bb));

                    ctx.builder().position_at_end(merge_bb);
                    let phi = ctx
                        .builder()
                        .build_phi(dyn_llvm_ty, "obj_get_result")
                        .unwrap();
                    phi.add_incoming(
                        &incoming
                            .iter()
                            .map(|(v, bb)| (v as &dyn inkwell::values::BasicValue, *bb))
                            .collect::<Vec<_>>(),
                    );
                    return Ok(ExprResult::new(phi.as_basic_value(), Type::Dynamic));
                }
            }

            let result =
                super::builtins::build_ruyi_obj_get(ctx.builder(), ctx.module, obj_ptr, key_ptr);
            Ok(ExprResult::new(
                BasicValueEnum::PointerValue(result),
                Type::Dynamic,
            ))
        }
        crate::parser::ast::MemberProperty::Ident(field_name) => {
            if optional {
                compile_optional_member_access(ctx, object, field_name)
            } else {
                // Static field access: `ClassName.fieldName` loads from a
                // module-level global (e.g. `Signal.TERM`). Must be checked
                // before compiling the object expression, since the class
                // name is not a runtime variable.
                if let Expr::Identifier(class_name) = object {
                    let static_key = format!("{}_{}", class_name, field_name);
                    if let Some(field_ty) = ctx.static_fields.get(&static_key).cloned() {
                        if let Some(global) = ctx.module.get_global(&static_key) {
                            let llvm_ty = ruyi_type_to_llvm(ctx.context, &field_ty);
                            let typed_ptr = ctx
                                .builder()
                                .build_bit_cast(
                                    global.as_pointer_value(),
                                    ctx.context.ptr_type(Default::default()),
                                    "static_field_ptr",
                                )
                                .unwrap()
                                .into_pointer_value();
                            let value = ctx
                                .builder()
                                .build_load(llvm_ty, typed_ptr, field_name)
                                .unwrap();
                            return Ok(ExprResult::new(value, field_ty));
                        }
                    }
                }

                let obj_result = compile_expr(ctx, object)?;
                if matches!(obj_result.ty, Type::Tuple(_)) {
                    compile_tuple_field_access(ctx, &obj_result, field_name)
                } else {
                    compile_simple_member_access(ctx, object, field_name)
                }
            }
        }
    }
}

fn value_to_i8_ptr<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    value: &BasicValueEnum<'ctx>,
) -> Result<inkwell::values::PointerValue<'ctx>, String> {
    let i8_ptr_ty = ctx.context.ptr_type(Default::default());
    match value {
        BasicValueEnum::PointerValue(p) => Ok(ctx
            .builder()
            .build_pointer_cast(*p, i8_ptr_ty, "cast_i8_ptr")
            .unwrap()),
        BasicValueEnum::IntValue(v) => {
            let func = ctx
                .module
                .get_function("ruyi_int_to_string")
                .ok_or_else(|| "ruyi_int_to_string not declared".to_string())?;
            let call = ctx
                .builder()
                .build_call(func, &[(*v).into()], "int_to_str")
                .unwrap()
                .try_as_basic_value()
                .basic()
                .ok_or_else(|| "ruyi_int_to_string did not return a value".to_string())?;
            Ok(call.into_pointer_value())
        }
        BasicValueEnum::FloatValue(v) => {
            let func = ctx
                .module
                .get_function("ruyi_float_to_string")
                .ok_or_else(|| "ruyi_float_to_string not declared".to_string())?;
            let call = ctx
                .builder()
                .build_call(func, &[(*v).into()], "float_to_str")
                .unwrap()
                .try_as_basic_value()
                .basic()
                .ok_or_else(|| "ruyi_float_to_string did not return a value".to_string())?;
            Ok(call.into_pointer_value())
        }
        _ => Err("Cannot convert value to i8* for runtime call".to_string()),
    }
}

fn compile_tuple_field_access<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    obj_result: &ExprResult<'ctx>,
    field_name: &str,
) -> Result<ExprResult<'ctx>, String> {
    let index = field_name.parse::<usize>().map_err(|_| {
        format!(
            "Tuple field access requires numeric index, got: {}",
            field_name
        )
    })?;

    let field_ty = match &obj_result.ty {
        Type::Tuple(types) => types.get(index).cloned().unwrap_or(Type::Dynamic),
        _ => return Err(format!("Not a tuple type: {:?}", obj_result.ty)),
    };

    let struct_val = match obj_result.value {
        BasicValueEnum::StructValue(s) => s,
        _ => return Err("Tuple value is not a struct".to_string()),
    };

    let field_val = ctx
        .builder()
        .build_extract_value(struct_val, index as u32, &format!("tuple_field_{}", index))
        .unwrap();

    Ok(ExprResult::new(field_val, field_ty))
}

/// Extract class name from a type, unwrapping `Type::Nullable` and
/// handling `Type::Named`, `Type::Array`, and `Type::Generic` variants.
/// Used by field access and `?.` paths to determine which class's field
/// table to query.
fn resolve_class_from_type(obj_ty: &Type) -> Option<String> {
    match obj_ty {
        Type::Named(n, _) => Some(n.clone()),
        Type::Array(_) => Some("Array".to_string()),
        Type::Generic { base, .. } => Some(base.clone()),
        Type::Nullable(inner) => resolve_class_from_type(inner),
        _ => None,
    }
}

/// Strip every `Type::Nullable` wrapper off a type.
///
/// `T?` carries no extra runtime representation — it shares `T`'s slot with
/// null as the sentinel — so any operation reached inside a non-null branch
/// can dispatch on the inner shape. Comparing a `string?` against a string
/// without this unwrap fell through to pointer identity, so two equal
/// contents at different addresses reported `false`.
fn strip_nullable(ty: &Type) -> Type {
    let mut unwrapped = ty.clone();
    while let Type::Nullable(inner) = unwrapped {
        unwrapped = *inner;
    }
    unwrapped
}

/// Substitute a generic class's type parameters into a declared field
/// type, so `Ok<T, E>.value` reports `int` on an `Ok<int, string>` rather
/// than the erased parameter `T`. Without this, method calls chained off
/// the field dispatch on the type parameter itself (`T_toString`) instead
/// of the concrete instantiation.
fn substitute_class_field_type(
    ctx: &CodegenContext<'_, '_, '_>,
    class_name: &str,
    receiver: &Type,
    raw_field_ty: &Type,
) -> Type {
    let receiver_type_args = receiver_type_args_from(ctx, receiver);
    if receiver_type_args.is_empty() {
        return raw_field_ty.clone();
    }
    let param_names: Vec<String> = ctx
        .generic_classes
        .get(class_name)
        .map(|(params, _)| params.clone())
        .unwrap_or_default();
    let mut bindings = std::collections::HashMap::new();
    for (name, arg) in param_names.iter().zip(receiver_type_args.iter()) {
        bindings.insert(name.clone(), arg.clone());
    }
    substitute_type_args_in_field(raw_field_ty, &bindings)
}

/// Perform GEP-based field access on a class instance.
///
/// Given a class name and field name, looks up the field index and type
/// from `ctx.class_fields`, gets the LLVM struct type from
/// `ctx.class_struct_types`, casts the object pointer, and computes
/// the GEP to the field.
///
/// `receiver` is the receiver's static type; its type arguments are
/// substituted into the declared field type so codegen produces the
/// correct LLVM slot layout for the actual instantiation. Without this
/// `Ok<T, E>.value` would be loaded as a bare `ptr` (the erased form for
/// any type parameter) instead of an `i64` for `Ok<int, void>`.
///
/// Returns the field pointer (ready for load/store) and the field type.
fn class_field_access<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    obj_ptr: inkwell::values::PointerValue<'ctx>,
    class_name: &str,
    field_name: &str,
    receiver: &Type,
) -> Result<(inkwell::values::PointerValue<'ctx>, Type), String> {
    let fields = ctx
        .class_fields
        .get(class_name)
        .ok_or_else(|| format!("Unknown class: {}", class_name))?;
    let raw_field_ty = fields
        .iter()
        .find(|(n, _)| n == field_name)
        .map(|(_, ty)| ty.clone())
        .ok_or_else(|| format!("Unknown field: {} in class {}", field_name, class_name))?;
    // Substitute the class's type parameters (`T` → int, `E` → string, …)
    // so a generic field reports its instantiated type rather than the
    // erased type parameter.
    let field_ty = substitute_class_field_type(ctx, class_name, receiver, &raw_field_ty);
    let struct_type = ctx
        .class_struct_types
        .get(class_name)
        .ok_or_else(|| format!("No struct type for class: {}", class_name))?;
    let struct_ptr = ctx
        .builder()
        .build_pointer_cast(
            obj_ptr,
            ctx.context.ptr_type(Default::default()),
            &format!("{}_cast", class_name),
        )
        .unwrap();
    let field_index = fields.iter().position(|(n, _)| n == field_name).unwrap();
    let i32_ty = ctx.context.i32_type();
    let field_ptr = unsafe {
        ctx.builder()
            .build_gep(
                *struct_type,
                struct_ptr,
                &[
                    i32_ty.const_int(0, false),
                    i32_ty.const_int(field_index as u64, false),
                ],
                &format!("{}_ptr", field_name),
            )
            .unwrap()
    };
    Ok((field_ptr, field_ty))
}

/// Extract a class instance's concrete type arguments from its static
/// type so the field/method lookup can instantiate the generic class's
/// declared types (`T`, `E`, …) against the actual values.
///
/// Returns an empty slice for non-generic receivers, so callers can pass
/// it unconditionally without first checking whether the class is generic.
///
/// When the receiver's static type is a bare `Type::Named("Ok", _)` but the
/// variable's actual type is `Ok<int, string>` (recorded in the type
/// environment), this walks the env to recover the missing type
/// arguments. Without this fallback, generic class receivers in
/// post-typecheck ASTs would be erased to their bare class name and the
/// field type would stay at its `T` placeholder.
fn receiver_type_args_from<'ctx>(ctx: &CodegenContext<'ctx, '_, '_>, ty: &Type) -> Vec<Type> {
    match ty {
        Type::Nullable(inner) => receiver_type_args_from(ctx, inner),
        Type::Generic { args, .. } => args.clone(),
        Type::Named(name, _) => {
            // Fallback for cases where the AST still carries the bare
            // class name (e.g. union alias field forwarding). Look up
            // the class definition in the type environment to recover
            // its generic parameters; this is a best-effort fallback
            // and silently returns an empty list when the env is
            // unavailable or the lookup misses.
            if let Some(env) = ctx.type_environment {
                if let Some(resolved) = env.lookup(name) {
                    if let Type::Generic { args, .. } = resolved {
                        return args.clone();
                    }
                }
            }
            Vec::new()
        }
        _ => Vec::new(),
    }
}

/// Walk `ty` and replace each `Type::Named(name, _)` whose `name` appears
/// in `bindings` with the bound type, recursing through the same
/// composite variants that `ruyi_type_to_llvm` understands. Pure copy
/// of the typechecker helper but lives in the codegen crate so it can be
/// invoked without crossing crate boundaries at runtime.
fn substitute_type_args_in_field(
    ty: &Type,
    bindings: &std::collections::HashMap<String, Type>,
) -> Type {
    if bindings.is_empty() {
        return ty.clone();
    }
    match ty {
        Type::Named(name, _) => match bindings.get(name) {
            Some(bound) => bound.clone(),
            None => ty.clone(),
        },
        Type::Nullable(inner) => {
            Type::Nullable(Box::new(substitute_type_args_in_field(inner, bindings)))
        }
        Type::Array(inner) => Type::Array(Box::new(substitute_type_args_in_field(inner, bindings))),
        Type::Future(inner) => {
            Type::Future(Box::new(substitute_type_args_in_field(inner, bindings)))
        }
        Type::Tuple(elems) => Type::Tuple(
            elems
                .iter()
                .map(|e| substitute_type_args_in_field(e, bindings))
                .collect(),
        ),
        Type::Object(fields) => Type::Object(
            fields
                .iter()
                .map(|f| crate::typechecker::ObjectField {
                    name: f.name.clone(),
                    ty: substitute_type_args_in_field(&f.ty, bindings),
                    optional: f.optional,
                })
                .collect(),
        ),
        Type::Function {
            params,
            return_type,
        } => Type::Function {
            params: params
                .iter()
                .map(|p| substitute_type_args_in_field(p, bindings))
                .collect(),
            return_type: Box::new(substitute_type_args_in_field(return_type, bindings)),
        },
        Type::Generic { base, args } => Type::Generic {
            base: base.clone(),
            args: args
                .iter()
                .map(|a| substitute_type_args_in_field(a, bindings))
                .collect(),
        },
        Type::Union(elems) => Type::Union(
            elems
                .iter()
                .map(|e| substitute_type_args_in_field(e, bindings))
                .collect(),
        ),
        _ => ty.clone(),
    }
}

/**
 * 判断 Ruyi 类型是否为泛型类型参数（T, U, V 等）。
 */
fn is_generic_type_param(ty: &Type) -> bool {
    match ty {
        Type::Named(name, _) => is_type_param_name(name),
        Type::TypeVar(_) => true,
        _ => false,
    }
}

/**
 * 将值存入类字段，自动处理泛型字段的类型适配。
 *
 * 当字段为泛型参数（T 等）时，结构体槽位统一为 i8*。
 * 若实际值为 i64/f64 等非指针类型，自动插入 inttoptr/bitcast 转换。
 *
 * @param field_ptr 字段指针（来自 class_field_access 的 GEP 结果）
 * @param field_ty  字段的 Ruyi 类型声明
 * @param value     待存入的 LLVM 值
 * @param value_ty  待存入值的 Ruyi 类型
 */
fn emit_field_store<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    field_ptr: inkwell::values::PointerValue<'ctx>,
    field_ty: &Type,
    value: BasicValueEnum<'ctx>,
    value_ty: &Type,
) {
    // Dynamic boxing 优先级最高
    if *field_ty == Type::Dynamic {
        // If the value is already a Dynamic struct, store it as-is.
        // Otherwise box the concrete LLVM value (pointer/int/float) so it
        // occupies the full {i64, i8*} slot. This matters for FFI functions
        // whose LLVM signature returns i8* but whose Ruyi type is Dynamic:
        // without boxing, only the first word of the slot is written and the
        // second word becomes garbage, corrupting later unboxing.
        if let BasicValueEnum::StructValue(_) = value {
            ctx.builder().build_store(field_ptr, value).unwrap();
        } else {
            let dyn_val = build_box_dynamic(ctx, value, value_ty);
            ctx.builder().build_store(field_ptr, dyn_val).unwrap();
        }
        return;
    }

    let field_llvm_ty = ruyi_type_to_llvm(ctx.context, field_ty);

    // 非泛型字段：类型匹配时直接 store
    if !is_generic_type_param(field_ty) && field_llvm_ty == value.get_type() {
        ctx.builder().build_store(field_ptr, value).unwrap();
        return;
    }

    // 泛型字段（槽位 = i8*）：根据实际值类型做适配
    if field_llvm_ty.is_pointer_type() {
        match value {
            BasicValueEnum::IntValue(int_val) => {
                // i64 / i1 → inttoptr i8*
                let ptr_val = ctx
                    .builder()
                    .build_int_to_ptr(
                        int_val,
                        ctx.context.ptr_type(Default::default()),
                        "field_int_to_ptr",
                    )
                    .unwrap();
                ctx.builder().build_store(field_ptr, ptr_val).unwrap();
            }
            BasicValueEnum::FloatValue(float_val) => {
                // f64 → bitcast i64 → inttoptr i8*
                let as_int = ctx
                    .builder()
                    .build_bit_cast(float_val, ctx.context.i64_type(), "field_f64_as_i64")
                    .unwrap();
                let as_ptr = ctx
                    .builder()
                    .build_int_to_ptr(
                        as_int.into_int_value(),
                        ctx.context.ptr_type(Default::default()),
                        "field_f64_to_ptr",
                    )
                    .unwrap();
                ctx.builder().build_store(field_ptr, as_ptr).unwrap();
            }
            // dyn {i64 tag, ptr data} stored into a class/reference field
            // must be unboxed: writing the full 16-byte struct into an
            // 8-byte slot leaves the tag word (e.g. 0x1d) in the field and
            // later method calls dereference garbage.
            BasicValueEnum::StructValue(sv) if *value_ty == Type::Dynamic => {
                let data = ctx
                    .builder()
                    .build_extract_value(sv, 1, "field_dyn_unbox")
                    .unwrap()
                    .into_pointer_value();
                ctx.builder().build_store(field_ptr, data).unwrap();
            }
            _ => {
                // 指针类型（i8*）直接 store
                ctx.builder().build_store(field_ptr, value).unwrap();
            }
        }
        return;
    }

    // 兑底：直接 store
    ctx.builder().build_store(field_ptr, value).unwrap();
}

/**
 * 从类字段加载值，自动处理泛型字段的类型适配。
 *
 * 当字段为泛型参数时，槽位为 i8*。若调用方期望 int/float 等值类型，
 * 自动插入 ptrtoint/bitcast 反向转换。
 *
 * @param field_ptr  字段指针（来自 class_field_access 的 GEP 结果）
 * @param field_ty   字段的 Ruyi 类型声明
 * @param field_name 字段名（用于 LLVM IR 中的调试标识）
 * @return 加载后的 LLVM 值
 */
fn emit_field_load<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    field_ptr: inkwell::values::PointerValue<'ctx>,
    field_ty: &Type,
    field_name: &str,
) -> BasicValueEnum<'ctx> {
    let llvm_ty = ruyi_type_to_llvm(ctx.context, field_ty);

    // 对于显式 Float 字段（非泛型场景），保留原有的 bitcast 工作流
    // （struct 可能以 i64 存储 f64，需要 reinterpret）
    if *field_ty == Type::Float && !llvm_ty.is_float_type() {
        let i64_ptr = ctx
            .builder()
            .build_bit_cast(
                field_ptr,
                ctx.context.ptr_type(Default::default()),
                "field_i64_ptr",
            )
            .unwrap()
            .into_pointer_value();
        let loaded = ctx
            .builder()
            .build_load(ctx.context.i64_type(), i64_ptr, field_name)
            .unwrap()
            .into_int_value();
        let float_val = ctx
            .builder()
            .build_bit_cast(loaded, ctx.context.f64_type(), "field_float")
            .unwrap();
        return BasicValueEnum::FloatValue(float_val.into_float_value());
    }

    // 将 field_ptr 转换为正确类型的指针后加载
    let typed_ptr = ctx
        .builder()
        .build_bit_cast(
            field_ptr,
            ctx.context.ptr_type(Default::default()),
            "field_typed_ptr",
        )
        .unwrap()
        .into_pointer_value();
    ctx.builder()
        .build_load(llvm_ty, typed_ptr, field_name)
        .unwrap()
}

fn compile_simple_member_access<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    object: &Expr,
    field_name: &str,
) -> Result<ExprResult<'ctx>, String> {
    let obj_result = compile_expr(ctx, object)?;

    // Object literal types have inline fields, not class-based struct types.
    //
    // Storage layout (see `store_value_at_slot`): every object slot is a
    // flat 8-byte word. Pointer values are ptrtoint-stored as i64, float
    // values are bitcast-stored as i64, and integer values are stored
    // directly as i64. To stay symmetric, we always load as i64 here and
    // then coerce back to the declared field LLVM type:
    //   - i64 field + i64 storage       -> identity
    //   - ptr field + i64 storage       -> inttoptr (round-trip the ptrtoint)
    //   - f64 field + i64 storage       -> bitcast back to f64
    //
    // The previous implementation used `ruyi_type_to_llvm(field_ty)`
    // directly, which caused SEGVs for heterogeneous fields like
    // `pair<int, string>`'s `second` slot: it stored i64=25 but the
    // caller tried `load ptr` and got `0x19` dereferenced as a string.
    if let Type::Object(fields) = &obj_result.ty {
        let obj_ptr = obj_result.value.into_pointer_value();
        let field_index = fields
            .iter()
            .position(|f| f.name == field_name)
            .ok_or_else(|| format!("Unknown field: {} in object", field_name))?;
        let field_ty = &fields[field_index].ty;
        let offset = ctx
            .context
            .i32_type()
            .const_int((field_index * 8) as u64, false);
        let field_ptr = unsafe {
            ctx.builder()
                .build_gep(
                    ctx.context.i8_type(),
                    obj_ptr,
                    &[offset],
                    &format!("{}_ptr", field_name),
                )
                .unwrap()
        };
        let typed_ptr = ctx
            .builder()
            .build_pointer_cast(
                field_ptr,
                ctx.context.ptr_type(Default::default()),
                "field_typed_ptr",
            )
            .unwrap();
        let raw = ctx
            .builder()
            .build_load(ctx.context.i64_type(), typed_ptr, field_name)
            .unwrap()
            .into_int_value();
        let value = match field_ty {
            Type::Float => ctx
                .builder()
                .build_bit_cast(raw, ctx.context.f64_type(), "slot_to_f64")
                .unwrap()
                .as_basic_value_enum(),
            // String / / Object / / Array / / Function / / Named / / TypeVar /
            // Generic (all lowered to `ptr` by ruyi_type_to_llvm): round-trip
            // through inttoptr so the caller sees a real pointer.
            Type::String
            | Type::Named(_, _)
            | Type::Array(_)
            | Type::Object(_)
            | Type::Function { .. }
            | Type::BigInt
            | Type::Null => ctx
                .builder()
                .build_int_to_ptr(raw, ctx.context.ptr_type(Default::default()), "slot_to_ptr")
                .unwrap()
                .as_basic_value_enum(),
            // Bool: stored as i64 (one word) but branch instructions require
            // i1, so truncate the bits back to a single-bit value. Storage
            // writes i64 (so callers of `compile_simple_member_access`
            // can keep a uniform int slot) but downstream codegen — `br`,
            // `icmp` results folded into conditionals, etc. — must see i1.
            Type::Bool => ctx
                .builder()
                .build_int_truncate(raw, ctx.context.bool_type(), "slot_to_bool")
                .unwrap()
                .as_basic_value_enum(),
            // Int / Byte / Void / Never: storage is already the right
            // shape (the int cast above preserves the value).
            _ => BasicValueEnum::IntValue(raw),
        };
        return Ok(ExprResult::new(value, field_ty.clone()));
    }

    // Primitive-typed receiver: bare `.method` (no parens) should dispatch
    // as a zero-arg method call via the same FFI plumbing compile_call uses
    // for the `obj.method(args)` form. We re-route by building a synthetic
    // `Expr::Member` callee and dispatching through compile_call with an
    // empty arg list — this re-uses the existing primitive FFI table
    // (`__string_<snake>`, `__builtin_array_<method>`, etc.).
    if matches!(
        obj_result.ty,
        Type::String | Type::Int | Type::Float | Type::Bool | Type::Array(_)
    ) {
        let synthetic_callee = Expr::Member {
            object: Box::new(object.clone()),
            property: crate::parser::ast::MemberProperty::Ident(field_name.to_string()),
            optional: false,
        };
        return compile_call(ctx, &synthetic_callee, &[]);
    }

    let class_name = resolve_class_from_type(&obj_result.ty)
        .ok_or_else(|| format!("Cannot access field on type: {:?}", obj_result.ty))?;
    let obj_ptr = obj_result.value.into_pointer_value();

    // Union alias field forwarding: `Option<T>.value` lives on `Some<T>`,
    // `Result<T,E>.error` lives on `Err<T,E>`. Forwarding is safe even
    // when the runtime value is the other variant — the resulting load
    // either returns the stored value (Some/Err case) or reads garbage
    // (None/Ok case), matching the spec's "no narrowing" semantics. The
    // caller is expected to pattern-match before reading discriminator
    // fields; we don't add implicit runtime checks here.
    //
    // Guard against name shadowing: a user may define a standalone class
    // named `Option` (e.g. the generics tutorial) that shadows the union
    // alias. If the local class already declares `field_name`, it is the
    // real field type and forwarding would corrupt the access (load as
    // `ptr` for a struct field of `int`). Only forward when the alias
    // does not also exist as a registered class with this field.
    let forwarded = union_alias_forwarded_field(&class_name, field_name).filter(|_| {
        !ctx.class_fields
            .get(&class_name)
            .is_some_and(|fields| fields.iter().any(|(n, _)| n == field_name))
    });
    let class_name = forwarded.unwrap_or(class_name);

    // Check if field_name is a getter property — if so, call the getter method
    if ctx
        .class_getters
        .get(&class_name)
        .is_some_and(|g| g.contains(field_name))
    {
        let getter_fn_name = format!("{}_{}", class_name, field_name);
        let getter_fn = ctx
            .module
            .get_function(&getter_fn_name)
            .ok_or_else(|| format!("Getter function not found: {}", getter_fn_name))?;
        let result = ctx
            .builder()
            .build_call(getter_fn, &[obj_ptr.into()], &format!("get_{}", field_name))
            .unwrap()
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| format!("Getter {} returned void", getter_fn_name))?;
        // Determine return type from the function's declared return type
        let ret_ty = ctx
            .function_types
            .get(&getter_fn_name)
            .cloned()
            .unwrap_or(Type::Dynamic);
        return Ok(ExprResult::new(result, ret_ty));
    }

    let (field_ptr, field_ty) =
        class_field_access(ctx, obj_ptr, &class_name, field_name, &obj_result.ty)?;

    let value = emit_field_load(ctx, field_ptr, &field_ty, field_name);

    Ok(ExprResult::new(value, field_ty))
}

fn compile_optional_member_access<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    object: &Expr,
    field_name: &str,
) -> Result<ExprResult<'ctx>, String> {
    let obj_result = compile_expr(ctx, object)?;
    let obj_ptr = obj_result.value.into_pointer_value();

    let class_name = resolve_class_from_type(&obj_result.ty)
        .ok_or_else(|| "Optional chaining only supported on class instances".to_string())?;
    let (field_ptr, field_ty) =
        class_field_access(ctx, obj_ptr, &class_name, field_name, &obj_result.ty)?;

    let func = ctx.current_function().ok_or("No current function")?;
    let i64_ty = ctx.context.i64_type();
    let obj_int = ctx
        .builder()
        .build_ptr_to_int(obj_ptr, i64_ty, "obj_int")
        .unwrap();
    let is_null = ctx
        .builder()
        .build_int_compare(
            inkwell::IntPredicate::EQ,
            obj_int,
            i64_ty.const_int(0, false),
            "is_null",
        )
        .unwrap();

    let non_null_bb = ctx.context.append_basic_block(func, "opt_non_null");
    let null_bb = ctx.context.append_basic_block(func, "opt_null");
    let merge_bb = ctx.context.append_basic_block(func, "opt_merge");

    ctx.builder()
        .build_conditional_branch(is_null, null_bb, non_null_bb)
        .unwrap();

    ctx.builder().position_at_end(null_bb);
    let llvm_ty = ruyi_type_to_llvm(ctx.context, &field_ty);
    let null_val = build_zero_value(llvm_ty);
    ctx.builder().build_unconditional_branch(merge_bb).unwrap();

    ctx.builder().position_at_end(non_null_bb);
    let value = emit_field_load(ctx, field_ptr, &field_ty, field_name);
    ctx.builder().build_unconditional_branch(merge_bb).unwrap();
    let non_null_bb_end = ctx.builder().get_insert_block().unwrap();

    ctx.builder().position_at_end(merge_bb);
    let phi = ctx.builder().build_phi(llvm_ty, "opt_phi").unwrap();
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
        _ => panic!("Unsupported type in build_zero_value"),
    }
}

/// Compile an `instanceof` expression. The LHS is a class instance; the RHS
/// is a class name identifier. We load the hidden `__typeid` field (struct
/// index 0) from the instance and compare it against the target class's
/// type ID, walking the inheritance chain so that `dog instanceof Animal`
/// returns true when `Dog extends Animal`.
fn compile_instanceof<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    left: &Expr,
    right: &Expr,
) -> Result<ExprResult<'ctx>, String> {
    let target_class = match right {
        Expr::Identifier(name) => name.clone(),
        _ => return Err("instanceof RHS must be a class name".to_string()),
    };

    let target_id = ctx
        .type_ids
        .get(&target_class)
        .copied()
        .ok_or_else(|| format!("Unknown class in instanceof: {}", target_class))?;

    // Collect the target class and all its DESCENDANTS so that
    // `dog instanceof Animal` succeeds (Dog extends Animal) but
    // `animal instanceof Dog` fails (Animal does not extend Dog).
    let mut accepted_ids: Vec<u64> = vec![target_id];
    for (class_name, &class_id) in &ctx.type_ids {
        if class_name == &target_class {
            continue;
        }
        // Walk up from class_name; if we reach target_class, it's a descendant.
        let mut cur = class_name.clone();
        let mut found = false;
        while let Some(parent) = ctx.class_extends.get(&cur).cloned() {
            if parent == target_class {
                found = true;
                break;
            }
            cur = parent;
        }
        if found {
            accepted_ids.push(class_id);
        }
    }

    let left_result = compile_expr(ctx, left)?;
    let obj_ptr = left_result.value.into_pointer_value();

    // Load __typeid (i64 at byte offset 0 — always the first field).
    let i64_ty = ctx.context.i64_type();
    let typeid_ptr = ctx
        .builder()
        .build_pointer_cast(
            obj_ptr,
            ctx.context.ptr_type(Default::default()),
            "typeid_i64_ptr",
        )
        .unwrap();
    let actual_id = ctx
        .builder()
        .build_load(i64_ty, typeid_ptr, "actual_typeid")
        .unwrap()
        .into_int_value();

    // Compare against each accepted ID (target + ancestors).
    let bool_ty = ctx.context.bool_type();
    let mut result = bool_ty.const_int(0, false);
    for &aid in &accepted_ids {
        let expected = i64_ty.const_int(aid, false);
        let cmp = ctx
            .builder()
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                actual_id,
                expected,
                "instanceof_cmp",
            )
            .unwrap();
        result = ctx
            .builder()
            .build_or(result, cmp, "instanceof_or")
            .unwrap();
    }

    Ok(ExprResult::new(
        BasicValueEnum::IntValue(result),
        Type::Bool,
    ))
}

/// True for the `null` literal, unwrapping any parentheses.
fn is_null_literal(e: &Expr) -> bool {
    match e {
        Expr::NullLiteral => true,
        Expr::Grouping(inner) => is_null_literal(inner),
        _ => false,
    }
}

fn compile_binary<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    op: &BinaryOp,
    left: &Expr,
    right: &Expr,
) -> Result<ExprResult<'ctx>, String> {
    // `instanceof` needs special handling: the RHS is a class name (not a
    // runtime expression), so we must not eagerly compile it.
    if matches!(op, BinaryOp::Instanceof) {
        return compile_instanceof(ctx, left, right);
    }

    // `x === null` / `x !== null`: compile the null literal as the generic
    // null value (null pointer, `Null` type) so the comparison dispatches on
    // the *other* operand's runtime representation. Compiling it as a
    // type-directed sentinel instead (e.g. all-ones float inside a
    // `float?`-returning function, via `current_return_type`) misrepresents
    // the literal: `string? === null` within such a function would then
    // compare a pointer against a float sentinel.
    let is_eq_op = matches!(
        op,
        BinaryOp::StrictEquals | BinaryOp::Equals | BinaryOp::StrictNotEquals | BinaryOp::NotEquals
    );
    if is_eq_op && (is_null_literal(left) || is_null_literal(right)) {
        let compile_operand = |ctx: &mut CodegenContext<'ctx, '_, '_>, e: &Expr| {
            if is_null_literal(e) {
                let null_ptr = ctx.context.ptr_type(Default::default()).const_null();
                Ok(ExprResult::new(
                    BasicValueEnum::PointerValue(null_ptr),
                    Type::Null,
                ))
            } else {
                compile_expr(ctx, e)
            }
        };
        let left_result = compile_operand(ctx, left)?;
        let right_result = compile_operand(ctx, right)?;
        return match op {
            BinaryOp::StrictEquals | BinaryOp::Equals => {
                compile_eq(ctx, &left_result, &right_result)
            }
            _ => compile_ne(ctx, &left_result, &right_result),
        };
    }

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
        BinaryOp::UShr => compile_ushr(ctx, &left_result, &right_result),
        BinaryOp::Nullish => compile_nullish(ctx, left, right, &left_result, &right_result),
        BinaryOp::Power => compile_power(ctx, &left_result, &right_result),
        _ => Err(format!("Unsupported binary operator: {:?}", op)),
    }
}

/// 当 Ruyi 类型为 Dynamic 时，从 LLVM 值推断实际类型
fn infer_actual_type(val: &ExprResult) -> Type {
    if val.ty == Type::Dynamic {
        match val.value {
            BasicValueEnum::IntValue(_) => Type::Int,
            BasicValueEnum::FloatValue(_) => Type::Float,
            BasicValueEnum::PointerValue(_) => Type::String,
            _ => val.ty.clone(),
        }
    } else {
        val.ty.clone()
    }
}

fn compile_add<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    // 推断 Dynamic 类型的实际类型
    let left_ty = infer_actual_type(left);
    let right_ty = infer_actual_type(right);

    if left_ty == Type::String && right_ty == Type::String {
        let l = left.value.into_pointer_value();
        let r = right.value.into_pointer_value();
        let concat_fn = ctx
            .module
            .get_function("ruyi_str_concat")
            .expect("ruyi_str_concat not declared");
        let res = ctx
            .builder()
            .build_call(concat_fn, &[l.into(), r.into()], "str_concat")
            .unwrap()
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        return Ok(ExprResult::new(
            BasicValueEnum::PointerValue(res),
            Type::String,
        ));
    }

    // String + Int: convert int to string, then concat
    if left_ty == Type::String && right_ty == Type::Int {
        let l = left.value.into_pointer_value();
        let r_int = right.value.into_int_value();
        let int_to_str_fn = ctx
            .module
            .get_function("ruyi_int_to_string")
            .expect("ruyi_int_to_string not declared");
        let r_str = ctx
            .builder()
            .build_call(int_to_str_fn, &[r_int.into()], "int_to_str")
            .unwrap()
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let concat_fn = ctx
            .module
            .get_function("ruyi_str_concat")
            .expect("ruyi_str_concat not declared");
        let res = ctx
            .builder()
            .build_call(concat_fn, &[l.into(), r_str.into()], "str_concat")
            .unwrap()
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        return Ok(ExprResult::new(
            BasicValueEnum::PointerValue(res),
            Type::String,
        ));
    }

    // String + Float: convert float to string, then concat
    if left_ty == Type::String && right_ty == Type::Float {
        let l = left.value.into_pointer_value();
        let r_float = right.value.into_float_value();
        let float_to_str_fn = ctx
            .module
            .get_function("ruyi_float_to_string")
            .expect("ruyi_float_to_string not declared");
        let r_str = ctx
            .builder()
            .build_call(float_to_str_fn, &[r_float.into()], "float_to_str")
            .unwrap()
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let concat_fn = ctx
            .module
            .get_function("ruyi_str_concat")
            .expect("ruyi_str_concat not declared");
        let res = ctx
            .builder()
            .build_call(concat_fn, &[l.into(), r_str.into()], "str_concat")
            .unwrap()
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        return Ok(ExprResult::new(
            BasicValueEnum::PointerValue(res),
            Type::String,
        ));
    }

    // Int + String: convert int to string, then concat
    if left_ty == Type::Int && right_ty == Type::String {
        let l_int = left.value.into_int_value();
        let r = right.value.into_pointer_value();
        let int_to_str_fn = ctx
            .module
            .get_function("ruyi_int_to_string")
            .expect("ruyi_int_to_string not declared");
        let l_str = ctx
            .builder()
            .build_call(int_to_str_fn, &[l_int.into()], "int_to_str")
            .unwrap()
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let concat_fn = ctx
            .module
            .get_function("ruyi_str_concat")
            .expect("ruyi_str_concat not declared");
        let res = ctx
            .builder()
            .build_call(concat_fn, &[l_str.into(), r.into()], "str_concat")
            .unwrap()
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        return Ok(ExprResult::new(
            BasicValueEnum::PointerValue(res),
            Type::String,
        ));
    }

    // Float + String: convert float to string, then concat
    if left_ty == Type::Float && right_ty == Type::String {
        let l_float = left.value.into_float_value();
        let r = right.value.into_pointer_value();
        let float_to_str_fn = ctx
            .module
            .get_function("ruyi_float_to_string")
            .expect("ruyi_float_to_string not declared");
        let l_str = ctx
            .builder()
            .build_call(float_to_str_fn, &[l_float.into()], "float_to_str")
            .unwrap()
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let concat_fn = ctx
            .module
            .get_function("ruyi_str_concat")
            .expect("ruyi_str_concat not declared");
        let res = ctx
            .builder()
            .build_call(concat_fn, &[l_str.into(), r.into()], "str_concat")
            .unwrap()
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        return Ok(ExprResult::new(
            BasicValueEnum::PointerValue(res),
            Type::String,
        ));
    }

    // String + Bool: convert bool to string, then concat
    if left_ty == Type::String && right_ty == Type::Bool {
        let l = left.value.into_pointer_value();
        let r_bool = right.value.into_int_value();
        let bool_to_str_fn = ctx
            .module
            .get_function("ruyi_bool_to_string")
            .expect("ruyi_bool_to_string not declared");
        let r_str = ctx
            .builder()
            .build_call(bool_to_str_fn, &[r_bool.into()], "bool_to_str")
            .unwrap()
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let concat_fn = ctx
            .module
            .get_function("ruyi_str_concat")
            .expect("ruyi_str_concat not declared");
        let res = ctx
            .builder()
            .build_call(concat_fn, &[l.into(), r_str.into()], "str_concat")
            .unwrap()
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        return Ok(ExprResult::new(
            BasicValueEnum::PointerValue(res),
            Type::String,
        ));
    }

    // Bool + String: convert bool to string, then concat
    if left_ty == Type::Bool && right_ty == Type::String {
        let l_bool = left.value.into_int_value();
        let r = right.value.into_pointer_value();
        let bool_to_str_fn = ctx
            .module
            .get_function("ruyi_bool_to_string")
            .expect("ruyi_bool_to_string not declared");
        let l_str = ctx
            .builder()
            .build_call(bool_to_str_fn, &[l_bool.into()], "bool_to_str")
            .unwrap()
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let concat_fn = ctx
            .module
            .get_function("ruyi_str_concat")
            .expect("ruyi_str_concat not declared");
        let res = ctx
            .builder()
            .build_call(concat_fn, &[l_str.into(), r.into()], "str_concat")
            .unwrap()
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        return Ok(ExprResult::new(
            BasicValueEnum::PointerValue(res),
            Type::String,
        ));
    }

    // String + Nullable(String) (and Nullable + String / Nullable + Nullable):
    // the nullable operand is already an i8* (possibly null); ruyi_str_concat
    // treats null as the empty string, so concat directly. Plain String+String
    // is handled above, so reaching here means at least one side is nullable.
    let left_is_str = left_ty == Type::String
        || matches!(&left_ty, Type::Nullable(inner) if **inner == Type::String);
    let right_is_str = right_ty == Type::String
        || matches!(&right_ty, Type::Nullable(inner) if **inner == Type::String);
    if left_is_str && right_is_str {
        let l = left.value.into_pointer_value();
        let r = right.value.into_pointer_value();
        let concat_fn = ctx
            .module
            .get_function("ruyi_str_concat")
            .expect("ruyi_str_concat not declared");
        let res = ctx
            .builder()
            .build_call(concat_fn, &[l.into(), r.into()], "str_concat")
            .unwrap()
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        return Ok(ExprResult::new(
            BasicValueEnum::PointerValue(res),
            Type::String,
        ));
    }

    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder().build_int_add(*l, *r, "add").unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
            let res = ctx.builder().build_float_add(*l, *r, "fadd").unwrap();
            Ok(ExprResult::new(
                BasicValueEnum::FloatValue(res),
                Type::Float,
            ))
        }
        (BasicValueEnum::IntValue(l), BasicValueEnum::FloatValue(r)) => {
            let l_f = ctx
                .builder()
                .build_signed_int_to_float(*l, ctx.context.f64_type(), "itof")
                .unwrap();
            let res = ctx.builder().build_float_add(l_f, *r, "fadd").unwrap();
            Ok(ExprResult::new(
                BasicValueEnum::FloatValue(res),
                Type::Float,
            ))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::IntValue(r)) => {
            let r_f = ctx
                .builder()
                .build_signed_int_to_float(*r, ctx.context.f64_type(), "itof")
                .unwrap();
            let res = ctx.builder().build_float_add(*l, r_f, "fadd").unwrap();
            Ok(ExprResult::new(
                BasicValueEnum::FloatValue(res),
                Type::Float,
            ))
        }
        // StructValue (Dynamic) 算术：提取内部 i64 值进行运算
        (BasicValueEnum::StructValue(s), BasicValueEnum::IntValue(r))
            if left.ty == Type::Dynamic =>
        {
            let l_int = ctx
                .builder()
                .build_extract_value(*s, 0, "dyn_int")
                .unwrap()
                .into_int_value();
            let res = ctx.builder().build_int_add(l_int, *r, "add").unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
        }
        (BasicValueEnum::IntValue(l), BasicValueEnum::StructValue(s))
            if right.ty == Type::Dynamic =>
        {
            let r_int = ctx
                .builder()
                .build_extract_value(*s, 0, "dyn_int")
                .unwrap()
                .into_int_value();
            let res = ctx.builder().build_int_add(*l, r_int, "add").unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
        }
        (BasicValueEnum::StructValue(l), BasicValueEnum::StructValue(r))
            if left.ty == Type::Dynamic && right.ty == Type::Dynamic =>
        {
            let l_int = ctx
                .builder()
                .build_extract_value(*l, 0, "dyn_l")
                .unwrap()
                .into_int_value();
            let r_int = ctx
                .builder()
                .build_extract_value(*r, 0, "dyn_r")
                .unwrap()
                .into_int_value();
            let res = ctx.builder().build_int_add(l_int, r_int, "add").unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
        }
        // PointerValue (String) + StructValue (Dynamic): 提取 i8* 做字符串拼接
        (BasicValueEnum::PointerValue(l), BasicValueEnum::StructValue(s))
            if right.ty == Type::Dynamic =>
        {
            let r_ptr = ctx
                .builder()
                .build_extract_value(*s, 1, "dyn_str")
                .unwrap()
                .into_pointer_value();
            let concat_fn = ctx
                .module
                .get_function("ruyi_str_concat")
                .expect("ruyi_str_concat not declared");
            let res = ctx
                .builder()
                .build_call(concat_fn, &[(*l).into(), r_ptr.into()], "str_concat")
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic()
                .into_pointer_value();
            Ok(ExprResult::new(
                BasicValueEnum::PointerValue(res),
                Type::String,
            ))
        }
        (BasicValueEnum::StructValue(s), BasicValueEnum::PointerValue(r))
            if left.ty == Type::Dynamic =>
        {
            let l_ptr = ctx
                .builder()
                .build_extract_value(*s, 1, "dyn_str")
                .unwrap()
                .into_pointer_value();
            let concat_fn = ctx
                .module
                .get_function("ruyi_str_concat")
                .expect("ruyi_str_concat not declared");
            let res = ctx
                .builder()
                .build_call(concat_fn, &[l_ptr.into(), (*r).into()], "str_concat")
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic()
                .into_pointer_value();
            Ok(ExprResult::new(
                BasicValueEnum::PointerValue(res),
                Type::String,
            ))
        }
        // PointerValue + PointerValue: 当任一操作数为 String 类型时，
        // 两个 i8* 指针均可直接传入 ruyi_str_concat 做字符串拼接。
        // 这覆盖了 Named(泛型实例化)、Generic 等指针类型与 String 的拼接。
        (BasicValueEnum::PointerValue(l), BasicValueEnum::PointerValue(r))
            if left_ty == Type::String || right_ty == Type::String =>
        {
            let concat_fn = ctx
                .module
                .get_function("ruyi_str_concat")
                .expect("ruyi_str_concat not declared");
            let res = ctx
                .builder()
                .build_call(concat_fn, &[(*l).into(), (*r).into()], "str_concat")
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic()
                .into_pointer_value();
            Ok(ExprResult::new(
                BasicValueEnum::PointerValue(res),
                Type::String,
            ))
        }
        _ => Err(format!(
            "Invalid operands for +: left={:?}({:?}), right={:?}({:?})",
            left.value, left_ty, right.value, right_ty
        )),
    }
}

fn compile_sub<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder().build_int_sub(*l, *r, "sub").unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
            let res = ctx.builder().build_float_sub(*l, *r, "fsub").unwrap();
            Ok(ExprResult::new(
                BasicValueEnum::FloatValue(res),
                Type::Float,
            ))
        }
        (BasicValueEnum::IntValue(l), BasicValueEnum::FloatValue(r)) => {
            let l_f = ctx
                .builder()
                .build_signed_int_to_float(*l, ctx.context.f64_type(), "itof")
                .unwrap();
            let res = ctx.builder().build_float_sub(l_f, *r, "fsub").unwrap();
            Ok(ExprResult::new(
                BasicValueEnum::FloatValue(res),
                Type::Float,
            ))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::IntValue(r)) => {
            let r_f = ctx
                .builder()
                .build_signed_int_to_float(*r, ctx.context.f64_type(), "itof")
                .unwrap();
            let res = ctx.builder().build_float_sub(*l, r_f, "fsub").unwrap();
            Ok(ExprResult::new(
                BasicValueEnum::FloatValue(res),
                Type::Float,
            ))
        }
        _ => Err(format!(
            "Invalid operands for -: left={:?}({:?}), right={:?}({:?})",
            left.value, left.ty, right.value, right.ty
        )),
    }
}

fn compile_mul<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder().build_int_mul(*l, *r, "mul").unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
            let res = ctx.builder().build_float_mul(*l, *r, "fmul").unwrap();
            Ok(ExprResult::new(
                BasicValueEnum::FloatValue(res),
                Type::Float,
            ))
        }
        _ => build_mixed_arith(
            ctx,
            left,
            right,
            |l, r| ctx.builder().build_float_mul(l, r, "fmul").unwrap(),
            "*",
        ),
    }
}

/// 辅助：为算术运算添加 Int-Float 和 Float-Int 混合类型支持
fn build_mixed_arith<'ctx, F>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
    float_op: F,
    float_op_name: &str,
) -> Result<ExprResult<'ctx>, String>
where
    F: FnOnce(
        inkwell::values::FloatValue<'ctx>,
        inkwell::values::FloatValue<'ctx>,
    ) -> inkwell::values::FloatValue<'ctx>,
{
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::FloatValue(r)) => {
            let l_f = ctx
                .builder()
                .build_signed_int_to_float(*l, ctx.context.f64_type(), "itof")
                .unwrap();
            let res = float_op(l_f, *r);
            Ok(ExprResult::new(
                BasicValueEnum::FloatValue(res),
                Type::Float,
            ))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::IntValue(r)) => {
            let r_f = ctx
                .builder()
                .build_signed_int_to_float(*r, ctx.context.f64_type(), "itof")
                .unwrap();
            let res = float_op(*l, r_f);
            Ok(ExprResult::new(
                BasicValueEnum::FloatValue(res),
                Type::Float,
            ))
        }
        _ => Err(format!("Invalid operands for {}", float_op_name)),
    }
}

/**
 * 辅助：统一处理数值类型的比较运算（Int-Int / Float-Float / Int-Float / Float-Int）。
 * 对于 Int-Float 或 Float-Int 混合比较，先将 Int 提升为 f64 再做浮点比较。
 * Int-Int 比较时自动对齐位宽（如 bool i1 vs int i64）。
 *
 * @param int_pred  整数比较谓词（EQ / NE / SLT / SGT / SLE / SGE 等）
 * @param float_pred 浮点比较谓词（OEQ / ONE / OLT / OGT / OLE / OGE）
 * @param op_name   操作符名称，用于错误消息
 * @return Ok(Some(result)) 若为数值类型比较；Ok(None) 若需要非数值匹配分支
 */
fn build_numeric_cmp<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
    int_pred: IntPredicate,
    float_pred: FloatPredicate,
    op_name: &str,
) -> Result<Option<ExprResult<'ctx>>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            // Handle mismatched int widths (e.g. bool i1 vs int i64)
            let (l_ext, r_ext) = if l.get_type().get_bit_width() != r.get_type().get_bit_width() {
                let max_bits =
                    std::cmp::max(l.get_type().get_bit_width(), r.get_type().get_bit_width());
                let wide_ty = ctx.context.custom_width_int_type(max_bits);
                let l_e = if l.get_type().get_bit_width() < max_bits {
                    ctx.builder()
                        .build_int_z_extend(*l, wide_ty, "cmp_zext_l")
                        .unwrap()
                } else {
                    *l
                };
                let r_e = if r.get_type().get_bit_width() < max_bits {
                    ctx.builder()
                        .build_int_z_extend(*r, wide_ty, "cmp_zext_r")
                        .unwrap()
                } else {
                    *r
                };
                (l_e, r_e)
            } else {
                (*l, *r)
            };
            let res = ctx
                .builder()
                .build_int_compare(int_pred, l_ext, r_ext, op_name)
                .unwrap();
            Ok(Some(ExprResult::new(
                BasicValueEnum::IntValue(res),
                Type::Bool,
            )))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
            let res = ctx
                .builder()
                .build_float_compare(float_pred, *l, *r, &format!("f{}", op_name))
                .unwrap();
            Ok(Some(ExprResult::new(
                BasicValueEnum::IntValue(res),
                Type::Bool,
            )))
        }
        // Mixed: Int vs Float — promote int to float, then compare
        (BasicValueEnum::IntValue(l), BasicValueEnum::FloatValue(r)) => {
            let l_f = ctx
                .builder()
                .build_signed_int_to_float(*l, ctx.context.f64_type(), "itof")
                .unwrap();
            let res = ctx
                .builder()
                .build_float_compare(float_pred, l_f, *r, &format!("mixed_f{}", op_name))
                .unwrap();
            Ok(Some(ExprResult::new(
                BasicValueEnum::IntValue(res),
                Type::Bool,
            )))
        }
        // Mixed: Float vs Int — promote int to float, then compare
        (BasicValueEnum::FloatValue(l), BasicValueEnum::IntValue(r)) => {
            let r_f = ctx
                .builder()
                .build_signed_int_to_float(*r, ctx.context.f64_type(), "itof")
                .unwrap();
            let res = ctx
                .builder()
                .build_float_compare(float_pred, *l, r_f, &format!("mixed_f{}", op_name))
                .unwrap();
            Ok(Some(ExprResult::new(
                BasicValueEnum::IntValue(res),
                Type::Bool,
            )))
        }
        _ => Ok(None),
    }
}

fn compile_div<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder().build_int_signed_div(*l, *r, "sdiv").unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
            let res = ctx.builder().build_float_div(*l, *r, "fdiv").unwrap();
            Ok(ExprResult::new(
                BasicValueEnum::FloatValue(res),
                Type::Float,
            ))
        }
        _ => build_mixed_arith(
            ctx,
            left,
            right,
            |l, r| ctx.builder().build_float_div(l, r, "fdiv").unwrap(),
            "/",
        ),
    }
}

fn compile_rem<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder().build_int_signed_rem(*l, *r, "srem").unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
        }
        _ => build_mixed_arith(
            ctx,
            left,
            right,
            |l, r| ctx.builder().build_float_rem(l, r, "frem").unwrap(),
            "%",
        ),
    }
}

fn compile_power<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    let pow_fn = ctx.module.get_function("pow").expect("pow not declared");

    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let l_f = ctx
                .builder()
                .build_signed_int_to_float(*l, ctx.context.f64_type(), "itof")
                .unwrap();
            let r_f = ctx
                .builder()
                .build_signed_int_to_float(*r, ctx.context.f64_type(), "itof")
                .unwrap();
            let res = ctx
                .builder()
                .build_call(pow_fn, &[l_f.into(), r_f.into()], "pow")
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic()
                .into_float_value();
            let res_int = ctx
                .builder()
                .build_float_to_signed_int(res, ctx.context.i64_type(), "ftoi")
                .unwrap();
            Ok(ExprResult::new(
                BasicValueEnum::IntValue(res_int),
                Type::Int,
            ))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
            let res = ctx
                .builder()
                .build_call(pow_fn, &[(*l).into(), (*r).into()], "pow")
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic()
                .into_float_value();
            Ok(ExprResult::new(
                BasicValueEnum::FloatValue(res),
                Type::Float,
            ))
        }
        (BasicValueEnum::IntValue(l), BasicValueEnum::FloatValue(r)) => {
            let l_f = ctx
                .builder()
                .build_signed_int_to_float(*l, ctx.context.f64_type(), "itof")
                .unwrap();
            let res = ctx
                .builder()
                .build_call(pow_fn, &[l_f.into(), (*r).into()], "pow")
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic()
                .into_float_value();
            Ok(ExprResult::new(
                BasicValueEnum::FloatValue(res),
                Type::Float,
            ))
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::IntValue(r)) => {
            let r_f = ctx
                .builder()
                .build_signed_int_to_float(*r, ctx.context.f64_type(), "itof")
                .unwrap();
            let res = ctx
                .builder()
                .build_call(pow_fn, &[(*l).into(), r_f.into()], "pow")
                .unwrap()
                .try_as_basic_value()
                .unwrap_basic()
                .into_float_value();
            Ok(ExprResult::new(
                BasicValueEnum::FloatValue(res),
                Type::Float,
            ))
        }
        _ => Err("Invalid operands for **".to_string()),
    }
}

/// Ensure the libc `strcmp` symbol is declared in the module and
/// return it. String values are C strings, so value-equality (`===`)
/// requires content comparison via `strcmp` rather than pointer
/// identity.
fn ensure_strcmp<'ctx>(ctx: &CodegenContext<'ctx, '_, '_>) -> inkwell::values::FunctionValue<'ctx> {
    ctx.module.get_function("strcmp").unwrap_or_else(|| {
        let i8_ptr = ctx.context.ptr_type(Default::default());
        let fn_ty = ctx
            .context
            .i32_type()
            .fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
        ctx.module.add_function("strcmp", fn_ty, None)
    })
}

/// Compare two string operands by content, returning `lhs !== rhs` when
/// `negate` is set and `lhs === rhs` otherwise.
///
/// A nullable string carries null as its sentinel value, and `strcmp`
/// dereferences both arguments — so the comparison is branched rather than
/// unconditional. Only present operands reach `strcmp`; a null operand takes
/// its answer from the two null bits, since null equals null but differs from
/// any present string.
fn build_string_cmp<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    lhs: inkwell::values::PointerValue<'ctx>,
    rhs: inkwell::values::PointerValue<'ctx>,
    lhs_nullable: bool,
    rhs_nullable: bool,
    negate: bool,
) -> Result<inkwell::values::IntValue<'ctx>, String> {
    let i64_ty = ctx.context.i64_type();
    let i1_ty = ctx.context.bool_type();
    let strcmp_fn = ensure_strcmp(ctx);
    // Emitted at the builder's current position. In the guarded case that is
    // inside `cmp_bb`, so the call must not run before the null branch.
    let emit_content_result = |ctx: &CodegenContext<'ctx, '_, '_>| {
        let cmp = ctx
            .builder()
            .build_call(strcmp_fn, &[lhs.into(), rhs.into()], "str_cmp")
            .unwrap()
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let equal = ctx
            .builder()
            .build_int_compare(
                IntPredicate::EQ,
                cmp,
                ctx.context.i32_type().const_int(0, false),
                "str_eq",
            )
            .unwrap();
        if negate {
            ctx.builder().build_not(equal, "str_ne").unwrap()
        } else {
            equal
        }
    };
    if !lhs_nullable && !rhs_nullable {
        return Ok(emit_content_result(ctx));
    }

    let func = ctx.current_function.ok_or("No current function")?;
    let null_bb = ctx.context.append_basic_block(func, "str_cmp_null");
    let cmp_bb = ctx.context.append_basic_block(func, "str_cmp_body");
    let done_bb = ctx.context.append_basic_block(func, "str_cmp_done");

    let is_null = |p: inkwell::values::PointerValue<'ctx>| {
        let as_int = ctx
            .builder()
            .build_ptr_to_int(p, i64_ty, "str_null_chk")
            .unwrap();
        ctx.builder()
            .build_int_compare(IntPredicate::EQ, as_int, i64_ty.const_zero(), "str_is_null")
            .unwrap()
    };
    let lhs_null = lhs_nullable.then(|| is_null(lhs));
    let rhs_null = rhs_nullable.then(|| is_null(rhs));
    let either_null = match (lhs_null, rhs_null) {
        (Some(a), Some(b)) => ctx.builder().build_or(a, b, "str_any_null").unwrap(),
        (Some(a), None) => a,
        (None, Some(b)) => b,
        (None, None) => return Ok(emit_content_result(ctx)),
    };
    let both_null = match (lhs_null, rhs_null) {
        (Some(a), Some(b)) => ctx.builder().build_and(a, b, "str_both_null").unwrap(),
        _ => i1_ty.const_int(0, false),
    };

    ctx.builder()
        .build_conditional_branch(either_null, null_bb, cmp_bb)
        .unwrap();
    ctx.builder().position_at_end(cmp_bb);
    let content_result = emit_content_result(ctx);
    ctx.builder().build_unconditional_branch(done_bb).unwrap();

    ctx.builder().position_at_end(null_bb);
    let null_equals_null = if negate { 0 } else { 1 };
    let null_differs = if negate { 1 } else { 0 };
    let null_result = ctx
        .builder()
        .build_select(
            both_null,
            i1_ty.const_int(null_equals_null, false),
            i1_ty.const_int(null_differs, false),
            "str_null_res",
        )
        .unwrap();
    ctx.builder().build_unconditional_branch(done_bb).unwrap();

    ctx.builder().position_at_end(done_bb);
    let phi = ctx.builder().build_phi(i1_ty, "str_cmp_res").unwrap();
    phi.add_incoming(&[(&null_result, null_bb), (&content_result, cmp_bb)]);
    Ok(phi.as_basic_value().into_int_value())
}

fn compile_eq<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    // 数值类型比较（Int-Int / Float-Float / Int-Float / Float-Int）统一处理
    if let Some(result) = build_numeric_cmp(
        ctx,
        left,
        right,
        IntPredicate::EQ,
        FloatPredicate::OEQ,
        "eq",
    )? {
        return Ok(result);
    }
    match (&left.value, &right.value) {
        (BasicValueEnum::PointerValue(l), BasicValueEnum::PointerValue(r))
            if matches!(&left.ty, Type::Generic { .. })
                && matches!(&right.ty, Type::Generic { .. }) =>
        {
            let enum_struct_ty = ctx.context.struct_type(
                &[
                    ctx.context.i8_type().into(),
                    ctx.context.ptr_type(Default::default()).into(),
                ],
                false,
            );
            let l_struct = ctx
                .builder()
                .build_load(enum_struct_ty, *l, "l_enum_loaded")
                .unwrap();
            let r_struct = ctx
                .builder()
                .build_load(enum_struct_ty, *r, "r_enum_loaded")
                .unwrap();
            let l_struct = match l_struct {
                BasicValueEnum::StructValue(s) => s,
                _ => return Err(format!("Enum pointer loaded to {:?}, not struct", l_struct)),
            };
            let r_struct = match r_struct {
                BasicValueEnum::StructValue(s) => s,
                _ => return Err(format!("Enum pointer loaded to {:?}, not struct", r_struct)),
            };
            let l_tag = ctx
                .builder()
                .build_extract_value(l_struct, 0, "l_tag")
                .unwrap()
                .into_int_value();
            let r_tag = ctx
                .builder()
                .build_extract_value(r_struct, 0, "r_tag")
                .unwrap()
                .into_int_value();
            let tag_eq = ctx
                .builder()
                .build_int_compare(IntPredicate::EQ, l_tag, r_tag, "tag_eq")
                .unwrap();
            let l_val = ctx
                .builder()
                .build_extract_value(l_struct, 1, "l_val")
                .unwrap()
                .into_pointer_value();
            let r_val = ctx
                .builder()
                .build_extract_value(r_struct, 1, "r_val")
                .unwrap()
                .into_pointer_value();
            let li = ctx
                .builder()
                .build_ptr_to_int(l_val, ctx.context.i64_type(), "l_val_int")
                .unwrap();
            let ri = ctx
                .builder()
                .build_ptr_to_int(r_val, ctx.context.i64_type(), "r_val_int")
                .unwrap();
            let val_eq = ctx
                .builder()
                .build_int_compare(IntPredicate::EQ, li, ri, "val_eq")
                .unwrap();
            let result = ctx.builder().build_and(tag_eq, val_eq, "enum_eq").unwrap();
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
            let i8_ptr_ty = ctx.context.ptr_type(Default::default());
            let option_struct = ctx
                .context
                .struct_type(&[i8_ty.into(), i8_ptr_ty.into()], false);
            let l_struct_ptr = ctx
                .builder()
                .build_bit_cast(*l, ctx.context.ptr_type(Default::default()), "l_struct_ptr")
                .unwrap()
                .into_pointer_value();
            let l_struct = ctx
                .builder()
                .build_load(option_struct, l_struct_ptr, "l_enum_loaded")
                .unwrap();
            let l_struct = match l_struct {
                BasicValueEnum::StructValue(s) => s,
                _ => return Err(format!("Enum pointer loaded to {:?}, not struct", l_struct)),
            };
            let l_tag = ctx
                .builder()
                .build_extract_value(l_struct, 0, "l_tag")
                .unwrap()
                .into_int_value();
            let r_tag = ctx
                .builder()
                .build_extract_value(*r, 0, "r_tag")
                .unwrap()
                .into_int_value();
            let tag_eq = ctx
                .builder()
                .build_int_compare(IntPredicate::EQ, l_tag, r_tag, "tag_eq")
                .unwrap();
            let l_val = ctx
                .builder()
                .build_extract_value(l_struct, 1, "l_val")
                .unwrap()
                .into_pointer_value();
            let r_val = ctx
                .builder()
                .build_extract_value(*r, 1, "r_val")
                .unwrap()
                .into_pointer_value();
            let li = ctx
                .builder()
                .build_ptr_to_int(l_val, ctx.context.i64_type(), "l_val_int")
                .unwrap();
            let ri = ctx
                .builder()
                .build_ptr_to_int(r_val, ctx.context.i64_type(), "r_val_int")
                .unwrap();
            let val_eq = ctx
                .builder()
                .build_int_compare(IntPredicate::EQ, li, ri, "val_eq")
                .unwrap();
            let result = ctx.builder().build_and(tag_eq, val_eq, "enum_eq").unwrap();
            Ok(ExprResult::new(
                BasicValueEnum::IntValue(result),
                Type::Bool,
            ))
        }
        (BasicValueEnum::StructValue(l), BasicValueEnum::PointerValue(r))
            if matches!(&left.ty, Type::Generic { .. })
                && matches!(&right.ty, Type::Generic { .. }) =>
        {
            let enum_struct_ty = ctx.context.struct_type(
                &[
                    ctx.context.i8_type().into(),
                    ctx.context.ptr_type(Default::default()).into(),
                ],
                false,
            );
            let r_struct = ctx
                .builder()
                .build_load(enum_struct_ty, *r, "r_enum_loaded")
                .unwrap();
            let r_struct = match r_struct {
                BasicValueEnum::StructValue(s) => s,
                _ => return Err(format!("Enum pointer loaded to {:?}, not struct", r_struct)),
            };
            let l_tag = ctx
                .builder()
                .build_extract_value(*l, 0, "l_tag")
                .unwrap()
                .into_int_value();
            let r_tag = ctx
                .builder()
                .build_extract_value(r_struct, 0, "r_tag")
                .unwrap()
                .into_int_value();
            let tag_eq = ctx
                .builder()
                .build_int_compare(IntPredicate::EQ, l_tag, r_tag, "tag_eq")
                .unwrap();
            let l_val = ctx
                .builder()
                .build_extract_value(*l, 1, "l_val")
                .unwrap()
                .into_pointer_value();
            let r_val = ctx
                .builder()
                .build_extract_value(r_struct, 1, "r_val")
                .unwrap()
                .into_pointer_value();
            let li = ctx
                .builder()
                .build_ptr_to_int(l_val, ctx.context.i64_type(), "l_val_int")
                .unwrap();
            let ri = ctx
                .builder()
                .build_ptr_to_int(r_val, ctx.context.i64_type(), "r_val_int")
                .unwrap();
            let val_eq = ctx
                .builder()
                .build_int_compare(IntPredicate::EQ, li, ri, "val_eq")
                .unwrap();
            let result = ctx.builder().build_and(tag_eq, val_eq, "enum_eq").unwrap();
            Ok(ExprResult::new(
                BasicValueEnum::IntValue(result),
                Type::Bool,
            ))
        }
        // String vs String: value equality via strcmp (spec: === is
        // "value and type equality"), not pointer identity. Nullable
        // operands compare by value too — the wrapper adds nothing at
        // runtime, so `v === "s"` must not degrade to pointer identity.
        (BasicValueEnum::PointerValue(l), BasicValueEnum::PointerValue(r))
            if strip_nullable(&left.ty) == Type::String
                && strip_nullable(&right.ty) == Type::String =>
        {
            let res = build_string_cmp(
                ctx,
                *l,
                *r,
                matches!(left.ty, Type::Nullable(_)),
                matches!(right.ty, Type::Nullable(_)),
                false,
            )?;
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        (BasicValueEnum::PointerValue(l), BasicValueEnum::PointerValue(r)) => {
            let l_int = ctx
                .builder()
                .build_ptr_to_int(*l, ctx.context.i64_type(), "l_ptr_int")
                .unwrap();
            let r_int = ctx
                .builder()
                .build_ptr_to_int(*r, ctx.context.i64_type(), "r_ptr_int")
                .unwrap();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::EQ, l_int, r_int, "ptr_eq")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        (BasicValueEnum::StructValue(l), BasicValueEnum::StructValue(r)) => {
            // Compare struct values by comparing each field
            let struct_ty = l.get_type();
            let count = struct_ty.count_fields();
            let mut result = ctx.context.bool_type().const_int(1, false);
            for i in 0..count {
                let l_field = ctx
                    .builder()
                    .build_extract_value(*l, i, &format!("l_field_{}", i))
                    .unwrap();
                let r_field = ctx
                    .builder()
                    .build_extract_value(*r, i, &format!("r_field_{}", i))
                    .unwrap();
                let field_eq = match (l_field, r_field) {
                    (BasicValueEnum::IntValue(lv), BasicValueEnum::IntValue(rv)) => ctx
                        .builder()
                        .build_int_compare(IntPredicate::EQ, lv, rv, &format!("field_eq_{}", i))
                        .unwrap(),
                    (BasicValueEnum::PointerValue(lp), BasicValueEnum::PointerValue(rp)) => {
                        let li = ctx
                            .builder()
                            .build_ptr_to_int(
                                lp,
                                ctx.context.i64_type(),
                                &format!("l_ptr_int_{}", i),
                            )
                            .unwrap();
                        let ri = ctx
                            .builder()
                            .build_ptr_to_int(
                                rp,
                                ctx.context.i64_type(),
                                &format!("r_ptr_int_{}", i),
                            )
                            .unwrap();
                        ctx.builder()
                            .build_int_compare(IntPredicate::EQ, li, ri, &format!("field_eq_{}", i))
                            .unwrap()
                    }
                    _ => ctx.context.bool_type().const_int(0, false),
                };
                result = ctx
                    .builder()
                    .build_and(result, field_eq, &format!("and_{}", i))
                    .unwrap();
            }
            Ok(ExprResult::new(
                BasicValueEnum::IntValue(result),
                Type::Bool,
            ))
        }
        // Nullable IntValue compared with null PointerValue:
        // The nullable int uses -1 (all ones) as null sentinel.
        (BasicValueEnum::IntValue(l), BasicValueEnum::PointerValue(_))
            if matches!(&left.ty, Type::Nullable(_)) && matches!(&right.ty, Type::Null) =>
        {
            let sentinel = ctx.context.i64_type().const_all_ones();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::EQ, *l, sentinel, "nullable_eq_null")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        // Reverse: null PointerValue compared with nullable IntValue
        (BasicValueEnum::PointerValue(_), BasicValueEnum::IntValue(r))
            if matches!(&left.ty, Type::Null) && matches!(&right.ty, Type::Nullable(_)) =>
        {
            let sentinel = ctx.context.i64_type().const_all_ones();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::EQ, *r, sentinel, "null_eq_nullable")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        // Nullable FloatValue compared with null PointerValue: the null
        // sentinel is the all-ones bit pattern (NaN), so test bits —
        // `NaN == NaN` is false under float semantics.
        (BasicValueEnum::FloatValue(l), BasicValueEnum::PointerValue(_))
            if matches!(&left.ty, Type::Nullable(_)) && matches!(&right.ty, Type::Null) =>
        {
            let bits = ctx
                .builder()
                .build_bit_cast(*l, ctx.context.i64_type(), "f_bits")
                .unwrap()
                .into_int_value();
            let res = ctx
                .builder()
                .build_int_compare(
                    IntPredicate::EQ,
                    bits,
                    ctx.context.i64_type().const_all_ones(),
                    "nullable_float_eq_null",
                )
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        // Reverse: null PointerValue compared with nullable FloatValue
        (BasicValueEnum::PointerValue(_), BasicValueEnum::FloatValue(r))
            if matches!(&left.ty, Type::Null) && matches!(&right.ty, Type::Nullable(_)) =>
        {
            let bits = ctx
                .builder()
                .build_bit_cast(*r, ctx.context.i64_type(), "f_bits")
                .unwrap()
                .into_int_value();
            let res = ctx
                .builder()
                .build_int_compare(
                    IntPredicate::EQ,
                    bits,
                    ctx.context.i64_type().const_all_ones(),
                    "null_eq_nullable_float",
                )
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        // StructValue (Dynamic) vs PointerValue (null): 检查 struct 的数据指针是否为 null
        (BasicValueEnum::StructValue(s), BasicValueEnum::PointerValue(_))
            if matches!(&right.ty, Type::Null) =>
        {
            if s.get_type().count_fields() >= 2 {
                let data_ptr = ctx
                    .builder()
                    .build_extract_value(*s, 1, "dyn_data")
                    .unwrap();
                if let BasicValueEnum::PointerValue(p) = data_ptr {
                    let ptr_int = ctx
                        .builder()
                        .build_ptr_to_int(p, ctx.context.i64_type(), "ptr_int")
                        .unwrap();
                    let zero = ctx.context.i64_type().const_int(0, false);
                    let res = ctx
                        .builder()
                        .build_int_compare(IntPredicate::EQ, ptr_int, zero, "dyn_null")
                        .unwrap();
                    Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
                } else {
                    Ok(ExprResult::new(
                        BasicValueEnum::IntValue(ctx.context.bool_type().const_int(0, false)),
                        Type::Bool,
                    ))
                }
            } else {
                Ok(ExprResult::new(
                    BasicValueEnum::IntValue(ctx.context.bool_type().const_int(0, false)),
                    Type::Bool,
                ))
            }
        }
        // Reverse: null PointerValue vs StructValue (Dynamic)
        (BasicValueEnum::PointerValue(_), BasicValueEnum::StructValue(s))
            if matches!(&left.ty, Type::Null) =>
        {
            if s.get_type().count_fields() >= 2 {
                let data_ptr = ctx
                    .builder()
                    .build_extract_value(*s, 1, "dyn_data")
                    .unwrap();
                if let BasicValueEnum::PointerValue(p) = data_ptr {
                    let ptr_int = ctx
                        .builder()
                        .build_ptr_to_int(p, ctx.context.i64_type(), "ptr_int")
                        .unwrap();
                    let zero = ctx.context.i64_type().const_int(0, false);
                    let res = ctx
                        .builder()
                        .build_int_compare(IntPredicate::EQ, ptr_int, zero, "dyn_null")
                        .unwrap();
                    Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
                } else {
                    Ok(ExprResult::new(
                        BasicValueEnum::IntValue(ctx.context.bool_type().const_int(0, false)),
                        Type::Bool,
                    ))
                }
            } else {
                Ok(ExprResult::new(
                    BasicValueEnum::IntValue(ctx.context.bool_type().const_int(0, false)),
                    Type::Bool,
                ))
            }
        }
        // ── Dynamic vs String: extract string pointer and compare ──
        (BasicValueEnum::StructValue(s), BasicValueEnum::PointerValue(r))
            if left.ty == Type::Dynamic && right.ty == Type::String =>
        {
            let dyn_str = ctx
                .builder()
                .build_extract_value(*s, 1, "dyn_str_ptr")
                .unwrap()
                .into_pointer_value();
            let l_int = ctx
                .builder()
                .build_ptr_to_int(dyn_str, ctx.context.i64_type(), "l_pi")
                .unwrap();
            let r_int = ctx
                .builder()
                .build_ptr_to_int(*r, ctx.context.i64_type(), "r_pi")
                .unwrap();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::EQ, l_int, r_int, "dyn_eq_str")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        (BasicValueEnum::PointerValue(l), BasicValueEnum::StructValue(s))
            if left.ty == Type::String && right.ty == Type::Dynamic =>
        {
            let dyn_str = ctx
                .builder()
                .build_extract_value(*s, 1, "dyn_str_ptr")
                .unwrap()
                .into_pointer_value();
            let l_int = ctx
                .builder()
                .build_ptr_to_int(*l, ctx.context.i64_type(), "l_pi")
                .unwrap();
            let r_int = ctx
                .builder()
                .build_ptr_to_int(dyn_str, ctx.context.i64_type(), "r_pi")
                .unwrap();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::EQ, l_int, r_int, "str_eq_dyn")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        // ── Dynamic vs Int: extract i64 value from data ptr and compare ──
        (BasicValueEnum::StructValue(s), BasicValueEnum::IntValue(r))
            if left.ty == Type::Dynamic =>
        {
            let dyn_data_ptr = ctx
                .builder()
                .build_extract_value(*s, 1, "dyn_data_ptr")
                .unwrap()
                .into_pointer_value();
            let dyn_val = ctx
                .builder()
                .build_ptr_to_int(dyn_data_ptr, ctx.context.i64_type(), "dyn_int_val")
                .unwrap();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::EQ, dyn_val, *r, "dyn_eq_int")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        (BasicValueEnum::IntValue(l), BasicValueEnum::StructValue(s))
            if right.ty == Type::Dynamic =>
        {
            let dyn_data_ptr = ctx
                .builder()
                .build_extract_value(*s, 1, "dyn_data_ptr")
                .unwrap()
                .into_pointer_value();
            let dyn_val = ctx
                .builder()
                .build_ptr_to_int(dyn_data_ptr, ctx.context.i64_type(), "dyn_int_val")
                .unwrap();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::EQ, *l, dyn_val, "int_eq_dyn")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        // IntValue vs PointerValue (不兼容类型): === 永远为 false
        (BasicValueEnum::IntValue(_), BasicValueEnum::PointerValue(_)) => Ok(ExprResult::new(
            BasicValueEnum::IntValue(ctx.context.bool_type().const_int(0, false)),
            Type::Bool,
        )),
        // PointerValue vs IntValue (不兼容类型): === 永远为 false
        (BasicValueEnum::PointerValue(_), BasicValueEnum::IntValue(_)) => Ok(ExprResult::new(
            BasicValueEnum::IntValue(ctx.context.bool_type().const_int(0, false)),
            Type::Bool,
        )),
        _ => Err(format!(
            "Invalid operands for ===: left={:?}({:?}), right={:?}({:?})",
            left.value, left.ty, right.value, right.ty
        )),
    }
}

fn compile_ne<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    // 数值类型比较（Int-Int / Float-Float / Int-Float / Float-Int）统一处理
    if let Some(result) = build_numeric_cmp(
        ctx,
        left,
        right,
        IntPredicate::NE,
        FloatPredicate::ONE,
        "ne",
    )? {
        return Ok(result);
    }
    match (&left.value, &right.value) {
        // String vs String: value inequality via strcmp (spec: !== negates
        // ===, which is content equality for strings). Nullable operands
        // unwrap the same way as in `compile_eq`.
        (BasicValueEnum::PointerValue(l), BasicValueEnum::PointerValue(r))
            if strip_nullable(&left.ty) == Type::String
                && strip_nullable(&right.ty) == Type::String =>
        {
            let res = build_string_cmp(
                ctx,
                *l,
                *r,
                matches!(left.ty, Type::Nullable(_)),
                matches!(right.ty, Type::Nullable(_)),
                true,
            )?;
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        (BasicValueEnum::PointerValue(l), BasicValueEnum::PointerValue(r)) => {
            let l_int = ctx
                .builder()
                .build_ptr_to_int(*l, ctx.context.i64_type(), "l_ptr_int")
                .unwrap();
            let r_int = ctx
                .builder()
                .build_ptr_to_int(*r, ctx.context.i64_type(), "r_ptr_int")
                .unwrap();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::NE, l_int, r_int, "ptr_ne")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        // Nullable IntValue compared with null PointerValue:
        (BasicValueEnum::IntValue(l), BasicValueEnum::PointerValue(_))
            if matches!(&left.ty, Type::Nullable(_)) && matches!(&right.ty, Type::Null) =>
        {
            let sentinel = ctx.context.i64_type().const_all_ones();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::NE, *l, sentinel, "nullable_ne_null")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        // Reverse: null PointerValue compared with nullable IntValue
        (BasicValueEnum::PointerValue(_), BasicValueEnum::IntValue(r))
            if matches!(&left.ty, Type::Null) && matches!(&right.ty, Type::Nullable(_)) =>
        {
            let sentinel = ctx.context.i64_type().const_all_ones();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::NE, *r, sentinel, "null_ne_nullable")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        // Nullable FloatValue compared with null PointerValue: the null
        // sentinel is the all-ones bit pattern (NaN), so test bits —
        // `NaN != NaN` is true under float semantics and would make
        // `x !== null` unconditionally true.
        (BasicValueEnum::FloatValue(l), BasicValueEnum::PointerValue(_))
            if matches!(&left.ty, Type::Nullable(_)) && matches!(&right.ty, Type::Null) =>
        {
            let bits = ctx
                .builder()
                .build_bit_cast(*l, ctx.context.i64_type(), "f_bits")
                .unwrap()
                .into_int_value();
            let res = ctx
                .builder()
                .build_int_compare(
                    IntPredicate::NE,
                    bits,
                    ctx.context.i64_type().const_all_ones(),
                    "nullable_float_ne_null",
                )
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        // Reverse: null PointerValue compared with nullable FloatValue
        (BasicValueEnum::PointerValue(_), BasicValueEnum::FloatValue(r))
            if matches!(&left.ty, Type::Null) && matches!(&right.ty, Type::Nullable(_)) =>
        {
            let bits = ctx
                .builder()
                .build_bit_cast(*r, ctx.context.i64_type(), "f_bits")
                .unwrap()
                .into_int_value();
            let res = ctx
                .builder()
                .build_int_compare(
                    IntPredicate::NE,
                    bits,
                    ctx.context.i64_type().const_all_ones(),
                    "null_ne_nullable_float",
                )
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        // StructValue (Dynamic) vs PointerValue (null): !== 检查 struct 是否不为 null
        (BasicValueEnum::StructValue(s), BasicValueEnum::PointerValue(_))
            if matches!(&right.ty, Type::Null) =>
        {
            if s.get_type().count_fields() >= 2 {
                let data_ptr = ctx
                    .builder()
                    .build_extract_value(*s, 1, "dyn_data")
                    .unwrap();
                if let BasicValueEnum::PointerValue(p) = data_ptr {
                    let ptr_int = ctx
                        .builder()
                        .build_ptr_to_int(p, ctx.context.i64_type(), "ptr_int")
                        .unwrap();
                    let zero = ctx.context.i64_type().const_int(0, false);
                    let res = ctx
                        .builder()
                        .build_int_compare(IntPredicate::NE, ptr_int, zero, "dyn_not_null")
                        .unwrap();
                    Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
                } else {
                    Ok(ExprResult::new(
                        BasicValueEnum::IntValue(ctx.context.bool_type().const_int(1, false)),
                        Type::Bool,
                    ))
                }
            } else {
                Ok(ExprResult::new(
                    BasicValueEnum::IntValue(ctx.context.bool_type().const_int(1, false)),
                    Type::Bool,
                ))
            }
        }
        // Reverse: null PointerValue vs StructValue (Dynamic)
        (BasicValueEnum::PointerValue(_), BasicValueEnum::StructValue(s))
            if matches!(&left.ty, Type::Null) =>
        {
            if s.get_type().count_fields() >= 2 {
                let data_ptr = ctx
                    .builder()
                    .build_extract_value(*s, 1, "dyn_data")
                    .unwrap();
                if let BasicValueEnum::PointerValue(p) = data_ptr {
                    let ptr_int = ctx
                        .builder()
                        .build_ptr_to_int(p, ctx.context.i64_type(), "ptr_int")
                        .unwrap();
                    let zero = ctx.context.i64_type().const_int(0, false);
                    let res = ctx
                        .builder()
                        .build_int_compare(IntPredicate::NE, ptr_int, zero, "dyn_not_null")
                        .unwrap();
                    Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
                } else {
                    Ok(ExprResult::new(
                        BasicValueEnum::IntValue(ctx.context.bool_type().const_int(1, false)),
                        Type::Bool,
                    ))
                }
            } else {
                Ok(ExprResult::new(
                    BasicValueEnum::IntValue(ctx.context.bool_type().const_int(1, false)),
                    Type::Bool,
                ))
            }
        }
        // IntValue vs PointerValue (不兼容类型): !== 永远为 true
        (BasicValueEnum::IntValue(_), BasicValueEnum::PointerValue(_)) => Ok(ExprResult::new(
            BasicValueEnum::IntValue(ctx.context.bool_type().const_int(1, false)),
            Type::Bool,
        )),
        // PointerValue vs IntValue (不兼容类型): !== 永远为 true
        (BasicValueEnum::PointerValue(_), BasicValueEnum::IntValue(_)) => Ok(ExprResult::new(
            BasicValueEnum::IntValue(ctx.context.bool_type().const_int(1, false)),
            Type::Bool,
        )),
        // StructValue vs StructValue (Dynamic): 按字段比较并取反
        (BasicValueEnum::StructValue(l), BasicValueEnum::StructValue(r)) => {
            let struct_ty = l.get_type();
            let count = struct_ty.count_fields();
            let mut result = ctx.context.bool_type().const_int(1, false);
            for i in 0..count {
                let l_field = ctx
                    .builder()
                    .build_extract_value(*l, i, &format!("l_field_{}", i))
                    .unwrap();
                let r_field = ctx
                    .builder()
                    .build_extract_value(*r, i, &format!("r_field_{}", i))
                    .unwrap();
                let field_eq = match (l_field, r_field) {
                    (BasicValueEnum::IntValue(lv), BasicValueEnum::IntValue(rv)) => ctx
                        .builder()
                        .build_int_compare(IntPredicate::EQ, lv, rv, &format!("field_eq_{}", i))
                        .unwrap(),
                    (BasicValueEnum::PointerValue(lp), BasicValueEnum::PointerValue(rp)) => {
                        let li = ctx
                            .builder()
                            .build_ptr_to_int(
                                lp,
                                ctx.context.i64_type(),
                                &format!("l_ptr_int_{}", i),
                            )
                            .unwrap();
                        let ri = ctx
                            .builder()
                            .build_ptr_to_int(
                                rp,
                                ctx.context.i64_type(),
                                &format!("r_ptr_int_{}", i),
                            )
                            .unwrap();
                        ctx.builder()
                            .build_int_compare(IntPredicate::EQ, li, ri, &format!("field_eq_{}", i))
                            .unwrap()
                    }
                    _ => ctx.context.bool_type().const_int(0, false),
                };
                result = ctx
                    .builder()
                    .build_and(result, field_eq, &format!("and_{}", i))
                    .unwrap();
            }
            let res = ctx.builder().build_not(result, "struct_ne").unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        // ── Dynamic vs String: extract string pointer and compare ──
        (BasicValueEnum::StructValue(s), BasicValueEnum::PointerValue(r))
            if left.ty == Type::Dynamic && right.ty == Type::String =>
        {
            let dyn_str = ctx
                .builder()
                .build_extract_value(*s, 1, "dyn_str_ptr")
                .unwrap()
                .into_pointer_value();
            let l_int = ctx
                .builder()
                .build_ptr_to_int(dyn_str, ctx.context.i64_type(), "l_pi")
                .unwrap();
            let r_int = ctx
                .builder()
                .build_ptr_to_int(*r, ctx.context.i64_type(), "r_pi")
                .unwrap();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::NE, l_int, r_int, "dyn_ne_str")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        (BasicValueEnum::PointerValue(l), BasicValueEnum::StructValue(s))
            if left.ty == Type::String && right.ty == Type::Dynamic =>
        {
            let dyn_str = ctx
                .builder()
                .build_extract_value(*s, 1, "dyn_str_ptr")
                .unwrap()
                .into_pointer_value();
            let l_int = ctx
                .builder()
                .build_ptr_to_int(*l, ctx.context.i64_type(), "l_pi")
                .unwrap();
            let r_int = ctx
                .builder()
                .build_ptr_to_int(dyn_str, ctx.context.i64_type(), "r_pi")
                .unwrap();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::NE, l_int, r_int, "str_ne_dyn")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        // ── Dynamic vs Int: extract i64 value from data ptr and compare ──
        (BasicValueEnum::StructValue(s), BasicValueEnum::IntValue(r))
            if left.ty == Type::Dynamic =>
        {
            let dyn_data_ptr = ctx
                .builder()
                .build_extract_value(*s, 1, "dyn_data_ptr")
                .unwrap()
                .into_pointer_value();
            let dyn_val = ctx
                .builder()
                .build_ptr_to_int(dyn_data_ptr, ctx.context.i64_type(), "dyn_int_val")
                .unwrap();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::NE, dyn_val, *r, "dyn_ne_int")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        (BasicValueEnum::IntValue(l), BasicValueEnum::StructValue(s))
            if right.ty == Type::Dynamic =>
        {
            let dyn_data_ptr = ctx
                .builder()
                .build_extract_value(*s, 1, "dyn_data_ptr")
                .unwrap()
                .into_pointer_value();
            let dyn_val = ctx
                .builder()
                .build_ptr_to_int(dyn_data_ptr, ctx.context.i64_type(), "dyn_int_val")
                .unwrap();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::NE, *l, dyn_val, "int_ne_dyn")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        _ => Err("Invalid operands for !==".to_string()),
    }
}

fn compile_lt<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    // 数值类型比较统一处理
    if let Some(result) = build_numeric_cmp(
        ctx,
        left,
        right,
        IntPredicate::SLT,
        FloatPredicate::OLT,
        "lt",
    )? {
        return Ok(result);
    }
    match (&left.value, &right.value) {
        // Generic/dyn pointer comparison: compare as unsigned integers
        (BasicValueEnum::PointerValue(l), BasicValueEnum::PointerValue(r)) => {
            let l_int = ctx
                .builder()
                .build_ptr_to_int(*l, ctx.context.i64_type(), "l_ptr")
                .unwrap();
            let r_int = ctx
                .builder()
                .build_ptr_to_int(*r, ctx.context.i64_type(), "r_ptr")
                .unwrap();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::ULT, l_int, r_int, "lt")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        // Tagged dyn struct comparison: extract i64 value and compare
        (BasicValueEnum::StructValue(l), BasicValueEnum::StructValue(r)) => {
            let l_val = ctx
                .builder()
                .build_extract_value(*l, 0, "l_val")
                .unwrap()
                .into_int_value();
            let r_val = ctx
                .builder()
                .build_extract_value(*r, 0, "r_val")
                .unwrap()
                .into_int_value();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::SLT, l_val, r_val, "lt")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        // Mixed: int vs dyn struct
        (BasicValueEnum::IntValue(l), BasicValueEnum::StructValue(r)) => {
            let r_val = ctx
                .builder()
                .build_extract_value(*r, 0, "r_val")
                .unwrap()
                .into_int_value();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::SLT, *l, r_val, "lt")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        // Mixed: dyn struct vs int
        (BasicValueEnum::StructValue(l), BasicValueEnum::IntValue(r)) => {
            let l_val = ctx
                .builder()
                .build_extract_value(*l, 0, "l_val")
                .unwrap()
                .into_int_value();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::SLT, l_val, *r, "lt")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        _ => Err(format!(
            "Invalid operands for <: left={:?}({}), right={:?}({})",
            left.value, left.ty, right.value, right.ty
        )),
    }
}

fn compile_gt<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    // 数值类型比较统一处理
    if let Some(result) = build_numeric_cmp(
        ctx,
        left,
        right,
        IntPredicate::SGT,
        FloatPredicate::OGT,
        "gt",
    )? {
        return Ok(result);
    }
    match (&left.value, &right.value) {
        // Generic/dyn pointer comparison: compare as unsigned integers
        (BasicValueEnum::PointerValue(l), BasicValueEnum::PointerValue(r)) => {
            let l_int = ctx
                .builder()
                .build_ptr_to_int(*l, ctx.context.i64_type(), "l_ptr")
                .unwrap();
            let r_int = ctx
                .builder()
                .build_ptr_to_int(*r, ctx.context.i64_type(), "r_ptr")
                .unwrap();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::UGT, l_int, r_int, "gt")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        // Tagged dyn struct comparison: extract i64 value and compare
        (BasicValueEnum::StructValue(l), BasicValueEnum::StructValue(r)) => {
            let l_val = ctx
                .builder()
                .build_extract_value(*l, 0, "l_val")
                .unwrap()
                .into_int_value();
            let r_val = ctx
                .builder()
                .build_extract_value(*r, 0, "r_val")
                .unwrap()
                .into_int_value();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::SGT, l_val, r_val, "gt")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        // Mixed: int vs dyn struct
        (BasicValueEnum::IntValue(l), BasicValueEnum::StructValue(r)) => {
            let r_val = ctx
                .builder()
                .build_extract_value(*r, 0, "r_val")
                .unwrap()
                .into_int_value();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::SGT, *l, r_val, "gt")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        // Mixed: dyn struct vs int
        (BasicValueEnum::StructValue(l), BasicValueEnum::IntValue(r)) => {
            let l_val = ctx
                .builder()
                .build_extract_value(*l, 0, "l_val")
                .unwrap()
                .into_int_value();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::SGT, l_val, *r, "gt")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        _ => Err("Invalid operands for >".to_string()),
    }
}

fn compile_le<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    // 数值类型比较统一处理
    if let Some(result) = build_numeric_cmp(
        ctx,
        left,
        right,
        IntPredicate::SLE,
        FloatPredicate::OLE,
        "le",
    )? {
        return Ok(result);
    }
    match (&left.value, &right.value) {
        // Generic/dyn pointer comparison: compare as unsigned integers
        (BasicValueEnum::PointerValue(l), BasicValueEnum::PointerValue(r)) => {
            let l_int = ctx
                .builder()
                .build_ptr_to_int(*l, ctx.context.i64_type(), "l_ptr")
                .unwrap();
            let r_int = ctx
                .builder()
                .build_ptr_to_int(*r, ctx.context.i64_type(), "r_ptr")
                .unwrap();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::ULE, l_int, r_int, "le")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        // Tagged dyn struct comparison: extract i64 value and compare
        (BasicValueEnum::StructValue(l), BasicValueEnum::StructValue(r)) => {
            let l_val = ctx
                .builder()
                .build_extract_value(*l, 0, "l_val")
                .unwrap()
                .into_int_value();
            let r_val = ctx
                .builder()
                .build_extract_value(*r, 0, "r_val")
                .unwrap()
                .into_int_value();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::SLE, l_val, r_val, "le")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        // Mixed: int vs dyn struct
        (BasicValueEnum::IntValue(l), BasicValueEnum::StructValue(r)) => {
            let r_val = ctx
                .builder()
                .build_extract_value(*r, 0, "r_val")
                .unwrap()
                .into_int_value();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::SLE, *l, r_val, "le")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        // Mixed: dyn struct vs int
        (BasicValueEnum::StructValue(l), BasicValueEnum::IntValue(r)) => {
            let l_val = ctx
                .builder()
                .build_extract_value(*l, 0, "l_val")
                .unwrap()
                .into_int_value();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::SLE, l_val, *r, "le")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        _ => Err("Invalid operands for <=".to_string()),
    }
}

fn compile_ge<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    // 数值类型比较统一处理
    if let Some(result) = build_numeric_cmp(
        ctx,
        left,
        right,
        IntPredicate::SGE,
        FloatPredicate::OGE,
        "ge",
    )? {
        return Ok(result);
    }
    match (&left.value, &right.value) {
        // Generic/dyn pointer comparison: compare as unsigned integers
        (BasicValueEnum::PointerValue(l), BasicValueEnum::PointerValue(r)) => {
            let l_int = ctx
                .builder()
                .build_ptr_to_int(*l, ctx.context.i64_type(), "l_ptr")
                .unwrap();
            let r_int = ctx
                .builder()
                .build_ptr_to_int(*r, ctx.context.i64_type(), "r_ptr")
                .unwrap();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::UGE, l_int, r_int, "ge")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        // Tagged dyn struct comparison: extract i64 value and compare
        (BasicValueEnum::StructValue(l), BasicValueEnum::StructValue(r)) => {
            let l_val = ctx
                .builder()
                .build_extract_value(*l, 0, "l_val")
                .unwrap()
                .into_int_value();
            let r_val = ctx
                .builder()
                .build_extract_value(*r, 0, "r_val")
                .unwrap()
                .into_int_value();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::SGE, l_val, r_val, "ge")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        // Mixed: int vs dyn struct
        (BasicValueEnum::IntValue(l), BasicValueEnum::StructValue(r)) => {
            let r_val = ctx
                .builder()
                .build_extract_value(*r, 0, "r_val")
                .unwrap()
                .into_int_value();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::SGE, *l, r_val, "ge")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        // Mixed: dyn struct vs int
        (BasicValueEnum::StructValue(l), BasicValueEnum::IntValue(r)) => {
            let l_val = ctx
                .builder()
                .build_extract_value(*l, 0, "l_val")
                .unwrap()
                .into_int_value();
            let res = ctx
                .builder()
                .build_int_compare(IntPredicate::SGE, l_val, *r, "ge")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        _ => Err("Invalid operands for >=".to_string()),
    }
}

fn compile_and<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder().build_and(*l, *r, "and").unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        _ => Err("Invalid operands for &&".to_string()),
    }
}

fn compile_or<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder().build_or(*l, *r, "or").unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
        }
        _ => Err("Invalid operands for ||".to_string()),
    }
}

fn compile_bitwise_and<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder().build_and(*l, *r, "band").unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
        }
        _ => Err("Invalid operands for &".to_string()),
    }
}

fn compile_bitwise_or<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder().build_or(*l, *r, "bor").unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
        }
        _ => Err("Invalid operands for |".to_string()),
    }
}

fn compile_bitwise_xor<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder().build_xor(*l, *r, "bxor").unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
        }
        _ => Err("Invalid operands for ^".to_string()),
    }
}

fn compile_shl<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx.builder().build_left_shift(*l, *r, "shl").unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
        }
        _ => Err("Invalid operands for <<".to_string()),
    }
}

fn compile_shr<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx
                .builder()
                .build_right_shift(*l, *r, true, "shr")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
        }
        _ => Err("Invalid operands for >>".to_string()),
    }
}

fn compile_ushr<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    left: &ExprResult<'ctx>,
    right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    match (&left.value, &right.value) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let res = ctx
                .builder()
                .build_right_shift(*l, *r, false, "ushr")
                .unwrap();
            Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
        }
        _ => Err("Invalid operands for >>>".to_string()),
    }
}

fn compile_nullish<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    left_expr: &Expr,
    right_expr: &Expr,
    _left: &ExprResult<'ctx>,
    _right: &ExprResult<'ctx>,
) -> Result<ExprResult<'ctx>, String> {
    let left_result = compile_expr(ctx, left_expr)?;

    if let BasicValueEnum::PointerValue(ptr) = left_result.value {
        let is_null = ctx
            .builder()
            .build_ptr_to_int(ptr, ctx.context.i64_type(), "ptr_to_int")
            .unwrap();
        let zero = ctx.context.i64_type().const_int(0, false);
        let cond = ctx
            .builder()
            .build_int_compare(IntPredicate::EQ, is_null, zero, "is_null")
            .unwrap();

        let current_bb = ctx.builder().get_insert_block().unwrap();
        let func = current_bb.get_parent().unwrap();
        let then_bb = ctx.context.append_basic_block(func, "nullish_then");
        let else_bb = ctx.context.append_basic_block(func, "nullish_else");
        let merge_bb = ctx.context.append_basic_block(func, "nullish_merge");

        ctx.builder()
            .build_conditional_branch(cond, then_bb, else_bb)
            .unwrap();

        ctx.builder().position_at_end(then_bb);
        let right_result = compile_expr(ctx, right_expr)?;
        ctx.builder().build_unconditional_branch(merge_bb).unwrap();
        let then_bb_end = ctx.builder().get_insert_block().unwrap();

        ctx.builder().position_at_end(else_bb);
        ctx.builder().build_unconditional_branch(merge_bb).unwrap();

        ctx.builder().position_at_end(merge_bb);
        let phi = ctx
            .builder()
            .build_phi(left_result.value.get_type(), "nullish_phi")
            .unwrap();
        phi.add_incoming(&[
            (&right_result.value, then_bb_end),
            (&left_result.value, else_bb),
        ]);

        return Ok(ExprResult::new(phi.as_basic_value(), right_result.ty));
    }

    Ok(left_result)
}

fn compile_tuple_literal<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    exprs: &[Expr],
) -> Result<ExprResult<'ctx>, String> {
    if exprs.is_empty() {
        return Ok(ExprResult::new(
            BasicValueEnum::StructValue(ctx.context.struct_type(&[], false).const_zero()),
            Type::Tuple(vec![]),
        ));
    }

    let mut elem_results = Vec::new();
    for e in exprs {
        elem_results.push(compile_expr(ctx, e)?);
    }

    let elem_types: Vec<Type> = elem_results.iter().map(|r| r.ty.clone()).collect();
    let tuple_type = Type::Tuple(elem_types);
    let llvm_struct_ty = ruyi_type_to_llvm(ctx.context, &tuple_type);
    let struct_type = match llvm_struct_ty {
        inkwell::types::BasicTypeEnum::StructType(st) => st,
        _ => return Err("Tuple type did not map to struct".to_string()),
    };

    let mut struct_val = struct_type.const_zero();
    for (i, elem) in elem_results.iter().enumerate() {
        struct_val = ctx
            .builder()
            .build_insert_value(
                struct_val,
                elem.value,
                i as u32,
                &format!("tuple_elem_{}", i),
            )
            .unwrap()
            .into_struct_value();
    }

    Ok(ExprResult::new(
        BasicValueEnum::StructValue(struct_val),
        tuple_type,
    ))
}

fn compile_unary<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    op: &UnaryOp,
    operand: &Expr,
) -> Result<ExprResult<'ctx>, String> {
    let operand_result = compile_expr(ctx, operand)?;

    match op {
        UnaryOp::Minus => match operand_result.value {
            BasicValueEnum::IntValue(v) => {
                let zero = ctx.context.i64_type().const_int(0, false);
                let res = ctx.builder().build_int_sub(zero, v, "neg").unwrap();
                Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
            }
            BasicValueEnum::FloatValue(v) => {
                let zero = ctx.context.f64_type().const_float(0.0);
                let res = ctx.builder().build_float_sub(zero, v, "fneg").unwrap();
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
                let res = ctx.builder().build_xor(v, one, "not").unwrap();
                Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Bool))
            }
            _ => Err(format!(
                "Invalid operand for ! (expected bool/int, got {:?})",
                operand_result.ty
            )),
        },
        UnaryOp::Tilde => match operand_result.value {
            BasicValueEnum::IntValue(v) => {
                let minus_one = ctx.context.i64_type().const_int(u64::MAX, true);
                let res = ctx.builder().build_xor(v, minus_one, "bitnot").unwrap();
                Ok(ExprResult::new(BasicValueEnum::IntValue(res), Type::Int))
            }
            _ => Err("Invalid operand for ~".to_string()),
        },
        UnaryOp::Await => super::async_codegen::compile_await(ctx, operand),
        UnaryOp::Typeof => {
            // Determine the type name string from the operand's static type.
            let type_name = match &operand_result.ty {
                Type::Int => "int",
                Type::Float => "float",
                Type::Bool => "bool",
                Type::String => "string",
                Type::Null => "null",
                Type::BigInt => "bigint",
                Type::Byte => "byte",
                Type::Array(_) => "array",
                Type::Named(n, _) => n.as_str(),
                Type::Nullable(inner) => {
                    // If the operand is a null literal, report "null".
                    if matches!(operand, Expr::NullLiteral) {
                        "null"
                    } else {
                        match inner.as_ref() {
                            Type::Int => "int",
                            Type::Float => "float",
                            Type::Bool => "bool",
                            Type::String => "string",
                            Type::Named(n, _) => n.as_str(),
                            _ => "object",
                        }
                    }
                }
                Type::Dynamic => "dynamic",
                _ => "object",
            };
            let global = ctx
                .builder()
                .build_global_string_ptr(type_name, "typeof_str")
                .unwrap();
            Ok(ExprResult::new(
                BasicValueEnum::PointerValue(global.as_pointer_value()),
                Type::String,
            ))
        }
        _ => Err(format!("Unsupported unary operator: {:?}", op)),
    }
}

/// Build a function call: uses `invoke` when inside a try context (for EH propagation
/// through landing pads), otherwise uses a plain `call` instruction.
fn build_call_or_invoke<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    func: FunctionValue<'ctx>,
    args: &[inkwell::values::BasicMetadataValueEnum<'ctx>],
    name: &str,
) -> inkwell::values::CallSiteValue<'ctx> {
    use inkwell::values::BasicMetadataValueEnum;
    match ctx.try_frame_stack.last().map(|f| f.landing_pad_bb) {
        Some(unwind_bb) => {
            let then_bb = ctx
                .context
                .append_basic_block(ctx.current_function().unwrap(), &format!("{}.then", name));
            let invoke_args: Vec<BasicValueEnum<'ctx>> = args
                .iter()
                .map(|v| match *v {
                    BasicMetadataValueEnum::IntValue(iv) => iv.as_basic_value_enum(),
                    BasicMetadataValueEnum::FloatValue(fv) => fv.as_basic_value_enum(),
                    BasicMetadataValueEnum::PointerValue(pv) => pv.as_basic_value_enum(),
                    BasicMetadataValueEnum::StructValue(sv) => sv.as_basic_value_enum(),
                    BasicMetadataValueEnum::ArrayValue(av) => av.as_basic_value_enum(),
                    BasicMetadataValueEnum::VectorValue(vv) => vv.as_basic_value_enum(),
                    BasicMetadataValueEnum::ScalableVectorValue(sv) => sv.as_basic_value_enum(),
                    BasicMetadataValueEnum::MetadataValue(_) => {
                        unreachable!("MetadataValue not used as function argument")
                    }
                })
                .collect();
            let lp_gen = LandingPadGenerator::new(ctx.context, ctx.module, ctx.builder());
            let invoke = lp_gen.build_invoke(func, &invoke_args, then_bb, unwind_bb, name);
            ctx.builder().position_at_end(then_bb);
            invoke
        }
        None => ctx.builder().build_call(func, args, name).unwrap(),
    }
}

fn emit_spread_args<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    args: &[crate::parser::ast::Argument],
    arg_values: &mut Vec<inkwell::values::BasicMetadataValueEnum<'ctx>>,
) -> Result<(), String> {
    for arg in args {
        match arg {
            crate::parser::ast::Argument::Expr(e) => {
                let result = compile_expr(ctx, e)?;
                arg_values.push(result.value.into());
            }
            crate::parser::ast::Argument::Spread(expr) => {
                let spread_result = compile_expr(ctx, expr)?;
                let arr_ptr = match spread_result.value {
                    BasicValueEnum::PointerValue(p) => p,
                    _ => return Err("Spread argument must be an array".to_string()),
                };
                let len_fn = ctx
                    .module
                    .get_function("__builtin_array_length")
                    .ok_or("__builtin_array_length not declared")?;
                let len_val = ctx
                    .builder()
                    .build_call(len_fn, &[arr_ptr.into()], "spread_len")
                    .unwrap()
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_int_value();
                let zero = ctx.context.i64_type().const_int(0, false);
                let one = ctx.context.i64_type().const_int(1, false);
                // For Dynamic arrays each slot is a 16-byte `{i64, ptr}`
                // struct; `__builtin_array_get` returns only the first
                // 8-byte word, so we cannot use it here. Detect the
                // element type from the spread expression and emit a
                // direct GEP+load when the element layout is Dynamic.
                let is_dyn_elem = matches!(
                    spread_result.ty,
                    Type::Array(ref inner) if matches!(inner.as_ref(), Type::Dynamic)
                );
                let dyn_struct_ty = if is_dyn_elem {
                    Some(ruyi_type_to_llvm(ctx.context, &Type::Dynamic).into_struct_type())
                } else {
                    None
                };
                let data_base = if is_dyn_elem {
                    Some(load_array_data_ptr(ctx, arr_ptr, "spread_dyn_data"))
                } else {
                    None
                };
                let get_fn = ctx
                    .module
                    .get_function("__builtin_array_get")
                    .ok_or("__builtin_array_get not declared")?;
                let idx_ptr = ctx
                    .builder()
                    .build_alloca(ctx.context.i64_type(), "spread_idx")
                    .unwrap();
                ctx.builder().build_store(idx_ptr, zero).unwrap();
                let loop_bb = ctx.context.append_basic_block(
                    ctx.current_function.ok_or("No current function")?,
                    "spread_loop",
                );
                let done_bb = ctx.context.append_basic_block(
                    ctx.current_function.ok_or("No current function")?,
                    "spread_done",
                );
                ctx.builder().build_unconditional_branch(loop_bb).unwrap();
                ctx.builder().position_at_end(loop_bb);
                let idx = ctx
                    .builder()
                    .build_load(ctx.context.i64_type(), idx_ptr, "idx")
                    .unwrap()
                    .into_int_value();
                let at_end = ctx
                    .builder()
                    .build_int_compare(inkwell::IntPredicate::UGE, idx, len_val, "at_end")
                    .unwrap();
                let body_bb = ctx.context.append_basic_block(
                    ctx.current_function.ok_or("No current function")?,
                    "spread_body",
                );
                ctx.builder()
                    .build_conditional_branch(at_end, done_bb, body_bb)
                    .unwrap();
                ctx.builder().position_at_end(body_bb);
                if let (Some(struct_ty), Some(data)) = (dyn_struct_ty, data_base) {
                    let stride = ctx.context.i64_type().const_int(16, false);
                    let byte_offset = ctx
                        .builder()
                        .build_int_mul(idx, stride, "spread_dyn_stride")
                        .unwrap();
                    let offset_i32 = ctx
                        .builder()
                        .build_int_truncate(byte_offset, ctx.context.i32_type(), "spread_dyn_off32")
                        .unwrap();
                    let elem_gep = unsafe {
                        ctx.builder()
                            .build_gep(ctx.context.i8_type(), data, &[offset_i32], "spread_dyn_gep")
                            .unwrap()
                    };
                    let struct_ptr = ctx
                        .builder()
                        .build_pointer_cast(
                            elem_gep,
                            ctx.context.ptr_type(Default::default()),
                            "spread_dyn_struct_ptr",
                        )
                        .unwrap();
                    let loaded = ctx
                        .builder()
                        .build_load(struct_ty, struct_ptr, "spread_dyn_elem")
                        .unwrap();
                    arg_values.push(loaded.into());
                } else {
                    let elem = ctx
                        .builder()
                        .build_call(get_fn, &[arr_ptr.into(), idx.into()], "spread_elem")
                        .unwrap()
                        .try_as_basic_value()
                        .unwrap_basic()
                        .into_int_value();
                    arg_values.push(elem.into());
                }
                let next_idx = ctx.builder().build_int_add(idx, one, "next_idx").unwrap();
                ctx.builder().build_store(idx_ptr, next_idx).unwrap();
                ctx.builder().build_unconditional_branch(loop_bb).unwrap();
                ctx.builder().position_at_end(done_bb);
            }
        }
    }
    Ok(())
}

/// Dispatch a method call on a Dynamic-typed receiver at runtime.
///
/// Two paths:
///   1. `toString()` — switch on the Dynamic tag (field 0) and call the
///      appropriate `ruyi_*_to_string` builtin.
///   2. Class methods — load the `__typeid` from byte offset 0 of the
///      data pointer (field 1), then branch to the matching
///      `{Class}_{method}` implementation.
fn compile_dynamic_method_call<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    object: &Expr,
    method_name: &str,
    args: &[crate::parser::ast::Argument],
) -> Result<ExprResult<'ctx>, String> {
    let obj_result = compile_expr(ctx, object)?;
    let dyn_struct = obj_result.value.into_struct_value();
    let tag = ctx
        .builder()
        .build_extract_value(dyn_struct, 0, "dyn_tag")
        .unwrap()
        .into_int_value();
    let data_ptr = ctx
        .builder()
        .build_extract_value(dyn_struct, 1, "dyn_data")
        .unwrap()
        .into_pointer_value();

    // ── Path 1: toString() — tag-based runtime dispatch ──────────────
    // Boxing convention for field 1 (data_ptr):
    //   inttoptr(1) = int, inttoptr(2) = float, inttoptr(3) = bool,
    //   real pointer (> 3) = string
    if method_name == "toString" {
        let func = ctx.current_function().ok_or("No current function")?;
        let i8_ptr_ty = ctx.context.ptr_type(Default::default());
        let i64_ty = ctx.context.i64_type();

        // Convert data_ptr to i64 to check the tag value
        let ptr_as_int = ctx
            .builder()
            .build_ptr_to_int(data_ptr, i64_ty, "ptr_tag")
            .unwrap();
        let three = i64_ty.const_int(3, false);
        let is_primitive = ctx
            .builder()
            .build_int_compare(IntPredicate::ULE, ptr_as_int, three, "is_prim")
            .unwrap();

        let prim_bb = ctx.context.append_basic_block(func, "dyn_ts_prim");
        let str_bb = ctx.context.append_basic_block(func, "dyn_ts_str");
        let merge_bb = ctx.context.append_basic_block(func, "dyn_ts_merge");
        ctx.builder()
            .build_conditional_branch(is_primitive, prim_bb, str_bb)
            .unwrap();

        // ── Primitive: dispatch on tag value (1=int, 2=float, 3=bool) ──
        ctx.builder().position_at_end(prim_bb);
        let int_bb = ctx.context.append_basic_block(func, "dyn_ts_int");
        let check_float_bb = ctx.context.append_basic_block(func, "dyn_ts_chk_f");
        let float_bb = ctx.context.append_basic_block(func, "dyn_ts_float");
        let bool_bb = ctx.context.append_basic_block(func, "dyn_ts_bool");

        let one = i64_ty.const_int(1, false);
        let two = i64_ty.const_int(2, false);

        let is_int = ctx
            .builder()
            .build_int_compare(IntPredicate::EQ, ptr_as_int, one, "is_int")
            .unwrap();
        ctx.builder()
            .build_conditional_branch(is_int, int_bb, check_float_bb)
            .unwrap();

        // int → ruyi_int_to_string(field0)
        ctx.builder().position_at_end(int_bb);
        let int_fn = ctx
            .module
            .get_function("ruyi_int_to_string")
            .ok_or("ruyi_int_to_string not declared")?;
        let int_str = ctx
            .builder()
            .build_call(int_fn, &[tag.into()], "int_to_str")
            .unwrap()
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        ctx.builder().build_unconditional_branch(merge_bb).unwrap();

        // check float
        ctx.builder().position_at_end(check_float_bb);
        let is_float = ctx
            .builder()
            .build_int_compare(IntPredicate::EQ, ptr_as_int, two, "is_float")
            .unwrap();
        ctx.builder()
            .build_conditional_branch(is_float, float_bb, bool_bb)
            .unwrap();

        // float → ruyi_float_to_string(bitcast field0 → f64)
        ctx.builder().position_at_end(float_bb);
        let float_fn = ctx
            .module
            .get_function("ruyi_float_to_string")
            .ok_or("ruyi_float_to_string not declared")?;
        let f64_val = ctx
            .builder()
            .build_bit_cast(tag, ctx.context.f64_type(), "tag_f64")
            .unwrap();
        let float_str = ctx
            .builder()
            .build_call(float_fn, &[f64_val.into()], "float_to_str")
            .unwrap()
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        ctx.builder().build_unconditional_branch(merge_bb).unwrap();

        // bool → ruyi_bool_to_string(trunc field0 → i1)
        ctx.builder().position_at_end(bool_bb);
        let bool_fn = ctx
            .module
            .get_function("ruyi_bool_to_string")
            .ok_or("ruyi_bool_to_string not declared")?;
        let b1 = ctx
            .builder()
            .build_int_truncate(tag, ctx.context.bool_type(), "tag_b1")
            .unwrap();
        let bool_str = ctx
            .builder()
            .build_call(bool_fn, &[b1.into()], "bool_to_str")
            .unwrap()
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        ctx.builder().build_unconditional_branch(merge_bb).unwrap();

        // string → data_ptr is already i8*
        ctx.builder().position_at_end(str_bb);
        ctx.builder().build_unconditional_branch(merge_bb).unwrap();

        // Merge via phi
        ctx.builder().position_at_end(merge_bb);
        let phi = ctx.builder().build_phi(i8_ptr_ty, "dyn_ts_result").unwrap();
        phi.add_incoming(&[
            (&int_str, int_bb),
            (&float_str, float_bb),
            (&bool_str, bool_bb),
            (&data_ptr, str_bb),
        ]);

        return Ok(ExprResult::new(
            BasicValueEnum::PointerValue(phi.as_basic_value().into_pointer_value()),
            Type::String,
        ));
    }

    // ── Path 2: class method — typeid-based runtime dispatch ─────────
    // Collect all classes that define a method with this name.
    // Search the LLVM module for functions matching `{Class}_{method}` pattern.
    let candidate_classes: Vec<(u64, String)> = ctx
        .type_ids
        .iter()
        .filter_map(|(class_name, &type_id)| {
            let fn_name = format!("{}_{}", class_name, method_name);
            if ctx.module.get_function(&fn_name).is_some() {
                Some((type_id, class_name.clone()))
            } else {
                None
            }
        })
        .collect();

    if candidate_classes.is_empty() {
        return Err(format!(
            "No class defines method '{}' for Dynamic dispatch",
            method_name
        ));
    }

    // Determine the return type from the first candidate's function signature.
    let first_fn_name = format!("{}_{}", candidate_classes[0].1, method_name);
    let _ret_ty = ctx
        .function_types
        .get(&first_fn_name)
        .and_then(|ft| {
            if let Type::Function { return_type, .. } = ft {
                Some(*return_type.clone())
            } else {
                None
            }
        })
        .unwrap_or(Type::Dynamic);

    // Compile method arguments.
    let mut user_arg_values: Vec<BasicValueEnum<'ctx>> = Vec::new();
    for arg in args {
        if let crate::parser::ast::Argument::Expr(e) = arg {
            let r = compile_expr(ctx, e)?;
            user_arg_values.push(r.value);
        } else {
            return Err("Spread arguments not supported in Dynamic dispatch".to_string());
        }
    }

    let func = ctx.current_function().ok_or("No current function")?;
    let i64_ty = ctx.context.i64_type();
    let i64_ptr_ty = ctx.context.ptr_type(Default::default());

    // Load __typeid from byte offset 0 of data_ptr.
    let casted_ptr = ctx
        .builder()
        .build_pointer_cast(data_ptr, i64_ptr_ty, "typeid_cast")
        .unwrap();
    let typeid = ctx
        .builder()
        .build_load(i64_ty, casted_ptr, "typeid")
        .unwrap()
        .into_int_value();

    // Save the entry block so we can add a branch to the first check block.
    let entry_bb = ctx.builder().get_insert_block().unwrap();

    // Build if-else chain: for each candidate class, compare typeid and call.
    let merge_bb = ctx.context.append_basic_block(func, "dyn_dispatch_merge");

    // Determine the effective return type by checking all candidates.
    // If they all agree (ignoring void), use that type; otherwise Dynamic.
    let i8_ptr_ty = ctx.context.ptr_type(Default::default());
    let candidate_ret_types: Vec<Type> = candidate_classes
        .iter()
        .map(|(_, cn)| {
            let fn_key = format!("{}_{}", cn, method_name);
            ctx.function_types
                .get(&fn_key)
                .and_then(|ft| {
                    if let Type::Function { return_type, .. } = ft {
                        Some(*return_type.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or(Type::Dynamic)
        })
        .collect();

    let effective_ret_ty = {
        let non_void: Vec<&Type> = candidate_ret_types
            .iter()
            .filter(|t| **t != Type::Void)
            .collect();
        if non_void.is_empty() {
            Type::Void
        } else if non_void.iter().all(|t| **t == *non_void[0]) {
            non_void[0].clone()
        } else {
            Type::Dynamic
        }
    };

    // Phi always holds a Dynamic struct {i64, i8*} to handle mixed return types.
    let dyn_struct_ty = ruyi_type_to_llvm(ctx.context, &Type::Dynamic).into_struct_type();
    let phi_type = dyn_struct_ty.as_basic_type_enum();

    let mut incoming: Vec<(BasicValueEnum<'ctx>, inkwell::basic_block::BasicBlock)> = Vec::new();
    let mut prev_check_bb: Option<inkwell::basic_block::BasicBlock> = None;
    let mut first_check_bb: Option<inkwell::basic_block::BasicBlock> = None;
    let mut last_check_bb: Option<inkwell::basic_block::BasicBlock> = None;

    for (i, (type_id, class_name)) in candidate_classes.iter().enumerate() {
        let fn_name = format!("{}_{}", class_name, method_name);
        let check_bb = ctx
            .context
            .append_basic_block(func, &format!("dyn_chk_{}", class_name));
        let call_bb = ctx
            .context
            .append_basic_block(func, &format!("dyn_call_{}", class_name));

        // From previous dyn_next block, branch to this iteration's check block.
        if let Some(prev_else) = prev_check_bb {
            ctx.builder().position_at_end(prev_else);
            ctx.builder().build_unconditional_branch(check_bb).unwrap();
        }

        // Remember the first check block so entry can branch to it.
        if i == 0 {
            first_check_bb = Some(check_bb);
        }

        ctx.builder().position_at_end(check_bb);
        let expected_id = i64_ty.const_int(*type_id, false);
        let matches = ctx
            .builder()
            .build_int_compare(IntPredicate::EQ, typeid, expected_id, "typeid_match")
            .unwrap();

        // If last candidate, else goes to merge with a default zero value.
        let else_bb = if i < candidate_classes.len() - 1 {
            ctx.context
                .append_basic_block(func, &format!("dyn_next_{}", class_name))
        } else {
            merge_bb
        };
        ctx.builder()
            .build_conditional_branch(matches, call_bb, else_bb)
            .unwrap();

        // Call block: invoke {Class}_{method}(data_ptr, args...).
        ctx.builder().position_at_end(call_bb);
        let target_fn = ctx.module.get_function(&fn_name).ok_or_else(|| {
            format!(
                "Dynamic dispatch: function '{}' not found in module",
                fn_name
            )
        })?;

        let mut call_args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> = Vec::new();
        // self argument = data_ptr (the class instance pointer).
        call_args.push(data_ptr.into());
        // User arguments.
        for v in &user_arg_values {
            call_args.push((*v).into());
        }

        let call_site = ctx
            .builder()
            .build_call(target_fn, &call_args, "dyn_call")
            .unwrap();
        let call_val = call_site.try_as_basic_value().basic();

        // Box the call result into a Dynamic struct for phi compatibility.
        let candidate_ty = &candidate_ret_types[i];
        let boxed: BasicValueEnum<'ctx> = match (candidate_ty, call_val) {
            (Type::Void, _) => {
                let zero_tag = i64_ty.const_int(0, false);
                let null_data = i8_ptr_ty.const_null();
                let s = dyn_struct_ty.const_zero();
                let s = ctx
                    .builder()
                    .build_insert_value(s, zero_tag, 0, "dt")
                    .unwrap()
                    .into_struct_value();
                let s = ctx
                    .builder()
                    .build_insert_value(s, null_data, 1, "dd")
                    .unwrap()
                    .into_struct_value();
                BasicValueEnum::StructValue(s)
            }
            (_, Some(BasicValueEnum::IntValue(v))) => {
                // int/bool/byte: tag = inttoptr(1), data = inttoptr(value)
                let tag = i64_ty.const_int(1, false);
                let data = ctx.builder().build_int_to_ptr(v, i8_ptr_ty, "d2p").unwrap();
                let s = dyn_struct_ty.const_zero();
                let s = ctx
                    .builder()
                    .build_insert_value(s, tag, 0, "dt")
                    .unwrap()
                    .into_struct_value();
                let s = ctx
                    .builder()
                    .build_insert_value(s, data, 1, "dd")
                    .unwrap()
                    .into_struct_value();
                BasicValueEnum::StructValue(s)
            }
            (_, Some(BasicValueEnum::FloatValue(v))) => {
                let bits = ctx
                    .builder()
                    .build_bit_cast(v, i64_ty, "fbits")
                    .unwrap()
                    .into_int_value();
                let tag = i64_ty.const_int(2, false);
                let data = ctx
                    .builder()
                    .build_int_to_ptr(bits, i8_ptr_ty, "d2p")
                    .unwrap();
                let s = dyn_struct_ty.const_zero();
                let s = ctx
                    .builder()
                    .build_insert_value(s, tag, 0, "dt")
                    .unwrap()
                    .into_struct_value();
                let s = ctx
                    .builder()
                    .build_insert_value(s, data, 1, "dd")
                    .unwrap()
                    .into_struct_value();
                BasicValueEnum::StructValue(s)
            }
            (_, Some(BasicValueEnum::PointerValue(p))) => {
                // string/named: tag = 0, data = ptr
                let tag = i64_ty.const_int(0, false);
                let data = ctx
                    .builder()
                    .build_pointer_cast(p, i8_ptr_ty, "dc")
                    .unwrap();
                let s = dyn_struct_ty.const_zero();
                let s = ctx
                    .builder()
                    .build_insert_value(s, tag, 0, "dt")
                    .unwrap()
                    .into_struct_value();
                let s = ctx
                    .builder()
                    .build_insert_value(s, data, 1, "dd")
                    .unwrap()
                    .into_struct_value();
                BasicValueEnum::StructValue(s)
            }
            _ => {
                // Fallback: zero struct
                BasicValueEnum::StructValue(dyn_struct_ty.const_zero())
            }
        };
        incoming.push((boxed, call_bb));
        ctx.builder().build_unconditional_branch(merge_bb).unwrap();

        prev_check_bb = Some(else_bb);
        last_check_bb = Some(check_bb);
    }

    // Branch from the entry block to the first check block.
    if let Some(first_chk) = first_check_bb {
        ctx.builder().position_at_end(entry_bb);
        ctx.builder().build_unconditional_branch(first_chk).unwrap();
    }

    // Default Dynamic struct for the no-match path.
    let default_val = {
        let zero_tag = i64_ty.const_int(0, false);
        let null_data = i8_ptr_ty.const_null();
        let s = dyn_struct_ty.const_zero();
        let s = ctx
            .builder()
            .build_insert_value(s, zero_tag, 0, "dt")
            .unwrap()
            .into_struct_value();
        let s = ctx
            .builder()
            .build_insert_value(s, null_data, 1, "dd")
            .unwrap()
            .into_struct_value();
        BasicValueEnum::StructValue(s)
    };

    // If the last candidate's else branch goes directly to merge_bb,
    // the edge is from check_bb → merge_bb (via the conditional branch).
    if let Some(last_else) = prev_check_bb {
        if last_else == merge_bb {
            if let Some(last_chk) = last_check_bb {
                incoming.push((default_val, last_chk));
            }
        }
    }

    ctx.builder().position_at_end(merge_bb);

    if effective_ret_ty == Type::Void {
        return Ok(ExprResult::new(
            BasicValueEnum::IntValue(i64_ty.const_int(0, false)),
            Type::Void,
        ));
    }

    let phi = ctx
        .builder()
        .build_phi(phi_type, "dyn_dispatch_result")
        .unwrap();
    let incoming_refs: Vec<(
        &dyn inkwell::values::BasicValue<'ctx>,
        inkwell::basic_block::BasicBlock,
    )> = incoming
        .iter()
        .map(|(v, bb)| (v as &dyn inkwell::values::BasicValue<'ctx>, *bb))
        .collect();
    phi.add_incoming(&incoming_refs);

    // Extract the data_ptr from the Dynamic phi result.
    let phi_struct = phi.as_basic_value().into_struct_value();
    let result_data = ctx
        .builder()
        .build_extract_value(phi_struct, 1, "rd")
        .unwrap()
        .into_pointer_value();

    // Convert the data pointer to the effective return type.
    let result_value: BasicValueEnum<'ctx> = match &effective_ret_ty {
        Type::String => BasicValueEnum::PointerValue(result_data),
        Type::Int | Type::Byte => {
            let v = ctx
                .builder()
                .build_ptr_to_int(result_data, i64_ty, "d2i")
                .unwrap();
            BasicValueEnum::IntValue(v)
        }
        Type::Bool => {
            let v64 = ctx
                .builder()
                .build_ptr_to_int(result_data, i64_ty, "d2i")
                .unwrap();
            let v1 = ctx
                .builder()
                .build_int_truncate(v64, ctx.context.bool_type(), "d2b")
                .unwrap();
            BasicValueEnum::IntValue(v1)
        }
        Type::Named(_, _) => BasicValueEnum::PointerValue(result_data),
        Type::Dynamic => BasicValueEnum::StructValue(phi_struct),
        _ => BasicValueEnum::IntValue(i64_ty.const_int(0, false)),
    };

    Ok(ExprResult::new(result_value, effective_ret_ty))
}

/// Map a union-type alias name (e.g. `Result`, `Option`) to the list of
/// concrete member classes it expands to. Returns `None` for non-alias names
/// so the caller can fall through to the regular dispatch path.
fn union_alias_members(union_name: &str) -> Option<&'static [&'static str]> {
    match union_name {
        "Result" => Some(&["Ok", "Err"]),
        "Option" => Some(&["Some", "None"]),
        _ => None,
    }
}

/// Resolve a union-alias field access to the union member that actually
/// stores it. Only fields that are unambiguous to a single member are
/// forwarded (`Option<T>.value` → `Some`, `Result<T,E>.error` → `Err`).
/// Other fields fall through to the normal dispatch, which then errors
/// out as before for missing members.
fn union_alias_forwarded_field(class_name: &str, field_name: &str) -> Option<String> {
    match (class_name, field_name) {
        ("Option", "value") => Some("Some".to_string()),
        ("Result", "error") => Some("Err".to_string()),
        _ => None,
    }
}

/// When a function exists only as a forward declaration (no body), replace
/// it with an `unreachable` body so calls into it still link. This is the
/// fallback path for `Result<T,E>.unwrap()` style methods whose body
/// codegen fails — typically because their body throws, which is
/// legitimately bottom. Marking the body `unreachable` preserves the
/// "this path should never be executed" intent at runtime: if a caller
/// does invoke it (e.g. union dispatch on a value that's actually the
/// "throw" variant), the program traps instead of crashing with a
/// linker error.
fn ensure_function_has_unreachable_body<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    func: inkwell::values::FunctionValue<'ctx>,
) {
    if func.count_basic_blocks() > 0 {
        return;
    }
    let saved_function = ctx.current_function();
    let saved_bb = ctx.builder().get_insert_block();

    ctx.set_current_function(Some(func));
    let entry = ctx.context.append_basic_block(func, "entry_fallback");
    ctx.builder().position_at_end(entry);
    ctx.builder().build_unreachable().unwrap();

    // Restore the caller's builder position and current function so
    // subsequent instructions go back into the union dispatch IR,
    // not the fallback block we just created.
    ctx.set_current_function(saved_function);
    if let Some(bb) = saved_bb {
        ctx.builder().position_at_end(bb);
    }
}

/// When the static receiver type is the union alias `Result<T, E>` or
/// `Option<T>`, extract the type arguments used to specialize the union
/// members' methods. The members inherit the alias's type parameters in
/// declaration order, so `Result<T, E>` becomes `[T, E]` for both `Ok` and
/// `Err`, and `Option<T>` becomes `[T]` for both `Some` and `None`.
fn union_alias_member_type_args(recv_ty: Option<&Type>, class_name: &str) -> Option<Vec<Type>> {
    union_alias_members(class_name)?;
    let ty = recv_ty?;
    // Strip nullable wrappers — `Result<int, string>?` is still the alias.
    let mut ty = ty;
    while let Type::Nullable(inner) = ty {
        ty = inner;
    }
    match ty {
        Type::Generic { args, .. } => Some(args.clone()),
        // Already-narrowed to a concrete member; nothing to specialize.
        Type::Named(_, _) => Some(vec![]),
        _ => None,
    }
}

/// Compile a method call whose receiver is statically known to be a union
/// alias (`Result<T,E>` or `Option<T>`) but whose concrete runtime class
/// (Ok vs Err, Some vs None) can only be decided at runtime via the
/// hidden `__typeid` field. This mirrors the typeid-based dispatch in
/// `compile_dynamic_method_call`, but operates on the class-instance
/// pointer directly and is restricted to the alias's known members so we
/// don't accidentally dispatch on unrelated classes that happen to define
/// the same method name.
fn compile_union_method_call<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    self_arg: inkwell::values::PointerValue<'ctx>,
    union_name: &str,
    method_name: &str,
    type_args: &[Type],
    args: &[crate::parser::ast::Argument],
) -> Result<ExprResult<'ctx>, String> {
    let members = union_alias_members(union_name)
        .ok_or_else(|| format!("Unknown union alias: {}", union_name))?;

    // For each member, attempt to locate (or instantiate) the method.
    // The mangled form is checked first because generic methods are
    // emitted under `mangle_name`, while non-generic ones stay plain.
    let mut candidates: Vec<(u64, String, inkwell::values::FunctionValue<'ctx>)> = Vec::new();
    for member in members.iter() {
        let plain = format!("{}_{}", member, method_name);
        let mut chosen: Option<inkwell::values::FunctionValue<'ctx>> = None;

        // 1. Try the mangled specialization when we have type arguments and
        //    the member is a known generic class. ensure_method_specialization
        //    is a no-op when the method isn't generic, so this is safe.
        //    `resolve_generic_method_target` appends the method's own type
        //    arguments, so the name must match the one the static
        //    method-call path would have produced for the same call.
        if !type_args.is_empty() && ctx.generic_classes.contains_key(*member) {
            if let Some((mangled, all_args)) =
                resolve_generic_method_target(ctx, member, method_name, type_args, args)
            {
                if super::specialize::ensure_method_specialization(
                    ctx,
                    member,
                    method_name,
                    &all_args,
                    &mangled,
                )? {
                    if let Some(f) = ctx.module.get_function(&mangled) {
                        if f.count_basic_blocks() > 0 {
                            chosen = Some(f);
                        }
                    }
                }
            }
        }

        // 2. Fall back to the unmangled name (covers non-generic methods).
        //    Accept both defined functions and mere declarations —
        //    stdlib methods may be forward-declared by an earlier pass
        //    and only have their bodies emitted on first use, in which
        //    case `count_basic_blocks()` is still 0 here.
        if chosen.is_none() {
            if let Some(f) = ctx.module.get_function(&plain) {
                chosen = Some(f);
            }
        }

        if let (Some(f), Some(&type_id)) = (chosen, ctx.type_ids.get(*member)) {
            // Some stdlib methods (notably ones that throw, like
            // `Err.unwrap`) compile-fail during the stdlib pass and end
            // up as bare declarations. Without a body, calls into them
            // fail at link time. Stamp an `unreachable` body so the
            // symbol is well-formed; union dispatch will reach it only
            // when the runtime type matches this variant, which for
            // "should-never-be-called" methods (Err.unwrap on Ok, etc.)
            // preserves the intended trap.
            ensure_function_has_unreachable_body(ctx, f);
            candidates.push((type_id, (*member).to_string(), f));
        }
    }

    if candidates.is_empty() {
        return Err(format!(
            "No union member of {} defines method '{}'",
            union_name, method_name
        ));
    }

    let i64_ty = ctx.context.i64_type();
    let i8_ptr_ty = ctx.context.ptr_type(Default::default());
    let func = ctx.current_function().ok_or("No current function")?;

    // Load the class instance pointer from the receiver slot, then read
    // the hidden __typeid at offset 0. The class instance layout is
    // `[__typeid: i64, ...]` (see decl.rs compile_class), so the cast
    // pointer aliases the start of the instance.
    let obj_ptr = ctx
        .builder()
        .build_load(i8_ptr_ty, self_arg, "union_recv_ptr")
        .unwrap()
        .into_pointer_value();
    let casted_ptr = ctx
        .builder()
        .build_pointer_cast(
            obj_ptr,
            ctx.context.ptr_type(Default::default()),
            "union_typeid_cast",
        )
        .unwrap();
    let typeid = ctx
        .builder()
        .build_load(i64_ty, casted_ptr, "union_typeid")
        .unwrap()
        .into_int_value();

    // Determine each candidate's return type by inspecting the function
    // signature registered in `function_types`. Methods defined on the
    // alias share a single signature, so all candidates should agree
    // (modulo `Dynamic` fallback). When they disagree, fall back to Dynamic
    // to keep the IR well-formed via boxed phi values.
    //
    // Generic class members are monomorphized under `mangle_name`, so when
    // `type_args` is non-empty we must look up the mangled signature
    // instead of the bare `{Class}_{method}` name. Without this fixup,
    // the dispatch sees the unresolved T/E in the unspecialized
    // signature (lowered to `ptr` in LLVM), boxes the result into a
    // Dynamic struct, and then the caller's `int` slot is filled with
    // `inttoptr i64` garbage — e.g. `Err.unwrapOrElse((e) => -1)` ends up
    // with `v2 = (ptr)-1` and SEGVs in `printf("%s", v2)`.
    let mut candidate_ret_types: Vec<Type> = Vec::with_capacity(candidates.len());
    for (_, member, _) in &candidates {
        let plain_key = format!("{}_{}", member, method_name);
        // Must match the name `resolve_generic_method_target` produced when
        // the candidate was selected above, method type arguments included.
        let mangled_key = if !type_args.is_empty()
            && ctx.generic_classes.contains_key(member.as_str())
        {
            resolve_generic_method_target(ctx, member.as_str(), method_name, type_args, args)
                .map(|(mangled, _)| mangled)
                .unwrap_or_else(|| crate::typechecker::generics::mangle_name(&plain_key, type_args))
        } else {
            plain_key.clone()
        };
        let ret_ty = ctx
            .function_types
            .get(&mangled_key)
            .or_else(|| ctx.function_types.get(&plain_key))
            .and_then(|ft| {
                if let Type::Function { return_type, .. } = ft {
                    Some(*return_type.clone())
                } else {
                    None
                }
            })
            .unwrap_or(Type::Dynamic);
        candidate_ret_types.push(ret_ty);
    }

    // Collapse the candidate return types into a single effective
    // type. When candidates disagree or any of them carries an
    // unresolved type parameter (`Named("U", …)` / `TypeVar`), fall
    // back to Dynamic so the phi retains its type tag and the caller
    // can dispatch on the tag instead of mis-interpreting the bits as
    // a concrete Ruyi type (e.g. `printf("%s", v)` SEGVs when `v` is
    // actually an int reboxed through Dynamic).
    let effective_ret_ty = {
        let non_void: Vec<&Type> = candidate_ret_types
            .iter()
            .filter(|t| **t != Type::Void)
            .collect();
        if non_void.is_empty() {
            Type::Void
        } else if non_void.iter().all(|t| **t == *non_void[0]) {
            non_void[0].clone()
        } else if non_void
            .iter()
            .any(|t| matches!(t, Type::Named(_, _) | Type::TypeVar(_)))
        {
            Type::Dynamic
        } else {
            Type::Dynamic
        }
    };

    // Compile the user-supplied method arguments once so we can reuse
    // them at every candidate call site.
    let mut user_arg_values: Vec<BasicValueEnum<'ctx>> = Vec::new();
    for arg in args {
        if let crate::parser::ast::Argument::Expr(e) = arg {
            let r = compile_expr(ctx, e)?;
            user_arg_values.push(r.value);
        } else {
            return Err("Spread arguments not supported in union dispatch".to_string());
        }
    }

    let entry_bb = ctx.builder().get_insert_block().unwrap();
    let merge_bb = ctx.context.append_basic_block(func, "union_dispatch_merge");

    let mut incoming: Vec<(BasicValueEnum<'ctx>, inkwell::basic_block::BasicBlock)> = Vec::new();
    let mut prev_check_bb: Option<inkwell::basic_block::BasicBlock> = None;
    let mut first_check_bb: Option<inkwell::basic_block::BasicBlock> = None;
    let mut last_check_bb: Option<inkwell::basic_block::BasicBlock> = None;

    for (i, (type_id, member, target_fn)) in candidates.iter().enumerate() {
        let fn_name = format!("{}_{}", member, method_name);
        let check_bb = ctx
            .context
            .append_basic_block(func, &format!("union_chk_{}", member));
        let call_bb = ctx
            .context
            .append_basic_block(func, &format!("union_call_{}", member));

        if let Some(prev_else) = prev_check_bb {
            ctx.builder().position_at_end(prev_else);
            ctx.builder().build_unconditional_branch(check_bb).unwrap();
        }

        if i == 0 {
            first_check_bb = Some(check_bb);
        }

        ctx.builder().position_at_end(check_bb);
        let expected_id = i64_ty.const_int(*type_id, false);
        let matches = ctx
            .builder()
            .build_int_compare(IntPredicate::EQ, typeid, expected_id, "union_typeid_match")
            .unwrap();

        let else_bb = if i < candidates.len() - 1 {
            ctx.context
                .append_basic_block(func, &format!("union_next_{}", member))
        } else {
            merge_bb
        };
        ctx.builder()
            .build_conditional_branch(matches, call_bb, else_bb)
            .unwrap();

        // Call block: invoke {member}_{method}(obj_ptr, ...user_args).
        ctx.builder().position_at_end(call_bb);
        let mut call_args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> = Vec::new();
        call_args.push(obj_ptr.into());
        for v in &user_arg_values {
            call_args.push((*v).into());
        }
        let call_site = ctx
            .builder()
            .build_call(*target_fn, &call_args, "union_call")
            .map_err(|e| format!("call {} failed: {}", fn_name, e))?;
        let call_val = call_site.try_as_basic_value().basic();

        // Box the per-candidate return value into a Dynamic struct so the
        // merge phi is uniform across candidates with differing return
        // types. When all candidates agree on a non-Dynamic type, the
        // struct is unboxed back to that scalar at the end.
        let dyn_struct_ty = ruyi_type_to_llvm(ctx.context, &Type::Dynamic).into_struct_type();
        let candidate_ty = &candidate_ret_types[i];
        let boxed: BasicValueEnum<'ctx> = match (candidate_ty, call_val) {
            (Type::Void, _) => {
                let zero_tag = i64_ty.const_int(0, false);
                let null_data = i8_ptr_ty.const_null();
                let s = dyn_struct_ty.const_zero();
                let s = ctx
                    .builder()
                    .build_insert_value(s, zero_tag, 0, "udt")
                    .unwrap()
                    .into_struct_value();
                let s = ctx
                    .builder()
                    .build_insert_value(s, null_data, 1, "udd")
                    .unwrap()
                    .into_struct_value();
                BasicValueEnum::StructValue(s)
            }
            (_, Some(BasicValueEnum::IntValue(v))) => {
                // B1 layout: the value itself in field 0, the type marker
                // pointer in field 1 (see build_box_dynamic). Narrow ints
                // (bool's i1) must be extended to the field's i64 width or
                // the insertvalue is invalid IR that ISel rejects.
                let v = if v.get_type().get_bit_width() < 64 {
                    ctx.builder()
                        .build_int_z_extend(v, i64_ty, "udzext")
                        .unwrap()
                } else {
                    v
                };
                let marker = ctx
                    .builder()
                    .build_int_to_ptr(i64_ty.const_int(1, false), i8_ptr_ty, "ud2p")
                    .unwrap();
                let s = dyn_struct_ty.const_zero();
                let s = ctx
                    .builder()
                    .build_insert_value(s, v, 0, "udt")
                    .unwrap()
                    .into_struct_value();
                let s = ctx
                    .builder()
                    .build_insert_value(s, marker, 1, "udd")
                    .unwrap()
                    .into_struct_value();
                BasicValueEnum::StructValue(s)
            }
            (_, Some(BasicValueEnum::FloatValue(v))) => {
                let bits = ctx
                    .builder()
                    .build_bit_cast(v, i64_ty, "ufbits")
                    .unwrap()
                    .into_int_value();
                let marker = ctx
                    .builder()
                    .build_int_to_ptr(i64_ty.const_int(2, false), i8_ptr_ty, "ud2p")
                    .unwrap();
                let s = dyn_struct_ty.const_zero();
                let s = ctx
                    .builder()
                    .build_insert_value(s, bits, 0, "udf")
                    .unwrap()
                    .into_struct_value();
                let s = ctx
                    .builder()
                    .build_insert_value(s, marker, 1, "udd")
                    .unwrap()
                    .into_struct_value();
                BasicValueEnum::StructValue(s)
            }
            (_, Some(BasicValueEnum::PointerValue(p))) => {
                let tag = i64_ty.const_int(0, false);
                let data = ctx
                    .builder()
                    .build_pointer_cast(p, i8_ptr_ty, "udc")
                    .unwrap();
                let s = dyn_struct_ty.const_zero();
                let s = ctx
                    .builder()
                    .build_insert_value(s, tag, 0, "udt")
                    .unwrap()
                    .into_struct_value();
                let s = ctx
                    .builder()
                    .build_insert_value(s, data, 1, "udd")
                    .unwrap()
                    .into_struct_value();
                BasicValueEnum::StructValue(s)
            }
            _ => BasicValueEnum::StructValue(dyn_struct_ty.const_zero()),
        };
        incoming.push((boxed, call_bb));
        ctx.builder().build_unconditional_branch(merge_bb).unwrap();

        prev_check_bb = Some(else_bb);
        last_check_bb = Some(check_bb);
    }

    // Wire the entry block into the first check.
    if let Some(first_chk) = first_check_bb {
        ctx.builder().position_at_end(entry_bb);
        ctx.builder().build_unconditional_branch(first_chk).unwrap();
    }

    // Default Dynamic zero for the no-match path.
    let default_val = {
        let zero_tag = i64_ty.const_int(0, false);
        let null_data = i8_ptr_ty.const_null();
        let dyn_struct_ty = ruyi_type_to_llvm(ctx.context, &Type::Dynamic).into_struct_type();
        let s = dyn_struct_ty.const_zero();
        let s = ctx
            .builder()
            .build_insert_value(s, zero_tag, 0, "udt")
            .unwrap()
            .into_struct_value();
        let s = ctx
            .builder()
            .build_insert_value(s, null_data, 1, "udd")
            .unwrap()
            .into_struct_value();
        BasicValueEnum::StructValue(s)
    };

    if let Some(last_else) = prev_check_bb {
        if last_else == merge_bb {
            if let Some(last_chk) = last_check_bb {
                incoming.push((default_val, last_chk));
            }
        }
    }

    ctx.builder().position_at_end(merge_bb);

    if effective_ret_ty == Type::Void {
        return Ok(ExprResult::new(
            BasicValueEnum::IntValue(i64_ty.const_int(0, false)),
            Type::Void,
        ));
    }

    let dyn_struct_ty = ruyi_type_to_llvm(ctx.context, &Type::Dynamic).into_struct_type();
    let phi = ctx
        .builder()
        .build_phi(dyn_struct_ty, "union_dispatch_result")
        .unwrap();
    let incoming_refs: Vec<(
        &dyn inkwell::values::BasicValue<'ctx>,
        inkwell::basic_block::BasicBlock,
    )> = incoming
        .iter()
        .map(|(v, bb)| (v as &dyn inkwell::values::BasicValue<'ctx>, *bb))
        .collect();
    phi.add_incoming(&incoming_refs);

    let phi_struct = phi.as_basic_value().into_struct_value();
    let result_data = ctx
        .builder()
        .build_extract_value(phi_struct, 1, "urd")
        .unwrap()
        .into_pointer_value();
    let result_word = ctx
        .builder()
        .build_extract_value(phi_struct, 0, "urw")
        .unwrap()
        .into_int_value();

    // Unbox back to the concrete return type. B1 layout: scalars live
    // in field 0, pointers in field 1 (see build_box_dynamic).
    let result_value: BasicValueEnum<'ctx> = match &effective_ret_ty {
        Type::String => BasicValueEnum::PointerValue(result_data),
        Type::Int | Type::Byte => BasicValueEnum::IntValue(result_word),
        Type::Float => {
            let f = ctx
                .builder()
                .build_bit_cast(result_word, ctx.context.f64_type(), "ud2f")
                .unwrap()
                .into_float_value();
            BasicValueEnum::FloatValue(f)
        }
        Type::Bool => {
            let v1 = ctx
                .builder()
                .build_int_truncate(result_word, ctx.context.bool_type(), "ud2b")
                .unwrap();
            BasicValueEnum::IntValue(v1)
        }
        Type::Dynamic => BasicValueEnum::StructValue(phi_struct),
        Type::Named(_, _) | Type::Nullable(_) | Type::Generic { .. } => {
            // Return the data pointer directly so the caller can either
            // (a) use it as a struct pointer for a follow-up union dispatch
            // (`doubled.unwrap()` reads `__typeid` from offset 0 of this
            // pointer) or (b) treat it as an opaque handle. Boxing to a
            // `{i64, ptr}` Dynamic struct would force the caller to
            // extract the data pointer again, and any union alias
            // chained after this call would SEGV reading the struct's
            // typeid slot.
            BasicValueEnum::PointerValue(result_data)
        }
        _ => BasicValueEnum::IntValue(i64_ty.const_int(0, false)),
    };

    Ok(ExprResult::new(result_value, effective_ret_ty))
}

/// Convert a compiled argument value to the LLVM parameter type expected by
/// an indirect function call. This is required when a generic array method
/// such as `get` returns a uniform i64 representation but the callee expects
/// a concrete type such as `float`.
fn adjust_value_to_param_type<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    value: BasicValueEnum<'ctx>,
    expected: Option<inkwell::types::BasicMetadataTypeEnum<'ctx>>,
) -> BasicValueEnum<'ctx> {
    let expected = match expected {
        Some(ty) => ty,
        None => return value,
    };
    if BasicMetadataTypeEnum::from(value.get_type()) == expected {
        return value;
    }
    match (value, expected) {
        (BasicValueEnum::IntValue(iv), inkwell::types::BasicMetadataTypeEnum::FloatType(ft)) => ctx
            .builder()
            .build_bit_cast(iv, ft, "i64_to_f64")
            .unwrap()
            .into_float_value()
            .into(),
        (BasicValueEnum::FloatValue(fv), inkwell::types::BasicMetadataTypeEnum::IntType(it)) => ctx
            .builder()
            .build_bit_cast(fv, it, "f64_to_i64")
            .unwrap()
            .into_int_value()
            .into(),
        (BasicValueEnum::PointerValue(pv), inkwell::types::BasicMetadataTypeEnum::IntType(it)) => {
            ctx.builder()
                .build_ptr_to_int(pv, it, "ptr_to_int")
                .unwrap()
                .into()
        }
        (BasicValueEnum::IntValue(iv), inkwell::types::BasicMetadataTypeEnum::PointerType(pt)) => {
            ctx.builder()
                .build_int_to_ptr(iv, pt, "int_to_ptr")
                .unwrap()
                .into()
        }
        (
            BasicValueEnum::PointerValue(pv),
            inkwell::types::BasicMetadataTypeEnum::PointerType(pt),
        ) => {
            if pv.get_type() == pt {
                pv.into()
            } else {
                ctx.builder()
                    .build_bit_cast(pv, pt, "ptr_cast")
                    .unwrap()
                    .into_pointer_value()
                    .into()
            }
        }
        (BasicValueEnum::IntValue(iv), inkwell::types::BasicMetadataTypeEnum::IntType(it)) => {
            let src_bits = iv.get_type().get_bit_width();
            let dst_bits = it.get_bit_width();
            if src_bits == dst_bits {
                iv.into()
            } else if src_bits > dst_bits {
                ctx.builder()
                    .build_int_truncate(iv, it, "trunc")
                    .unwrap()
                    .into()
            } else {
                ctx.builder()
                    .build_int_s_extend(iv, it, "sext")
                    .unwrap()
                    .into()
            }
        }
        _ => value,
    }
}

/// Resolve an inherited instance method to the declaring ancestor's
/// mangled name. Subclasses do not re-declare inherited methods, so a
/// call on the subclass would otherwise look up `Sub_method` (absent).
/// Layouts share the parent's field prefix, so the subclass pointer is a
/// valid `self` for the ancestor's method.
fn inherited_method_owner<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    class_name: &str,
    method_name: &str,
) -> Option<String> {
    let mut current = ctx.class_extends.get(class_name)?;
    let mut depth = 0;
    while depth < 32 {
        let candidate = format!("{}_{}", current, method_name);
        if ctx.module.get_function(&candidate).is_some() {
            return Some(candidate);
        }
        current = ctx.class_extends.get(current)?;
        depth += 1;
    }
    None
}

fn compile_call<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    callee: &Expr,
    args: &[crate::parser::ast::Argument],
) -> Result<ExprResult<'ctx>, String> {
    // Concrete receiver type (when known) for on-demand specialization
    // and return-type parameter substitution below.
    let mut call_receiver_ty: Option<Type> = None;
    // `self_arg` is always a memory slot holding the receiver value
    // (a variable alloca, a spilled call/index result, or a field
    // address); the actual value must be loaded before being passed as
    // the implicit first argument.
    let (mut name, self_arg) = match callee {
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
            // Imported aliases (`import { x as y }` desugars to
            // `const y = x;`) must call the target directly: the generic
            // function-value path passes arguments 1:1, so a rest
            // parameter never gets its packaged array and omitted
            // defaults are never synthesized.
            let resolved = ctx
                .function_aliases
                .get(n.as_str())
                .cloned()
                .filter(|target| ctx.module.get_function(target).is_some())
                .unwrap_or_else(|| n.clone());
            (resolved, None)
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
                    Expr::Identifier(class_name) => {
                        // If the class defines a static `new` method, call it
                        // as a free function instead of running the constructor
                        // path. Static `new` and the constructor share the
                        // mangled name `{class}_new`, but the static variant
                        // takes no implicit `self` pointer.
                        if ctx
                            .class_static_methods
                            .get(class_name)
                            .is_some_and(|s| s.contains("new"))
                        {
                            let static_new_name = format!("{}_new", class_name);
                            return compile_call(ctx, &Expr::Identifier(static_new_name), args);
                        }
                        return compile_new(ctx, object.as_ref(), args);
                    }
                    Expr::Super => {
                        return compile_super_new(ctx, args);
                    }
                    _ => return Err("new() must be called on a class name or super".to_string()),
                }
            }

            // Trait object dispatch: when the receiver is a `dyn Trait` variable,
            // load the fat pointer and dispatch through the vtable.
            if let Expr::Identifier(var_name) = object.as_ref() {
                if let Some((ptr, ty)) = ctx.lookup_variable(var_name) {
                    if let Type::Trait(trait_name) = &ty {
                        let trait_obj_ty = ctx.context.struct_type(
                            &[
                                ctx.context.ptr_type(Default::default()).into(),
                                ctx.context.ptr_type(Default::default()).into(),
                            ],
                            false,
                        );
                        let trait_obj_struct = ctx
                            .builder()
                            .build_load(trait_obj_ty, ptr, "trait_obj_load")
                            .unwrap()
                            .into_struct_value();
                        let data_ptr = ctx
                            .builder()
                            .build_extract_value(trait_obj_struct, 0, "trait_data")
                            .unwrap()
                            .into_pointer_value();
                        let vtable_ptr = ctx
                            .builder()
                            .build_extract_value(trait_obj_struct, 1, "trait_vtable")
                            .unwrap()
                            .into_pointer_value();
                        let trait_obj = super::traits::TraitObject {
                            data: data_ptr,
                            vtable: vtable_ptr,
                            trait_name: trait_name.clone(),
                        };
                        let mut call_args: Vec<BasicValueEnum<'ctx>> = Vec::new();
                        for arg in args {
                            if let crate::parser::ast::Argument::Expr(e) = arg {
                                let r = compile_expr(ctx, e)?;
                                call_args.push(r.value);
                            } else {
                                return Err(
                                    "Spread arguments not supported in trait dispatch".to_string()
                                );
                            }
                        }
                        let registry = ctx
                            .vtable_registry
                            .clone()
                            .ok_or("VTable registry not initialized")?;
                        let result = super::traits::build_dynamic_dispatch(
                            ctx,
                            &registry,
                            &trait_obj,
                            &method_name,
                            &call_args,
                        )?;
                        return Ok(ExprResult::new(result, Type::String));
                    }
                }
            }

            let (obj_ptr, class_name, recv_ty) = match object.as_ref() {
                Expr::Identifier(var_name) => {
                    if let Some((ptr, ty)) = ctx.lookup_variable(var_name) {
                        // Resolve user-defined type aliases (e.g. StringArray -> Array<string>)
                        // so method dispatch selects the underlying builtin/class method.
                        let resolved_ty = ctx.resolve_type_aliases(&ty);
                        let class_name = match &resolved_ty {
                            Type::Named(n, _) => n.clone(),
                            Type::Array(_) => "Array".to_string(),
                            Type::Generic { base, .. } => base.clone(),
                            Type::Int => "Int".to_string(),
                            Type::Float => "Float".to_string(),
                            Type::Bool => "Bool".to_string(),
                            Type::Byte => "Byte".to_string(),
                            Type::String => "String".to_string(),
                            Type::Nullable(inner) => match inner.as_ref() {
                                Type::Named(n, _) => n.clone(),
                                Type::Array(_) => "Array".to_string(),
                                Type::Generic { base, .. } => base.clone(),
                                Type::Int => "Int".to_string(),
                                Type::Float => "Float".to_string(),
                                Type::Bool => "Bool".to_string(),
                                Type::Byte => "Byte".to_string(),
                                Type::String => "String".to_string(),
                                Type::Dynamic => {
                                    return compile_dynamic_method_call(
                                        ctx,
                                        object,
                                        &method_name,
                                        args,
                                    );
                                }
                                _ => return Err(format!("Cannot call method on type: {:?}", ty)),
                            },
                            Type::Dynamic => {
                                return compile_dynamic_method_call(
                                    ctx,
                                    object,
                                    &method_name,
                                    args,
                                );
                            }
                            _ => return Err(format!("Cannot call method on type: {:?}", ty)),
                        };
                        (Some(ptr), class_name, Some(ty))
                    } else if ctx.class_struct_types.contains_key(var_name) {
                        (None, var_name.clone(), None)
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
                        Type::Named(n, _) => n.clone(),
                        Type::Array(_) => "Array".to_string(),
                        Type::Generic { base, .. } => base.clone(),
                        Type::Int => "Int".to_string(),
                        Type::Float => "Float".to_string(),
                        Type::Bool => "Bool".to_string(),
                        Type::Byte => "Byte".to_string(),
                        Type::String => "String".to_string(),
                        Type::Dynamic => {
                            return compile_dynamic_method_call(ctx, object, &method_name, args);
                        }
                        Type::Nullable(inner) => match inner.as_ref() {
                            Type::Named(n, _) => n.clone(),
                            Type::Array(_) => "Array".to_string(),
                            Type::Generic { base, .. } => base.clone(),
                            Type::Int => "Int".to_string(),
                            Type::Float => "Float".to_string(),
                            Type::Bool => "Bool".to_string(),
                            Type::Byte => "Byte".to_string(),
                            Type::String => "String".to_string(),
                            Type::Dynamic => {
                                return compile_dynamic_method_call(
                                    ctx,
                                    object,
                                    &method_name,
                                    args,
                                );
                            }
                            _ => return Err(format!("Cannot call method on type: {:?}", ty)),
                        },
                        _ => return Err(format!("Cannot call method on type: {:?}", ty)),
                    };
                    (Some(*ptr), class_name, Some(ty.clone()))
                }
                // Computed member access as a method receiver (e.g.
                // `parts[i].toString()`): compile the index expression and
                // use its result as the receiver instead of treating it as a
                // simple field access.
                Expr::Member {
                    property: inner_prop,
                    ..
                } if !matches!(inner_prop, crate::parser::ast::MemberProperty::Ident(_)) => {
                    let result = compile_expr(ctx, object.as_ref())?;
                    let class_name = match &result.ty {
                        Type::Named(n, _) => n.clone(),
                        Type::String => "String".to_string(),
                        Type::Array(_) => "Array".to_string(),
                        Type::Generic { base, .. } => base.clone(),
                        Type::Int => "Int".to_string(),
                        Type::Float => "Float".to_string(),
                        Type::Bool => "Bool".to_string(),
                        Type::Byte => "Byte".to_string(),
                        Type::Nullable(inner) => match inner.as_ref() {
                            Type::Named(n, _) => n.clone(),
                            Type::String => "String".to_string(),
                            Type::Array(_) => "Array".to_string(),
                            Type::Generic { base, .. } => base.clone(),
                            Type::Dynamic => {
                                return compile_dynamic_method_call(
                                    ctx,
                                    object,
                                    &method_name,
                                    args,
                                );
                            }
                            _ => {
                                return Err(format!(
                                    "Cannot call method on indexed type: {:?}",
                                    result.ty
                                ))
                            }
                        },
                        Type::Dynamic => {
                            return compile_dynamic_method_call(ctx, object, &method_name, args);
                        }
                        _ => {
                            return Err(format!(
                                "Cannot call method on indexed type: {:?}",
                                result.ty
                            ))
                        }
                    };
                    // Spill the receiver value into a slot so the method call
                    // can take a pointer to it (mirrors the call-result
                    // receiver handling below).
                    let slot = match result.value {
                        BasicValueEnum::PointerValue(p) => {
                            let slot = ctx
                                .builder()
                                .build_alloca(p.get_type(), "idx_recv_slot")
                                .unwrap();
                            ctx.builder().build_store(slot, p).unwrap();
                            slot
                        }
                        BasicValueEnum::IntValue(iv) => {
                            let slot = ctx
                                .builder()
                                .build_alloca(iv.get_type(), "idx_recv_slot")
                                .unwrap();
                            ctx.builder().build_store(slot, iv).unwrap();
                            slot
                        }
                        BasicValueEnum::FloatValue(fv) => {
                            let slot = ctx
                                .builder()
                                .build_alloca(fv.get_type(), "idx_recv_slot")
                                .unwrap();
                            ctx.builder().build_store(slot, fv).unwrap();
                            slot
                        }
                        _ => {
                            return Err(format!(
                                "Cannot use indexed value as method receiver: {:?}",
                                result.ty
                            ))
                        }
                    };
                    (Some(slot), class_name, Some(result.ty.clone()))
                }
                Expr::Member {
                    object: inner_obj,
                    property: inner_prop,
                    ..
                } => {
                    let inner_field = match inner_prop {
                        crate::parser::ast::MemberProperty::Ident(n) => n.clone(),
                        // Unreachable: the guarded arm above handles computed
                        // (non-Ident) member receivers.
                        _ => return Err("Only simple field access supported".to_string()),
                    };
                    // Get field pointer and type from inner member access
                    let (inner_var_ptr, field_owner_class, field_ty, field_index) = match inner_obj
                        .as_ref()
                    {
                        Expr::SelfExpr => {
                            let (ptr, ty) = ctx
                                .variables
                                .get("self")
                                .ok_or_else(|| "self not in scope".to_string())?;
                            let class_name = match ty {
                                Type::Named(n, _) => n.clone(),
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
                            // The raw declared field type (e.g. `T` for
                            // `Ok<T,E>.value`) must be substituted with the
                            // receiver's actual type arguments before
                            // chaining further method calls on the field
                            // result. Without this, `ok.value.toString()`
                            // would dispatch on the bare type parameter
                            // `T` (looking up `T_toString`) instead of the
                            // concrete instantiation (`Int_toString` /
                            // `ruyi_int_to_string`).
                            let raw_field_ty = fields[idx].1.clone();
                            let fty =
                                substitute_class_field_type(ctx, &class_name, ty, &raw_field_ty);
                            (*ptr, class_name, fty, idx)
                        }
                        Expr::Identifier(name) => {
                            let (ptr, ty) = ctx
                                .variables
                                .get(name.as_str())
                                .ok_or_else(|| format!("Undefined variable: {}", name))?;
                            // Anonymous objects store fields in flat 8-byte
                            // slots with no class struct type (see
                            // `compile_simple_member_access`), so the GEP
                            // below needs the tagged path.
                            if let Type::Object(obj_fields) = ctx.resolve_type_aliases(ty) {
                                let idx = obj_fields
                                    .iter()
                                    .position(|f| f.name == inner_field)
                                    .ok_or_else(|| {
                                        format!("Unknown field: {} in object", inner_field)
                                    })?;
                                let fty = obj_fields[idx].ty.clone();
                                (*ptr, "__object__".to_string(), fty, idx)
                            } else {
                                let class_name = match ty {
                                    Type::Named(n, _) => n.clone(),
                                    Type::Array(_) => "Array".to_string(),
                                    Type::Generic { base, .. } => base.clone(),
                                    _ => {
                                        return Err(format!(
                                            "Cannot access field on type: {:?}",
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
                                let raw_field_ty = fields[idx].1.clone();
                                let fty = substitute_class_field_type(
                                    ctx,
                                    &class_name,
                                    ty,
                                    &raw_field_ty,
                                );
                                (*ptr, class_name, fty, idx)
                            }
                        }
                        _ => return Err("Nested member access not yet supported".to_string()),
                    };
                    let obj_ptr = ctx
                        .builder()
                        .build_load(
                            ctx.context.ptr_type(Default::default()),
                            inner_var_ptr,
                            "obj",
                        )
                        .unwrap()
                        .into_pointer_value();
                    // Field addressing must follow the class struct layout:
                    // dyn/function fields occupy 16 bytes, so a flat
                    // `index * 8` byte offset aliases later fields (e.g.
                    // RWLock._readers landed on _guard and push/pop silently
                    // corrupted the lock). The struct GEP carries the real
                    // offsets. Anonymous objects have no class struct type;
                    // their fields are flat 8-byte slots.
                    let field_ptr = if field_owner_class == "__object__" {
                        let offset = ctx
                            .context
                            .i32_type()
                            .const_int((field_index * 8) as u64, false);
                        let byte_ptr = unsafe {
                            ctx.builder()
                                .build_gep(
                                    ctx.context.i8_type(),
                                    obj_ptr,
                                    &[offset],
                                    &format!("{}_ptr", inner_field),
                                )
                                .unwrap()
                        };
                        ctx.builder()
                            .build_pointer_cast(
                                byte_ptr,
                                ctx.context.ptr_type(Default::default()),
                                "obj_field_ptr",
                            )
                            .unwrap()
                    } else {
                        let struct_type = ctx
                            .class_struct_types
                            .get(&field_owner_class)
                            .ok_or_else(|| {
                                format!("No struct type for class: {}", field_owner_class)
                            })?;
                        let struct_ptr = ctx
                            .builder()
                            .build_pointer_cast(
                                obj_ptr,
                                ctx.context.ptr_type(Default::default()),
                                &format!("{}_cast", field_owner_class),
                            )
                            .unwrap();
                        unsafe {
                            ctx.builder()
                                .build_gep(
                                    *struct_type,
                                    struct_ptr,
                                    &[
                                        ctx.context.i32_type().const_int(0, false),
                                        ctx.context.i32_type().const_int(field_index as u64, false),
                                    ],
                                    &format!("{}_ptr", inner_field),
                                )
                                .unwrap()
                        }
                    };
                    let class_name = match field_ty {
                        Type::Named(ref n, _) => n.clone(),
                        Type::Array(_) => "Array".to_string(),
                        Type::Generic { ref base, .. } => base.clone(),
                        Type::String => "String".to_string(),
                        Type::Int => "Int".to_string(),
                        Type::Float => "Float".to_string(),
                        Type::Bool => "Bool".to_string(),
                        Type::Byte => "Byte".to_string(),
                        Type::Dynamic => {
                            return compile_dynamic_method_call(ctx, object, &method_name, args);
                        }
                        _ => {
                            return Err(format!("Cannot call method on field type: {:?}", field_ty))
                        }
                    };
                    (Some(field_ptr), class_name, Some(field_ty.clone()))
                }
                Expr::StringLiteral(_) => {
                    let str_result = compile_expr(ctx, object.as_ref())?;
                    let str_ptr = str_result.value.into_pointer_value();
                    let slot = ctx
                        .builder()
                        .build_alloca(str_ptr.get_type(), "str_slot")
                        .unwrap();
                    ctx.builder().build_store(slot, str_ptr).unwrap();
                    (Some(slot), "String".to_string(), Some(Type::String))
                }
                Expr::Call {
                    callee: inner_callee,
                    args: inner_args,
                } => {
                    let result = compile_call(ctx, inner_callee, inner_args)?;
                    // Resolve user-defined type aliases so chained method calls
                    // (e.g. `rows.get(i).length()`) dispatch on the underlying type.
                    let resolved_ty = ctx.resolve_type_aliases(&result.ty);
                    let class_name = match &resolved_ty {
                        Type::Named(n, _) => n.clone(),
                        Type::String => "String".to_string(),
                        Type::Array(_) => "Array".to_string(),
                        Type::Generic { base, .. } => base.clone(),
                        Type::Int => "Int".to_string(),
                        Type::Float => "Float".to_string(),
                        Type::Bool => "Bool".to_string(),
                        Type::Byte => "Byte".to_string(),
                        Type::Nullable(inner) => match inner.as_ref() {
                            Type::Named(n, _) => n.clone(),
                            Type::String => "String".to_string(),
                            Type::Array(_) => "Array".to_string(),
                            Type::Generic { base, .. } => base.clone(),
                            Type::Int => "Int".to_string(),
                            Type::Float => "Float".to_string(),
                            Type::Bool => "Bool".to_string(),
                            Type::Byte => "Byte".to_string(),
                            Type::Dynamic => {
                                return compile_dynamic_method_call(
                                    ctx,
                                    object,
                                    &method_name,
                                    args,
                                );
                            }
                            _ => {
                                return Err(format!(
                                    "Cannot call method on call result type: {:?}",
                                    result.ty
                                ))
                            }
                        },
                        Type::Dynamic => {
                            return compile_dynamic_method_call(ctx, object, &method_name, args);
                        }
                        _ => {
                            return Err(format!(
                                "Cannot call method on call result type: {:?}",
                                result.ty
                            ))
                        }
                    };
                    // Method calls expect a pointer to the object slot, not
                    // the value itself; spill the call result into a local
                    // slot. Pointer results (strings, objects) and scalar
                    // results (ints/floats/bools from e.g. `nowMs().toString()`)
                    // are both supported.
                    let slot = match result.value {
                        BasicValueEnum::PointerValue(p) => {
                            let slot = ctx
                                .builder()
                                .build_alloca(p.get_type(), "call_recv_slot")
                                .unwrap();
                            ctx.builder().build_store(slot, p).unwrap();
                            slot
                        }
                        BasicValueEnum::IntValue(iv) => {
                            let slot = ctx
                                .builder()
                                .build_alloca(iv.get_type(), "call_recv_slot")
                                .unwrap();
                            ctx.builder().build_store(slot, iv).unwrap();
                            slot
                        }
                        BasicValueEnum::FloatValue(fv) => {
                            let slot = ctx
                                .builder()
                                .build_alloca(fv.get_type(), "call_recv_slot")
                                .unwrap();
                            ctx.builder().build_store(slot, fv).unwrap();
                            slot
                        }
                        _ => {
                            return Err(format!(
                                "Call result cannot be a method receiver: {:?}",
                                result.ty
                            ))
                        }
                    };
                    (Some(slot), class_name, Some(result.ty.clone()))
                }
                _ => return Err("Method calls only supported on identifiers".to_string()),
            };
            call_receiver_ty = recv_ty;
            // Function-pointer field call: `self.cb(args)` / `obj.cb(args)`
            // where `cb` is a class field whose type is a function. Load the
            // stored function pointer from the field slot and call it
            // indirectly (no implicit self argument), instead of forming the
            // method name `{Class}_cb` which does not exist.
            if let Some(recv_slot) = obj_ptr {
                let fn_field_ty = ctx
                    .class_fields
                    .get(&class_name)
                    .and_then(|fields| {
                        fields
                            .iter()
                            .find(|(n, _)| n == &method_name)
                            .map(|(_, ty)| ty.clone())
                    })
                    .filter(|ty| matches!(ty, Type::Function { .. }));
                if let Some(Type::Function {
                    params: fn_params,
                    return_type: fn_ret,
                }) = fn_field_ty
                {
                    let obj = ctx
                        .builder()
                        .build_load(
                            ctx.context.ptr_type(Default::default()),
                            recv_slot,
                            "fn_recv",
                        )
                        .unwrap()
                        .into_pointer_value();
                    let receiver_ty = call_receiver_ty.clone().unwrap_or(Type::Dynamic);
                    let (field_ptr, field_ty) =
                        class_field_access(ctx, obj, &class_name, &method_name, &receiver_ty)?;
                    let fn_value = emit_field_load(ctx, field_ptr, &field_ty, "fn_field");
                    let mut arg_values: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> =
                        Vec::new();
                    emit_spread_args(ctx, args, &mut arg_values)?;
                    return emit_fn_value_call(
                        ctx,
                        fn_value,
                        &fn_params,
                        &fn_ret,
                        arg_values,
                        "fn_field_call",
                    );
                }
            }
            // `Thread.spawn(closure)` / `Fiber.spawn(closure)` are
            // language-level builtins: the stdlib stubs cannot pass a Ruyi
            // closure to `__thread_spawn` / `__fiber_spawn` (ABI mismatch:
            // closures take a `{i64, ptr}` Dynamic while the runtime
            // expects `extern "C" fn(usize)`), so emit the real spawn here.
            if (class_name == "Thread" || class_name == "Fiber") && method_name == "spawn" {
                let kind = if class_name == "Fiber" {
                    SpawnKind::Fiber
                } else {
                    SpawnKind::Thread
                };
                return compile_spawn_builtin(ctx, args, kind);
            }
            let func_name = format!("{}_{}", class_name, method_name);
            // On-demand specialization: when the receiver is a generic
            // class instance with fully concrete type arguments,
            // instantiate a monomorphized copy of the method so that
            // trait-method calls on type-parameter receivers resolve to
            // the concrete impl (e.g. add_Add_for_int).
            let mut specialized: Option<String> = None;
            let call_args = args;
            if let Some(recv) = call_receiver_ty.as_ref() {
                let mut recv = recv;
                while let Type::Nullable(inner) = recv {
                    recv = inner;
                }
                if let Type::Generic { base, args } = recv {
                    if !args.is_empty()
                        && args.iter().all(super::specialize::is_concrete)
                        && ctx.generic_classes.contains_key(base)
                    {
                        if let Some((sname, all_args)) =
                            resolve_generic_method_target(ctx, base, &method_name, args, call_args)
                        {
                            if super::specialize::ensure_method_specialization(
                                ctx,
                                base,
                                &method_name,
                                &all_args,
                                &sname,
                            )? {
                                specialized = Some(sname);
                            }
                        }
                    }
                }
            }
            let func_name = if let Some(sname) = specialized {
                sname
            } else if ctx.module.get_function(&func_name).is_some() {
                func_name
            } else if let Some(owner) = inherited_method_owner(ctx, &class_name, &method_name) {
                owner
            } else if let Some(recv_slot) = obj_ptr {
                // Result/Option are type aliases for union types
                // (`Ok<T,E> | Err<T,E>` and `Some<T> | None`). The alias
                // doesn't define methods directly; the actual implementations
                // live on the union members. When the mangled name lookup
                // misses, fall through to a typeid-based runtime dispatch
                // over the union members.
                if let Some(member_args) =
                    union_alias_member_type_args(call_receiver_ty.as_ref(), class_name.as_str())
                {
                    return compile_union_method_call(
                        ctx,
                        recv_slot,
                        class_name.as_str(),
                        method_name.as_str(),
                        &member_args,
                        args,
                    );
                }
                // Handle primitive type methods: Int.toString -> ruyi_int_to_string
                if class_name == "Int" && method_name == "toString" {
                    "ruyi_int_to_string".to_string()
                } else if class_name == "Float" && method_name == "toString" {
                    "ruyi_float_to_string".to_string()
                } else if class_name == "Bool" && method_name == "toString" {
                    "ruyi_bool_to_string".to_string()
                } else if class_name == "String" {
                    let snake_name = method_name
                        .chars()
                        .enumerate()
                        .flat_map(|(i, c)| {
                            if i > 0 && c.is_ascii_uppercase() {
                                vec!['_', c.to_ascii_lowercase()]
                            } else {
                                vec![c.to_ascii_lowercase()]
                            }
                        })
                        .collect::<String>();
                    format!("__string_{}", snake_name)
                } else if class_name == "Array" {
                    // Only known builtins use the __builtin_array_ prefix;
                    // other methods (slice, sort, map, etc.) are user-defined.
                    let known_builtins = ["create", "get", "set", "push", "pop", "length"];
                    if known_builtins.contains(&method_name.as_str()) {
                        format!("__builtin_array_{}", method_name)
                    } else {
                        // Look for trait impl function: {method}_for_Array or similar
                        let prefix = format!("{}_", method_name);
                        let mut found = None;
                        for func in ctx.module.get_functions() {
                            let fname = func.get_name().to_string_lossy().to_string();
                            if fname.starts_with(&prefix) && fname.contains("Array") {
                                found = Some(fname);
                                break;
                            }
                        }
                        found.unwrap_or_else(|| format!("__builtin_array_{}", method_name))
                    }
                } else {
                    // Trait impl pattern: {method}_{trait}_for_{type}
                    // Also try: {method}_for_{type} for simpler cases.
                    // Class-style receiver names (Int) and impl target
                    // names (int) differ in case, so match both.
                    let suffix = format!("_for_{}", class_name);
                    let suffix_lower = format!("_for_{}", class_name.to_lowercase());
                    let prefix = format!("{}_", method_name);
                    let mut found = None;
                    for func in ctx.module.get_functions() {
                        let fname = func.get_name().to_string_lossy().to_string();
                        if fname.starts_with(&prefix)
                            && (fname.ends_with(&suffix) || fname.ends_with(&suffix_lower))
                        {
                            found = Some(fname);
                            break;
                        }
                    }
                    found.unwrap_or(func_name)
                }
            } else {
                // No receiver slot — fall back to the original dispatch.
                if class_name == "Int" && method_name == "toString" {
                    "ruyi_int_to_string".to_string()
                } else if class_name == "Float" && method_name == "toString" {
                    "ruyi_float_to_string".to_string()
                } else if class_name == "Bool" && method_name == "toString" {
                    "ruyi_bool_to_string".to_string()
                } else if class_name == "String" {
                    let snake_name = method_name
                        .chars()
                        .enumerate()
                        .flat_map(|(i, c)| {
                            if i > 0 && c.is_ascii_uppercase() {
                                vec!['_', c.to_ascii_lowercase()]
                            } else {
                                vec![c.to_ascii_lowercase()]
                            }
                        })
                        .collect::<String>();
                    format!("__string_{}", snake_name)
                } else if class_name == "Array" {
                    let known_builtins = ["create", "get", "set", "push", "pop", "length"];
                    if known_builtins.contains(&method_name.as_str()) {
                        format!("__builtin_array_{}", method_name)
                    } else {
                        let prefix = format!("{}_", method_name);
                        let mut found = None;
                        for func in ctx.module.get_functions() {
                            let fname = func.get_name().to_string_lossy().to_string();
                            if fname.starts_with(&prefix) && fname.contains("Array") {
                                found = Some(fname);
                                break;
                            }
                        }
                        found.unwrap_or_else(|| format!("__builtin_array_{}", method_name))
                    }
                } else {
                    let suffix = format!("_for_{}", class_name);
                    let suffix_lower = format!("_for_{}", class_name.to_lowercase());
                    let prefix = format!("{}_", method_name);
                    let mut found = None;
                    for func in ctx.module.get_functions() {
                        let fname = func.get_name().to_string_lossy().to_string();
                        if fname.starts_with(&prefix)
                            && (fname.ends_with(&suffix) || fname.ends_with(&suffix_lower))
                        {
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
                        ctx.context,
                        ctx.builder(),
                        ctx.module,
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
                    // Function values are `{ fnptr, env }` structs; the
                    // async-runtime spawn entry only takes the fnptr (the
                    // builtin spawn has no capture-argument channel).
                    let entry_ptr = match result.value {
                        BasicValueEnum::StructValue(s) => ctx
                            .builder()
                            .build_extract_value(s, 0, "spawn_entry")
                            .unwrap()
                            .into_pointer_value(),
                        other => other.into_pointer_value(),
                    };
                    let task_handle =
                        super::builtins::build_ruyi_spawn_fn(ctx.builder(), ctx.module, entry_ptr);
                    // A synchronous main returns without draining the
                    // scheduler, which would drop the task before a worker
                    // ever picks it up.
                    ctx.used_spawn_builtin = true;
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
        if let Some((ptr, ty)) = ctx.lookup_variable(&name) {
            if let Type::Function {
                params: fn_params,
                return_type: fn_ret,
            } = &ty
            {
                // Function values are `{ fnptr, env }` structs; the env
                // word selects the plain or env-parameter ABI at runtime.
                let fn_struct_val = ctx
                    .builder()
                    .build_load(ruyi_type_to_llvm(ctx.context, &ty), ptr, "func_val")
                    .unwrap();

                let fn_type = function_type_from_ruyi(ctx.context, fn_params, fn_ret);
                let llvm_param_types = fn_type.get_param_types();
                let mut arg_values: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> = Vec::new();
                let mut use_spread_fallback = false;
                for (idx, arg) in args.iter().enumerate() {
                    match arg {
                        crate::parser::ast::Argument::Expr(e) => {
                            let expected_ruyi_ty = fn_params
                                .get(idx)
                                .cloned()
                                .filter(|t| !t.contains_dynamic() && !t.contains_type_param());
                            let prev_expected = ctx.expected_expr_type().cloned();
                            ctx.set_expected_expr_type(expected_ruyi_ty);
                            let result = compile_expr(ctx, e)?;
                            ctx.set_expected_expr_type(prev_expected);

                            // Box concrete values only for genuine `dyn` parameters.
                            // Type parameters lower to `ptr` in LLVM, so the callee
                            // signature already expects a raw pointer — boxing to
                            // `{i64, ptr}` here would mismatch the function
                            // signature and trip the LLVM verifier.
                            let param_ty = fn_params.get(idx);
                            let needs_dyn_box =
                                param_ty.is_some_and(|p| matches!(p, Type::Dynamic));
                            if needs_dyn_box && result.ty != Type::Dynamic {
                                let dyn_val = build_box_dynamic(ctx, result.value, &result.ty);
                                arg_values.push(dyn_val.into());
                                continue;
                            }

                            let adjusted = adjust_value_to_param_type(
                                ctx,
                                result.value,
                                llvm_param_types.get(idx).copied(),
                            );
                            arg_values.push(adjusted.into());
                        }
                        crate::parser::ast::Argument::Spread(_) => {
                            use_spread_fallback = true;
                            break;
                        }
                    }
                }
                if use_spread_fallback {
                    arg_values.clear();
                    emit_spread_args(ctx, args, &mut arg_values)?;
                }

                return emit_fn_value_call(
                    ctx,
                    fn_struct_val,
                    fn_params,
                    fn_ret,
                    arg_values,
                    "call",
                );
            }
        }
    }

    // Route generic calls to their concrete monomorphized specialization
    // when we can infer the type arguments from the call-site arguments.
    // This lets arrow-function arguments be compiled with the concrete
    // parameter/return types expected by the specialized callee.
    if self_arg.is_none() {
        if let Some((mangled_name, _params)) = resolve_generic_specialization(ctx, &name, args) {
            if ctx.module.get_function(&mangled_name).is_some() {
                name = mangled_name;
            }
        }
    }

    let func = ctx
        .module
        .get_function(&name)
        .ok_or_else(|| format!("Function not found: {}", name))?;

    let mut arg_values: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> = Vec::new();
    let mut arg_types: Vec<Type> = Vec::new(); // track types for rest args boxing
                                               // Track positions in arg_values that already hold a pre-built rest Array
                                               // (a spread that was packaged inline). The rest_info drain below must
                                               // skip these so the existing Array isn't repackaged as a 1-element Array.
    let mut prebuilt_rest_positions: std::collections::HashSet<usize> =
        std::collections::HashSet::new();
    if let Some(self_ptr) = self_arg {
        // Under LLVM opaque pointers every pointer shares the same type,
        // so the old `is_i8_ptr` heuristic can no longer distinguish a
        // receiver slot from a receiver value. It therefore mis-passed
        // the slot address itself to runtime builtins such as
        // __builtin_array_* and __string_*, which then read the slot
        // address as array/string data (garbage `length()`, memory
        // corruption in `push()`). Always load the receiver value from
        // its slot instead; for scalar receivers the load preserves the
        // value bit pattern.
        let loaded = ctx
            .builder()
            .build_load(ctx.context.ptr_type(Default::default()), self_ptr, "obj")
            .unwrap();
        arg_values.push(loaded.into());
    }
    // Get the function's LLVM parameter types for type-aware conversion
    let func_param_types: Vec<_> = func.get_type().get_param_types();

    // Check rest params early so arg collection can skip type adjustment
    let rest_info = ctx.rest_params.get(&name).cloned();

    let self_arg_offset = if self_arg.is_some() { 1 } else { 0 };

    // Array element box protocol (write side): `__builtin_array_push` /
    // `__builtin_array_set` store one 8-byte word per element. Dynamic and
    // Function elements are 16-byte structs, so they are heap-boxed and the
    // word carries the box pointer; reads reverse this in the get/pop/index
    // recovery paths below. Both call forms reach these builtins: the
    // method fast path (`arr.push(v)`, receiver pre-pushed via `self_arg`)
    // and the plain stdlib call (`__builtin_array_push(self, v)`).
    let boxed_elem_receiver = name == "__builtin_array_push" || name == "__builtin_array_set";
    let mut array_elem_ty: Option<Type> = None;
    if boxed_elem_receiver {
        if let Some(recv) = call_receiver_ty.as_ref() {
            let mut t = recv;
            while let Type::Nullable(inner) = t {
                t = inner;
            }
            if let Type::Array(inner) = t {
                array_elem_ty = Some(ctx.resolve_type_aliases(inner));
            }
        }
    }

    for (arg_idx, arg) in args.iter().enumerate() {
        match arg {
            crate::parser::ast::Argument::Expr(e) => {
                let func_param_idx = self_arg_offset + arg_idx;
                let expected_ruyi_ty = ctx
                    .function_types
                    .get(&name)
                    .and_then(|ft| {
                        if let Type::Function { params, .. } = ft {
                            params.get(func_param_idx).cloned()
                        } else {
                            None
                        }
                    })
                    .filter(|t| !t.contains_dynamic() && !t.contains_type_param());
                let expected_ruyi_ty_for_unbox = expected_ruyi_ty.clone();
                let prev_expected = ctx.expected_expr_type().cloned();
                ctx.set_expected_expr_type(expected_ruyi_ty);
                let result = compile_expr(ctx, e)?;
                ctx.set_expected_expr_type(prev_expected);
                let func_param_idx = self_arg_offset + arg_idx;
                let expected_ty = func_param_types.get(func_param_idx);
                let i8_ptr = ctx.context.ptr_type(Default::default());

                // Check if this arg is part of rest params — skip type adjustment
                let is_rest_arg = rest_info
                    .as_ref()
                    .map(|(ri, _)| func_param_idx >= *ri)
                    .unwrap_or(false);

                // Trait object coercion: if the function parameter is `dyn Trait`,
                // wrap the argument into a { data, vtable } fat pointer.
                let trait_param_name = ctx.function_types.get(&name).and_then(|ft| {
                    if let Type::Function {
                        params: fn_params, ..
                    } = ft
                    {
                        fn_params.get(func_param_idx).and_then(|p| {
                            if let Type::Trait(n) = p {
                                Some(n.clone())
                            } else {
                                None
                            }
                        })
                    } else {
                        None
                    }
                });
                if let Some(trait_name) = trait_param_name {
                    let trait_obj_val = super::traits::build_trait_object_value(
                        ctx,
                        result.value,
                        &result.ty,
                        &trait_name,
                    )?;
                    arg_values.push(trait_obj_val.into());
                    continue;
                }

                if boxed_elem_receiver {
                    // Plain-call form: arg 0 is the array; record its
                    // (resolved) element type for the value arg below.
                    if self_arg.is_none() && arg_idx == 0 {
                        let mut t = &result.ty;
                        while let Type::Nullable(inner) = t {
                            t = inner;
                        }
                        if let Type::Array(inner) = t {
                            array_elem_ty = Some(ctx.resolve_type_aliases(inner));
                        }
                    }
                    // Stored-value position (receiver already consumed for
                    // the method fast path): push → 1 value, set → index+value.
                    let value_pos = match (self_arg.is_some(), name == "__builtin_array_push") {
                        (true, true) => 0,
                        (true, false) => 1,
                        (false, true) => 1,
                        (false, false) => 2,
                    };
                    if arg_idx == value_pos
                        && matches!(
                            array_elem_ty.as_ref(),
                            Some(Type::Dynamic) | Some(Type::Function { .. })
                        )
                    {
                        let word: inkwell::values::BasicMetadataValueEnum<'ctx> = match result.value
                        {
                            BasicValueEnum::StructValue(_) => {
                                build_box_struct_to_word(ctx, result.value).into()
                            }
                            other => {
                                let boxed = build_box_dynamic(ctx, other, &result.ty);
                                build_box_struct_to_word(ctx, boxed).into()
                            }
                        };
                        arg_values.push(word);
                        arg_types.push(Type::Dynamic);
                        continue;
                    }
                }

                // Dynamic parameter boxing: if the function expects `dyn` but
                // the argument has a concrete type, box it into {i64, i8*}.
                let is_dyn_param = ctx
                    .function_types
                    .get(&name)
                    .and_then(|ft| {
                        if let Type::Function {
                            params: fn_params, ..
                        } = ft
                        {
                            fn_params
                                .get(func_param_idx)
                                .map(|p| matches!(p, Type::Dynamic))
                        } else {
                            None
                        }
                    })
                    .unwrap_or(false);
                if is_dyn_param && result.ty != Type::Dynamic {
                    // Only box if the LLVM parameter type is a struct (Dynamic).
                    // Skip boxing for FFI functions where dyn maps to i8*/i64.
                    if let Some(expected) = expected_ty {
                        let dyn_llvm_ty = ruyi_type_to_llvm(ctx.context, &Type::Dynamic);
                        if *expected == dyn_llvm_ty.into() {
                            let dyn_val = build_box_dynamic(ctx, result.value, &result.ty);
                            arg_values.push(dyn_val.into());
                            arg_types.push(Type::Dynamic);
                            continue;
                        }
                    }
                }

                // Rest args: keep original value and type for Dynamic boxing
                if is_rest_arg {
                    arg_values.push(result.value.into());
                    arg_types.push(result.ty);
                    continue;
                }

                // Dynamic argument unboxing: a `dyn`-typed value flowing
                // into a concrete parameter must be unpacked from its
                // `{value, marker}` struct. Passing the raw struct would
                // consume two parameter registers and shift every
                // following argument.
                if result.ty == Type::Dynamic && !is_dyn_param {
                    if let (Some(expected), BasicValueEnum::StructValue(sv)) =
                        (expected_ty, result.value)
                    {
                        let dyn_llvm_ty = ruyi_type_to_llvm(ctx.context, &Type::Dynamic);
                        if *expected != dyn_llvm_ty.into() {
                            let unboxed: BasicValueEnum = if expected.is_int_type() {
                                let raw = ctx
                                    .builder()
                                    .build_extract_value(sv, 0, "dyn_unbox")
                                    .unwrap()
                                    .into_int_value();
                                let tgt = expected.into_int_type();
                                if raw.get_type() == tgt {
                                    raw.into()
                                } else {
                                    ctx.builder()
                                        .build_int_cast(raw, tgt, "dyn_int")
                                        .unwrap()
                                        .into()
                                }
                            } else if expected.is_float_type() {
                                let raw = ctx
                                    .builder()
                                    .build_extract_value(sv, 0, "dyn_unbox")
                                    .unwrap()
                                    .into_int_value();
                                ctx.builder()
                                    .build_bit_cast(raw, expected.into_float_type(), "dyn_float")
                                    .unwrap()
                            } else if expected.is_pointer_type() {
                                let expected_ptr = expected.into_pointer_type();
                                let raw = ctx
                                    .builder()
                                    .build_extract_value(sv, 1, "dyn_ptr")
                                    .unwrap()
                                    .into_pointer_value();
                                if raw.get_type() == expected_ptr {
                                    raw.into()
                                } else {
                                    ctx.builder()
                                        .build_pointer_cast(raw, expected_ptr, "dyn_ptrcast")
                                        .unwrap()
                                        .into()
                                }
                            } else {
                                result.value
                            };
                            if !matches!(unboxed, BasicValueEnum::StructValue(_)) {
                                arg_values.push(unboxed.into());
                                arg_types.push(expected_ruyi_ty_for_unbox.unwrap_or(Type::Dynamic));
                                continue;
                            }
                        }
                    }
                }

                // Convert argument to match the function's expected parameter type
                let i64_ty = ctx.context.i64_type();
                let adjusted_value = if expected_ty == Some(&i8_ptr.into()) {
                    // Function expects i8* - convert value to i8*
                    match result.value {
                        BasicValueEnum::PointerValue(pv) => {
                            if pv.get_type() != i8_ptr {
                                ctx.builder()
                                    .build_bit_cast(pv, i8_ptr, "ptr_cast")
                                    .unwrap()
                                    .into()
                            } else {
                                pv.into()
                            }
                        }
                        BasicValueEnum::IntValue(iv) => ctx
                            .builder()
                            .build_int_to_ptr(iv, i8_ptr, "int_to_ptr")
                            .unwrap()
                            .into(),
                        BasicValueEnum::FloatValue(fv) => {
                            let i64_val = ctx
                                .builder()
                                .build_bit_cast(fv, ctx.context.i64_type(), "f_to_i")
                                .unwrap();
                            ctx.builder()
                                .build_int_to_ptr(i64_val.into_int_value(), i8_ptr, "int_to_ptr")
                                .unwrap()
                                .into()
                        }
                        BasicValueEnum::StructValue(sv) => {
                            // Dynamic struct → i8*: extract data pointer
                            ctx.builder()
                                .build_extract_value(sv, 1, "dyn_to_ptr")
                                .unwrap()
                                .into_pointer_value()
                                .into()
                        }
                        other => other.into(),
                    }
                } else if expected_ty == Some(&i64_ty.into()) {
                    // Function expects i64 - convert pointer to i64 if needed
                    match result.value {
                        BasicValueEnum::PointerValue(pv) => ctx
                            .builder()
                            .build_ptr_to_int(pv, i64_ty, "ptr_to_i64")
                            .unwrap()
                            .into(),
                        BasicValueEnum::FloatValue(fv) => ctx
                            .builder()
                            .build_bit_cast(fv, i64_ty, "f_to_i64")
                            .unwrap()
                            .into(),
                        other => other.into(),
                    }
                } else {
                    result.value.into()
                };
                arg_values.push(adjusted_value);
                arg_types.push(result.ty);
            }
            crate::parser::ast::Argument::Spread(spread_expr) => {
                let func_param_idx = self_arg_offset + arg_idx;
                // When the spread feeds a rest parameter, package it directly
                // into an Array inside the loop. Building the Array inline
                // (a) gives each store a unique GEP offset so LLVM cannot fold
                // them into a single load even though all iterations share the
                // same SSA value name, and (b) avoids a second pass that would
                // re-store every iteration's load into one slot.
                let rest_elem_ty: Option<Type> = rest_info
                    .as_ref()
                    .filter(|(ri, _)| func_param_idx >= *ri)
                    .map(|(_, et)| et.clone());
                if let Some(elem_ty) = rest_elem_ty {
                    let spread_result = compile_expr(ctx, spread_expr)?;
                    let arr_ptr = match spread_result.value {
                        BasicValueEnum::PointerValue(p) => p,
                        _ => return Err("Spread argument must be an array".to_string()),
                    };
                    let len_fn = ctx
                        .module
                        .get_function("__builtin_array_length")
                        .ok_or("__builtin_array_length not declared")?;
                    let len_val = ctx
                        .builder()
                        .build_call(len_fn, &[arr_ptr.into()], "spread_len")
                        .unwrap()
                        .try_as_basic_value()
                        .unwrap_basic()
                        .into_int_value();
                    let zero = ctx.context.i64_type().const_int(0, false);
                    let one = ctx.context.i64_type().const_int(1, false);

                    let elem_size = if matches!(elem_ty, Type::Dynamic) {
                        16u64
                    } else {
                        8u64
                    };

                    // Allocate Array header (24 bytes: len, cap, data_ptr)
                    let header_total = ctx.context.i64_type().const_int(24, false);
                    let header_ptr = GcAllocFn::for_mode(ctx.gc_mode).emit(
                        ctx.builder(),
                        ctx.module,
                        header_total,
                    );
                    // Allocate data buffer (cap = len, dyn_size = 16 each).
                    // Run-time cap = len_val; the multiplication keeps the
                    // total dynamic so the buffer fits any spread length.
                    let elem_size_const = ctx.context.i64_type().const_int(elem_size, false);
                    let data_total = ctx
                        .builder()
                        .build_int_mul(len_val, elem_size_const, "spread_data_total")
                        .unwrap();
                    let data_ptr = GcAllocFn::for_mode(ctx.gc_mode).emit(
                        ctx.builder(),
                        ctx.module,
                        data_total,
                    );

                    // Store len
                    let len_slot = ctx
                        .builder()
                        .build_pointer_cast(
                            header_ptr,
                            ctx.context.ptr_type(Default::default()),
                            "spread_len_slot",
                        )
                        .unwrap();
                    ctx.builder().build_store(len_slot, len_val).unwrap();
                    // Store cap (= len)
                    let cap_off = ctx.context.i32_type().const_int(8, false);
                    let cap_slot = unsafe {
                        ctx.builder()
                            .build_gep(
                                ctx.context.i8_type(),
                                header_ptr,
                                &[cap_off],
                                "spread_cap_slot",
                            )
                            .unwrap()
                    };
                    let cap_i64 = ctx
                        .builder()
                        .build_pointer_cast(
                            cap_slot,
                            ctx.context.ptr_type(Default::default()),
                            "spread_cap_i64",
                        )
                        .unwrap();
                    ctx.builder().build_store(cap_i64, len_val).unwrap();
                    // Store data_ptr
                    let data_off = ctx.context.i32_type().const_int(16, false);
                    let data_slot = unsafe {
                        ctx.builder()
                            .build_gep(
                                ctx.context.i8_type(),
                                header_ptr,
                                &[data_off],
                                "spread_data_slot",
                            )
                            .unwrap()
                    };
                    let data_slot_i64 = ctx
                        .builder()
                        .build_pointer_cast(
                            data_slot,
                            ctx.context.ptr_type(Default::default()),
                            "spread_data_slot_i64",
                        )
                        .unwrap();
                    let data_addr = ctx
                        .builder()
                        .build_ptr_to_int(data_ptr, ctx.context.i64_type(), "spread_data_addr")
                        .unwrap();
                    ctx.builder().build_store(data_slot_i64, data_addr).unwrap();

                    // Source-side data pointer for dynamic spread arrays
                    let src_data_base = if matches!(elem_ty, Type::Dynamic) {
                        Some(load_array_data_ptr(ctx, arr_ptr, "spread_src_data"))
                    } else {
                        None
                    };
                    let dyn_struct_ty = if matches!(elem_ty, Type::Dynamic) {
                        Some(ruyi_type_to_llvm(ctx.context, &Type::Dynamic).into_struct_type())
                    } else {
                        None
                    };

                    let idx_ptr = ctx
                        .builder()
                        .build_alloca(ctx.context.i64_type(), "spread_idx_r")
                        .unwrap();
                    ctx.builder().build_store(idx_ptr, zero).unwrap();
                    let func = ctx.current_function.ok_or("No current function")?;
                    let loop_bb = ctx.context.append_basic_block(func, "spread_loop_r");
                    let done_bb = ctx.context.append_basic_block(func, "spread_done_r");
                    let body_bb = ctx.context.append_basic_block(func, "spread_body_r");
                    ctx.builder().build_unconditional_branch(loop_bb).unwrap();
                    ctx.builder().position_at_end(loop_bb);
                    let idx = ctx
                        .builder()
                        .build_load(ctx.context.i64_type(), idx_ptr, "idx_r")
                        .unwrap()
                        .into_int_value();
                    let at_end = ctx
                        .builder()
                        .build_int_compare(inkwell::IntPredicate::UGE, idx, len_val, "at_end_r")
                        .unwrap();
                    ctx.builder()
                        .build_conditional_branch(at_end, done_bb, body_bb)
                        .unwrap();
                    ctx.builder().position_at_end(body_bb);

                    // Compute GEP offsets
                    let stride_const = ctx.context.i64_type().const_int(elem_size, false);
                    let byte_offset = ctx
                        .builder()
                        .build_int_mul(idx, stride_const, "spread_byte_offset")
                        .unwrap();
                    let offset_i32 = ctx
                        .builder()
                        .build_int_truncate(byte_offset, ctx.context.i32_type(), "spread_off_i32")
                        .unwrap();
                    // Destination: store to pre-allocated buffer
                    let dest_raw_ptr = unsafe {
                        ctx.builder()
                            .build_gep(
                                ctx.context.i8_type(),
                                data_ptr,
                                &[offset_i32],
                                "spread_dest_raw",
                            )
                            .unwrap()
                    };
                    if let (Some(struct_ty), Some(src_base)) = (dyn_struct_ty, src_data_base) {
                        // Dynamic element path: load src struct, store into dest
                        let src_raw_ptr = unsafe {
                            ctx.builder()
                                .build_gep(
                                    ctx.context.i8_type(),
                                    src_base,
                                    &[offset_i32],
                                    "spread_src_raw",
                                )
                                .unwrap()
                        };
                        let src_struct_ptr = ctx
                            .builder()
                            .build_pointer_cast(
                                src_raw_ptr,
                                ctx.context.ptr_type(Default::default()),
                                "spread_src_struct_ptr",
                            )
                            .unwrap();
                        let loaded = ctx
                            .builder()
                            .build_load(struct_ty, src_struct_ptr, "spread_src_loaded")
                            .unwrap();
                        let dest_struct_ptr = ctx
                            .builder()
                            .build_pointer_cast(
                                dest_raw_ptr,
                                ctx.context.ptr_type(Default::default()),
                                "spread_dest_struct_ptr",
                            )
                            .unwrap();
                        ctx.builder().build_store(dest_struct_ptr, loaded).unwrap();
                    } else {
                        // Scalar (i64) element path
                        let get_fn = ctx
                            .module
                            .get_function("__builtin_array_get")
                            .ok_or("__builtin_array_get not declared")?;
                        let elem = ctx
                            .builder()
                            .build_call(get_fn, &[arr_ptr.into(), idx.into()], "spread_src_elem")
                            .unwrap()
                            .try_as_basic_value()
                            .unwrap_basic()
                            .into_int_value();
                        let elem_ptr = ctx
                            .builder()
                            .build_pointer_cast(
                                dest_raw_ptr,
                                ctx.context.ptr_type(Default::default()),
                                "spread_dest_elem_ptr",
                            )
                            .unwrap();
                        ctx.builder().build_store(elem_ptr, elem).unwrap();
                    }
                    let next_idx = ctx.builder().build_int_add(idx, one, "next_idx_r").unwrap();
                    ctx.builder().build_store(idx_ptr, next_idx).unwrap();
                    ctx.builder().build_unconditional_branch(loop_bb).unwrap();
                    ctx.builder().position_at_end(done_bb);

                    arg_values.push(header_ptr.into());
                    prebuilt_rest_positions.insert(arg_values.len() - 1);
                } else {
                    let spread_result = compile_expr(ctx, spread_expr)?;
                    let arr_ptr = match spread_result.value {
                        BasicValueEnum::PointerValue(p) => p,
                        _ => return Err("Spread argument must be an array".to_string()),
                    };
                    let len_fn = ctx
                        .module
                        .get_function("__builtin_array_length")
                        .ok_or("__builtin_array_length not declared")?;
                    let len_val = ctx
                        .builder()
                        .build_call(len_fn, &[arr_ptr.into()], "spread_len")
                        .unwrap()
                        .try_as_basic_value()
                        .unwrap_basic()
                        .into_int_value();
                    let zero = ctx.context.i64_type().const_int(0, false);
                    let one = ctx.context.i64_type().const_int(1, false);
                    // For Dynamic arrays each slot is a 16-byte `{i64, ptr}`
                    // struct; `__builtin_array_get` returns only the first
                    // 8-byte word, so we cannot use it here. Detect the
                    // element type from the spread expression and emit a
                    // direct GEP+load when the element layout is Dynamic.
                    let is_dyn_elem = matches!(
                        spread_result.ty,
                        Type::Array(ref inner) if matches!(inner.as_ref(), Type::Dynamic)
                    );
                    let dyn_struct_ty = if is_dyn_elem {
                        Some(ruyi_type_to_llvm(ctx.context, &Type::Dynamic).into_struct_type())
                    } else {
                        None
                    };
                    let data_base = if is_dyn_elem {
                        Some(load_array_data_ptr(ctx, arr_ptr, "spread_dyn_data2"))
                    } else {
                        None
                    };
                    let get_fn = ctx
                        .module
                        .get_function("__builtin_array_get")
                        .ok_or("__builtin_array_get not declared")?;
                    let idx_ptr = ctx
                        .builder()
                        .build_alloca(ctx.context.i64_type(), "spread_idx2")
                        .unwrap();
                    ctx.builder().build_store(idx_ptr, zero).unwrap();
                    let loop_bb = ctx.context.append_basic_block(
                        ctx.current_function.ok_or("No current function")?,
                        "spread_loop2",
                    );
                    let done_bb = ctx.context.append_basic_block(
                        ctx.current_function.ok_or("No current function")?,
                        "spread_done2",
                    );
                    ctx.builder().build_unconditional_branch(loop_bb).unwrap();
                    ctx.builder().position_at_end(loop_bb);
                    let idx = ctx
                        .builder()
                        .build_load(ctx.context.i64_type(), idx_ptr, "idx2")
                        .unwrap()
                        .into_int_value();
                    let at_end = ctx
                        .builder()
                        .build_int_compare(inkwell::IntPredicate::UGE, idx, len_val, "at_end2")
                        .unwrap();
                    let body_bb = ctx.context.append_basic_block(
                        ctx.current_function.ok_or("No current function")?,
                        "spread_body2",
                    );
                    ctx.builder()
                        .build_conditional_branch(at_end, done_bb, body_bb)
                        .unwrap();
                    ctx.builder().position_at_end(body_bb);
                    if let (Some(struct_ty), Some(data)) = (dyn_struct_ty, data_base) {
                        let stride = ctx.context.i64_type().const_int(16, false);
                        let byte_offset = ctx
                            .builder()
                            .build_int_mul(idx, stride, "spread_dyn_stride2")
                            .unwrap();
                        let offset_i32 = ctx
                            .builder()
                            .build_int_truncate(
                                byte_offset,
                                ctx.context.i32_type(),
                                "spread_dyn_off32b",
                            )
                            .unwrap();
                        let elem_gep = unsafe {
                            ctx.builder()
                                .build_gep(
                                    ctx.context.i8_type(),
                                    data,
                                    &[offset_i32],
                                    "spread_dyn_gep2",
                                )
                                .unwrap()
                        };
                        let struct_ptr = ctx
                            .builder()
                            .build_pointer_cast(
                                elem_gep,
                                ctx.context.ptr_type(Default::default()),
                                "spread_dyn_struct_ptr2",
                            )
                            .unwrap();
                        let loaded = ctx
                            .builder()
                            .build_load(struct_ty, struct_ptr, "spread_dyn_elem2")
                            .unwrap();
                        arg_values.push(loaded.into());
                    } else {
                        let elem = ctx
                            .builder()
                            .build_call(get_fn, &[arr_ptr.into(), idx.into()], "spread_elem2")
                            .unwrap()
                            .try_as_basic_value()
                            .unwrap_basic()
                            .into_int_value();
                        arg_values.push(elem.into());
                    }
                    let next_idx = ctx.builder().build_int_add(idx, one, "next_idx2").unwrap();
                    ctx.builder().build_store(idx_ptr, next_idx).unwrap();
                    ctx.builder().build_unconditional_branch(loop_bb).unwrap();
                    ctx.builder().position_at_end(done_bb);
                }
            }
        }
    }

    // Handle rest parameters: package extra arguments into an Array
    if let Some((rest_idx, elem_ty)) = rest_info {
        // If the rest slot was already filled by a spread that we packaged
        // inline (see the `if let Some(elem_ty) = rest_elem_ty` branch above)
        // then we must NOT drain+repackage it: the spread loop already wrote
        // the array contents into the destination buffer. Any non-spread args
        // appearing after the spread are concatenated into the prebuilt Array
        // (see append_to_prebuilt_rest_array below).
        let has_prebuilt = prebuilt_rest_positions
            .iter()
            .any(|&p| p >= rest_idx && p < arg_values.len());
        if !has_prebuilt && arg_values.len() > rest_idx {
            let rest_args: Vec<_> = arg_values.drain(rest_idx..).collect();
            let rest_types: Vec<_> = if arg_types.len() > rest_idx {
                arg_types.drain(rest_idx..).collect()
            } else {
                vec![Type::Dynamic; rest_args.len()]
            };
            let array_ptr = compile_rest_args_to_array(ctx, &rest_args, &rest_types, &elem_ty)?;
            arg_values.push(array_ptr.into());
        }
    }

    // Intercept __builtin_array_get for Dynamic arrays: the runtime uses
    // 8-byte stride, but Dynamic elements are 16-byte {i64, i8*} structs.
    // Use direct 16-byte stride GEP instead.
    if name == "__builtin_array_get" || name == "__builtin_array_pop" {
        // Determine the array element type from the first argument's type.
        let arr_elem_ty = arg_types.first().and_then(|t| {
            let mut inner = t;
            while let Type::Nullable(i) = inner {
                inner = i;
            }
            if let Type::Array(elem) = inner {
                Some(*elem.clone())
            } else {
                None
            }
        });
        // Check if the current function's LLVM return type is wider than i64
        // (e.g., a struct {i64, i8*} for Dynamic). This handles generic
        // methods where the type parameter T is not resolved to Dynamic.
        let current_fn_ret_is_struct = ctx
            .current_function
            .and_then(|f| {
                let fn_ty = f.get_type();
                let ret = fn_ty.get_return_type()?;
                Some(ret.is_struct_type())
            })
            .unwrap_or(false);
        let is_dynamic_array = matches!(arr_elem_ty, Some(Type::Dynamic))
            || (matches!(arr_elem_ty, Some(Type::Named(_, _)))
                && !arr_elem_ty.as_ref().map(|t| matches!(t, Type::Named(n, _) if ctx.class_struct_types.contains_key(n))).unwrap_or(false)
                && current_fn_ret_is_struct);
        if is_dynamic_array {
            let arr_ptr = match arg_values[0] {
                inkwell::values::BasicMetadataValueEnum::PointerValue(p) => p,
                _ => return Err("Expected pointer arg for array_get".to_string()),
            };
            let index_val = match arg_values[1] {
                inkwell::values::BasicMetadataValueEnum::IntValue(v) => v,
                _ => return Err("Expected int arg for array_get".to_string()),
            };
            let dyn_struct_ty = ruyi_type_to_llvm(ctx.context, &Type::Dynamic).into_struct_type();
            let i64_ty = ctx.context.i64_type();
            let data_base = load_array_data_ptr(ctx, arr_ptr, "dyn_data2");
            let stride = i64_ty.const_int(16, false);
            let byte_offset = ctx
                .builder()
                .build_int_mul(index_val, stride, "dyn_stride2")
                .unwrap();
            let offset_i32 = ctx
                .builder()
                .build_int_truncate(byte_offset, ctx.context.i32_type(), "off32b")
                .unwrap();
            let elem_gep = unsafe {
                ctx.builder()
                    .build_gep(
                        ctx.context.i8_type(),
                        data_base,
                        &[offset_i32],
                        "dyn_elem_gep2",
                    )
                    .unwrap()
            };
            let struct_ptr = ctx
                .builder()
                .build_pointer_cast(
                    elem_gep,
                    ctx.context.ptr_type(Default::default()),
                    "dyn_struct_ptr2",
                )
                .unwrap();
            let loaded = ctx
                .builder()
                .build_load(dyn_struct_ty, struct_ptr, "dyn_elem2")
                .unwrap();
            return Ok(ExprResult::new(loaded, Type::Dynamic));
        }
    }

    // Default parameters: synthesize omitted trailing arguments at the
    // call site. LLVM has no default-argument concept; without this the
    // call passes too few arguments and the callee reads undefined
    // registers (e.g. a wild pointer in an omitted string parameter).
    if arg_values.len() < func_param_types.len() {
        let defaults = ctx.default_params.get(&name).cloned();
        for param_idx in arg_values.len()..func_param_types.len() {
            let default_expr = defaults
                .as_ref()
                .and_then(|d| d.get(param_idx))
                .and_then(|d| d.as_ref());
            let value: BasicValueEnum<'ctx> = match default_expr {
                Some(def_expr) => compile_expr(ctx, def_expr)?.value,
                // A one-argument `s.substring(start)` means "to the end of
                // the string". The generic zero default would hand the
                // runtime `end = 0`, which clamps it to `start` and yields
                // "". The stdlib parses URLs and handshake headers with
                // this form, so synthesize the length instead.
                None if name == "__string_substring" && param_idx == 2 => {
                    let s_ptr = match arg_values.first() {
                        Some(inkwell::values::BasicMetadataValueEnum::PointerValue(pv)) => *pv,
                        _ => return Err("__string_substring: missing string receiver".to_string()),
                    };
                    let i8_ptr = ctx.context.ptr_type(Default::default());
                    let len_fn = match ctx.module.get_function("__string_length") {
                        Some(f) => f,
                        None => {
                            let len_ty = ctx.context.i64_type().fn_type(&[i8_ptr.into()], false);
                            ctx.module.add_function("__string_length", len_ty, None)
                        }
                    };
                    let len = ctx
                        .builder()
                        .build_call(len_fn, &[s_ptr.into()], "sub_len")
                        .unwrap()
                        .try_as_basic_value()
                        .unwrap_basic()
                        .into_int_value();
                    len.as_basic_value_enum()
                }
                // No default expression: pass the null representation for
                // the expected parameter type (optional-param convention).
                None => match func_param_types.get(param_idx) {
                    Some(t) if t.is_int_type() => {
                        t.into_int_type().const_zero().as_basic_value_enum()
                    }
                    Some(t) if t.is_float_type() => {
                        t.into_float_type().const_zero().as_basic_value_enum()
                    }
                    Some(t) if t.is_struct_type() => {
                        let st = t.into_struct_type();
                        if st.count_fields() == 2 {
                            // Dynamic {i64 tag, ptr data} — null dynamic.
                            ctx.context
                                .const_struct(
                                    &[
                                        ctx.context.i64_type().const_zero().into(),
                                        ctx.context
                                            .ptr_type(Default::default())
                                            .const_null()
                                            .into(),
                                    ],
                                    false,
                                )
                                .as_basic_value_enum()
                        } else {
                            st.get_undef().as_basic_value_enum()
                        }
                    }
                    _ => ctx
                        .context
                        .ptr_type(Default::default())
                        .const_null()
                        .as_basic_value_enum(),
                },
            };
            // Adjust the synthesized value to the declared LLVM parameter
            // type (mirrors the main argument loop above).
            let i8_ptr = ctx.context.ptr_type(Default::default());
            let i64_ty = ctx.context.i64_type();
            let adjusted: inkwell::values::BasicMetadataValueEnum<'ctx> =
                match func_param_types.get(param_idx) {
                    Some(t) if *t == i8_ptr.into() => match value {
                        BasicValueEnum::IntValue(iv) => ctx
                            .builder()
                            .build_int_to_ptr(iv, i8_ptr, "def_to_ptr")
                            .unwrap()
                            .into(),
                        other => other.into(),
                    },
                    Some(t) if *t == i64_ty.into() => match value {
                        BasicValueEnum::PointerValue(pv) => ctx
                            .builder()
                            .build_ptr_to_int(pv, i64_ty, "def_to_i64")
                            .unwrap()
                            .into(),
                        other => other.into(),
                    },
                    _ => value.into(),
                };
            arg_values.push(adjusted);
        }
    }

    let call_site = build_call_or_invoke(ctx, func, &arg_values, "call");
    let mut value = call_site.try_as_basic_value().basic();

    let is_async = ctx.module.get_function(&format!("{}$poll", name)).is_some();

    let mut ret_ty = if is_async {
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
    } else if let Some(sig) = super::builtins_table::builtin_ret_sig(&name) {
        // Builtins with a known return signature keep their precise type
        // (e.g. `Int` for `__string_char_code_at`) instead of collapsing to
        // `Dynamic`, so arithmetic on their results compiles correctly.
        super::builtins_table::sig_to_type(&name, sig)
    } else {
        Type::Dynamic
    };

    // For trait impl methods that return a method-level type parameter
    // (e.g. `reduce<U>(self, init: U, ...): U`), recover the declared
    // return type from `impl_method_sigs` so generic U can be substituted
    // from the actual argument types below.
    if matches!(ret_ty, Type::Dynamic) {
        if let Some((_tparams, _for_ann)) = ctx.impl_method_sigs.get(&name).cloned() {
            // Look up the method's declared return type by scanning the
            // impl block's body. Falls through to Dynamic when unavailable.
            ret_ty = lookup_impl_return_type(ctx, &name).unwrap_or(Type::Dynamic);
        }
    }
    // For ArrayOps methods with generic U init (e.g. `reduce<U>`), the
    // erased impl returns `ptr` (U → ptr) but the call site passed in
    // a concrete init. Infer U from the second argument and override
    // ret_ty so downstream prints and arithmetic get the right type.
    if name.ends_with("_ArrayOps_for_Array") && !super::specialize::is_concrete(&ret_ty) {
        // arg_types skips the receiver (no entry for self), so the init
        // sits at index 0 when self_arg_offset == 1.
        if let Some(init_arg_ty) = arg_types.first() {
            if matches!(
                init_arg_ty,
                Type::Int | Type::Float | Type::Bool | Type::String
            ) {
                ret_ty = init_arg_ty.clone();
            }
        }
    }

    // `__builtin_array_get` / `__builtin_array_pop` return the element as an
    // i64 universal register; recover the precise element type from the
    // receiver and convert the value back to its natural representation so
    // downstream arithmetic and method dispatch see the correct type
    // (mirrors the element recovery done in `compile_member_access`).
    if name == "__builtin_array_get" || name == "__builtin_array_pop" {
        // Stdlib wrappers call these as plain functions (`get(self, i)`),
        // so the receiver type usually comes from arg 0 rather than a
        // method receiver.
        let recv_from_args = arg_types.first().cloned();
        if let Some(recv) = call_receiver_ty.as_ref().or(recv_from_args.as_ref()) {
            let mut recv = recv;
            while let Type::Nullable(inner) = recv {
                recv = inner;
            }
            if let Type::Array(elem_ty) = recv {
                if let Some(BasicValueEnum::IntValue(word)) = value {
                    // Resolve type aliases (e.g. StringArray → Array<string>)
                    // so element type recovery works correctly for aliased types.
                    let resolved_elem_ty = ctx.resolve_type_aliases(elem_ty);
                    let (new_val, new_ty) = match &resolved_elem_ty {
                        Type::Float => {
                            let f = ctx
                                .builder()
                                .build_bit_cast(word, ctx.context.f64_type(), "elem_f64")
                                .unwrap();
                            (f, Type::Float)
                        }
                        // Dynamic/Function elements are heap-boxed 16-byte
                        // structs behind the stored pointer word; load the
                        // struct back out of the box.
                        Type::Dynamic | Type::Function { .. } => {
                            let box_ptr = ctx
                                .builder()
                                .build_int_to_ptr(
                                    word,
                                    ctx.context.ptr_type(Default::default()),
                                    "elem_box_ptr",
                                )
                                .unwrap();
                            let struct_ty =
                                ruyi_type_to_llvm(ctx.context, &Type::Dynamic).into_struct_type();
                            let loaded = ctx
                                .builder()
                                .build_load(struct_ty, box_ptr, "elem_box_load")
                                .unwrap();
                            (loaded, resolved_elem_ty.clone())
                        }
                        Type::String | Type::Array(_) | Type::Object(_) => {
                            let ptr = ctx
                                .builder()
                                .build_int_to_ptr(
                                    word,
                                    ctx.context.ptr_type(Default::default()),
                                    "elem_ptr",
                                )
                                .unwrap();
                            (BasicValueEnum::PointerValue(ptr), resolved_elem_ty.clone())
                        }
                        // Named types: only convert to pointer if it's a real class,
                        // not a type parameter (like "T" from generic Array<T>).
                        // Type parameters should stay as i64 since the actual type
                        // is unknown at compile time.
                        Type::Named(name, _) if ctx.class_struct_types.contains_key(name) => {
                            let ptr = ctx
                                .builder()
                                .build_int_to_ptr(
                                    word,
                                    ctx.context.ptr_type(Default::default()),
                                    "elem_ptr",
                                )
                                .unwrap();
                            (BasicValueEnum::PointerValue(ptr), resolved_elem_ty.clone())
                        }
                        // Named types that are aliases already resolved above should
                        // not reach here; only unresolved type parameters remain.
                        _ => (BasicValueEnum::IntValue(word), Type::Int),
                    };
                    value = Some(new_val);
                    ret_ty = new_ty;
                }
            }
        }
    }

    // `__builtin_array_push` returns the (possibly reallocated) array itself.
    // The builtin signature reports a raw pointer (Dynamic); recover the precise
    // array type from the receiver so chained re-binding like
    // `let nums = nums.push(4)` keeps the element type instead of poisoning it
    // to Dynamic (which would break a subsequent `nums.push(...)` dispatch).
    if name == "__builtin_array_push" {
        if let Some(recv) = call_receiver_ty.as_ref() {
            let mut recv = recv;
            while let Type::Nullable(inner) = recv {
                recv = inner;
            }
            if matches!(recv, Type::Array(_)) {
                ret_ty = recv.clone();
            }
        }
    }

    // Substitute type parameters left in the return type using the
    // concrete receiver type (impl methods and erased generic-class
    // methods keep T/V in their registered signatures).
    if super::specialize::has_type_params(&ret_ty) {
        if let Some(recv) = call_receiver_ty.as_ref() {
            let mut recv = recv;
            while let Type::Nullable(inner) = recv {
                recv = inner;
            }
            let mut bindings = std::collections::HashMap::new();
            if let Some((tparams, for_ann)) = ctx.impl_method_sigs.get(&name).cloned() {
                super::specialize::bind_type_params(&for_ann, recv, &tparams, &mut bindings);
            } else if let Type::Generic { base, args } = recv {
                if let Some((tparams, _)) = ctx.generic_classes.get(base) {
                    for (p, a) in tparams.iter().zip(args.iter()) {
                        bindings.insert(p.clone(), a.clone());
                    }
                }
            }
            if !bindings.is_empty() {
                ret_ty = super::specialize::subst_type(&ret_ty, &bindings);
            }
        }
    }

    // 泛型擦除后的调用结果类型适配：
    // 当返回类型经 substitution 后已具体化，但 LLVM 值的实际类型与期望不符时
    // （典型场景：函数返回 T 擦除为 i8*，但 substitution 后期望 i64/int），
    // 在此处插入 ptrtoint / inttoptr 转换。
    if let Some(v) = value.as_ref() {
        if super::specialize::is_concrete(&ret_ty) {
            let expected_llvm = ruyi_type_to_llvm(ctx.context, &ret_ty);
            let actual_llvm = v.get_type();
            if expected_llvm != actual_llvm {
                if actual_llvm.is_pointer_type() && expected_llvm.is_int_type() {
                    value = Some(BasicValueEnum::IntValue(
                        ctx.builder()
                            .build_ptr_to_int(
                                v.into_pointer_value(),
                                expected_llvm.into_int_type(),
                                "call_ptr_to_int",
                            )
                            .unwrap(),
                    ));
                } else if actual_llvm.is_int_type() && expected_llvm.is_pointer_type() {
                    value = Some(BasicValueEnum::PointerValue(
                        ctx.builder()
                            .build_int_to_ptr(
                                v.into_int_value(),
                                expected_llvm.into_pointer_type(),
                                "call_int_to_ptr",
                            )
                            .unwrap(),
                    ));
                } else if actual_llvm.is_struct_type() && expected_llvm.is_pointer_type() {
                    // Dynamic struct → i8*：提取 data_ptr
                    let sv = v.into_struct_value();
                    let data_ptr = ctx
                        .builder()
                        .build_extract_value(sv, 1, "call_s2p")
                        .unwrap()
                        .into_pointer_value();
                    value = Some(BasicValueEnum::PointerValue(data_ptr));
                } else if actual_llvm.is_struct_type() && expected_llvm.is_int_type() {
                    // Dynamic struct → i64：提取 data_ptr + ptrtoint
                    let sv = v.into_struct_value();
                    let data_ptr = ctx
                        .builder()
                        .build_extract_value(sv, 1, "call_s2i_ptr")
                        .unwrap()
                        .into_pointer_value();
                    value = Some(BasicValueEnum::IntValue(
                        ctx.builder()
                            .build_ptr_to_int(data_ptr, expected_llvm.into_int_type(), "call_s2i")
                            .unwrap(),
                    ));
                }
            }
        }
    }

    match value {
        Some(v) => Ok(ExprResult::new(v, ret_ty)),
        None => Ok(ExprResult::new(
            BasicValueEnum::IntValue(ctx.context.i64_type().const_int(0, false)),
            Type::Void,
        )),
    }
}

/// Box a value into a Dynamic `{i64, i8*}` struct.
///
/// For class instances: tag = typeid, data = object pointer.
/// For primitives: tag = value (int/float bits/bool), data = null.
/// For strings: tag = 0, data = string pointer.
// Primitive-to-string discriminator constants. The toString dispatch
// in the stdlib branches on `ptrtoint(data)`: values <= 3 are
// primitives (the exact value selects the int/float/bool branch), any
// higher address is treated as a string payload. These tags MUST stay
// in sync with the runtime helpers in
// `crates/ruyi_runtime/src/fmt_ffi.rs` (`DYN_TAG_*`) and the FFI
// dispatcher `__fmt_format_dyn`.
const DYN_TAG_INT: i32 = 1;
const DYN_TAG_FLOAT: i32 = 2;
const DYN_TAG_BOOL: i32 = 3;
pub(crate) fn build_box_dynamic<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    value: BasicValueEnum<'ctx>,
    ty: &Type,
) -> BasicValueEnum<'ctx> {
    let dyn_llvm_ty = ruyi_type_to_llvm(ctx.context, &Type::Dynamic).into_struct_type();
    let i64_ty = ctx.context.i64_type();
    let i8_ptr_ty = ctx.context.ptr_type(Default::default());
    let zero_i64 = i64_ty.const_int(0, false);
    let _null_ptr = i8_ptr_ty.const_null();

    let mut s = dyn_llvm_ty.const_zero();
    match ty {
        Type::Named(class_name, _) => {
            let typeid = ctx.type_ids.get(class_name).copied().unwrap_or(0);
            let tag_val = i64_ty.const_int(typeid, false);
            let ptr = value.into_pointer_value();
            let data = ctx
                .builder()
                .build_pointer_cast(ptr, i8_ptr_ty, "box_dyn_data")
                .unwrap();
            s = ctx
                .builder()
                .build_insert_value(s, tag_val, 0, "box_tag")
                .unwrap()
                .into_struct_value();
            s = ctx
                .builder()
                .build_insert_value(s, data, 1, "box_ptr")
                .unwrap()
                .into_struct_value();
        }
        Type::Int => {
            let int_val = value.into_int_value();
            let extended = if int_val.get_type().get_bit_width() < 64 {
                ctx.builder()
                    .build_int_z_extend(int_val, i64_ty, "box_int_ext")
                    .unwrap()
            } else {
                int_val
            };
            // Tag: field 0 = the int value itself. The toString
            // dispatch (`compile_dynamic_method_call`) reads
            // `extractvalue {i64, ptr} ... 0` and forwards it to
            // `ruyi_int_to_string`, so the value must live in this
            // slot. Previously this slot held the int type marker
            // (constant 1), which made every int arg print as "1"
            // regardless of its actual value (see v0.6.0-fix-examples
            // B1).
            s = ctx
                .builder()
                .build_insert_value(s, extended, 0, "box_int_tag")
                .unwrap()
                .into_struct_value();
            // Data: field 1 = inttoptr(DYN_TAG_INT) so the
            // primitive-vs-string discriminator (`ptrtoint(data) <= 3`)
            // picks the int branch.
            let data = ctx
                .builder()
                .build_int_to_ptr(
                    i64_ty.const_int(DYN_TAG_INT as u64, false),
                    i8_ptr_ty,
                    "box_int_data",
                )
                .unwrap();
            s = ctx
                .builder()
                .build_insert_value(s, data, 1, "box_int_ptr")
                .unwrap()
                .into_struct_value();
        }
        Type::Float => {
            let f = value.into_float_value();
            let bits = ctx
                .builder()
                .build_bit_cast(f, i64_ty, "box_fbits")
                .unwrap();
            s = ctx
                .builder()
                .build_insert_value(s, bits, 0, "box_float")
                .unwrap()
                .into_struct_value();
            // Tag: field 1 = inttoptr(DYN_TAG_FLOAT) so the
            // primitive-vs-string discriminator (`ptrtoint(data) <= 3`)
            // picks the float branch.
            let float_tag = ctx
                .builder()
                .build_int_to_ptr(
                    i64_ty.const_int(DYN_TAG_FLOAT as u64, false),
                    i8_ptr_ty,
                    "float_tag",
                )
                .unwrap();
            s = ctx
                .builder()
                .build_insert_value(s, float_tag, 1, "box_float_tag")
                .unwrap()
                .into_struct_value();
        }
        Type::Bool => {
            let b = value.into_int_value();
            let extended = ctx
                .builder()
                .build_int_z_extend(b, i64_ty, "box_bool_ext")
                .unwrap();
            s = ctx
                .builder()
                .build_insert_value(s, extended, 0, "box_bool")
                .unwrap()
                .into_struct_value();
            // Tag: field 1 = inttoptr(DYN_TAG_BOOL) so the
            // primitive-vs-string discriminator (`ptrtoint(data) <= 3`)
            // picks the bool branch.
            let bool_tag = ctx
                .builder()
                .build_int_to_ptr(
                    i64_ty.const_int(DYN_TAG_BOOL as u64, false),
                    i8_ptr_ty,
                    "bool_tag",
                )
                .unwrap();
            s = ctx
                .builder()
                .build_insert_value(s, bool_tag, 1, "box_bool_tag")
                .unwrap()
                .into_struct_value();
        }
        Type::String => {
            let ptr = value.into_pointer_value();
            let data = ctx
                .builder()
                .build_pointer_cast(ptr, i8_ptr_ty, "box_str_data")
                .unwrap();
            s = ctx
                .builder()
                .build_insert_value(s, zero_i64, 0, "box_str_tag")
                .unwrap()
                .into_struct_value();
            s = ctx
                .builder()
                .build_insert_value(s, data, 1, "box_str_ptr")
                .unwrap()
                .into_struct_value();
        }
        _ => {
            // For Type::Dynamic or unknown types, inspect the LLVM value
            // to determine the boxing strategy.
            match value {
                BasicValueEnum::StructValue(s) => {
                    // Already a Dynamic `{i64, ptr}` (e.g. spread out of
                    // another `Array<dyn>`). Forward as-is instead of
                    // re-boxing, which would corrupt the tag/data fields
                    // and drop primitives to `{0, null}` (see
                    // v0.6.0-fix-examples B1).
                    return s.into();
                }
                BasicValueEnum::IntValue(v) => {
                    // Box as int: {type_tag=1, inttoptr(value)}
                    let extended = if v.get_type().get_bit_width() < 64 {
                        ctx.builder()
                            .build_int_z_extend(v, i64_ty, "box_dyn_ext")
                            .unwrap()
                    } else {
                        v
                    };
                    let type_tag = i64_ty.const_int(1, false);
                    s = ctx
                        .builder()
                        .build_insert_value(s, type_tag, 0, "box_dyn_tag")
                        .unwrap()
                        .into_struct_value();
                    let data = ctx
                        .builder()
                        .build_int_to_ptr(extended, i8_ptr_ty, "box_dyn_data")
                        .unwrap();
                    s = ctx
                        .builder()
                        .build_insert_value(s, data, 1, "box_dyn_ptr")
                        .unwrap()
                        .into_struct_value();
                }
                BasicValueEnum::PointerValue(p) => {
                    // Box as pointer/string: {0, ptr}
                    let data = ctx
                        .builder()
                        .build_pointer_cast(p, i8_ptr_ty, "box_dyn_ptr")
                        .unwrap();
                    s = ctx
                        .builder()
                        .build_insert_value(s, data, 1, "box_dyn_ptr")
                        .unwrap()
                        .into_struct_value();
                }
                BasicValueEnum::FloatValue(f) => {
                    // Box as float: {type_tag=2, inttoptr(bitcast(f64→i64))}
                    let bits = ctx
                        .builder()
                        .build_bit_cast(f, i64_ty, "box_dyn_fbits")
                        .unwrap()
                        .into_int_value();
                    let type_tag = i64_ty.const_int(2, false);
                    s = ctx
                        .builder()
                        .build_insert_value(s, type_tag, 0, "box_dyn_tag")
                        .unwrap()
                        .into_struct_value();
                    let data = ctx
                        .builder()
                        .build_int_to_ptr(bits, i8_ptr_ty, "box_dyn_data")
                        .unwrap();
                    s = ctx
                        .builder()
                        .build_insert_value(s, data, 1, "box_dyn_ptr")
                        .unwrap()
                        .into_struct_value();
                }
                _ => {
                    // For other value types, return zero-initialized Dynamic.
                }
            }
        }
    }
    BasicValueEnum::StructValue(s)
}

/// Store a word-struct value (`Dynamic`/`Function`, 16 bytes) into an
/// `Array<T>` element slot by heap-boxing it: the 16-byte struct lives
/// in a GC cell and the stored 8-byte word is the cell pointer. Reads
/// reverse this via `int_to_ptr` + struct load (see the get/pop/index
/// recovery paths), so the pointer round-trips losslessly.
pub(crate) fn build_box_struct_to_word<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    value: BasicValueEnum<'ctx>,
) -> BasicValueEnum<'ctx> {
    let sv = match value {
        BasicValueEnum::StructValue(sv) => sv,
        _ => return value,
    };
    let cell = super::gc_alloc::GcAllocFn::for_mode(ctx.gc_mode).emit(
        ctx.builder(),
        ctx.module,
        ctx.context.i64_type().const_int(16, false),
    );
    ctx.builder().build_store(cell, sv).unwrap();
    let addr = ctx
        .builder()
        .build_ptr_to_int(cell, ctx.context.i64_type(), "box_word_addr")
        .unwrap();
    BasicValueEnum::IntValue(addr)
}

fn compile_assignment<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    left: &Expr,
    op: &crate::parser::ast::AssignOp,
    right: &Expr,
) -> Result<ExprResult<'ctx>, String> {
    let right_result = compile_expr(ctx, right)?;

    let effective = match op {
        crate::parser::ast::AssignOp::Assign => right_result,
        crate::parser::ast::AssignOp::PlusAssign => {
            compile_binary(ctx, &BinaryOp::Plus, left, right)?
        }
        crate::parser::ast::AssignOp::MinusAssign => {
            compile_binary(ctx, &BinaryOp::Minus, left, right)?
        }
        crate::parser::ast::AssignOp::StarAssign => {
            compile_binary(ctx, &BinaryOp::Star, left, right)?
        }
        crate::parser::ast::AssignOp::SlashAssign => {
            compile_binary(ctx, &BinaryOp::Slash, left, right)?
        }
        crate::parser::ast::AssignOp::PercentAssign => {
            compile_binary(ctx, &BinaryOp::Percent, left, right)?
        }
        crate::parser::ast::AssignOp::PowerAssign => {
            compile_binary(ctx, &BinaryOp::Power, left, right)?
        }
        crate::parser::ast::AssignOp::ShlAssign => {
            compile_binary(ctx, &BinaryOp::Shl, left, right)?
        }
        crate::parser::ast::AssignOp::ShrAssign => {
            compile_binary(ctx, &BinaryOp::Shr, left, right)?
        }
        crate::parser::ast::AssignOp::UShrAssign => {
            compile_binary(ctx, &BinaryOp::UShr, left, right)?
        }
        crate::parser::ast::AssignOp::AmpAssign => {
            compile_binary(ctx, &BinaryOp::Amp, left, right)?
        }
        crate::parser::ast::AssignOp::PipeAssign => {
            compile_binary(ctx, &BinaryOp::Pipe, left, right)?
        }
        crate::parser::ast::AssignOp::CaretAssign => {
            compile_binary(ctx, &BinaryOp::Caret, left, right)?
        }
        crate::parser::ast::AssignOp::AndAssign => {
            compile_binary(ctx, &BinaryOp::And, left, right)?
        }
        crate::parser::ast::AssignOp::OrAssign => compile_binary(ctx, &BinaryOp::Or, left, right)?,
        crate::parser::ast::AssignOp::NullishAssign => {
            compile_binary(ctx, &BinaryOp::Nullish, left, right)?
        }
    };

    match left {
        Expr::Identifier(name) => {
            if let Some((ptr, var_ty)) = ctx.lookup_variable(name) {
                // Trait object coercion: when assigning to a `dyn Trait` variable,
                // wrap the concrete value into a { data, vtable } fat pointer.
                if let Type::Trait(trait_name) = &var_ty {
                    let trait_obj_val = super::traits::build_trait_object_value(
                        ctx,
                        effective.value,
                        &effective.ty,
                        trait_name,
                    )?;
                    ctx.builder().build_store(ptr, trait_obj_val).unwrap();
                } else if var_ty == Type::Dynamic && effective.ty != Type::Dynamic {
                    // Named→Dynamic boxing: construct {i64, i8*} struct
                    let dyn_val = build_box_dynamic(ctx, effective.value, &effective.ty);
                    ctx.builder().build_store(ptr, dyn_val).unwrap();
                } else {
                    ctx.builder().build_store(ptr, effective.value).unwrap();
                }
                Ok(effective)
            } else {
                Err(format!("Undefined variable: {}", name))
            }
        }
        Expr::Member {
            object, property, ..
        } => {
            match property {
                crate::parser::ast::MemberProperty::Ident(field_name) => {
                    let obj_result = compile_expr(ctx, object)?;

                    // Object literal types: inline fields, flat offset GEP
                    if let Type::Object(fields) = &obj_result.ty {
                        let obj_ptr = obj_result.value.into_pointer_value();
                        let field_index = fields
                            .iter()
                            .position(|f| f.name == *field_name)
                            .ok_or_else(|| format!("Unknown field: {} in object", field_name))?;
                        let offset = ctx
                            .context
                            .i32_type()
                            .const_int((field_index * 8) as u64, false);
                        let field_ptr = unsafe {
                            ctx.builder()
                                .build_gep(
                                    ctx.context.i8_type(),
                                    obj_ptr,
                                    &[offset],
                                    &format!("{}_ptr", field_name),
                                )
                                .unwrap()
                        };
                        let typed_ptr = ctx
                            .builder()
                            .build_pointer_cast(
                                field_ptr,
                                ctx.context.ptr_type(Default::default()),
                                &format!("{}_typed", field_name),
                            )
                            .unwrap();
                        ctx.builder()
                            .build_store(typed_ptr, effective.value)
                            .unwrap();
                        return Ok(effective);
                    }

                    let class_name = resolve_class_from_type(&obj_result.ty).ok_or_else(|| {
                        format!("Cannot write field on type: {:?}", obj_result.ty)
                    })?;
                    let obj_ptr = obj_result.value.into_pointer_value();

                    // Check if field_name is a setter property
                    if ctx
                        .class_setters
                        .get(&class_name)
                        .is_some_and(|s| s.contains(field_name))
                    {
                        let setter_fn_name = format!("{}_set_{}", class_name, field_name);
                        let setter_fn =
                            ctx.module.get_function(&setter_fn_name).ok_or_else(|| {
                                format!("Setter function not found: {}", setter_fn_name)
                            })?;
                        ctx.builder()
                            .build_call(
                                setter_fn,
                                &[obj_ptr.into(), effective.value.into()],
                                &format!("set_{}", field_name),
                            )
                            .unwrap();
                        return Ok(effective);
                    }

                    let (field_ptr, field_ty) =
                        class_field_access(ctx, obj_ptr, &class_name, field_name, &obj_result.ty)?;

                    emit_field_store(ctx, field_ptr, &field_ty, effective.value, &effective.ty);
                    Ok(effective)
                }
                crate::parser::ast::MemberProperty::Expr(key_expr) => {
                    let obj_result = compile_expr(ctx, object)?;
                    let key_result = compile_expr(ctx, key_expr)?;
                    let obj_ptr = obj_result.value.into_pointer_value();
                    let index_val = match key_result.value {
                        BasicValueEnum::IntValue(v) => v,
                        BasicValueEnum::FloatValue(v) => ctx
                            .builder()
                            .build_float_to_signed_int(v, ctx.context.i64_type(), "idx_f2i")
                            .unwrap(),
                        _ => return Err("Array index must be an integer".to_string()),
                    };
                    // Every element is stored as one 8-byte word. Dynamic and
                    // Function elements are 16-byte structs, so the value is
                    // heap-boxed and the word is the box pointer (reads
                    // unbox; see the member-index read path).
                    let mut indexed_ty = obj_result.ty.clone();
                    while let Type::Nullable(inner) = indexed_ty {
                        indexed_ty = *inner;
                    }
                    let elem_ty = match &indexed_ty {
                        Type::Array(inner) => ctx.resolve_type_aliases(inner),
                        _ => Type::Int,
                    };
                    let effective_val: BasicValueEnum<'ctx> =
                        if matches!(&elem_ty, Type::Dynamic | Type::Function { .. }) {
                            match effective.value {
                                BasicValueEnum::StructValue(_) => {
                                    build_box_struct_to_word(ctx, effective.value)
                                }
                                other => {
                                    let boxed = build_box_dynamic(ctx, other, &effective.ty);
                                    build_box_struct_to_word(ctx, boxed)
                                }
                            }
                        } else {
                            match effective.value {
                                BasicValueEnum::IntValue(v) => BasicValueEnum::IntValue(v),
                                BasicValueEnum::FloatValue(v) => BasicValueEnum::IntValue(
                                    ctx.builder()
                                        .build_float_to_signed_int(v, ctx.context.i64_type(), "f2i")
                                        .unwrap(),
                                ),
                                BasicValueEnum::PointerValue(p) => BasicValueEnum::IntValue(
                                    ctx.builder()
                                        .build_ptr_to_int(p, ctx.context.i64_type(), "w2i")
                                        .unwrap(),
                                ),
                                _ => {
                                    return Err(
                                        "Unsupported element type for array assignment".to_string()
                                    )
                                }
                            }
                        };
                    let word = match effective_val {
                        BasicValueEnum::IntValue(v) => v,
                        _ => {
                            return Err("Unsupported element type for array assignment".to_string())
                        }
                    };
                    super::builtins::build_builtin_array_set(
                        ctx.builder(),
                        ctx.module,
                        obj_ptr,
                        index_val,
                        word,
                    );
                    Ok(effective)
                }
            }
        }
        _ => Err("Complex assignments not yet supported".to_string()),
    }
}

fn compile_conditional<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    condition: &Expr,
    then_branch: &Expr,
    else_branch: &Expr,
) -> Result<ExprResult<'ctx>, String> {
    let cond_result = compile_expr(ctx, condition)?;
    let cond_val = match cond_result.value {
        BasicValueEnum::IntValue(v) => v,
        _ => return Err("Condition must be boolean".to_string()),
    };

    let func = ctx.current_function().ok_or("No current function")?;

    let then_bb = ctx.context.append_basic_block(func, "then");
    let else_bb = ctx.context.append_basic_block(func, "else");
    let merge_bb = ctx.context.append_basic_block(func, "merge");

    ctx.builder()
        .build_conditional_branch(cond_val, then_bb, else_bb)
        .unwrap();

    ctx.builder().position_at_end(then_bb);
    let then_result = compile_expr(ctx, then_branch)?;
    ctx.builder().build_unconditional_branch(merge_bb).unwrap();
    let then_bb_end = ctx.builder().get_insert_block().unwrap();

    ctx.builder().position_at_end(else_bb);
    let else_result = compile_expr(ctx, else_branch)?;
    ctx.builder().build_unconditional_branch(merge_bb).unwrap();
    let else_bb_end = ctx.builder().get_insert_block().unwrap();

    ctx.builder().position_at_end(merge_bb);

    let phi_ty = ruyi_type_to_llvm(ctx.context, &then_result.ty);
    let phi = ctx.builder().build_phi(phi_ty, "cond_phi").unwrap();
    phi.add_incoming(&[
        (&then_result.value, then_bb_end),
        (&else_result.value, else_bb_end),
    ]);

    let result_ty = then_result.ty.least_upper_bound(&else_result.ty);
    Ok(ExprResult::new(phi.as_basic_value(), result_ty))
}

fn compile_if_expr<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    condition: &Expr,
    then_branch: &Expr,
    else_branch: Option<&Expr>,
) -> Result<ExprResult<'ctx>, String> {
    let cond_result = compile_expr(ctx, condition)?;
    let cond_val = match cond_result.value {
        BasicValueEnum::IntValue(v) => v,
        _ => return Err("Condition must be boolean".to_string()),
    };

    let func = ctx.current_function().ok_or("No current function")?;

    let then_bb = ctx.context.append_basic_block(func, "then");
    let else_bb = ctx.context.append_basic_block(func, "else");
    let merge_bb = ctx.context.append_basic_block(func, "merge");

    ctx.builder()
        .build_conditional_branch(cond_val, then_bb, else_bb)
        .unwrap();

    ctx.builder().position_at_end(then_bb);
    let then_result = compile_expr(ctx, then_branch)?;
    ctx.builder().build_unconditional_branch(merge_bb).unwrap();
    let then_bb_end = ctx.builder().get_insert_block().unwrap();

    ctx.builder().position_at_end(else_bb);
    let else_result = if let Some(else_expr) = else_branch {
        compile_expr(ctx, else_expr)?
    } else {
        ExprResult::new(
            BasicValueEnum::IntValue(ctx.context.i64_type().const_int(0, false)),
            Type::Void,
        )
    };
    ctx.builder().build_unconditional_branch(merge_bb).unwrap();
    let else_bb_end = ctx.builder().get_insert_block().unwrap();

    ctx.builder().position_at_end(merge_bb);

    let phi_ty = ruyi_type_to_llvm(ctx.context, &then_result.ty);
    let phi = ctx.builder().build_phi(phi_ty, "if_phi").unwrap();
    phi.add_incoming(&[
        (&then_result.value, then_bb_end),
        (&else_result.value, else_bb_end),
    ]);

    let result_ty = then_result.ty.least_upper_bound(&else_result.ty);
    Ok(ExprResult::new(phi.as_basic_value(), result_ty))
}

/// 从数组头（24 字节：[len][cap][data_ptr]）加载元素缓冲指针。
///
/// 数据缓冲与头部分离，扩容时只替换缓冲、头指针保持稳定，
/// 因此所有元素访问必须先经此函数取到当前缓冲地址。
fn load_array_data_ptr<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    arr_ptr: inkwell::values::PointerValue<'ctx>,
    name: &str,
) -> inkwell::values::PointerValue<'ctx> {
    let slot = unsafe {
        ctx.builder()
            .build_gep(
                ctx.context.i8_type(),
                arr_ptr,
                &[ctx.context.i32_type().const_int(16, false)],
                &format!("{}_slot", name),
            )
            .unwrap()
    };
    let slot_ptr = ctx
        .builder()
        .build_bit_cast(
            slot,
            ctx.context.ptr_type(Default::default()),
            &format!("{}_slot_ptr", name),
        )
        .unwrap()
        .into_pointer_value();
    let addr = ctx
        .builder()
        .build_load(ctx.context.i64_type(), slot_ptr, &format!("{}_addr", name))
        .unwrap()
        .into_int_value();
    ctx.builder()
        .build_int_to_ptr(addr, ctx.context.ptr_type(Default::default()), name)
        .unwrap()
}

/// Look up the declared return type of a trait impl method by inspecting
/// the registered impl signature / LLVM function signature. We lean on the
/// LLVM function signature because `impl_method_sigs` doesn't carry the
/// return annotation. Returns `None` when no impl is registered.
fn lookup_impl_return_type<'ctx>(
    ctx: &CodegenContext<'ctx, '_, '_>,
    mangled_name: &str,
) -> Option<Type> {
    let func = ctx.module.get_function(mangled_name)?;
    let fn_ty = func.get_type();
    let ret_ty = fn_ty.get_return_type()?;
    if ret_ty.is_int_type() {
        let bits = ret_ty.into_int_type().get_bit_width();
        if bits == 1 {
            return Some(Type::Bool);
        }
        return Some(Type::Int);
    }
    if ret_ty.is_float_type() {
        return Some(Type::Float);
    }
    if ret_ty.is_pointer_type() {
        return Some(Type::Array(Box::new(Type::Dynamic)));
    }
    None
}

fn compile_array_literal<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    elements: &[crate::parser::ast::ArrayElement],
) -> Result<ExprResult<'ctx>, String> {
    // 检查是否包含 spread 元素，如果有则走动态构建路径
    let has_spread = elements
        .iter()
        .any(|e| matches!(e, crate::parser::ast::ArrayElement::Spread(_)));

    if has_spread {
        return compile_array_literal_with_spread(ctx, elements);
    }

    // 快速路径：无 spread 元素时使用静态分配。
    // 布局：24 字节头 [len][cap][data_ptr] + 独立元素缓冲，
    // 保证数组头指针在扩容时保持稳定（引用语义）。
    let len = elements.len() as u64;
    let cap = if len == 0 { 4 } else { len };
    let header_size = ctx.context.i64_type().const_int(24, false);
    let ptr = GcAllocFn::for_mode(ctx.gc_mode).emit(ctx.builder(), ctx.module, header_size);

    let data_size = ctx.context.i64_type().const_int(cap * 8, false);
    let data_ptr = GcAllocFn::for_mode(ctx.gc_mode).emit(ctx.builder(), ctx.module, data_size);

    let len_ptr = ctx
        .builder()
        .build_bit_cast(ptr, ctx.context.ptr_type(Default::default()), "len_ptr")
        .unwrap()
        .into_pointer_value();
    ctx.builder()
        .build_store(len_ptr, ctx.context.i64_type().const_int(len, false))
        .unwrap();

    let cap_ptr = unsafe {
        ctx.builder()
            .build_gep(
                ctx.context.i8_type(),
                ptr,
                &[ctx.context.i32_type().const_int(8, false)],
                "cap_ptr",
            )
            .unwrap()
    };
    let cap_i64_ptr = ctx
        .builder()
        .build_bit_cast(
            cap_ptr,
            ctx.context.ptr_type(Default::default()),
            "cap_i64_ptr",
        )
        .unwrap()
        .into_pointer_value();
    ctx.builder()
        .build_store(cap_i64_ptr, ctx.context.i64_type().const_int(cap, false))
        .unwrap();

    // 头部第三槽存储数据缓冲指针。
    let data_slot_ptr = unsafe {
        ctx.builder()
            .build_gep(
                ctx.context.i8_type(),
                ptr,
                &[ctx.context.i32_type().const_int(16, false)],
                "data_slot_ptr",
            )
            .unwrap()
    };
    let data_slot_i64_ptr = ctx
        .builder()
        .build_bit_cast(
            data_slot_ptr,
            ctx.context.ptr_type(Default::default()),
            "data_slot_i64_ptr",
        )
        .unwrap()
        .into_pointer_value();
    let data_addr = ctx
        .builder()
        .build_ptr_to_int(data_ptr, ctx.context.i64_type(), "data_addr")
        .unwrap();
    ctx.builder()
        .build_store(data_slot_i64_ptr, data_addr)
        .unwrap();

    // Phase 1: compile every element up front so the final element type
    // is known before any word is produced. The box decision must use the
    // array's element type (mixed literals degrade to dyn; a declared
    // `Array<dyn>` expectation forces dyn even for homogeneous literals).
    let expected_arr_elem = ctx
        .expected_expr_type()
        .or_else(|| ctx.current_return_type())
        .cloned()
        .map(|t| ctx.resolve_type_aliases(&t))
        .map(|t| match t {
            Type::Nullable(inner) => ctx.resolve_type_aliases(&inner),
            other => other,
        })
        .and_then(|t| match t {
            Type::Array(inner) => Some(ctx.resolve_type_aliases(&inner)),
            _ => None,
        });
    let mut compiled_elems: Vec<(BasicValueEnum<'ctx>, Type)> = Vec::new();
    let mut elem_ty: Option<Type> = None;
    for elem in elements {
        match elem {
            crate::parser::ast::ArrayElement::Expr(e) => {
                let val = compile_expr(ctx, e)?;
                match &elem_ty {
                    None => elem_ty = Some(val.ty.clone()),
                    Some(t) if *t != val.ty => elem_ty = Some(Type::Dynamic),
                    _ => {}
                }
                compiled_elems.push((val.value, val.ty.clone()));
            }
            crate::parser::ast::ArrayElement::Elision => {
                compiled_elems.push((
                    BasicValueEnum::IntValue(ctx.context.i64_type().const_int(0, false)),
                    Type::Int,
                ));
            }
            crate::parser::ast::ArrayElement::Spread(_) => {
                unreachable!("spread element should be handled by dynamic path")
            }
        }
    }
    let mut final_elem = elem_ty.unwrap_or(Type::Dynamic);
    if let Some(inner) = expected_arr_elem {
        // An empty literal has no element type of its own; adopt the
        // expected one (`let xs: Array<byte> = []`, `return []`).
        if compiled_elems.is_empty() || matches!(inner, Type::Dynamic | Type::Function { .. }) {
            final_elem = inner;
        }
    }
    let box_elems = matches!(final_elem, Type::Dynamic | Type::Function { .. });

    // Phase 2: store one 8-byte word per element. Under the box protocol a
    // dyn/function element word is the pointer to a heap-boxed 16-byte
    // struct; plain words (int/float-bits/raw pointer) only for concrete
    // element types.
    for (i, (value, val_ty)) in compiled_elems.iter().enumerate() {
        let offset = ctx.context.i32_type().const_int((i * 8) as u64, false);
        let elem_ptr = unsafe {
            ctx.builder()
                .build_gep(ctx.context.i8_type(), data_ptr, &[offset], "elem_ptr")
                .unwrap()
        };
        let i64_ptr = ctx
            .builder()
            .build_bit_cast(
                elem_ptr,
                ctx.context.ptr_type(Default::default()),
                "elem_i64_ptr",
            )
            .unwrap()
            .into_pointer_value();

        let stored_val: BasicValueEnum<'ctx> = if box_elems {
            match value {
                BasicValueEnum::StructValue(_) => build_box_struct_to_word(ctx, *value),
                other => {
                    let boxed = build_box_dynamic(ctx, *other, val_ty);
                    build_box_struct_to_word(ctx, boxed)
                }
            }
        } else {
            match value {
                BasicValueEnum::IntValue(v) => BasicValueEnum::IntValue(*v),
                BasicValueEnum::FloatValue(v) => ctx
                    .builder()
                    .build_bit_cast(*v, ctx.context.i64_type(), "f_to_i")
                    .unwrap()
                    .as_basic_value_enum(),
                BasicValueEnum::PointerValue(v) => ctx
                    .builder()
                    .build_ptr_to_int(*v, ctx.context.i64_type(), "ptr_to_i")
                    .unwrap()
                    .as_basic_value_enum(),
                // Word-struct elements (Dynamic/Function) cannot live in
                // an 8-byte slot raw; heap-box them and store the box
                // pointer word (get/index unbox symmetrically).
                BasicValueEnum::StructValue(_) => build_box_struct_to_word(ctx, *value),
                _ => *value,
            }
        };
        ctx.builder().build_store(i64_ptr, stored_val).unwrap();

        if super::builtins::is_gc_managed(val_ty) {
            if let BasicValueEnum::PointerValue(pv) = value {
                super::builtins::build_gc_write_barrier(ctx.builder(), ctx.module, data_ptr, *pv);
            }
        }
    }

    Ok(ExprResult::new(
        BasicValueEnum::PointerValue(ptr),
        Type::Array(Box::new(final_elem)),
    ))
}

/// 编译包含 spread 元素的数组字面量。
///
/// 使用 alloca 存储中间数组指针（push 可能导致数组重新分配），
/// 通过 __builtin_array_create + __builtin_array_push 动态构建数组。
fn compile_array_literal_with_spread<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    elements: &[crate::parser::ast::ArrayElement],
) -> Result<ExprResult<'ctx>, String> {
    let i8_ptr_ty = ctx.context.ptr_type(Default::default());
    let i64_ty = ctx.context.i64_type();

    let create_fn = ctx
        .module
        .get_function("__builtin_array_create")
        .ok_or("__builtin_array_create not declared")?;
    let push_fn = ctx
        .module
        .get_function("__builtin_array_push")
        .ok_or("__builtin_array_push not declared")?;
    let len_fn = ctx
        .module
        .get_function("__builtin_array_length")
        .ok_or("__builtin_array_length not declared")?;
    let get_fn = ctx
        .module
        .get_function("__builtin_array_get")
        .ok_or("__builtin_array_get not declared")?;

    // Box decision up front: a declared `Array<dyn>`/`Array<fn>` expectation
    // forces the box word protocol for every element, including words copied
    // out of concrete-typed spread sources.
    let expected_arr_elem = ctx
        .expected_expr_type()
        .or_else(|| ctx.current_return_type())
        .cloned()
        .map(|t| ctx.resolve_type_aliases(&t))
        .map(|t| match t {
            Type::Nullable(inner) => ctx.resolve_type_aliases(&inner),
            other => other,
        })
        .and_then(|t| match t {
            Type::Array(inner) => Some(ctx.resolve_type_aliases(&inner)),
            _ => None,
        });
    let box_elems = matches!(
        &expected_arr_elem,
        Some(inner) if matches!(inner, Type::Dynamic | Type::Function { .. })
    );

    // 使用 alloca 保存数组指针，push 可能触发重新分配，
    // 必须通过内存位置跟踪最新的指针值。
    let arr_ptr_alloca = ctx
        .builder()
        .build_alloca(i8_ptr_ty, "arr_ptr_alloca")
        .unwrap();
    let initial_arr = ctx
        .builder()
        .build_call(create_fn, &[], "arr_create")
        .unwrap()
        .try_as_basic_value()
        .unwrap_basic()
        .into_pointer_value();
    ctx.builder()
        .build_store(arr_ptr_alloca, initial_arr)
        .unwrap();

    let mut elem_ty: Option<Type> = None;

    for elem in elements {
        match elem {
            crate::parser::ast::ArrayElement::Expr(e) => {
                let val = compile_expr(ctx, e)?;
                match &elem_ty {
                    None => elem_ty = Some(val.ty.clone()),
                    Some(t) if *t != val.ty => elem_ty = Some(Type::Dynamic),
                    _ => {}
                }
                let word = if box_elems {
                    // Box protocol: a dyn destination stores box-pointer
                    // words, so heap-box every element (build_box_dynamic
                    // forwards already-boxed Dynamic structs as-is).
                    let boxed = build_box_dynamic(ctx, val.value, &val.ty);
                    build_box_struct_to_word(ctx, boxed).into_int_value()
                } else {
                    match val.value {
                        BasicValueEnum::IntValue(v) => v,
                        BasicValueEnum::FloatValue(v) => ctx
                            .builder()
                            .build_bit_cast(v, i64_ty, "f_to_i")
                            .unwrap()
                            .into_int_value(),
                        BasicValueEnum::PointerValue(v) => ctx
                            .builder()
                            .build_ptr_to_int(v, i64_ty, "ptr_to_i")
                            .unwrap(),
                        // Word-struct elements (Dynamic/Function) are heap-boxed;
                        // the stored word is the box pointer.
                        BasicValueEnum::StructValue(_) => {
                            build_box_struct_to_word(ctx, val.value).into_int_value()
                        }
                        _ => i64_ty.const_int(0, false),
                    }
                };
                // 从 alloca 加载当前数组指针
                let cur_arr = ctx
                    .builder()
                    .build_load(
                        ctx.context.ptr_type(Default::default()),
                        arr_ptr_alloca,
                        "cur_arr",
                    )
                    .unwrap()
                    .into_pointer_value();
                // push 返回（可能重新分配的）新数组指针
                let new_arr = ctx
                    .builder()
                    .build_call(push_fn, &[cur_arr.into(), word.into()], "arr_push")
                    .unwrap()
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_pointer_value();
                ctx.builder().build_store(arr_ptr_alloca, new_arr).unwrap();
            }
            crate::parser::ast::ArrayElement::Spread(expr) => {
                let spread_result = compile_expr(ctx, expr)?;
                let src_arr = match spread_result.value {
                    BasicValueEnum::PointerValue(p) => p,
                    _ => return Err("Spread element must be an array".to_string()),
                };
                // 源数组的元素类型决定复制时是否需要重新装箱。
                let spread_src_inner: Option<Type> = match &spread_result.ty {
                    Type::Array(inner) => Some((**inner).clone()),
                    _ => None,
                };
                // 从 spread 源数组类型推断元素类型
                if let Some(inner) = &spread_src_inner {
                    match &elem_ty {
                        None => elem_ty = Some(inner.clone()),
                        Some(t) if *t != *inner => elem_ty = Some(Type::Dynamic),
                        _ => {}
                    }
                } else {
                    elem_ty = Some(Type::Dynamic);
                }
                // 获取 spread 源数组长度
                let src_len = ctx
                    .builder()
                    .build_call(len_fn, &[src_arr.into()], "spread_len")
                    .unwrap()
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_int_value();

                let func = ctx.current_function().ok_or("No current function")?;
                let idx_ptr = ctx.builder().build_alloca(i64_ty, "spread_idx").unwrap();
                ctx.builder()
                    .build_store(idx_ptr, i64_ty.const_int(0, false))
                    .unwrap();

                let loop_bb = ctx.context.append_basic_block(func, "spread_loop");
                let body_bb = ctx.context.append_basic_block(func, "spread_body");
                let done_bb = ctx.context.append_basic_block(func, "spread_done");

                ctx.builder().build_unconditional_branch(loop_bb).unwrap();

                // 循环条件：idx < src_len
                ctx.builder().position_at_end(loop_bb);
                let idx = ctx
                    .builder()
                    .build_load(ctx.context.i64_type(), idx_ptr, "idx")
                    .unwrap()
                    .into_int_value();
                let at_end = ctx
                    .builder()
                    .build_int_compare(inkwell::IntPredicate::UGE, idx, src_len, "spread_at_end")
                    .unwrap();
                ctx.builder()
                    .build_conditional_branch(at_end, done_bb, body_bb)
                    .unwrap();

                // 循环体：取元素并 push
                ctx.builder().position_at_end(body_bb);
                let idx_val = ctx
                    .builder()
                    .build_load(i64_ty, idx_ptr, "idx_val")
                    .unwrap()
                    .into_int_value();
                let spread_elem = ctx
                    .builder()
                    .build_call(get_fn, &[src_arr.into(), idx_val.into()], "spread_elem")
                    .unwrap()
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_int_value();
                // Box protocol: a dyn destination stores box-pointer words.
                // Concrete-typed sources contribute raw words, so re-box each
                // fetched word per the source element type; dyn/function
                // sources already store box words and pass through.
                let spread_word = if box_elems {
                    let src_is_word_proto = matches!(
                        spread_src_inner.as_ref(),
                        None | Some(Type::Dynamic) | Some(Type::Function { .. })
                    );
                    if src_is_word_proto {
                        spread_elem
                    } else {
                        let src_ty = spread_src_inner.clone().unwrap_or(Type::Dynamic);
                        let raw: BasicValueEnum<'ctx> = match &src_ty {
                            Type::Float => ctx
                                .builder()
                                .build_bit_cast(spread_elem, ctx.context.f64_type(), "sp_f")
                                .unwrap(),
                            Type::Bool => ctx
                                .builder()
                                .build_int_truncate(spread_elem, ctx.context.bool_type(), "sp_b")
                                .unwrap()
                                .into(),
                            Type::Int => spread_elem.into(),
                            _ => ctx
                                .builder()
                                .build_int_to_ptr(spread_elem, i8_ptr_ty, "sp_p")
                                .unwrap()
                                .into(),
                        };
                        let boxed = build_box_dynamic(ctx, raw, &src_ty);
                        build_box_struct_to_word(ctx, boxed).into_int_value()
                    }
                } else {
                    spread_elem
                };
                // 从 alloca 加载当前数组指针
                let cur_arr = ctx
                    .builder()
                    .build_load(
                        ctx.context.ptr_type(Default::default()),
                        arr_ptr_alloca,
                        "cur_arr_sp",
                    )
                    .unwrap()
                    .into_pointer_value();
                let new_arr = ctx
                    .builder()
                    .build_call(
                        push_fn,
                        &[cur_arr.into(), spread_word.into()],
                        "spread_push",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_pointer_value();
                ctx.builder().build_store(arr_ptr_alloca, new_arr).unwrap();
                // 递增索引
                let next_idx = ctx
                    .builder()
                    .build_int_add(idx_val, i64_ty.const_int(1, false), "next_idx")
                    .unwrap();
                ctx.builder().build_store(idx_ptr, next_idx).unwrap();
                ctx.builder().build_unconditional_branch(loop_bb).unwrap();

                // 循环结束
                ctx.builder().position_at_end(done_bb);
            }
            crate::parser::ast::ArrayElement::Elision => {
                let cur_arr = ctx
                    .builder()
                    .build_load(
                        ctx.context.ptr_type(Default::default()),
                        arr_ptr_alloca,
                        "cur_arr_el",
                    )
                    .unwrap()
                    .into_pointer_value();
                let new_arr = ctx
                    .builder()
                    .build_call(
                        push_fn,
                        &[cur_arr.into(), i64_ty.const_int(0, false).into()],
                        "elision_push",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_pointer_value();
                ctx.builder().build_store(arr_ptr_alloca, new_arr).unwrap();
            }
        }
    }

    // 从 alloca 加载最终的数组指针
    let final_arr = ctx
        .builder()
        .build_load(
            ctx.context.ptr_type(Default::default()),
            arr_ptr_alloca,
            "final_arr",
        )
        .unwrap()
        .into_pointer_value();

    Ok(ExprResult::new(
        BasicValueEnum::PointerValue(final_arr),
        Type::Array(Box::new(match &expected_arr_elem {
            Some(inner) if box_elems => inner.clone(),
            _ => elem_ty.unwrap_or(Type::Dynamic),
        })),
    ))
}

fn compile_rest_args_to_array<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    rest_args: &[inkwell::values::BasicMetadataValueEnum<'ctx>],
    rest_types: &[Type],
    elem_ty: &Type,
) -> Result<inkwell::values::PointerValue<'ctx>, String> {
    let len = rest_args.len() as u64;
    let cap = if len == 0 { 4 } else { len };
    // Box protocol: every Array<dyn> element slot is one 8-byte word
    // holding the pointer to a heap-boxed 16-byte Dynamic struct — the
    // same layout `__builtin_array_push` produces, so get/index read it
    // back symmetrically. Inline 16-byte slots would be misread as words.
    let total_size = ctx.context.i64_type().const_int(24, false);
    let ptr = GcAllocFn::for_mode(ctx.gc_mode).emit(ctx.builder(), ctx.module, total_size);

    // 独立元素缓冲：头指针保持稳定，扩容只替换数据缓冲。
    let data_total = ctx.context.i64_type().const_int(cap * 8, false);
    let data_ptr = GcAllocFn::for_mode(ctx.gc_mode).emit(ctx.builder(), ctx.module, data_total);

    let len_ptr = ctx
        .builder()
        .build_bit_cast(ptr, ctx.context.ptr_type(Default::default()), "len_ptr")
        .unwrap()
        .into_pointer_value();
    ctx.builder()
        .build_store(len_ptr, ctx.context.i64_type().const_int(len, false))
        .unwrap();

    let cap_ptr = unsafe {
        ctx.builder()
            .build_gep(
                ctx.context.i8_type(),
                ptr,
                &[ctx.context.i32_type().const_int(8, false)],
                "cap_ptr",
            )
            .unwrap()
    };
    let cap_i64_ptr = ctx
        .builder()
        .build_bit_cast(
            cap_ptr,
            ctx.context.ptr_type(Default::default()),
            "cap_i64_ptr",
        )
        .unwrap()
        .into_pointer_value();
    ctx.builder()
        .build_store(cap_i64_ptr, ctx.context.i64_type().const_int(cap, false))
        .unwrap();

    // 头部第三槽存储数据缓冲指针。
    let data_slot_ptr = unsafe {
        ctx.builder()
            .build_gep(
                ctx.context.i8_type(),
                ptr,
                &[ctx.context.i32_type().const_int(16, false)],
                "rest_data_slot",
            )
            .unwrap()
    };
    let data_slot_i64_ptr = ctx
        .builder()
        .build_bit_cast(
            data_slot_ptr,
            ctx.context.ptr_type(Default::default()),
            "rest_data_slot_i64",
        )
        .unwrap()
        .into_pointer_value();
    let data_addr = ctx
        .builder()
        .build_ptr_to_int(data_ptr, ctx.context.i64_type(), "rest_data_addr")
        .unwrap();
    ctx.builder()
        .build_store(data_slot_i64_ptr, data_addr)
        .unwrap();

    if matches!(elem_ty, Type::Dynamic) {
        // Dynamic rest args: heap-box each value as a Dynamic struct and
        // store the box-pointer word (8-byte stride).
        for (i, (val, arg_ty)) in rest_args.iter().zip(rest_types.iter()).enumerate() {
            let offset = ctx.context.i32_type().const_int((i * 8) as u64, false);
            let raw_elem_ptr = unsafe {
                ctx.builder()
                    .build_gep(ctx.context.i8_type(), data_ptr, &[offset], "dyn_elem_ptr")
                    .unwrap()
            };
            let i64_ptr = ctx
                .builder()
                .build_bit_cast(
                    raw_elem_ptr,
                    ctx.context.ptr_type(Default::default()),
                    "dyn_elem_i64_ptr",
                )
                .unwrap()
                .into_pointer_value();
            // Convert BasicMetadataValueEnum back to BasicValueEnum for boxing
            let basic_val: BasicValueEnum<'ctx> = match val {
                inkwell::values::BasicMetadataValueEnum::IntValue(v) => {
                    BasicValueEnum::IntValue(*v)
                }
                inkwell::values::BasicMetadataValueEnum::FloatValue(v) => {
                    BasicValueEnum::FloatValue(*v)
                }
                inkwell::values::BasicMetadataValueEnum::PointerValue(v) => {
                    BasicValueEnum::PointerValue(*v)
                }
                inkwell::values::BasicMetadataValueEnum::StructValue(s) => {
                    // Spread-out Dynamic value: 16-byte `{i64, ptr}` struct.
                    BasicValueEnum::StructValue(*s)
                }
                _ => continue,
            };
            let dyn_struct = build_box_dynamic(ctx, basic_val, arg_ty);
            let word = build_box_struct_to_word(ctx, dyn_struct);
            ctx.builder()
                .build_store(i64_ptr, word.into_int_value())
                .unwrap();
        }
    } else {
        // Non-Dynamic: store as i64 values (legacy behavior)
        for (i, val) in rest_args.iter().enumerate() {
            let offset = ctx.context.i32_type().const_int((i * 8) as u64, false);
            let elem_ptr = unsafe {
                ctx.builder()
                    .build_gep(ctx.context.i8_type(), data_ptr, &[offset], "elem_ptr")
                    .unwrap()
            };
            let i64_ptr = ctx
                .builder()
                .build_bit_cast(
                    elem_ptr,
                    ctx.context.ptr_type(Default::default()),
                    "elem_i64_ptr",
                )
                .unwrap()
                .into_pointer_value();

            let stored_val = match val {
                inkwell::values::BasicMetadataValueEnum::IntValue(v) => v.as_basic_value_enum(),
                inkwell::values::BasicMetadataValueEnum::FloatValue(v) => ctx
                    .builder()
                    .build_bit_cast(*v, ctx.context.i64_type(), "f_to_i")
                    .unwrap()
                    .as_basic_value_enum(),
                inkwell::values::BasicMetadataValueEnum::PointerValue(v) => ctx
                    .builder()
                    .build_ptr_to_int(*v, ctx.context.i64_type(), "ptr_to_i")
                    .unwrap()
                    .as_basic_value_enum(),
                _ => continue,
            };
            ctx.builder().build_store(i64_ptr, stored_val).unwrap();
        }
    }

    Ok(ptr)
}

/// 编译对象字面量，支持 spread 属性（`{...obj, extra}`）。
///
/// 算法：
/// 1. 预先编译所有 spread 源表达式并收集唯一字段名列表；
/// 2. 根据唯一字段数分配目标对象内存；
/// 3. 按属性顺序写入字段值，spread 字段从源对象拷贝，后续属性覆盖同名前面属性。
fn compile_object_literal<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    properties: &[crate::parser::ast::ObjectProperty],
) -> Result<ExprResult<'ctx>, String> {
    // 第一步：预先编译所有 spread 源表达式，用于后续字段名收集和值拷贝
    let mut spread_sources: Vec<(
        usize,
        ExprResult<'ctx>,
        Vec<crate::typechecker::types::ObjectField>,
    )> = Vec::new();
    for (i, prop) in properties.iter().enumerate() {
        if let crate::parser::ast::ObjectProperty::Spread(expr) = prop {
            let result = compile_expr(ctx, expr)?;
            let src_fields = match &result.ty {
                Type::Object(fields) => fields.clone(),
                _ => vec![],
            };
            spread_sources.push((i, result, src_fields));
        }
    }

    // 第二步：收集唯一字段名及其顺序（后面的属性覆盖前面的同名属性，字段位置不变）
    let mut all_field_names: Vec<String> = Vec::new();
    let mut spread_idx = 0usize;
    for prop in properties {
        match prop {
            crate::parser::ast::ObjectProperty::Property { key, .. } => {
                let name = match key {
                    crate::parser::ast::PropertyName::Ident(n) => n.clone(),
                    crate::parser::ast::PropertyName::String(n) => n.clone(),
                    crate::parser::ast::PropertyName::Number(n) => format!("{}", n),
                    crate::parser::ast::PropertyName::Computed(_) => "[computed]".to_string(),
                };
                if !all_field_names.contains(&name) {
                    all_field_names.push(name);
                }
            }
            crate::parser::ast::ObjectProperty::Shorthand(name) => {
                if !all_field_names.contains(name) {
                    all_field_names.push(name.clone());
                }
            }
            crate::parser::ast::ObjectProperty::Spread(_) => {
                if spread_idx < spread_sources.len() {
                    for sf in &spread_sources[spread_idx].2 {
                        // 跳过占位符字段（来自未解析的 spread）
                        if sf.name == "..." {
                            continue;
                        }
                        if !all_field_names.contains(&sf.name) {
                            all_field_names.push(sf.name.clone());
                        }
                    }
                    spread_idx += 1;
                }
            }
            crate::parser::ast::ObjectProperty::ComputedProperty { .. } => {
                let name = "[computed]".to_string();
                if !all_field_names.contains(&name) {
                    all_field_names.push(name);
                }
            }
        }
    }

    let field_count = all_field_names.len() as u64;
    if field_count == 0 {
        // 空对象：分配最小内存块
        let ptr = GcAllocFn::for_mode(ctx.gc_mode).emit(
            ctx.builder(),
            ctx.module,
            ctx.context.i64_type().const_int(8, false),
        );
        return Ok(ExprResult::new(
            BasicValueEnum::PointerValue(ptr),
            Type::Object(vec![]),
        ));
    }

    // 第三步：根据唯一字段数分配目标对象
    let total_size = ctx.context.i64_type().const_int(field_count * 8, false);
    let ptr = GcAllocFn::for_mode(ctx.gc_mode).emit(ctx.builder(), ctx.module, total_size);

    // 字段名 → 槽位偏移索引
    let name_to_slot: std::collections::HashMap<String, usize> = all_field_names
        .iter()
        .enumerate()
        .map(|(idx, name)| (name.clone(), idx))
        .collect();

    // 第四步：按属性顺序写入字段值，并记录每个字段最终的类型
    let mut field_types: std::collections::HashMap<String, Type> = std::collections::HashMap::new();
    let mut spread_iter_idx = 0usize;
    for prop in properties {
        match prop {
            crate::parser::ast::ObjectProperty::Property { key, value } => {
                let val = compile_expr(ctx, value)?;
                let name = match key {
                    crate::parser::ast::PropertyName::Ident(n) => n.clone(),
                    crate::parser::ast::PropertyName::String(n) => n.clone(),
                    crate::parser::ast::PropertyName::Number(n) => format!("{}", n),
                    crate::parser::ast::PropertyName::Computed(_) => "[computed]".to_string(),
                };
                let slot = name_to_slot[&name];
                store_value_at_slot(ctx, ptr, slot, &val)?;
                field_types.insert(name.clone(), val.ty.clone());
            }
            crate::parser::ast::ObjectProperty::Shorthand(name) => {
                let val = compile_expr(ctx, &Expr::Identifier(name.clone()))?;
                let slot = name_to_slot[name];
                store_value_at_slot(ctx, ptr, slot, &val)?;
                field_types.insert(name.clone(), val.ty.clone());
            }
            crate::parser::ast::ObjectProperty::Spread(_) => {
                if spread_iter_idx < spread_sources.len() {
                    let (_, ref spread_result, _) = spread_sources[spread_iter_idx];
                    spread_iter_idx += 1;
                    if let Type::Object(sobj_fields) = &spread_result.ty {
                        let src_ptr = match spread_result.value {
                            BasicValueEnum::PointerValue(p) => p,
                            _ => {
                                return Err(
                                    "Spread source must evaluate to an object pointer".to_string()
                                )
                            }
                        };
                        // 从源对象按字段名拷贝到目标对象对应槽位
                        for (src_idx, sf) in sobj_fields.iter().enumerate() {
                            if sf.name == "..." {
                                continue;
                            }
                            if let Some(&target_slot) = name_to_slot.get(&sf.name) {
                                // 记录该字段类型（后续 spread 或属性会覆盖）
                                field_types.insert(sf.name.clone(), sf.ty.clone());
                                // 从源对象读取字段值
                                let src_offset = ctx
                                    .context
                                    .i32_type()
                                    .const_int((src_idx * 8) as u64, false);
                                let src_field_ptr = unsafe {
                                    ctx.builder()
                                        .build_gep(
                                            ctx.context.i8_type(),
                                            src_ptr,
                                            &[src_offset],
                                            "spread_src_field",
                                        )
                                        .unwrap()
                                };
                                let src_i64_ptr = ctx
                                    .builder()
                                    .build_bit_cast(
                                        src_field_ptr,
                                        ctx.context.ptr_type(Default::default()),
                                        "spread_src_i64_ptr",
                                    )
                                    .unwrap()
                                    .into_pointer_value();
                                let loaded = ctx
                                    .builder()
                                    .build_load(ctx.context.i64_type(), src_i64_ptr, "spread_val")
                                    .unwrap();

                                // 写入目标对象对应槽位
                                let tgt_offset = ctx
                                    .context
                                    .i32_type()
                                    .const_int((target_slot * 8) as u64, false);
                                let tgt_field_ptr = unsafe {
                                    ctx.builder()
                                        .build_gep(
                                            ctx.context.i8_type(),
                                            ptr,
                                            &[tgt_offset],
                                            "spread_tgt_field",
                                        )
                                        .unwrap()
                                };
                                let tgt_i64_ptr = ctx
                                    .builder()
                                    .build_bit_cast(
                                        tgt_field_ptr,
                                        ctx.context.ptr_type(Default::default()),
                                        "spread_tgt_i64_ptr",
                                    )
                                    .unwrap()
                                    .into_pointer_value();
                                ctx.builder().build_store(tgt_i64_ptr, loaded).unwrap();

                                // 为 GC 管理的指针类型写屏障
                                if super::builtins::is_gc_managed(&sf.ty) {
                                    if let BasicValueEnum::PointerValue(pv) = loaded {
                                        super::builtins::build_gc_write_barrier(
                                            ctx.builder(),
                                            ctx.module,
                                            ptr,
                                            pv,
                                        );
                                    }
                                }
                            }
                        }
                    }
                    // 非 Object 类型的 spread 源：无法静态展开，忽略
                }
            }
            crate::parser::ast::ObjectProperty::ComputedProperty { value, .. } => {
                // ComputedProperty 字段名为 "[computed]"，写入对应槽位
                let val = compile_expr(ctx, value)?;
                if let Some(&slot) = name_to_slot.get("[computed]") {
                    store_value_at_slot(ctx, ptr, slot, &val)?;
                }
                field_types.insert("[computed]".to_string(), val.ty.clone());
            }
        }
    }

    // 按 all_field_names 顺序构建最终字段列表（保证与内存布局一致）
    let fields: Vec<crate::typechecker::types::ObjectField> = all_field_names
        .iter()
        .map(|name| crate::typechecker::types::ObjectField {
            name: name.clone(),
            ty: field_types.get(name).cloned().unwrap_or(Type::Dynamic),
            optional: false,
        })
        .collect();

    Ok(ExprResult::new(
        BasicValueEnum::PointerValue(ptr),
        Type::Object(fields),
    ))
}

/// 将 `ExprResult` 的值存储到对象 `ptr` 的第 `slot` 个字段（偏移 `slot * 8` 字节）。
fn store_value_at_slot<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    ptr: inkwell::values::PointerValue<'ctx>,
    slot: usize,
    val: &ExprResult<'ctx>,
) -> Result<(), String> {
    let offset = ctx.context.i32_type().const_int((slot * 8) as u64, false);
    let field_ptr = unsafe {
        ctx.builder()
            .build_gep(ctx.context.i8_type(), ptr, &[offset], "field_ptr")
            .unwrap()
    };
    let i64_ptr = ctx
        .builder()
        .build_bit_cast(
            field_ptr,
            ctx.context.ptr_type(Default::default()),
            "field_i64_ptr",
        )
        .unwrap()
        .into_pointer_value();

    let stored_val = match val.value {
        BasicValueEnum::IntValue(v) => v.as_basic_value_enum(),
        BasicValueEnum::FloatValue(v) => ctx
            .builder()
            .build_bit_cast(v, ctx.context.i64_type(), "f_to_i")
            .unwrap()
            .as_basic_value_enum(),
        BasicValueEnum::PointerValue(v) => ctx
            .builder()
            .build_ptr_to_int(v, ctx.context.i64_type(), "ptr_to_i")
            .unwrap()
            .as_basic_value_enum(),
        _ => val.value,
    };
    ctx.builder().build_store(i64_ptr, stored_val).unwrap();

    if super::builtins::is_gc_managed(&val.ty) {
        if let BasicValueEnum::PointerValue(pv) = val.value {
            super::builtins::build_gc_write_barrier(ctx.builder(), ctx.module, ptr, pv);
        }
    }
    Ok(())
}

/// Compile `Thread.spawn(closure[, arg])` as a language-level builtin.
///
/// Emits a per-closure trampoline matching the runtime's
/// `extern "C" fn(usize)` thread-entry ABI, calls `__thread_spawn`,
/// and returns a `Thread` instance whose `_handle` field holds the
/// registry handle used by `join` / `detach` / `isFinished`.
///
/// Captures: closures compiled inside another function resolve outer
/// locals to the enclosing frame's allocas (cross-function references).
/// Those slots are invalid from another OS thread, so every outer local
/// the closure actually touches is copied by value into a heap context
/// block at spawn time, and the closure's loads/stores are patched to
/// use that block (spec: closures capture by value, deep copy).
/// Which stdlib class a `spawn(closure)` call targets; both share the
/// same `{fnptr, env}` cell + trampoline lowering and differ only in the
/// runtime entry (`__thread_spawn` vs `__fiber_spawn`) and the returned
/// handle class.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SpawnKind {
    Thread,
    Fiber,
}

impl SpawnKind {
    fn spawn_fn_name(self) -> &'static str {
        match self {
            SpawnKind::Thread => "__thread_spawn",
            SpawnKind::Fiber => "__fiber_spawn",
        }
    }

    fn class_name(self) -> &'static str {
        match self {
            SpawnKind::Thread => "Thread",
            SpawnKind::Fiber => "Fiber",
        }
    }
}

fn compile_spawn_builtin<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    args: &[crate::parser::ast::Argument],
    kind: SpawnKind,
) -> Result<ExprResult<'ctx>, String> {
    let closure_expr = match args.first() {
        Some(crate::parser::ast::Argument::Expr(e)) => e,
        _ => return Err("Thread.spawn expects a closure argument".to_string()),
    };
    // Optional explicit user argument (`Thread.spawn(fn, arg)`), carried
    // to the closure via the context block header.
    let user_arg_expr = match args.get(1) {
        Some(crate::parser::ast::Argument::Expr(e)) => Some(e),
        _ => None,
    };

    let i64_ty = ctx.context.i64_type();
    let ptr_ty = ctx.context.ptr_type(Default::default());
    let dyn_ty = ruyi_type_to_llvm(ctx.context, &Type::Dynamic);

    // Compile the closure first so its function and registered type
    // signature exist before the trampoline is emitted. Closure values
    // are `{ fnptr, env }` structs; captures already live in the env
    // block built at the creation site, so no IR patching happens here.
    let closure_result = compile_expr(ctx, closure_expr)?;
    let fn_value_ty = super::types::fn_value_type(ctx.context);
    let fn_struct = match closure_result.value {
        BasicValueEnum::StructValue(s) => s,
        other => {
            // Legacy bare-pointer producers (e.g. async arrows): treat
            // as an env-less function value.
            let s = fn_value_ty
                .const_zero()
                .as_basic_value_enum()
                .into_struct_value();
            let s = ctx
                .builder()
                .build_insert_value(s, other.into_pointer_value(), 0, "spawn_fnptr")
                .unwrap()
                .into_struct_value();
            let s = ctx
                .builder()
                .build_insert_value(s, ptr_ty.const_null(), 1, "spawn_env")
                .unwrap()
                .into_struct_value();
            s
        }
    };
    // Named/arrow closures carry their real signature in the ExprResult;
    // named `fn` expressions report an empty placeholder, so fall back
    // to the signature registered under the callee's own name.
    let mut param_count = match &closure_result.ty {
        Type::Function { params, .. } => params.len(),
        _ => 1,
    };
    let mut closure_ret = match &closure_result.ty {
        Type::Function { return_type, .. } => (**return_type).clone(),
        _ => Type::Void,
    };
    let fn_key = match closure_expr.as_ref() {
        Expr::Identifier(n) => n.clone(),
        _ => String::new(),
    };
    if let Some(Type::Function {
        params,
        return_type,
    }) = ctx.function_types.get(&fn_key)
    {
        param_count = params.len();
        closure_ret = (**return_type).clone();
    }

    // ── User arg: box into the Dynamic `{tag, data}` convention so
    // the closure receives it unchanged despite the arg channel being
    // repurposed to carry the context pointer. ────────────────────
    let boxed_arg: Option<(
        inkwell::values::IntValue<'ctx>,
        inkwell::values::PointerValue<'ctx>,
    )> = match user_arg_expr {
        Some(e) => {
            let r = compile_expr(ctx, e)?;
            let (tag, data) = match &r.ty {
                Type::Dynamic => {
                    let s = r.value.into_struct_value();
                    let t = ctx
                        .builder()
                        .build_extract_value(s, 0, "ua_tag")
                        .unwrap()
                        .into_int_value();
                    let d = ctx
                        .builder()
                        .build_extract_value(s, 1, "ua_data")
                        .unwrap()
                        .into_pointer_value();
                    (t, d)
                }
                Type::Int => (
                    r.value.into_int_value(),
                    ctx.builder()
                        .build_int_to_ptr(i64_ty.const_int(1, false), ptr_ty, "ua_marker")
                        .unwrap(),
                ),
                Type::Float => {
                    let bits = ctx
                        .builder()
                        .build_bit_cast(r.value.into_float_value(), i64_ty, "ua_fbits")
                        .unwrap()
                        .into_int_value();
                    let marker = ctx
                        .builder()
                        .build_int_to_ptr(i64_ty.const_int(2, false), ptr_ty, "ua_data")
                        .unwrap();
                    (bits, marker)
                }
                Type::Bool => {
                    let z = ctx
                        .builder()
                        .build_int_z_extend(r.value.into_int_value(), i64_ty, "ua_b")
                        .unwrap();
                    let marker = ctx
                        .builder()
                        .build_int_to_ptr(i64_ty.const_int(3, false), ptr_ty, "ua_data")
                        .unwrap();
                    (z, marker)
                }
                Type::String => (i64_ty.const_int(0, false), r.value.into_pointer_value()),
                Type::Named(cn, _) => {
                    let tid = ctx.type_ids.get(cn).copied().unwrap_or(0);
                    (i64_ty.const_int(tid, false), r.value.into_pointer_value())
                }
                _ => (
                    i64_ty.const_int(0, false),
                    ctx.context.ptr_type(Default::default()).const_null(),
                ),
            };
            Some((tag, data))
        }
        None => None,
    };

    // ── Spawn cell: [0..16) the `{ fnptr, env }` value; [16..32) the
    // boxed user arg Dynamic (zeroed when absent). The cell pointer
    // travels through the single `extern "C" fn(usize)` thread-entry
    // word. ─────────────────────────────────────────────────────────
    let cell_size = ctx.context.i64_type().const_int(32, false);
    let cell = GcAllocFn::for_mode(ctx.gc_mode).emit(ctx.builder(), ctx.module, cell_size);
    ctx.builder().build_store(cell, fn_struct).unwrap();
    let dyn_slot = unsafe {
        ctx.builder()
            .build_gep(
                ctx.context.i8_type(),
                cell,
                &[ctx.context.i32_type().const_int(16, false)],
                "spawn_arg_slot",
            )
            .unwrap()
    };
    let boxed_dyn: BasicValueEnum<'ctx> = match boxed_arg {
        Some((tag, data)) => {
            let mut s = dyn_ty.into_struct_type().const_zero();
            s = ctx
                .builder()
                .build_insert_value(s, tag, 0, "spawn_arg_tag")
                .unwrap()
                .into_struct_value();
            s = ctx
                .builder()
                .build_insert_value(s, data, 1, "spawn_arg_data")
                .unwrap()
                .into_struct_value();
            s.into()
        }
        None => dyn_ty.into_struct_type().const_zero().into(),
    };
    ctx.builder().build_store(dyn_slot, boxed_dyn).unwrap();
    let ctx_ptr_val = ctx
        .builder()
        .build_ptr_to_int(cell, i64_ty, "spawn_cell_word")
        .unwrap();

    // Trampoline: extern "C" fn(usize) that forwards to the closure.
    // The spawn word (context block pointer) is boxed as a Dynamic
    // `{0, inttoptr(word)}` and passed as the closure's first parameter;
    // the closure's patched entry recovers the real user arg and
    // captures from the block.
    let trampoline_name = format!("__ruyi_thread_trampoline_{}", ctx.anon_counter);
    ctx.anon_counter += 1;
    let trampoline_fn = ctx.module.add_function(
        &trampoline_name,
        ctx.context.void_type().fn_type(&[i64_ty.into()], false),
        None,
    );
    trampoline_fn.set_linkage(inkwell::module::Linkage::Internal);
    // Unwinding must be able to cross the trampoline frame when the
    // closure throws.
    let lp_gen = LandingPadGenerator::new(ctx.context, ctx.module, ctx.builder());
    trampoline_fn.set_personality_function(lp_gen.get_personality_function());

    // The context word only needs forwarding when the closure declares
    // a parameter (zero-parameter closures take no user arg). The
    // trampoline passes the closure's real first parameter type: scalar
    // parameters get the boxed Dynamic unboxed, Dynamic parameters
    // receive the raw `{value, marker}` struct.
    let needs_arg = param_count > 0;
    let first_param_ty = match &closure_result.ty {
        Type::Function { params, .. } => params.first().cloned().unwrap_or(Type::Dynamic),
        _ => Type::Dynamic,
    };
    let closure_params: Vec<Type> = if needs_arg {
        vec![first_param_ty.clone()]
    } else {
        Vec::new()
    };

    let caller_bb = ctx.builder().get_insert_block();
    let tramp_entry = ctx.context.append_basic_block(trampoline_fn, "entry");
    ctx.builder().position_at_end(tramp_entry);
    let cell_word = trampoline_fn.get_nth_param(0).unwrap().into_int_value();
    let cell_ptr = ctx
        .builder()
        .build_int_to_ptr(cell_word, ptr_ty, "spawn_cell_ptr")
        .unwrap();
    let fn_value = ctx
        .builder()
        .build_load(
            super::types::fn_value_type(ctx.context),
            cell_ptr,
            "spawn_fn_value",
        )
        .unwrap();
    let call_args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> = if needs_arg {
        let dyn_slot = unsafe {
            ctx.builder()
                .build_gep(
                    ctx.context.i8_type(),
                    cell_ptr,
                    &[ctx.context.i32_type().const_int(16, false)],
                    "spawn_arg_slot",
                )
                .unwrap()
        };
        let boxed = ctx
            .builder()
            .build_load(dyn_ty, dyn_slot, "spawn_arg")
            .unwrap();
        let arg_meta: inkwell::values::BasicMetadataValueEnum<'ctx> = match (boxed, &first_param_ty)
        {
            (BasicValueEnum::StructValue(sv), ty) if *ty != Type::Dynamic => match ty {
                Type::Int => {
                    let raw = ctx
                        .builder()
                        .build_extract_value(sv, 0, "spawn_arg_i")
                        .unwrap()
                        .into_int_value();
                    let tgt = super::types::ruyi_type_to_llvm(ctx.context, ty).into_int_type();
                    if raw.get_type() == tgt {
                        raw.into()
                    } else {
                        ctx.builder()
                            .build_int_cast(raw, tgt, "spawn_arg_ic")
                            .unwrap()
                            .into()
                    }
                }
                Type::Float => {
                    let raw = ctx
                        .builder()
                        .build_extract_value(sv, 0, "spawn_arg_f")
                        .unwrap()
                        .into_int_value();
                    ctx.builder()
                        .build_bit_cast(
                            raw,
                            super::types::ruyi_type_to_llvm(ctx.context, ty).into_float_type(),
                            "spawn_arg_fc",
                        )
                        .unwrap()
                        .into()
                }
                Type::Bool => {
                    let raw = ctx
                        .builder()
                        .build_extract_value(sv, 0, "spawn_arg_b")
                        .unwrap()
                        .into_int_value();
                    ctx.builder()
                        .build_int_cast(
                            raw,
                            super::types::ruyi_type_to_llvm(ctx.context, ty).into_int_type(),
                            "spawn_arg_bc",
                        )
                        .unwrap()
                        .into()
                }
                Type::String | Type::Named(..) => ctx
                    .builder()
                    .build_extract_value(sv, 1, "spawn_arg_p")
                    .unwrap()
                    .into_pointer_value()
                    .into(),
                _ => boxed.into(),
            },
            _ => boxed.into(),
        };
        vec![arg_meta]
    } else {
        Vec::new()
    };
    emit_fn_value_call(
        ctx,
        fn_value,
        &closure_params,
        &closure_ret,
        call_args,
        "closure_call",
    )?;
    ctx.builder().build_return(None).unwrap();
    if let Some(bb) = caller_bb {
        ctx.builder().position_at_end(bb);
    }

    // handle = __thread_spawn(trampoline, context_word)
    let spawn_fn = match ctx.module.get_function(kind.spawn_fn_name()) {
        Some(f) => f,
        None => {
            let fty = i64_ty.fn_type(&[ptr_ty.into(), ptr_ty.into()], false);
            ctx.module.add_function(kind.spawn_fn_name(), fty, None)
        }
    };
    let trampoline_ptr = trampoline_fn.as_global_value().as_pointer_value();
    let ctx_word_ptr = ctx
        .builder()
        .build_int_to_ptr(ctx_ptr_val, ptr_ty, "ctx_word_ptr")
        .unwrap();
    let handle = ctx
        .builder()
        .build_call(
            spawn_fn,
            &[trampoline_ptr.into(), ctx_word_ptr.into()],
            "thread_handle",
        )
        .unwrap()
        .try_as_basic_value()
        .unwrap_basic()
        .into_int_value();

    // Allocate the Thread instance (typeid + zeroed fields via the
    // standard `new` path) and store the handle into `_handle`.
    let class_name = kind.class_name();
    let new_expr = crate::parser::ast::Expr::Identifier(class_name.to_string());
    let new_result = compile_new(ctx, &new_expr, &[])?;
    let thread_obj = new_result.value.into_pointer_value();
    let (handle_ptr, _) = class_field_access(
        ctx,
        thread_obj,
        class_name,
        "_handle",
        &Type::Named(class_name.to_string(), Vec::new()),
    )?;
    ctx.builder().build_store(handle_ptr, handle).unwrap();

    Ok(ExprResult::new(
        BasicValueEnum::PointerValue(thread_obj),
        Type::Named(class_name.to_string(), Vec::new()),
    ))
}

/// Emit stores for a class's instance field initializers (`field: T = expr`).
///
/// Called right after allocation and before the constructor, so an explicit
/// assignment in the constructor still wins over the declared default. The
/// initializers see the fresh instance as `self`.
fn emit_field_initializers<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    class_name: &str,
    ptr: inkwell::values::PointerValue<'ctx>,
) -> Result<(), String> {
    let inits = match ctx.class_field_inits.get(class_name) {
        Some(v) => v.clone(),
        None => return Ok(()),
    };
    let saved_self = ctx.variables.insert(
        "self".to_string(),
        (ptr, Type::Named(class_name.to_string(), vec![])),
    );
    let result = (|| {
        for (field_name, init_expr) in &inits {
            let declared_ty = ctx
                .class_fields
                .get(class_name)
                .and_then(|fs| {
                    fs.iter()
                        .find(|(n, _)| n == field_name)
                        .map(|(_, t)| t.clone())
                })
                .ok_or_else(|| format!("Unknown field: {} in class {}", field_name, class_name))?;
            let (field_ptr, field_ty) =
                class_field_access(ctx, ptr, class_name, field_name, &Type::Dynamic)?;
            let saved_expected = ctx.expected_expr_type.replace(declared_ty);
            let init_result = compile_expr(ctx, init_expr);
            ctx.expected_expr_type = saved_expected;
            let init_result = init_result?;
            emit_field_store(
                ctx,
                field_ptr,
                &field_ty,
                init_result.value,
                &init_result.ty,
            );
        }
        Ok(())
    })();
    match saved_self {
        Some(previous) => {
            ctx.variables.insert("self".to_string(), previous);
        }
        None => {
            ctx.variables.remove("self");
        }
    }
    result
}

pub(crate) fn compile_new<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    callee: &crate::parser::ast::Expr,
    args: &[crate::parser::ast::Argument],
) -> Result<ExprResult<'ctx>, String> {
    let class_name = match callee {
        crate::parser::ast::Expr::Identifier(name) => name.clone(),
        crate::parser::ast::Expr::Call {
            callee: call_callee,
            args: call_args,
        } => match call_callee.as_ref() {
            crate::parser::ast::Expr::Identifier(name)
                if name.chars().next().is_some_and(|c| c.is_uppercase()) =>
            {
                return compile_new(
                    ctx,
                    &crate::parser::ast::Expr::Identifier(name.clone()),
                    call_args,
                );
            }
            _ => return Err("Complex new expressions not yet supported".to_string()),
        },
        crate::parser::ast::Expr::Member {
            object, property, ..
        } => match (object.as_ref(), property) {
            (
                crate::parser::ast::Expr::Identifier(name),
                crate::parser::ast::MemberProperty::Ident(method),
            ) if method == "new" => name.clone(),
            _ => return Err("Complex new expressions not yet supported".to_string()),
        },
        _ => return Err("Complex new expressions not yet supported".to_string()),
    };

    let struct_ty = ctx
        .class_struct_types
        .get(&class_name)
        .ok_or_else(|| format!("compile_new: unknown class '{}'", class_name))?;
    let total_size = struct_ty
        .size_of()
        .ok_or_else(|| format!("compile_new: class '{}' has no size", class_name))?;
    let ptr = if ctx.arc_registry.is_arc_class(&class_name) {
        let type_info_name = format!("ruyi_type_info_{}", class_name);
        let type_info_global = ctx.module.get_global(&type_info_name).ok_or_else(|| {
            format!(
                "compile_new: ARC type info '{}' not found for '{}'",
                type_info_name, class_name
            )
        })?;
        let type_info_ptr = type_info_global.as_pointer_value();
        super::arc_ops::emit_arc_alloc(ctx, total_size, type_info_ptr)
    } else {
        GcAllocFn::for_mode(ctx.gc_mode).emit(ctx.builder(), ctx.module, total_size)
    };

    emit_field_initializers(ctx, &class_name, ptr)?;

    let ctor_name = format!("{}_new", class_name);
    // If `new` is a static method, `{class}_new` is the static factory, not
    // a constructor. Raw `new Class()` allocation must not invoke it.
    let has_static_new = ctx
        .class_static_methods
        .get(&class_name)
        .is_some_and(|s| s.contains("new"));
    if !has_static_new {
        if let Some(ctor) = ctx.module.get_function(&ctor_name) {
            let mut arg_values = vec![ptr.into()];
            emit_spread_args(ctx, args, &mut arg_values)?;

            let ctor_param_types = ctor.get_type().get_param_types();
            // Coerce the user-supplied argument values to the ctor's declared
            // parameter LLVM types. For generic ctors the type parameters are
            // erased to `ptr` slots, so a concrete `i64` value (e.g. `Ok.new(42)`)
            // must be bit-cast to `ptr` before the call. Without this the
            // generic-class ctor receives the wrong type and stores garbage
            // in the field slot, later causing segfaults on field access.
            let i8_ptr_ty = ctx.context.ptr_type(Default::default());
            let i64_ty = ctx.context.i64_type();
            let f64_ty = ctx.context.f64_type();
            let mut coerced_values: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> =
                Vec::with_capacity(arg_values.len());
            for (idx, value) in arg_values.iter().enumerate() {
                let coerced: inkwell::values::BasicMetadataValueEnum<'ctx> =
                    if let Some(param_ty) = ctor_param_types.get(idx) {
                        if *param_ty == i8_ptr_ty.into() {
                            // param expects ptr; if value is int, bitcast.
                            if let Ok(bv) = (*value).try_into() {
                                match bv {
                                    BasicValueEnum::IntValue(iv) => ctx
                                        .builder()
                                        .build_int_to_ptr(iv, i8_ptr_ty, "ctor_arg_to_ptr")
                                        .unwrap()
                                        .into(),
                                    other => other.into(),
                                }
                            } else {
                                *value
                            }
                        } else if *param_ty == i64_ty.into() {
                            // param expects i64; if value is ptr, bitcast.
                            if let Ok(bv) = (*value).try_into() {
                                match bv {
                                    BasicValueEnum::PointerValue(pv) => ctx
                                        .builder()
                                        .build_ptr_to_int(pv, i64_ty, "ctor_arg_to_i64")
                                        .unwrap()
                                        .into(),
                                    other => other.into(),
                                }
                            } else {
                                *value
                            }
                        } else if *param_ty == f64_ty.into() {
                            // Pass through; emit_spread_args already produces
                            // the right float type for Float literals.
                            *value
                        } else {
                            *value
                        }
                    } else {
                        *value
                    };
                coerced_values.push(coerced);
            }
            arg_values = coerced_values;

            if arg_values.len() < ctor_param_types.len() {
                let defaults = ctx.default_params.get(&ctor_name).cloned();
                let i8_ptr = ctx.context.ptr_type(Default::default());
                let i64_ty = ctx.context.i64_type();
                for param_idx in arg_values.len()..ctor_param_types.len() {
                    let default_expr = defaults
                        .as_ref()
                        .and_then(|d| d.get(param_idx))
                        .and_then(|d| d.as_ref());
                    let value: BasicValueEnum<'ctx> = match default_expr {
                        Some(def_expr) => compile_expr(ctx, def_expr)?.value,
                        None => match ctor_param_types.get(param_idx) {
                            Some(t) if t.is_int_type() => {
                                t.into_int_type().const_zero().as_basic_value_enum()
                            }
                            Some(t) if t.is_float_type() => {
                                t.into_float_type().const_zero().as_basic_value_enum()
                            }
                            _ => i8_ptr.const_null().as_basic_value_enum(),
                        },
                    };
                    let adjusted: inkwell::values::BasicMetadataValueEnum<'ctx> =
                        match ctor_param_types.get(param_idx) {
                            Some(t) if *t == i8_ptr.into() => match value {
                                BasicValueEnum::IntValue(iv) => ctx
                                    .builder()
                                    .build_int_to_ptr(iv, i8_ptr, "def_to_ptr")
                                    .unwrap()
                                    .into(),
                                other => other.into(),
                            },
                            Some(t) if *t == i64_ty.into() => match value {
                                BasicValueEnum::PointerValue(pv) => ctx
                                    .builder()
                                    .build_ptr_to_int(pv, i64_ty, "def_to_i64")
                                    .unwrap()
                                    .into(),
                                other => other.into(),
                            },
                            _ => value.into(),
                        };
                    arg_values.push(adjusted);
                }
            }

            ctx.builder()
                .build_call(ctor, &arg_values, "ctor_call")
                .unwrap();
        }
    }

    // Store the runtime type ID at struct index 0 (__typeid field) so
    // `instanceof` can compare against the expected class at runtime.
    if let Some(&type_id) = ctx.type_ids.get(&class_name) {
        let struct_ty = ctx.class_struct_types.get(&class_name).unwrap();
        let struct_ptr = ctx
            .builder()
            .build_pointer_cast(ptr, ctx.context.ptr_type(Default::default()), "typeid_cast")
            .unwrap();
        let typeid_ptr = unsafe {
            ctx.builder()
                .build_gep(
                    *struct_ty,
                    struct_ptr,
                    &[
                        ctx.context.i32_type().const_int(0, false),
                        ctx.context.i32_type().const_int(0, false),
                    ],
                    "typeid_ptr",
                )
                .unwrap()
        };
        ctx.builder()
            .build_store(typeid_ptr, ctx.context.i64_type().const_int(type_id, false))
            .unwrap();
    }

    // 对于泛型类，尝试从构造函数参数推断具体类型参数，
    // 返回 Type::Generic 以保留类型信息供后续方法调用 substitute。
    let result_ty =
        if let Some((tparam_names, _body)) = ctx.generic_classes.get(&class_name).cloned() {
            let mut inferred_args = Vec::new();
            for arg in args {
                if let crate::parser::ast::Argument::Expr(e) = arg {
                    // For literal/identifier expressions we can infer the
                    // type at compile time without compiling the full
                    // expression (which would call the constructor again
                    // recursively). For complex expressions (calls,
                    // member access, etc.) fall back to compiling once
                    // and using the resulting type.
                    let arg_ty = match e.as_ref() {
                        Expr::IntLiteral(_) => Some(Type::Int),
                        Expr::FloatLiteral(_) => Some(Type::Float),
                        Expr::BooleanLiteral(_) => Some(Type::Bool),
                        Expr::StringLiteral(_) => Some(Type::String),
                        Expr::BigIntLiteral(_) => Some(Type::BigInt),
                        Expr::NullLiteral => Some(Type::Null),
                        Expr::Identifier(name) => ctx
                            .variables
                            .get(name)
                            .map(|(_, ty)| ty.clone())
                            .or_else(|| {
                                ctx.type_environment
                                    .and_then(|env| env.lookup(name))
                                    .cloned()
                            }),
                        _ => None,
                    };
                    inferred_args.push(arg_ty.unwrap_or(Type::Dynamic));
                } else {
                    inferred_args.push(Type::Dynamic);
                }
            }
            // 从构造函数的声明参数类型中提取类型参数绑定。
            // 构造函数在 function_types 中的 params 包含 self（第一个），
            // 而 inferred_args 只有用户传入的参数，需要 skip(1) 对齐。
            let ctor_fn_ty = ctx.function_types.get(&ctor_name);
            let mut bindings = std::collections::HashMap::new();
            if let Some(Type::Function { params, .. }) = ctor_fn_ty {
                for (param_ty, arg_ty) in params.iter().skip(1).zip(inferred_args.iter()) {
                    super::specialize::bind_type_params_from_type(
                        param_ty,
                        arg_ty,
                        &tparam_names,
                        &mut bindings,
                    );
                }
            }
            let concrete_args: Vec<Type> = tparam_names
                .iter()
                .map(|n| bindings.get(n).cloned().unwrap_or(Type::Dynamic))
                .collect();
            if concrete_args.iter().any(|t| *t != Type::Dynamic) {
                Type::Generic {
                    base: class_name.clone(),
                    args: concrete_args,
                }
            } else {
                Type::Named(class_name, vec![])
            }
        } else {
            Type::Named(class_name, vec![])
        };

    Ok(ExprResult::new(
        BasicValueEnum::PointerValue(ptr),
        result_ty,
    ))
}

fn compile_super_new<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    args: &[crate::parser::ast::Argument],
) -> Result<ExprResult<'ctx>, String> {
    let current_class = ctx
        .current_class_name
        .as_deref()
        .ok_or("super.new() can only be called within a class method")?;

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
        let self_loaded = ctx
            .builder()
            .build_load(
                ctx.context.ptr_type(Default::default()),
                self_ptr_copy,
                "super_self",
            )
            .unwrap();
        let mut arg_values = vec![self_loaded.into()];
        if let BasicValueEnum::PointerValue(instance) = self_loaded {
            emit_field_initializers(ctx, &parent_class, instance)?;
        }
        emit_spread_args(ctx, args, &mut arg_values)?;
        ctx.builder()
            .build_call(ctor, &arg_values, "super_ctor_call")
            .unwrap();
    }

    Ok(ExprResult::new(
        BasicValueEnum::PointerValue(self_ptr_copy),
        self_ty_copy,
    ))
}

/// Compile enum variant constructors: Some(value), None, Ok(value), Err(value)
/// Layout: { tag: i8, value: i8* } where tag 0 = None/Err, tag 1 = Some/Ok
fn compile_enum_variant<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    _variant: &str,
    args: &[crate::parser::ast::Argument],
    tag: u64,
) -> Result<ExprResult<'ctx>, String> {
    let i8_ty = ctx.context.i8_type();
    let i8_ptr_ty = ctx.context.ptr_type(Default::default());
    let option_struct = *ctx
        .enum_struct_types
        .entry("Option".to_string())
        .or_insert_with(|| {
            ctx.context
                .struct_type(&[i8_ty.into(), i8_ptr_ty.into()], false)
        });
    let ptr = ctx
        .builder()
        .build_alloca(option_struct, "enum_variant")
        .unwrap();

    let tag_ptr = ctx
        .builder()
        .build_struct_gep(option_struct, ptr, 0, "tag_ptr")
        .unwrap();
    ctx.builder()
        .build_store(tag_ptr, i8_ty.const_int(tag, false))
        .unwrap();

    let value_ptr = ctx
        .builder()
        .build_struct_gep(option_struct, ptr, 1, "value_ptr")
        .unwrap();
    if tag == 1 && !args.is_empty() {
        if let crate::parser::ast::Argument::Expr(e) = &args[0] {
            let result = compile_expr(ctx, e)?;
            let casted = match result.value {
                BasicValueEnum::PointerValue(p) => ctx
                    .builder()
                    .build_bit_cast(p, i8_ptr_ty, "value_cast")
                    .unwrap(),
                BasicValueEnum::IntValue(i) => BasicValueEnum::PointerValue(
                    ctx.builder()
                        .build_int_to_ptr(i, i8_ptr_ty, "value_cast")
                        .unwrap(),
                ),
                BasicValueEnum::FloatValue(f) => ctx
                    .builder()
                    .build_bit_cast(f, i8_ptr_ty, "value_cast")
                    .unwrap(),
                _ => return Err("Unsupported enum variant value type".to_string()),
            };
            let ptr_val = match casted {
                BasicValueEnum::PointerValue(p) => p,
                _ => return Err("Enum variant value must be a pointer".to_string()),
            };
            ctx.builder().build_store(value_ptr, ptr_val).unwrap();
        }
    } else {
        ctx.builder()
            .build_store(value_ptr, i8_ptr_ty.const_null())
            .unwrap();
    }

    let enum_struct_ty = ctx.context.struct_type(
        &[
            ctx.context.i8_type().into(),
            ctx.context.ptr_type(Default::default()).into(),
        ],
        false,
    );
    let loaded = ctx
        .builder()
        .build_load(enum_struct_ty, ptr, "enum_loaded")
        .unwrap();
    Ok(ExprResult::new(
        loaded,
        Type::Generic {
            base: "Option".to_string(),
            args: vec![Type::Dynamic],
        },
    ))
}

fn compile_match_expr<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    value: &crate::parser::ast::Expr,
    arms: &[crate::parser::ast::MatchArm],
) -> Result<ExprResult<'ctx>, String> {
    let func = ctx.current_function().ok_or("No current function")?;
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
    let result_ptr = ctx.builder().build_alloca(llvm_ty, "match_result").unwrap();

    ctx.builder()
        .build_unconditional_branch(arm_bbs[0])
        .unwrap();

    for (i, arm) in arms.iter().enumerate() {
        ctx.builder().position_at_end(arm_bbs[i]);

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
            ctx.builder()
                .build_conditional_branch(guard_val.value.into_int_value(), body_bb, next_bb)
                .unwrap();
            ctx.builder().position_at_end(body_bb);
        }

        let body_len = arm.body.len();
        let mut wrote_result = false;
        for (j, stmt) in arm.body.iter().enumerate() {
            wrote_result =
                compile_stmt_for_match(ctx, stmt, j == body_len - 1, result_ptr, llvm_ty)?;
            if let Some(bb) = ctx.builder().get_insert_block() {
                if bb.get_terminator().is_some() {
                    break;
                }
            }
        }

        if let Some(bb) = ctx.builder().get_insert_block() {
            if bb.get_terminator().is_none() {
                if !wrote_result {
                    // The arm body wrote no result (e.g. an empty block or a
                    // non-expression tail). Seed the slot with a null Dynamic
                    // `{0, null}` instead of undef so the merge block never
                    // observes garbage. A result already written by the last
                    // statement must never be overwritten.
                    let null_dyn = llvm_ty.into_struct_type().const_zero();
                    ctx.builder().build_store(result_ptr, null_dyn).unwrap();
                }
                ctx.builder().build_unconditional_branch(merge_bb).unwrap();
            }
        }
    }

    ctx.builder().position_at_end(merge_bb);
    let loaded = ctx
        .builder()
        .build_load(
            ruyi_type_to_llvm(ctx.context, &result_ty),
            result_ptr,
            "match_result_final",
        )
        .unwrap();
    Ok(ExprResult::new(loaded, result_ty))
}

fn compile_stmt_for_match<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    stmt: &Statement,
    is_last: bool,
    result_ptr: inkwell::values::PointerValue<'ctx>,
    _llvm_ty: inkwell::types::BasicTypeEnum<'ctx>,
) -> Result<bool, String> {
    use crate::parser::ast::Statement;
    match stmt {
        Statement::Expression(expr) => {
            let result = compile_expr(ctx, expr)?;
            if is_last {
                // The result slot is a Dynamic `{tag, data}` struct. Box the
                // arm value accordingly; storing a raw i64/pointer into the
                // struct slot would leave it half-written and type-mismatched.
                let boxed: BasicValueEnum<'ctx> = if matches!(result.ty, Type::Dynamic) {
                    match result.value {
                        BasicValueEnum::StructValue(s) => s.into(),
                        v => build_box_dynamic(ctx, v, &result.ty),
                    }
                } else {
                    build_box_dynamic(ctx, result.value, &result.ty)
                };
                ctx.builder().build_store(result_ptr, boxed).unwrap();
                return Ok(true);
            }
            Ok(false)
        }
        _ => super::stmt::compile_stmt(ctx, stmt).map(|_| false),
    }
}
