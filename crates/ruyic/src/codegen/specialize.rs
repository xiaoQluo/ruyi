/**
 * On-demand specialization of generic class methods.
 *
 * The erased codegen path maps type parameters to i64, which breaks
 * methods that call trait methods on a type-parameter receiver (e.g.
 * `total.add(...)` inside `ArrayIterator<T>::sum`). When a call site
 * knows the concrete type arguments of the receiver, this module
 * instantiates a specialized copy of the method with the type
 * parameters substituted, so receiver-method dispatch resolves to the
 * concrete impl (`add_Add_for_int` etc.).
 *
 * @author Ruyi Team
 * @date 2026-07-25
 */
use std::collections::HashMap;

use super::generator::CodegenContext;
use crate::parser::ast::{
    Argument, ArrayElement, ArrowBody, Binding, CatchClause, ClassElement, Declaration, Expr,
    ForInit, MatchArm, ObjectProperty, Param, Pattern, PropertyName, Statement, TemplatePart,
    TypeAnnotation, TypeField,
};
use crate::typechecker::types::Type;

/// Returns true when the type contains no unresolved type parameters or
/// inference variables, i.e. it is safe to use as a specialization key.
pub fn is_concrete(ty: &Type) -> bool {
    match ty {
        Type::Named(name, _) => !is_type_param_name(name),
        Type::TypeVar(_) | Type::Dynamic | Type::Error => false,
        Type::Nullable(inner) | Type::Array(inner) | Type::Future(inner) => is_concrete(inner),
        Type::Generic { args, .. } => args.iter().all(is_concrete),
        Type::Tuple(types) | Type::Union(types) => types.iter().all(is_concrete),
        Type::Function {
            params,
            return_type,
        } => params.iter().all(is_concrete) && is_concrete(return_type),
        Type::Object(fields) => fields.iter().all(|f| is_concrete(&f.ty)),
        _ => true,
    }
}

/// Heuristic shared with `ruyi_type_to_llvm`: a single uppercase ASCII
/// letter names a type parameter (T, U, K, V, ...).
fn is_type_param_name(name: &str) -> bool {
    name.len() == 1
        && name
            .chars()
            .next()
            .map(|c| c.is_ascii_uppercase())
            .unwrap_or(false)
}

/// Returns true when the type still mentions a type parameter.
pub fn has_type_params(ty: &Type) -> bool {
    match ty {
        Type::Named(name, _) => is_type_param_name(name),
        Type::Nullable(inner) | Type::Array(inner) | Type::Future(inner) => has_type_params(inner),
        Type::Generic { args, .. } => args.iter().any(has_type_params),
        Type::Tuple(types) | Type::Union(types) => types.iter().any(has_type_params),
        Type::Function {
            params,
            return_type,
        } => params.iter().any(has_type_params) || has_type_params(return_type),
        Type::Object(fields) => fields.iter().any(|f| has_type_params(&f.ty)),
        _ => false,
    }
}

/// Substitute bound type parameters inside a checker-level `Type`.
pub fn subst_type(ty: &Type, bindings: &HashMap<String, Type>) -> Type {
    match ty {
        Type::Named(name, _) => {
            if let Some(bound) = bindings.get(name) {
                bound.clone()
            } else {
                ty.clone()
            }
        }
        Type::Nullable(inner) => Type::Nullable(Box::new(subst_type(inner, bindings))),
        Type::Array(inner) => Type::Array(Box::new(subst_type(inner, bindings))),
        Type::Future(inner) => Type::Future(Box::new(subst_type(inner, bindings))),
        Type::Generic { base, args } => Type::Generic {
            base: base.clone(),
            args: args.iter().map(|a| subst_type(a, bindings)).collect(),
        },
        Type::Tuple(types) => Type::Tuple(types.iter().map(|t| subst_type(t, bindings)).collect()),
        Type::Union(types) => Type::Union(types.iter().map(|t| subst_type(t, bindings)).collect()),
        Type::Function {
            params,
            return_type,
        } => Type::Function {
            params: params.iter().map(|p| subst_type(p, bindings)).collect(),
            return_type: Box::new(subst_type(return_type, bindings)),
        },
        other => other.clone(),
    }
}

/// Unify a declared annotation (which may mention type parameters from
/// `params`) against a concrete receiver type, collecting bindings.
/// Best-effort: mismatched shapes simply add no bindings.
pub fn bind_type_params(
    pattern: &TypeAnnotation,
    concrete: &Type,
    params: &[String],
    out: &mut HashMap<String, Type>,
) {
    match (pattern, concrete) {
        (TypeAnnotation::Identifier(name), ty) => {
            if params.iter().any(|p| p == name) {
                out.entry(name.clone()).or_insert_with(|| ty.clone());
            }
        }
        (TypeAnnotation::Nullable(inner), Type::Nullable(cty)) => {
            bind_type_params(inner, cty, params, out);
        }
        (TypeAnnotation::Array(inner), Type::Array(cty)) => {
            bind_type_params(inner, cty, params, out);
        }
        (TypeAnnotation::Generic { base, args }, Type::Array(cty))
            if base == "Array" && args.len() == 1 =>
        {
            bind_type_params(&args[0], cty, params, out);
        }
        (
            TypeAnnotation::Generic { base, args },
            Type::Generic {
                base: cbase,
                args: cargs,
            },
        ) if base == cbase && args.len() == cargs.len() => {
            for (a, c) in args.iter().zip(cargs.iter()) {
                bind_type_params(a, c, params, out);
            }
        }
        _ => {}
    }
}

/// Convert a concrete checker-level `Type` back into a parser-level
/// annotation so it can be substituted into a method AST. Returns `None`
/// for types that have no annotation form.
pub fn type_to_annotation(ty: &Type) -> Option<TypeAnnotation> {
    match ty {
        Type::Int => Some(TypeAnnotation::Builtin("int".to_string())),
        Type::Float => Some(TypeAnnotation::Builtin("float".to_string())),
        Type::Bool => Some(TypeAnnotation::Builtin("bool".to_string())),
        Type::Byte => Some(TypeAnnotation::Builtin("byte".to_string())),
        Type::String => Some(TypeAnnotation::Builtin("string".to_string())),
        Type::BigInt => Some(TypeAnnotation::Builtin("bigint".to_string())),
        Type::Null => Some(TypeAnnotation::Builtin("null".to_string())),
        Type::Void => Some(TypeAnnotation::Builtin("void".to_string())),
        Type::Nullable(inner) => Some(TypeAnnotation::Nullable(Box::new(type_to_annotation(
            inner,
        )?))),
        Type::Array(inner) => Some(TypeAnnotation::Array(Box::new(type_to_annotation(inner)?))),
        Type::Named(name, _) => Some(TypeAnnotation::Identifier(name.clone())),
        Type::Generic { base, args } => Some(TypeAnnotation::Generic {
            base: base.clone(),
            args: args
                .iter()
                .map(type_to_annotation)
                .collect::<Option<Vec<_>>>()?,
        }),
        Type::Function {
            params,
            return_type,
        } => Some(TypeAnnotation::Function {
            params: params
                .iter()
                .map(type_to_annotation)
                .collect::<Option<Vec<_>>>()?,
            return_type: Box::new(type_to_annotation(return_type)?),
        }),
        Type::Tuple(types) => Some(TypeAnnotation::Tuple(
            types
                .iter()
                .map(type_to_annotation)
                .collect::<Option<Vec<_>>>()?,
        )),
        _ => None,
    }
}

// ── AST substitution walker ──────────────────────────────────

type AnnMap = HashMap<String, TypeAnnotation>;

pub fn subst_annotation(ann: &TypeAnnotation, map: &AnnMap) -> TypeAnnotation {
    match ann {
        TypeAnnotation::Identifier(name) => map.get(name).cloned().unwrap_or_else(|| ann.clone()),
        TypeAnnotation::Builtin(_) => ann.clone(),
        TypeAnnotation::Nullable(inner) => {
            TypeAnnotation::Nullable(Box::new(subst_annotation(inner, map)))
        }
        TypeAnnotation::Function {
            params,
            return_type,
        } => TypeAnnotation::Function {
            params: params.iter().map(|p| subst_annotation(p, map)).collect(),
            return_type: Box::new(subst_annotation(return_type, map)),
        },
        TypeAnnotation::Generic { base, args } => TypeAnnotation::Generic {
            base: base.clone(),
            args: args.iter().map(|a| subst_annotation(a, map)).collect(),
        },
        TypeAnnotation::Object(fields) => TypeAnnotation::Object(
            fields
                .iter()
                .map(|f| TypeField {
                    name: f.name.clone(),
                    ty: subst_annotation(&f.ty, map),
                })
                .collect(),
        ),
        TypeAnnotation::Array(inner) => {
            TypeAnnotation::Array(Box::new(subst_annotation(inner, map)))
        }
        TypeAnnotation::Tuple(types) => {
            TypeAnnotation::Tuple(types.iter().map(|t| subst_annotation(t, map)).collect())
        }
        TypeAnnotation::Dyn(inner) => TypeAnnotation::Dyn(Box::new(subst_annotation(inner, map))),
        TypeAnnotation::Union(types) => {
            TypeAnnotation::Union(types.iter().map(|t| subst_annotation(t, map)).collect())
        }
    }
}

pub fn subst_params(params: &[Param], map: &AnnMap) -> Vec<Param> {
    params
        .iter()
        .map(|p| Param {
            pattern: p.pattern.clone(),
            ty: p.ty.as_ref().map(|t| subst_annotation(t, map)),
            init: p.init.as_ref().map(|e| Box::new(subst_expr(e, map))),
            is_rest: p.is_rest,
            is_optional: p.is_optional,
        })
        .collect()
}

pub fn subst_statements(stmts: &[Statement], map: &AnnMap) -> Vec<Statement> {
    stmts.iter().map(|s| subst_statement(s, map)).collect()
}

fn subst_statement(stmt: &Statement, map: &AnnMap) -> Statement {
    match stmt {
        Statement::Block(stmts) => Statement::Block(subst_statements(stmts, map)),
        Statement::Expression(e) => Statement::Expression(Box::new(subst_expr(e, map))),
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => Statement::If {
            condition: Box::new(subst_expr(condition, map)),
            then_branch: Box::new(subst_statement(then_branch, map)),
            else_branch: else_branch
                .as_ref()
                .map(|s| Box::new(subst_statement(s, map))),
        },
        Statement::IfLet {
            pattern,
            value,
            then_branch,
            else_branch,
        } => Statement::IfLet {
            pattern: pattern.clone(),
            value: Box::new(subst_expr(value, map)),
            then_branch: Box::new(subst_statement(then_branch, map)),
            else_branch: else_branch
                .as_ref()
                .map(|s| Box::new(subst_statement(s, map))),
        },
        Statement::While { condition, body } => Statement::While {
            condition: Box::new(subst_expr(condition, map)),
            body: Box::new(subst_statement(body, map)),
        },
        Statement::WhileLet {
            pattern,
            value,
            body,
        } => Statement::WhileLet {
            pattern: pattern.clone(),
            value: Box::new(subst_expr(value, map)),
            body: Box::new(subst_statement(body, map)),
        },
        Statement::For {
            init,
            condition,
            update,
            body,
        } => Statement::For {
            init: init.as_ref().map(|i| match i {
                ForInit::VarDecl(d) => ForInit::VarDecl(subst_declaration(d, map)),
                ForInit::Expr(e) => ForInit::Expr(Box::new(subst_expr(e, map))),
            }),
            condition: condition.as_ref().map(|e| Box::new(subst_expr(e, map))),
            update: update.as_ref().map(|e| Box::new(subst_expr(e, map))),
            body: Box::new(subst_statement(body, map)),
        },
        Statement::ForIn {
            variable,
            iterable,
            body,
        } => Statement::ForIn {
            variable: variable.clone(),
            iterable: Box::new(subst_expr(iterable, map)),
            body: Box::new(subst_statement(body, map)),
        },
        Statement::ForOf {
            variable,
            iterable,
            body,
            is_async,
        } => Statement::ForOf {
            variable: variable.clone(),
            iterable: Box::new(subst_expr(iterable, map)),
            body: Box::new(subst_statement(body, map)),
            is_async: *is_async,
        },
        Statement::Return(e) => Statement::Return(e.as_ref().map(|e| Box::new(subst_expr(e, map)))),
        Statement::Throw(e) => Statement::Throw(Box::new(subst_expr(e, map))),
        Statement::Try {
            body,
            catch,
            finally,
        } => Statement::Try {
            body: subst_statements(body, map),
            catch: catch
                .iter()
                .map(|c| CatchClause {
                    pattern: c.pattern.clone(),
                    ty: c.ty.as_ref().map(|t| subst_annotation(t, map)),
                    body: subst_statements(&c.body, map),
                })
                .collect(),
            finally: finally.as_ref().map(|f| subst_statements(f, map)),
        },
        Statement::Match { value, arms } => Statement::Match {
            value: Box::new(subst_expr(value, map)),
            arms: arms.iter().map(|a| subst_match_arm(a, map)).collect(),
        },
        Statement::Labeled { label, body } => Statement::Labeled {
            label: label.clone(),
            body: Box::new(subst_statement(body, map)),
        },
        Statement::Declaration(decl) => Statement::Declaration(subst_declaration(decl, map)),
        Statement::Yield(e) => Statement::Yield(e.as_ref().map(|e| Box::new(subst_expr(e, map)))),
        Statement::Break(_) | Statement::Continue(_) | Statement::Empty => stmt.clone(),
    }
}

fn subst_match_arm(arm: &MatchArm, map: &AnnMap) -> MatchArm {
    MatchArm {
        pattern: arm.pattern.clone(),
        guard: arm.guard.as_ref().map(|g| Box::new(subst_expr(g, map))),
        body: subst_statements(&arm.body, map),
    }
}

fn subst_declaration(decl: &Declaration, map: &AnnMap) -> Declaration {
    match decl {
        Declaration::Let(bindings) => Declaration::Let(subst_bindings(bindings, map)),
        Declaration::Const(bindings) => Declaration::Const(subst_bindings(bindings, map)),
        Declaration::Function {
            name,
            type_params,
            params,
            return_type,
            body,
            is_async,
            annotations,
        } => Declaration::Function {
            name: name.clone(),
            type_params: type_params.clone(),
            params: subst_params(params, map),
            return_type: return_type.as_ref().map(|t| subst_annotation(t, map)),
            body: subst_statements(body, map),
            is_async: *is_async,
            annotations: annotations.clone(),
        },
        other => other.clone(),
    }
}

fn subst_bindings(bindings: &[Binding], map: &AnnMap) -> Vec<Binding> {
    bindings
        .iter()
        .map(|b| Binding {
            pattern: b.pattern.clone(),
            init: b.init.as_ref().map(|e| Box::new(subst_expr(e, map))),
            ty: b.ty.as_ref().map(|t| subst_annotation(t, map)),
        })
        .collect()
}

fn subst_expr(expr: &Expr, map: &AnnMap) -> Expr {
    match expr {
        Expr::Binary { op, left, right } => Expr::Binary {
            op: op.clone(),
            left: Box::new(subst_expr(left, map)),
            right: Box::new(subst_expr(right, map)),
        },
        Expr::Unary { op, operand } => Expr::Unary {
            op: op.clone(),
            operand: Box::new(subst_expr(operand, map)),
        },
        Expr::Call { callee, args } => Expr::Call {
            callee: Box::new(subst_expr(callee, map)),
            args: subst_args(args, map),
        },
        Expr::OptionalCall { callee, args } => Expr::OptionalCall {
            callee: Box::new(subst_expr(callee, map)),
            args: subst_args(args, map),
        },
        Expr::New { callee, args } => Expr::New {
            callee: Box::new(subst_expr(callee, map)),
            args: subst_args(args, map),
        },
        Expr::Member {
            object,
            property,
            optional,
        } => Expr::Member {
            object: Box::new(subst_expr(object, map)),
            property: match property {
                crate::parser::ast::MemberProperty::Expr(e) => {
                    crate::parser::ast::MemberProperty::Expr(Box::new(subst_expr(e, map)))
                }
                other => other.clone(),
            },
            optional: *optional,
        },
        Expr::Conditional {
            condition,
            then_branch,
            else_branch,
        } => Expr::Conditional {
            condition: Box::new(subst_expr(condition, map)),
            then_branch: Box::new(subst_expr(then_branch, map)),
            else_branch: Box::new(subst_expr(else_branch, map)),
        },
        Expr::Assignment { left, op, right } => Expr::Assignment {
            left: Box::new(subst_expr(left, map)),
            op: op.clone(),
            right: Box::new(subst_expr(right, map)),
        },
        Expr::ArrowFunction {
            params,
            return_type,
            body,
            is_async,
        } => Expr::ArrowFunction {
            params: subst_params(params, map),
            return_type: return_type.as_ref().map(|t| subst_annotation(t, map)),
            body: match body {
                ArrowBody::Expr(e) => ArrowBody::Expr(Box::new(subst_expr(e, map))),
                ArrowBody::Block(stmts) => ArrowBody::Block(subst_statements(stmts, map)),
            },
            is_async: *is_async,
        },
        Expr::Await(e) => Expr::Await(Box::new(subst_expr(e, map))),
        Expr::Sequence(exprs) => Expr::Sequence(exprs.iter().map(|e| subst_expr(e, map)).collect()),
        Expr::Grouping(e) => Expr::Grouping(Box::new(subst_expr(e, map))),
        Expr::NullAssert(e) => Expr::NullAssert(Box::new(subst_expr(e, map))),
        Expr::Block(stmts) => Expr::Block(subst_statements(stmts, map)),
        Expr::ArrayLiteral(elements) => Expr::ArrayLiteral(
            elements
                .iter()
                .map(|el| match el {
                    ArrayElement::Expr(e) => ArrayElement::Expr(Box::new(subst_expr(e, map))),
                    ArrayElement::Spread(e) => ArrayElement::Spread(Box::new(subst_expr(e, map))),
                    ArrayElement::Elision => ArrayElement::Elision,
                })
                .collect(),
        ),
        Expr::ObjectLiteral(props) => Expr::ObjectLiteral(
            props
                .iter()
                .map(|p| match p {
                    ObjectProperty::Property { key, value } => ObjectProperty::Property {
                        key: key.clone(),
                        value: Box::new(subst_expr(value, map)),
                    },
                    ObjectProperty::ComputedProperty { key, value } => {
                        ObjectProperty::ComputedProperty {
                            key: Box::new(subst_expr(key, map)),
                            value: Box::new(subst_expr(value, map)),
                        }
                    }
                    ObjectProperty::Spread(e) => {
                        ObjectProperty::Spread(Box::new(subst_expr(e, map)))
                    }
                    other => other.clone(),
                })
                .collect(),
        ),
        Expr::TemplateLiteral(parts) => Expr::TemplateLiteral(
            parts
                .iter()
                .map(|p| match p {
                    TemplatePart::Expr(e) => TemplatePart::Expr(Box::new(subst_expr(e, map))),
                    other => other.clone(),
                })
                .collect(),
        ),
        Expr::Match { value, arms } => Expr::Match {
            value: Box::new(subst_expr(value, map)),
            arms: arms.iter().map(|a| subst_match_arm(a, map)).collect(),
        },
        Expr::If {
            condition,
            then_branch,
            else_branch,
        } => Expr::If {
            condition: Box::new(subst_expr(condition, map)),
            then_branch: Box::new(subst_expr(then_branch, map)),
            else_branch: else_branch.as_ref().map(|e| Box::new(subst_expr(e, map))),
        },
        other => other.clone(),
    }
}

fn subst_args(args: &[Argument], map: &AnnMap) -> Vec<Argument> {
    args.iter()
        .map(|a| match a {
            Argument::Expr(e) => Argument::Expr(Box::new(subst_expr(e, map))),
            Argument::Spread(e) => Argument::Spread(Box::new(subst_expr(e, map))),
        })
        .collect()
}

// ── On-demand instantiation ──────────────────────────────────

/// Try to instantiate `{base}_{method}` specialized for `type_args`,
/// producing the LLVM function `specialized_name`. Returns `Ok(true)`
/// when the specialized function is available (already compiled or
/// freshly instantiated), `Ok(false)` when specialization is not
/// possible and the caller should fall back to the erased path.
pub fn ensure_method_specialization<'ctx>(
    ctx: &mut CodegenContext<'ctx, '_, '_>,
    base: &str,
    method_name: &str,
    type_args: &[Type],
    specialized_name: &str,
) -> Result<bool, String> {
    // Already instantiated (or currently being instantiated higher up
    // the recursion): reuse. A body-less function here means a previous
    // attempt failed; do not retry.
    if let Some(func) = ctx.module.get_function(specialized_name) {
        return Ok(func.count_basic_blocks() > 0);
    }
    if !ctx
        .attempted_specializations
        .insert(specialized_name.to_string())
    {
        return Ok(false);
    }

    let (type_params, body) = match ctx.generic_classes.get(base) {
        Some(entry) => entry.clone(),
        None => return Ok(false),
    };
    if type_params.len() != type_args.len() {
        return Ok(false);
    }

    // Build the annotation substitution map T -> int, ...
    let mut map: AnnMap = HashMap::new();
    for (param, arg) in type_params.iter().zip(type_args.iter()) {
        match type_to_annotation(arg) {
            Some(ann) => {
                map.insert(param.clone(), ann);
            }
            None => return Ok(false),
        }
    }

    // Locate the method AST inside the class body.
    let element = body.iter().find_map(|el| match el {
        ClassElement::Method {
            name: PropertyName::Ident(n),
            is_static: false,
            ..
        } if n == method_name => Some(el.clone()),
        _ => None,
    });
    let (params, return_type, method_body, is_async) = match element {
        Some(ClassElement::Method {
            params,
            return_type,
            body,
            is_async,
            ..
        }) => (params, return_type, body, is_async),
        _ => return Ok(false),
    };
    if is_async {
        // Async state-machine codegen is not specialization-aware yet.
        return Ok(false);
    }

    // Substitute type parameters into the method signature and body.
    let mut method_params = vec![Param {
        pattern: Pattern::Identifier("self".to_string()),
        ty: Some(TypeAnnotation::Identifier(base.to_string())),
        init: None,
        is_rest: false,
        is_optional: false,
    }];
    method_params.extend(
        subst_params(&params, &map)
            .into_iter()
            .filter(|p| !matches!(&p.pattern, Pattern::Identifier(n) if n == "self")),
    );
    let subst_ret = return_type.as_ref().map(|t| subst_annotation(t, &map));
    let subst_body = subst_statements(&method_body, &map);

    // Compile re-entrantly. compile_function saves/restores the current
    // function and builder position, but local variables would leak into
    // the caller's scope, so snapshot and restore them here.
    let saved_vars = ctx.variables.clone();
    let result = super::decl::compile_function(
        ctx,
        specialized_name,
        &method_params,
        subst_ret.as_ref(),
        None,
        None,
        &subst_body,
    );
    ctx.variables = saved_vars;

    match result {
        Ok(()) => Ok(true),
        Err(e) => {
            log::warn!(
                "Specialization of {} failed, falling back to erased method: {}",
                specialized_name,
                e
            );
            super::decl::reset_to_declaration(ctx, specialized_name);
            Ok(false)
        }
    }
}
