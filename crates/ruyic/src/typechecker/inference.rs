/**
 * Type inference for the Ruyi gradual type system.
 *
 * Implements bidirectional type inference (spec Section 8.2):
 * - Synthesize mode: determine the type of an expression
 * - Check mode: verify an expression has an expected type
 * - Local inference for let bindings and function returns
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */
use crate::parser::ast::{
    ArrayElement, ArrowBody, BinaryOp, Declaration, Expr, MemberProperty, ModuleItem,
    ObjectProperty, Pattern, PropertyName, Statement, UnaryOp,
};
use crate::typechecker::constraints::ConstraintSolver;
use crate::typechecker::diagnostics::{DiagnosticBag, DiagnosticKind};
use crate::typechecker::environment::TypeEnvironment;
use crate::typechecker::generics::{
    make_generic_class_def, make_generic_function_def, make_generic_trait_def,
    MonomorphizationTracker,
};
use crate::typechecker::traits::TraitRegistry;
use crate::typechecker::types::{ObjectField, Type};
use std::collections::HashMap;

/// Recognizes stdlib-internal names (FFI builtins + stdlib type names) so the
/// typechecker does not flag them as "Unknown variable" when stdlib code
/// references them.
///
/// - `__builtin_array_*` are FFI symbols declared in the codegen layer
///   (`codegen/builtins.rs`) and implemented in the runtime
///   (`ruyi_runtime/src/builtins.rs`).
/// - `RangeError`, `ArrayIterator` are types defined inside `stdlib/`
///   itself (not user variables).
///
/// Returns `None` for unrecognized names so the caller falls through to the
/// normal "Unknown variable" diagnostic. Parameter and return types use
/// `Type::Dynamic` because the FFI signatures operate on `*mut i8` / `i64`
/// — typechecker's job is to enable compilation, not enforce FFI type safety.
fn resolve_builtin_name(name: &str) -> Option<Type> {
    match name {
        "__builtin_array_create" => Some(Type::Function {
            params: vec![],
            return_type: Box::new(Type::Dynamic),
        }),
        "__builtin_array_length" => Some(Type::Function {
            params: vec![Type::Dynamic],
            return_type: Box::new(Type::Int),
        }),
        "__builtin_array_get" => Some(Type::Function {
            params: vec![Type::Dynamic, Type::Dynamic],
            return_type: Box::new(Type::Dynamic),
        }),
        "__builtin_array_set" => Some(Type::Function {
            params: vec![Type::Dynamic, Type::Dynamic, Type::Dynamic],
            return_type: Box::new(Type::Void),
        }),
        "__builtin_array_push" => Some(Type::Function {
            params: vec![Type::Dynamic, Type::Dynamic],
            return_type: Box::new(Type::Dynamic),
        }),
        "__builtin_array_pop" => Some(Type::Function {
            params: vec![Type::Dynamic],
            return_type: Box::new(Type::Dynamic),
        }),
        "RangeError" => Some(Type::Named("RangeError".to_string(), vec![])),
        "ArrayIterator" => Some(Type::Named("ArrayIterator".to_string(), vec![])),
        _ => None,
    }
}

/// Result of type inference on a program.
#[derive(Debug)]
pub struct InferenceResult {
    pub typed_env: TypeEnvironment,
    pub diagnostics: DiagnosticBag,
    pub tracker: MonomorphizationTracker,
}

/// Bidirectional type inference engine.
///
/// Walks the AST in synthesize mode to determine types of expressions,
/// and in check mode to verify expressions against expected types.
/// Unannotated variables default to `dyn`; annotated variables use
/// static checking per the gradual typing model.
///
/// Also tracks generic definitions and their monomorphized specializations
/// per spec Sections 5 and 10.
pub struct TypeInference {
    env: TypeEnvironment,
    diagnostics: DiagnosticBag,
    #[allow(dead_code)]
    solver: ConstraintSolver,
    return_type_stack: Vec<Type>,
    tracker: MonomorphizationTracker,
    trait_registry: TraitRegistry,
    class_stack: Vec<String>,
    /// Fields declared directly on each class body. Populated in
    /// `infer_class_element` for `ClassElement::Field` and consulted in
    /// `synthesize_member_access` to resolve `instance.field` accesses.
    class_fields: HashMap<String, Vec<ObjectField>>,
    /// Methods declared directly on each class body (not via `impl Trait`).
    /// Populated in `infer_class_element` for `ClassElement::Method` and
    /// consulted in `synthesize_member_access` to resolve `instance.method`
    /// accesses as first-class function values.
    class_methods: HashMap<String, Vec<ObjectField>>,
}

impl TypeInference {
    pub fn new(trait_registry: TraitRegistry) -> Self {
        let mut env = TypeEnvironment::new();
        env.declare_let(
            "print",
            Type::Function {
                params: vec![Type::Dynamic],
                return_type: Box::new(Type::Void),
            },
        );
        env.declare_let(
            "spawn",
            Type::Function {
                params: vec![Type::Dynamic],
                return_type: Box::new(Type::Dynamic),
            },
        );
        env.declare_let(
            "ruyi_run_scheduler",
            Type::Function {
                params: vec![],
                return_type: Box::new(Type::Void),
            },
        );
        // Pre-declare common stdlib functions
        env.declare_let(
            "toString",
            Type::Function {
                params: vec![Type::Dynamic],
                return_type: Box::new(Type::String),
            },
        );
        // Pre-declare Error class type
        env.declare_let(
            "Error",
            Type::Function {
                params: vec![Type::String],
                return_type: Box::new(Type::Named("Error".into(), vec![])),
            },
        );
        Self {
            env,
            diagnostics: DiagnosticBag::new(),
            solver: ConstraintSolver::new(),
            return_type_stack: Vec::new(),
            tracker: MonomorphizationTracker::new(),
            trait_registry,
            class_stack: Vec::new(),
            class_fields: HashMap::new(),
            class_methods: HashMap::new(),
        }
    }

    pub fn infer_program(&mut self, program: &crate::parser::ast::Program) -> InferenceResult {
        // Pass 1: Collect all function declarations (signatures only)
        // This enables forward references - functions can call other functions defined later
        for item in &program.items {
            if let ModuleItem::Declaration(decl) = item {
                if let Declaration::Function {
                    name,
                    type_params,
                    params,
                    return_type,
                    is_async,
                    ..
                } = decl
                {
                    let param_types: Vec<Type> = params
                        .iter()
                        .map(|p| {
                            p.ty.as_ref()
                                .map(Type::from_annotation)
                                .unwrap_or(Type::Dynamic)
                        })
                        .collect();
                    let ret_type = return_type
                        .as_ref()
                        .map(Type::from_annotation)
                        .unwrap_or(Type::Dynamic);
                    let fn_ret_type = if *is_async {
                        Type::Future(Box::new(ret_type.clone()))
                    } else {
                        ret_type.clone()
                    };
                    let fn_type = Type::Function {
                        params: param_types.clone(),
                        return_type: Box::new(fn_ret_type),
                    };
                    if !type_params.is_empty() {
                        let generic_def = make_generic_function_def(
                            name,
                            type_params,
                            &param_types,
                            &ret_type,
                            &mut self.tracker,
                        );
                        self.tracker.register_generic(generic_def);
                    }
                    self.env.declare_let(name, fn_type);
                }
            }
        }

        // Pass 2: Full type inference (bodies, statements, etc.)
        for item in &program.items {
            self.infer_module_item(item);
        }
        InferenceResult {
            typed_env: std::mem::take(&mut self.env),
            diagnostics: std::mem::take(&mut self.diagnostics),
            tracker: std::mem::take(&mut self.tracker),
        }
    }

    /// Type-checks a program and then synthesizes an expression in an optional
    /// class/function context. Primarily intended for tests that need to inspect
    /// the inferred type of expressions inside classes or nested functions.
    pub fn synthesize_after_check(
        &mut self,
        program: &crate::parser::ast::Program,
        expr: &Expr,
        class_name: Option<&str>,
        in_function: bool,
    ) -> Type {
        self.infer_program(program);
        if let Some(class_name) = class_name {
            self.class_stack.push(class_name.to_string());
            if let Some(fields) = self.class_fields.get(class_name).cloned() {
                for field in fields {
                    self.env.declare_let(&field.name, field.ty);
                }
            }
        }
        if in_function {
            self.return_type_stack.push(Type::Void);
        }
        self.synthesize(expr)
    }

    fn infer_module_item(&mut self, item: &ModuleItem) {
        match item {
            ModuleItem::Import(_) | ModuleItem::Export(_) => {
                // Imports/exports don't need type inference in this phase
            }
            ModuleItem::Statement(stmt) => {
                self.infer_statement(stmt);
            }
            ModuleItem::Declaration(decl) => {
                self.infer_declaration(decl);
            }
        }
    }

    fn infer_declaration(&mut self, decl: &Declaration) -> Type {
        match decl {
            Declaration::Let(bindings) | Declaration::Const(bindings) => {
                let mutable = matches!(decl, Declaration::Let(_));
                for binding in bindings {
                    let ty = if let Some(init) = &binding.init {
                        let init_ty = self.synthesize(init);
                        if let Some(annotation) = &binding.ty {
                            let expected = Type::from_annotation(annotation);
                            self.check(init, &expected);
                            expected
                        } else {
                            init_ty
                        }
                    } else if let Some(annotation) = &binding.ty {
                        Type::from_annotation(annotation)
                    } else {
                        Type::Dynamic
                    };
                    if mutable {
                        self.env.declare_let(&pattern_name(&binding.pattern), ty);
                    } else {
                        self.env.declare_const(&pattern_name(&binding.pattern), ty);
                    }
                }
                Type::Void
            }
            Declaration::Function {
                name,
                type_params,
                params,
                return_type,
                body,
                is_async,
            } => {
                let param_types: Vec<Type> = params
                    .iter()
                    .map(|p| {
                        p.ty.as_ref()
                            .map(Type::from_annotation)
                            .unwrap_or(Type::Dynamic)
                    })
                    .collect();

                // Phase 1: Infer return type (needs parameters in scope)
                self.env.push_scope();
                for (param, ty) in params.iter().zip(param_types.iter()) {
                    self.env
                        .declare_param(&pattern_name(&param.pattern), ty.clone());
                }

                let ret_type = return_type
                    .as_ref()
                    .map(Type::from_annotation)
                    .unwrap_or_else(|| self.infer_return_type(body));

                let fn_ret_type = if *is_async {
                    Type::Future(Box::new(ret_type.clone()))
                } else {
                    ret_type.clone()
                };

                let fn_type = Type::Function {
                    params: param_types.clone(),
                    return_type: Box::new(fn_ret_type.clone()),
                };

                if !type_params.is_empty() {
                    let generic_def = make_generic_function_def(
                        name,
                        type_params,
                        &param_types,
                        &ret_type,
                        &mut self.tracker,
                    );
                    self.tracker.register_generic(generic_def);
                }

                // Pop the temporary scope used for return type inference
                self.env.pop_scope();

                // Phase 2: Declare function name in outer (global) scope
                self.env.declare_let(name, fn_type.clone());

                // Phase 3: Type check the function body
                self.env.push_scope();
                for (param, ty) in params.iter().zip(param_types.iter()) {
                    self.env
                        .declare_param(&pattern_name(&param.pattern), ty.clone());
                }
                self.return_type_stack.push(ret_type.clone());
                for stmt in body {
                    self.infer_statement(stmt);
                }
                self.return_type_stack.pop();
                self.env.pop_scope();

                fn_type
            }
            Declaration::Class {
                name,
                type_params,
                extends: _,
                body,
                ..
            } => {
                if !type_params.is_empty() {
                    let generic_def = make_generic_class_def(name, type_params, &mut self.tracker);
                    self.tracker.register_generic(generic_def);
                }
                let class_type = Type::Named(name.clone(), vec![]);
                self.env.declare_let(name, class_type.clone());

                self.class_stack.push(name.clone());
                self.env.push_scope();
                for element in body {
                    self.infer_class_element(element);
                }
                self.env.pop_scope();
                self.class_stack.pop();

                class_type
            }
            Declaration::Trait {
                name,
                type_params,
                supertraits: _,
                body: _,
            } => {
                if !type_params.is_empty() {
                    let generic_def = make_generic_trait_def(name, type_params, &mut self.tracker);
                    self.tracker.register_generic(generic_def);
                }
                let trait_type = Type::Trait(name.clone());
                self.env.declare_let(name, trait_type.clone());
                trait_type
            }
            Declaration::Impl {
                type_params: _,
                trait_name,
                trait_args: _,
                for_type,
                body,
            } => {
                let impl_type = Type::from_annotation(for_type);
                self.env.push_scope();
                for element in body {
                    self.infer_class_element(element);
                }
                self.env.pop_scope();
                self.env.declare_let(
                    &format!("impl_{}_for_{}", trait_name, impl_type),
                    impl_type.clone(),
                );
                impl_type
            }
            Declaration::TypeAlias {
                name: _,
                type_params: _,
                ty,
            } => Type::from_annotation(ty),
            Declaration::Macro { name, rules: _ } => {
                self.env.declare_let(name, Type::Dynamic);
                Type::Dynamic
            }
        }
    }

    fn infer_class_element(&mut self, element: &crate::parser::ast::ClassElement) {
        match element {
            crate::parser::ast::ClassElement::Method {
                name: prop_name,
                type_params: _,
                params,
                return_type,
                body,
                is_async,
                is_static: _,
                is_getter: _,
                is_setter: _,
            } => {
                let method_name = property_name_str(prop_name);
                let param_types: Vec<Type> = params
                    .iter()
                    .map(|p| {
                        p.ty.as_ref()
                            .map(Type::from_annotation)
                            .unwrap_or(Type::Dynamic)
                    })
                    .collect();
                let ret_type = return_type
                    .as_ref()
                    .map(Type::from_annotation)
                    .unwrap_or_else(|| self.infer_return_type(body));

                let fn_ret_type = if *is_async {
                    Type::Future(Box::new(ret_type.clone()))
                } else {
                    ret_type.clone()
                };

                let method_type = Type::Function {
                    params: param_types.clone(),
                    return_type: Box::new(fn_ret_type),
                };
                self.env.declare_let(&method_name, method_type.clone());
                if let Some(class_name) = self.class_stack.last() {
                    self.class_methods
                        .entry(class_name.clone())
                        .or_default()
                        .push(ObjectField {
                            name: method_name.clone(),
                            ty: method_type,
                            optional: false,
                        });
                }

                self.env.push_scope();
                for (param, ty) in params.iter().zip(param_types.iter()) {
                    self.env
                        .declare_param(&pattern_name(&param.pattern), ty.clone());
                }
                self.return_type_stack.push(ret_type.clone());
                for stmt in body {
                    self.infer_statement(stmt);
                }
                self.return_type_stack.pop();
                self.env.pop_scope();
            }
            crate::parser::ast::ClassElement::Field {
                name: prop_name,
                ty,
                init,
                is_static: _,
            } => {
                let field_name = property_name_str(prop_name);

                // Detect self-referential field types
                if let Some(annotation) = ty {
                    self.check_self_referential_field(annotation, &field_name);
                }

                let field_type = if let Some(annotation) = ty {
                    Type::from_annotation(annotation)
                } else if let Some(init_expr) = init {
                    self.synthesize(init_expr)
                } else {
                    Type::Dynamic
                };
                self.env.declare_let(&field_name, field_type.clone());
                if let Some(class_name) = self.class_stack.last() {
                    self.class_fields
                        .entry(class_name.clone())
                        .or_default()
                        .push(ObjectField {
                            name: field_name.clone(),
                            ty: field_type,
                            optional: false,
                        });
                }
            }
            crate::parser::ast::ClassElement::Empty => {}
        }
    }

    /**
     * Check for self-referential class field types.
     *
     * Non-nullable self-references (e.g. `next: ListNode`) create infinite-size types
     * and are reported as errors. Nullable self-references (e.g. `next: ListNode?`)
     * are structurally valid (e.g. linked lists, trees) but produce a warning.
     *
     * @param annotation  The type annotation on the field
     * @param field_name  The field name for diagnostic messages
     */
    fn check_self_referential_field(
        &mut self,
        annotation: &crate::parser::ast::TypeAnnotation,
        field_name: &str,
    ) {
        let current_class = match self.class_stack.last() {
            Some(name) => name,
            None => return,
        };

        match annotation {
            crate::parser::ast::TypeAnnotation::Identifier(ty_name) => {
                if ty_name == current_class {
                    self.diagnostics.add_error(DiagnosticKind::Other {
                        message: format!(
                            "Non-nullable self-referential field '{}' in class '{}': creates infinite-size type",
                            field_name, current_class,
                        ),
                    });
                }
            }
            crate::parser::ast::TypeAnnotation::Nullable(inner) => {
                if let crate::parser::ast::TypeAnnotation::Identifier(ty_name) = inner.as_ref() {
                    if ty_name == current_class {
                        self.diagnostics.add_warning(DiagnosticKind::Other {
                            message: format!(
                                "Self-referential field '{}' in class '{}': consider using a reference or box for non-nullable self-references",
                                field_name, current_class,
                            ),
                        });
                    }
                }
            }
            _ => {}
        }
    }

    fn infer_statement(&mut self, stmt: &Statement) -> Type {
        match stmt {
            Statement::Expression(expr) => self.synthesize(expr),
            Statement::Return(expr) => {
                let ret_ty = expr
                    .as_ref()
                    .map(|e| self.synthesize(e))
                    .unwrap_or(Type::Null);
                if let Some(expected) = self.return_type_stack.last() {
                    if !ret_ty.is_consistent_with(expected) {
                        self.diagnostics.add_error(DiagnosticKind::TypeMismatch {
                            expected: expected.clone(),
                            found: ret_ty,
                        });
                    }
                }
                Type::Never
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_ty = self.synthesize(condition);
                if !cond_ty.is_consistent_with(&Type::Bool) && !cond_ty.is_dynamic() {
                    self.diagnostics.add_error(DiagnosticKind::TypeMismatch {
                        expected: Type::Bool,
                        found: cond_ty,
                    });
                }

                self.env.push_scope();
                self.narrow_for_condition(condition, true);
                let then_ty = self.infer_statement(then_branch);
                self.env.pop_scope();

                let else_ty = if let Some(else_stmt) = else_branch {
                    self.env.push_scope();
                    self.narrow_for_condition(condition, false);
                    let ty = self.infer_statement(else_stmt);
                    self.env.pop_scope();
                    ty
                } else {
                    Type::Void
                };

                then_ty.least_upper_bound(&else_ty)
            }
            Statement::IfLet {
                pattern,
                value,
                then_branch,
                else_branch,
            } => {
                let val_ty = self.synthesize(value);
                self.env.push_scope();
                self.bind_pattern_type(pattern, &val_ty);
                let then_ty = self.infer_statement(then_branch);
                self.env.pop_scope();

                let else_ty = if let Some(else_stmt) = else_branch {
                    self.env.push_scope();
                    let ty = self.infer_statement(else_stmt);
                    self.env.pop_scope();
                    ty
                } else {
                    Type::Void
                };

                then_ty.least_upper_bound(&else_ty)
            }
            Statement::While { condition, body } => {
                let cond_ty = self.synthesize(condition);
                if !cond_ty.is_consistent_with(&Type::Bool) && !cond_ty.is_dynamic() {
                    self.diagnostics.add_warning(DiagnosticKind::TypeMismatch {
                        expected: Type::Bool,
                        found: cond_ty,
                    });
                }
                self.env.push_scope();
                self.infer_statement(body);
                self.env.pop_scope();
                Type::Void
            }
            Statement::WhileLet {
                pattern,
                value,
                body,
            } => {
                let val_ty = self.synthesize(value);
                self.env.push_scope();
                self.bind_pattern_type(pattern, &val_ty);
                self.infer_statement(body);
                self.env.pop_scope();
                Type::Void
            }
            Statement::For {
                init,
                condition,
                update,
                body,
            } => {
                self.env.push_scope();
                if let Some(for_init) = init {
                    match for_init {
                        crate::parser::ast::ForInit::VarDecl(decl) => {
                            self.infer_declaration(decl);
                        }
                        crate::parser::ast::ForInit::Expr(expr) => {
                            self.synthesize(expr);
                        }
                    }
                }
                if let Some(cond) = condition {
                    self.synthesize(cond);
                }
                if let Some(upd) = update {
                    self.synthesize(upd);
                }
                self.infer_statement(body);
                self.env.pop_scope();
                Type::Void
            }
            Statement::ForIn {
                variable,
                iterable,
                body,
            } => {
                let iter_ty = self.synthesize(iterable);
                self.env.push_scope();
                self.env.declare_let(variable, Type::Dynamic);
                self.infer_statement(body);
                self.env.pop_scope();
                let _ = iter_ty;
                Type::Void
            }
            Statement::ForOf {
                variable,
                iterable,
                body,
                is_async: _,
            } => {
                let iter_ty = self.synthesize(iterable);
                let elem_type = match &iter_ty {
                    Type::Array(elem) => *elem.clone(),
                    Type::Dynamic => Type::Dynamic,
                    _ => Type::Dynamic,
                };
                self.env.push_scope();
                self.env.declare_let(variable, elem_type);
                self.infer_statement(body);
                self.env.pop_scope();
                Type::Void
            }
            Statement::Throw(expr) => {
                let ty = self.synthesize(expr);
                let _ = ty;
                Type::Never
            }
            Statement::Try {
                body,
                catch,
                finally,
            } => {
                self.env.push_scope();
                for stmt in body {
                    self.infer_statement(stmt);
                }
                self.env.pop_scope();

                for catch_clause in catch {
                    self.env.push_scope();
                    if let Some(pattern) = &catch_clause.pattern {
                        let catch_type = catch_clause
                            .ty
                            .as_ref()
                            .map(Type::from_annotation)
                            .unwrap_or(Type::Dynamic);
                        self.bind_pattern_type(pattern, &catch_type);
                    }
                    for stmt in &catch_clause.body {
                        self.infer_statement(stmt);
                    }
                    self.env.pop_scope();
                }

                if let Some(finally_stmts) = finally {
                    self.env.push_scope();
                    for stmt in finally_stmts {
                        self.infer_statement(stmt);
                    }
                    self.env.pop_scope();
                }

                Type::Void
            }
            Statement::Match { value, arms } => {
                let match_ty = self.synthesize(value);

                // Analyze patterns for exhaustiveness and redundancy
                let arm_refs: Vec<(&Pattern, &Type)> =
                    arms.iter().map(|arm| (&arm.pattern, &match_ty)).collect();
                let analysis = crate::typechecker::patterns::analyze_patterns(&arm_refs);

                // Report non-exhaustive match
                if !analysis.is_exhaustive {
                    if match_ty.requires_exhaustive_match() {
                        self.diagnostics
                            .add_error(DiagnosticKind::NonExhaustiveMatch {
                                scrutinee_type: match_ty.clone(),
                                missing: analysis.missing_cases.clone(),
                            });
                    } else {
                        self.diagnostics
                            .add_warning(DiagnosticKind::NonExhaustiveMatch {
                                scrutinee_type: match_ty.clone(),
                                missing: analysis.missing_cases.clone(),
                            });
                    }
                }

                // Report redundant pattern
                if let Some(redundant_idx) = analysis.redundant_arm {
                    self.diagnostics
                        .add_warning(DiagnosticKind::RedundantPattern { arm: redundant_idx });
                }

                let mut result_types = Vec::new();
                for arm in arms {
                    self.env.push_scope();
                    self.bind_pattern_type(&arm.pattern, &match_ty);
                    let mut arm_type = Type::Void;
                    for stmt in &arm.body {
                        arm_type = self.infer_statement(stmt);
                    }
                    self.env.pop_scope();
                    result_types.push(arm_type);
                }
                result_types
                    .into_iter()
                    .fold(Type::Void, |acc, ty| acc.least_upper_bound(&ty))
            }
            Statement::Block(stmts) => {
                self.env.push_scope();
                let mut last_ty = Type::Void;
                for stmt in stmts {
                    last_ty = self.infer_statement(stmt);
                }
                self.env.pop_scope();
                last_ty
            }
            Statement::Break(_) | Statement::Continue(_) => Type::Never,
            Statement::Yield(expr) => {
                if let Some(e) = expr {
                    self.synthesize(e);
                }
                Type::Void
            }
            Statement::Labeled { body, .. } => self.infer_statement(body),
            Statement::Declaration(decl) => self.infer_declaration(decl),
            Statement::Empty => Type::Void,
        }
    }

    /// Synthesize mode: determine the type of an expression.
    pub fn synthesize(&mut self, expr: &Expr) -> Type {
        match expr {
            Expr::IntLiteral(_) => Type::Int,
            Expr::FloatLiteral(_) => Type::Float,
            Expr::StringLiteral(_) => Type::String,
            Expr::BigIntLiteral(_) => Type::BigInt,
            Expr::BooleanLiteral(_) => Type::Bool,
            Expr::NullLiteral => Type::Null,
            Expr::Identifier(name) => self.env.lookup(name).cloned()
                .or_else(|| resolve_builtin_name(name))
                .unwrap_or_else(|| {
                    self.diagnostics
                        .add_error(DiagnosticKind::UnknownVariable { name: name.clone() });
                    Type::Error
                }),
            Expr::This | Expr::Super => Type::Dynamic,
            Expr::SelfExpr => {
                if let Some(class_name) = self.class_stack.last() {
                    Type::Named(class_name.clone(), vec![])
                } else if self.return_type_stack.is_empty() {
                    self.diagnostics.add_error(DiagnosticKind::Other {
                        message: "E4002: self used outside of class method".into(),
                    });
                    Type::Error
                } else {
                    Type::Dynamic
                }
            }
            Expr::TemplateLiteral(parts) => {
                for part in parts {
                    if let crate::parser::ast::TemplatePart::Expr(e) = part {
                        self.synthesize(e);
                    }
                }
                Type::String
            }
            Expr::ArrayLiteral(elements) => {
                if elements.is_empty() {
                    return Type::Array(Box::new(Type::Dynamic));
                }
                let mut elem_type = Type::Never;
                for elem in elements {
                    let ty = match elem {
                        ArrayElement::Expr(e) => self.synthesize(e),
                        ArrayElement::Spread(e) => {
                            let spread_ty = self.synthesize(e);
                            match spread_ty {
                                Type::Array(inner) => *inner,
                                _ => Type::Dynamic,
                            }
                        }
                        ArrayElement::Elision => Type::Dynamic,
                    };
                    elem_type = elem_type.least_upper_bound(&ty);
                }
                Type::Array(Box::new(elem_type))
            }
            Expr::ObjectLiteral(props) => {
                let fields: Vec<ObjectField> = props
                    .iter()
                    .map(|prop| match prop {
                        ObjectProperty::Property { key, value } => ObjectField {
                            name: property_name_str(key),
                            ty: self.synthesize(value),
                            optional: false,
                        },
                        ObjectProperty::Shorthand(name) => {
                            let ty = self.env.lookup(name).cloned().unwrap_or(Type::Dynamic);
                            ObjectField {
                                name: name.clone(),
                                ty,
                                optional: false,
                            }
                        }
                        ObjectProperty::Spread(_) => ObjectField {
                            name: "...".into(),
                            ty: Type::Dynamic,
                            optional: false,
                        },
                        ObjectProperty::ComputedProperty { key, value } => ObjectField {
                            name: format!("[{}]", self.synthesize(key)),
                            ty: self.synthesize(value),
                            optional: false,
                        },
                    })
                    .collect();
                Type::Object(fields)
            }
            Expr::Binary { op, left, right } => {
                let left_ty = self.synthesize(left);
                let right_ty = self.synthesize(right);
                self.synthesize_binary(op, left_ty, right_ty)
            }
            Expr::Unary { op, operand } => {
                let operand_ty = self.synthesize(operand);
                self.synthesize_unary(op, operand_ty)
            }
            Expr::NullAssert(expr) => {
                let ty = self.synthesize(expr);
                match &ty {
                    Type::Nullable(inner) => *inner.clone(),
                    Type::Null => Type::Never,
                    _ => {
                        self.diagnostics.add_error(DiagnosticKind::Other {
                            message: format!(
                                "cannot use null assertion on non-nullable type `{}`",
                                ty
                            ),
                        });
                        ty
                    }
                }
            }
            Expr::Call { callee, args } => {
                let callee_ty = self.synthesize(callee);
                let arg_types: Vec<Type> = args
                    .iter()
                    .map(|arg| match arg {
                        crate::parser::ast::Argument::Expr(e) => self.synthesize(e),
                        crate::parser::ast::Argument::Spread(e) => {
                            let ty = self.synthesize(e);
                            match ty {
                                Type::Array(inner) => *inner,
                                _ => Type::Dynamic,
                            }
                        }
                    })
                    .collect();

                // Check if callee is a generic function and try to specialize
                if let Expr::Identifier(name) = callee.as_ref() {
                    if self.tracker.is_generic(name) {
                        if let Some(inferred_args) =
                            self.tracker
                                .infer_type_args(name, &arg_types, &mut self.diagnostics)
                        {
                            if let Some(spec) =
                                self.tracker
                                    .specialize(name, inferred_args, &mut self.diagnostics)
                            {
                                return spec.specialized_type;
                            }
                        }
                    }
                }

                match callee_ty {
                    Type::Function {
                        params,
                        return_type,
                    } => {
                        // Support default params: allow fewer args (missing args have defaults)
                        // Support rest params: allow more args when last param is Array<T>
                        // Only treat as rest if we have more args than regular params
                        let last_is_array = params
                            .last()
                            .map(|p| match p {
                                Type::Array(_) => true,
                                Type::Generic { base, .. } if base == "Array" => true,
                                _ => false,
                            })
                            .unwrap_or(false);
                        let has_rest = last_is_array && arg_types.len() > params.len();
                        let min_args = 0; // Allow all args to have defaults
                        let max_args = if has_rest { usize::MAX } else { params.len() };
                        if arg_types.len() < min_args || arg_types.len() > max_args {
                            self.diagnostics.add_error(DiagnosticKind::ArgumentCount {
                                expected: params.len(),
                                found: arg_types.len(),
                            });
                        }
                        if has_rest {
                            // Regular params before rest
                            let regular_count = params.len().saturating_sub(1);
                            for (i, (arg, param)) in arg_types
                                .iter()
                                .zip(params.iter())
                                .enumerate()
                                .take(regular_count)
                            {
                                if !arg.is_consistent_with(param) {
                                    self.diagnostics.add_error(DiagnosticKind::TypeMismatch {
                                        expected: param.clone(),
                                        found: arg.clone(),
                                    });
                                    let _ = i;
                                }
                            }
                            // Rest param: all remaining args must match element type
                            if let Some(Type::Array(elem)) = params.last() {
                                for arg in arg_types.iter().skip(regular_count) {
                                    if !arg.is_consistent_with(elem) {
                                        self.diagnostics.add_error(DiagnosticKind::TypeMismatch {
                                            expected: *elem.clone(),
                                            found: arg.clone(),
                                        });
                                    }
                                }
                            }
                        } else {
                            for (i, (arg, param)) in arg_types.iter().zip(params.iter()).enumerate()
                            {
                                if !arg.is_consistent_with(param) {
                                    self.diagnostics.add_error(DiagnosticKind::TypeMismatch {
                                        expected: param.clone(),
                                        found: arg.clone(),
                                    });
                                    let _ = i;
                                }
                            }
                        }
                        *return_type
                    }
                    Type::Dynamic => Type::Dynamic,
                    Type::Error => Type::Error,
                    _ => {
                        self.diagnostics.add_error(DiagnosticKind::NotCallable {
                            ty: callee_ty.clone(),
                        });
                        Type::Error
                    }
                }
            }
            Expr::Member {
                object,
                property,
                optional,
            } => {
                let obj_ty = self.synthesize(object);
                let prop_name = match property {
                    MemberProperty::Ident(name) => name.clone(),
                    MemberProperty::Expr(e) => format!("[{}]", self.synthesize(e)),
                };
                self.synthesize_member_access(&obj_ty, &prop_name, *optional)
            }
            Expr::OptionalCall { callee, args } => {
                let callee_ty = self.synthesize(callee);
                let _ = args;
                match callee_ty {
                    Type::Nullable(inner) => {
                        let result = match *inner {
                            Type::Function { return_type, .. } => *return_type,
                            _ => Type::Dynamic,
                        };
                        result.make_nullable()
                    }
                    Type::Function { return_type, .. } => *return_type,
                    Type::Dynamic => Type::Dynamic,
                    _ => Type::Dynamic,
                }
            }
            Expr::Conditional {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_ty = self.synthesize(condition);
                if !cond_ty.is_consistent_with(&Type::Bool) && !cond_ty.is_dynamic() {
                    self.diagnostics.add_warning(DiagnosticKind::TypeMismatch {
                        expected: Type::Bool,
                        found: cond_ty,
                    });
                }
                let then_ty = self.synthesize(then_branch);
                let else_ty = self.synthesize(else_branch);
                then_ty.least_upper_bound(&else_ty)
            }
            Expr::Assignment { left, op: _, right } => {
                let right_ty = self.synthesize(right);
                match left.as_ref() {
                    Expr::Identifier(name) => {
                        if let Some(existing_ty) = self.env.lookup(name).cloned() {
                            if !right_ty.is_consistent_with(&existing_ty) {
                                self.diagnostics.add_error(DiagnosticKind::TypeMismatch {
                                    expected: existing_ty,
                                    found: right_ty.clone(),
                                });
                            }
                            if !self.env.update(name, right_ty.clone()) {
                                self.diagnostics.add_error(DiagnosticKind::ImmutableAssign {
                                    name: name.clone(),
                                });
                            }
                        } else {
                            self.diagnostics
                                .add_error(DiagnosticKind::UnknownVariable { name: name.clone() });
                        }
                    }
                    Expr::Member {
                        object,
                        property,
                        optional: _,
                    } => {
                        let _ = self.synthesize(object);
                        let _ = property;
                    }
                    _ => {
                        self.synthesize(left);
                    }
                }
                right_ty
            }
            Expr::ArrowFunction {
                params,
                return_type,
                body,
                is_async,
            } => {
                let saved_class_stack = std::mem::take(&mut self.class_stack);
                let param_types: Vec<Type> = params
                    .iter()
                    .map(|p| {
                        p.ty.as_ref()
                            .map(Type::from_annotation)
                            .unwrap_or(Type::Dynamic)
                    })
                    .collect();

                self.env.push_scope();
                for (param, ty) in params.iter().zip(param_types.iter()) {
                    self.env
                        .declare_param(&pattern_name(&param.pattern), ty.clone());
                }

                let ret_type = return_type
                    .as_ref()
                    .map(Type::from_annotation)
                    .unwrap_or_else(|| match body {
                        ArrowBody::Expr(e) => self.synthesize_expr_fresh(e),
                        ArrowBody::Block(stmts) => self.infer_return_type(stmts),
                    });

                if let ArrowBody::Block(stmts) = body {
                    let expected_ret = return_type
                        .as_ref()
                        .map(Type::from_annotation)
                        .unwrap_or_else(|| ret_type.clone());
                    self.return_type_stack.push(expected_ret);
                    for stmt in stmts {
                        let _ = self.infer_statement(stmt);
                    }
                    self.return_type_stack.pop();
                }

                self.env.pop_scope();
                self.class_stack = saved_class_stack;

                let fn_ret_type = if *is_async {
                    Type::Future(Box::new(ret_type))
                } else {
                    ret_type
                };

                Type::Function {
                    params: param_types,
                    return_type: Box::new(fn_ret_type),
                }
            }
            Expr::Await(inner) => {
                let inner_ty = self.synthesize(inner);
                match inner_ty {
                    Type::Future(inner) => *inner,
                    Type::Generic { base, args } if base == "Future" && args.len() == 1 => {
                        args[0].clone()
                    }
                    Type::Dynamic => Type::Dynamic,
                    _ => {
                        self.diagnostics.add_error(DiagnosticKind::TypeMismatch {
                            expected: Type::Future(Box::new(Type::Dynamic)),
                            found: inner_ty.clone(),
                        });
                        Type::Error
                    }
                }
            }
            Expr::Sequence(exprs) => {
                if exprs.len() > 1 {
                    Type::Tuple(exprs.iter().map(|e| self.synthesize(e)).collect())
                } else if let Some(e) = exprs.last() {
                    self.synthesize(e)
                } else {
                    Type::Void
                }
            }
            Expr::Function {
                name: _,
                type_params: _,
                params,
                return_type,
                body,
                is_async,
            } => {
                let saved_class_stack = std::mem::take(&mut self.class_stack);
                let param_types: Vec<Type> = params
                    .iter()
                    .map(|p| {
                        p.ty.as_ref()
                            .map(Type::from_annotation)
                            .unwrap_or(Type::Dynamic)
                    })
                    .collect();
                let ret_type = return_type
                    .as_ref()
                    .map(Type::from_annotation)
                    .unwrap_or_else(|| self.infer_return_type(body));

                self.env.push_scope();
                for (param, ty) in params.iter().zip(param_types.iter()) {
                    self.env
                        .declare_param(&pattern_name(&param.pattern), ty.clone());
                }
                let expected_ret = return_type
                    .as_ref()
                    .map(Type::from_annotation)
                    .unwrap_or_else(|| ret_type.clone());
                self.return_type_stack.push(expected_ret);
                for stmt in body {
                    let _ = self.infer_statement(stmt);
                }
                self.return_type_stack.pop();
                self.env.pop_scope();
                self.class_stack = saved_class_stack;

                let fn_ret_type = if *is_async {
                    Type::Future(Box::new(ret_type))
                } else {
                    ret_type
                };

                Type::Function {
                    params: param_types,
                    return_type: Box::new(fn_ret_type),
                }
            }
            Expr::Class { .. } => Type::Dynamic,
            Expr::New { callee, args: _ } => {
                let callee_ty = self.synthesize(callee);
                match callee_ty {
                    Type::Named(name, fields) => Type::Named(name, fields),
                    Type::Generic { base, .. } => Type::Named(base, vec![]),
                    _ => Type::Dynamic,
                }
            }
            Expr::Match { value, arms } => {
                let match_ty = self.synthesize(value);
                let mut result_types = Vec::new();
                for arm in arms {
                    self.env.push_scope();
                    self.bind_pattern_type(&arm.pattern, &match_ty);
                    let arm_type = self.synthesize_expr_fresh(&Expr::Block(arm.body.clone()));
                    self.env.pop_scope();
                    result_types.push(arm_type);
                }
                result_types
                    .into_iter()
                    .fold(Type::Dynamic, |acc, ty| acc.least_upper_bound(&ty))
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let _ = self.synthesize(condition);
                let then_ty = self.synthesize(then_branch);
                let else_ty = else_branch
                    .as_ref()
                    .map(|e| self.synthesize(e))
                    .unwrap_or(Type::Void);
                then_ty.least_upper_bound(&else_ty)
            }
            Expr::Grouping(inner) => self.synthesize(inner),
            Expr::Block(stmts) => self.infer_block_return_type(stmts),
            Expr::ArrowParams(_) => {
                unreachable!(
                    "ArrowParams should be converted to ArrowFunction before typechecking"
                );
            }
        }
    }

    /// Check mode: verify an expression has an expected type.
    pub fn check(&mut self, expr: &Expr, expected: &Type) -> Type {
        let synthesized = self.synthesize(expr);

        if expected.is_dynamic() {
            self.diagnostics.add_warning(DiagnosticKind::DynCast {
                from: synthesized.clone(),
                to: expected.clone(),
            });
            return expected.clone();
        }

        // Special case: concrete type -> dyn Trait coercion
        // Allow concrete-to-trait assignment when the concrete type implements the trait
        if let Type::Trait(trait_name) = expected {
            if self.trait_registry.implements(&synthesized, trait_name) {
                return synthesized;
            }
        }

        if !synthesized.is_consistent_with(expected) {
            self.diagnostics.add_error(DiagnosticKind::TypeMismatch {
                expected: expected.clone(),
                found: synthesized.clone(),
            });
        }

        synthesized
    }

    fn synthesize_binary(&mut self, op: &BinaryOp, left_ty: Type, right_ty: Type) -> Type {
        match op {
            // Arithmetic: int + int = int, int + float = float, otherwise dyn
            BinaryOp::Plus
            | BinaryOp::Minus
            | BinaryOp::Star
            | BinaryOp::Percent
            | BinaryOp::Power => {
                if left_ty == Type::Int && right_ty == Type::Int {
                    Type::Int
                } else if left_ty == Type::String || right_ty == Type::String {
                    if matches!(op, BinaryOp::Plus) {
                        Type::String
                    } else {
                        Type::Dynamic
                    }
                } else if (left_ty == Type::Int || left_ty == Type::Float)
                    && (right_ty == Type::Int || right_ty == Type::Float)
                {
                    Type::Float
                } else if left_ty.is_dynamic() || right_ty.is_dynamic() {
                    Type::Dynamic
                } else {
                    self.diagnostics.add_error(DiagnosticKind::TypeMismatch {
                        expected: Type::Float,
                        found: left_ty,
                    });
                    Type::Error
                }
            }
            BinaryOp::Slash => {
                if left_ty == Type::Int && right_ty == Type::Int {
                    Type::Int
                } else if (left_ty == Type::Int || left_ty == Type::Float)
                    && (right_ty == Type::Int || right_ty == Type::Float)
                {
                    Type::Float
                } else if left_ty.is_dynamic() || right_ty.is_dynamic() {
                    Type::Dynamic
                } else {
                    Type::Error
                }
            }
            // Comparison: always bool
            BinaryOp::StrictEquals
            | BinaryOp::StrictNotEquals
            | BinaryOp::Less
            | BinaryOp::Greater
            | BinaryOp::LessEq
            | BinaryOp::GreaterEq => Type::Bool,
            // Equality with coercion (deprecated in Ruyi, but handled)
            BinaryOp::Equals | BinaryOp::NotEquals => Type::Bool,
            // Logical: bool
            BinaryOp::And | BinaryOp::Or => left_ty.least_upper_bound(&right_ty),
            // Nullish coalescing
            BinaryOp::Nullish => {
                let non_null = left_ty.non_null();
                non_null.least_upper_bound(&right_ty)
            }
            // Bitwise: int
            BinaryOp::Amp
            | BinaryOp::Pipe
            | BinaryOp::Caret
            | BinaryOp::Shl
            | BinaryOp::Shr
            | BinaryOp::UShr => {
                if left_ty == Type::Int && right_ty == Type::Int {
                    Type::Int
                } else if left_ty.is_dynamic() || right_ty.is_dynamic() {
                    Type::Dynamic
                } else {
                    Type::Error
                }
            }
            // Instanceof, in
            BinaryOp::In | BinaryOp::Instanceof => Type::Bool,
        }
    }

    fn synthesize_unary(&mut self, op: &UnaryOp, operand_ty: Type) -> Type {
        match op {
            UnaryOp::Not => Type::Bool,
            UnaryOp::Minus => match operand_ty {
                Type::Int => Type::Int,
                Type::Float => Type::Float,
                Type::Dynamic => Type::Dynamic,
                _ => {
                    self.diagnostics.add_error(DiagnosticKind::TypeMismatch {
                        expected: Type::Float,
                        found: operand_ty,
                    });
                    Type::Error
                }
            },
            UnaryOp::Plus => match operand_ty {
                Type::Int => Type::Int,
                Type::Float => Type::Float,
                Type::Dynamic => Type::Dynamic,
                _ => Type::Error,
            },
            UnaryOp::Tilde => match operand_ty {
                Type::Int => Type::Int,
                Type::Dynamic => Type::Dynamic,
                _ => Type::Error,
            },
            UnaryOp::PreIncrement | UnaryOp::PreDecrement => match operand_ty {
                Type::Int => Type::Int,
                Type::Float => Type::Float,
                Type::Dynamic => Type::Dynamic,
                _ => Type::Error,
            },
            UnaryOp::Typeof => Type::String,
            UnaryOp::Void => Type::Void,
            UnaryOp::Delete => Type::Bool,
            UnaryOp::Await => match operand_ty {
                Type::Future(inner) => *inner,
                Type::Dynamic => Type::Dynamic,
                _ => {
                    self.diagnostics.add_error(DiagnosticKind::TypeMismatch {
                        expected: Type::Future(Box::new(Type::Dynamic)),
                        found: operand_ty.clone(),
                    });
                    Type::Error
                }
            },
        }
    }

    fn substitute_self_type(&self, ty: &Type, self_type: &Type) -> Type {
        match ty {
            Type::Named(name, _) if name == "Self" || name == "self" => self_type.clone(),
            Type::Nullable(inner) => {
                Type::Nullable(Box::new(self.substitute_self_type(inner, self_type)))
            }
            Type::Function {
                params,
                return_type,
            } => Type::Function {
                params: params
                    .iter()
                    .map(|p| self.substitute_self_type(p, self_type))
                    .collect(),
                return_type: Box::new(self.substitute_self_type(return_type, self_type)),
            },
            Type::Array(inner) => {
                Type::Array(Box::new(self.substitute_self_type(inner, self_type)))
            }
            Type::Future(inner) => {
                Type::Future(Box::new(self.substitute_self_type(inner, self_type)))
            }
            Type::Object(fields) => Type::Object(
                fields
                    .iter()
                    .map(|f| ObjectField {
                        name: f.name.clone(),
                        ty: self.substitute_self_type(&f.ty, self_type),
                        optional: f.optional,
                    })
                    .collect(),
            ),
            Type::Generic { base, args } => Type::Generic {
                base: base.clone(),
                args: args
                    .iter()
                    .map(|a| self.substitute_self_type(a, self_type))
                    .collect(),
            },
            other => other.clone(),
        }
    }

    fn synthesize_member_access(&mut self, obj_ty: &Type, prop_name: &str, optional: bool) -> Type {
        // Check for unsafe nullable access (non-optional member access on nullable type)
        if !optional && obj_ty.is_nullable() && !obj_ty.is_dynamic() {
            self.diagnostics
                .add_error(DiagnosticKind::UnsafeNullableAccess { ty: obj_ty.clone() });
        }

        let result = match obj_ty {
            Type::Tuple(types) => {
                if let Ok(index) = prop_name.parse::<usize>() {
                    types.get(index).cloned().unwrap_or(Type::Dynamic)
                } else {
                    Type::Dynamic
                }
            }
            Type::Object(fields) => fields
                .iter()
                .find(|f| f.name == prop_name)
                .map(|f| f.ty.clone())
                .unwrap_or(Type::Dynamic),
            Type::Named(ref type_name, _) => {
                if let Some(field_ty) = self
                    .class_fields
                    .get(type_name)
                    .and_then(|fields| fields.iter().find(|f| f.name == prop_name))
                    .map(|f| f.ty.clone())
                {
                    field_ty
                } else if let Some(method_ty) = self
                    .class_methods
                    .get(type_name)
                    .and_then(|methods| methods.iter().find(|m| m.name == prop_name))
                    .map(|m| m.ty.clone())
                {
                    // Class's own method: bind `self` to the class type so
                    // `instance.method` becomes a function value
                    // `function (self: Class, ...) -> ret_type`.
                    if let Type::Function { params, return_type } = method_ty {
                        let new_params: Vec<Type> = params
                            .iter()
                            .enumerate()
                            .map(|(idx, p)| {
                                if idx == 0 {
                                    obj_ty.clone()
                                } else {
                                    self.substitute_self_type(p, obj_ty)
                                }
                            })
                            .collect();
                        let new_ret = self.substitute_self_type(&return_type, obj_ty);
                        Type::Function {
                            params: new_params,
                            return_type: Box::new(new_ret),
                        }
                    } else {
                        method_ty
                    }
                } else if let Some((_trait_name, method)) = self
                    .trait_registry
                    .resolve_impl_method(type_name, prop_name)
                {
                    let ret_ty = self.substitute_self_type(&method.return_type, obj_ty);
                    let param_types: Vec<Type> = if method.param_types.len() >= 1 {
                        method.param_types[1..]
                            .iter()
                            .map(|p| self.substitute_self_type(p, obj_ty))
                            .collect()
                    } else {
                        vec![]
                    };
                    Type::Function {
                        params: param_types,
                        return_type: Box::new(ret_ty),
                    }
                } else {
                    Type::Dynamic
                }
            }
            Type::Array(_) => Type::Dynamic,
            Type::Generic { .. } => Type::Dynamic,
            Type::Dynamic => Type::Dynamic,
            Type::Error => Type::Error,
            _ => {
                self.diagnostics
                    .add_warning(DiagnosticKind::NotIndexable { ty: obj_ty.clone() });
                Type::Dynamic
            }
        };

        if optional {
            result.make_nullable()
        } else {
            result
        }
    }

    /// Narrows types based on a condition expression.
    fn narrow_for_condition(&mut self, condition: &Expr, true_branch: bool) {
        match condition {
            Expr::Binary { op, left, right } => {
                match op {
                    BinaryOp::StrictEquals | BinaryOp::Equals => {
                        if true_branch {
                            // x === null narrows x to null in true branch
                            if let Expr::NullLiteral = right.as_ref() {
                                if let Expr::Identifier(name) = left.as_ref() {
                                    self.env.narrow(name, Type::Null);
                                }
                            }
                            if let Expr::NullLiteral = left.as_ref() {
                                if let Expr::Identifier(name) = right.as_ref() {
                                    self.env.narrow(name, Type::Null);
                                }
                            }
                        }
                        // typeof x === "type" narrowing
                        if let Expr::Unary {
                            op: UnaryOp::Typeof,
                            operand,
                        } = left.as_ref()
                        {
                            if let Expr::StringLiteral(type_str) = right.as_ref() {
                                if let Expr::Identifier(name) = operand.as_ref() {
                                    let narrowed = match type_str.as_str() {
                                        "int" | "i64" => Type::Int,
                                        "float" | "f64" => Type::Float,
                                        "bool" => Type::Bool,
                                        "string" => Type::String,
                                        "bigint" => Type::BigInt,
                                        _ => return,
                                    };
                                    if true_branch {
                                        self.env.narrow(name, narrowed);
                                    }
                                }
                            }
                        }
                        // Handle reversed: "type" === typeof x
                        if let Expr::Unary {
                            op: UnaryOp::Typeof,
                            operand,
                        } = right.as_ref()
                        {
                            if let Expr::StringLiteral(type_str) = left.as_ref() {
                                if let Expr::Identifier(name) = operand.as_ref() {
                                    let narrowed = match type_str.as_str() {
                                        "int" | "i64" => Type::Int,
                                        "float" | "f64" => Type::Float,
                                        "bool" => Type::Bool,
                                        "string" => Type::String,
                                        "bigint" => Type::BigInt,
                                        _ => return,
                                    };
                                    if true_branch {
                                        self.env.narrow(name, narrowed);
                                    }
                                }
                            }
                        }
                    }
                    BinaryOp::StrictNotEquals | BinaryOp::NotEquals => {
                        if true_branch {
                            // x !== null narrows x to non-null in true branch
                            if let Expr::NullLiteral = right.as_ref() {
                                if let Expr::Identifier(name) = left.as_ref() {
                                    if let Some(ty) = self.env.lookup(name).cloned() {
                                        self.env.narrow(name, ty.non_null());
                                    }
                                }
                            }
                            if let Expr::NullLiteral = left.as_ref() {
                                if let Expr::Identifier(name) = right.as_ref() {
                                    if let Some(ty) = self.env.lookup(name).cloned() {
                                        self.env.narrow(name, ty.non_null());
                                    }
                                }
                            }
                        }
                    }
                    BinaryOp::Instanceof => {
                        if let Expr::Identifier(name) = left.as_ref() {
                            if let Expr::Identifier(class_name) = right.as_ref() {
                                if true_branch {
                                    self.env
                                        .narrow(name, Type::Named(class_name.clone(), vec![]));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Expr::Unary { op, operand } => {
                if matches!(op, UnaryOp::Not) && !true_branch {
                    // !expr in false branch => narrow for expr being true
                    self.narrow_for_condition(operand, true);
                }
            }
            _ => {}
        }
    }

    fn bind_pattern_type(&mut self, pattern: &Pattern, ty: &Type) {
        match pattern {
            Pattern::Identifier(name) => {
                self.env.declare_let(name, ty.clone());
            }
            Pattern::Wildcard => {}
            Pattern::Literal(_) => {}
            Pattern::Object(fields) => {
                let obj_fields = match ty {
                    Type::Object(f) => Some(f),
                    Type::Named(_, _) => None,
                    _ => return,
                };

                for field in fields {
                    match field {
                        crate::parser::ast::ObjectPatternField::Property {
                            key,
                            pattern: inner,
                        } => {
                            let field_ty = obj_fields
                                .map(|f| {
                                    f.iter()
                                        .find(|f| f.name == *key)
                                        .map(|f| f.ty.clone())
                                        .unwrap_or(Type::Dynamic)
                                })
                                .unwrap_or(Type::Dynamic);
                            self.bind_pattern_type(inner, &field_ty);
                        }
                        crate::parser::ast::ObjectPatternField::Shorthand(name) => {
                            let field_ty = obj_fields
                                .map(|f| {
                                    f.iter()
                                        .find(|f| f.name == *name)
                                        .map(|f| f.ty.clone())
                                        .unwrap_or(Type::Dynamic)
                                })
                                .unwrap_or(Type::Dynamic);
                            self.env.declare_let(name, field_ty);
                        }
                        crate::parser::ast::ObjectPatternField::Rest(_) => {}
                    }
                }
            }
            Pattern::Array(elements) => {
                if let Type::Array(elem_ty) = ty {
                    for (i, elem) in elements.iter().enumerate() {
                        match elem {
                            crate::parser::ast::ArrayPatternElement::Pattern(p) => {
                                self.bind_pattern_type(p, elem_ty);
                            }
                            crate::parser::ast::ArrayPatternElement::Rest(p) => {
                                self.bind_pattern_type(p, &Type::Array(elem_ty.clone()));
                            }
                            crate::parser::ast::ArrayPatternElement::Elision => {}
                        }
                        let _ = i;
                    }
                }
            }
            Pattern::Rest(name) => {
                self.env.declare_let(name, ty.clone());
            }
            Pattern::As(inner, _alias) => {
                self.bind_pattern_type(inner, ty);
            }
            Pattern::Or(patterns) => {
                if let Some(first) = patterns.first() {
                    self.bind_pattern_type(first, ty);
                }
            }
        }
    }

    fn infer_return_type(&mut self, body: &[Statement]) -> Type {
        let mut return_types = Vec::new();
        self.collect_return_types(body, &mut return_types);
        if return_types.is_empty() {
            Type::Void
        } else {
            return_types
                .into_iter()
                .fold(Type::Never, |acc, ty| acc.least_upper_bound(&ty))
        }
    }

    fn collect_return_types(&mut self, stmts: &[Statement], return_types: &mut Vec<Type>) {
        for stmt in stmts {
            match stmt {
                Statement::Return(expr) => {
                    let ty = expr
                        .as_ref()
                        .map(|e| self.synthesize(e))
                        .unwrap_or(Type::Null);
                    return_types.push(ty);
                }
                Statement::If {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    self.collect_return_types_from_stmt(then_branch, return_types);
                    if let Some(else_stmt) = else_branch {
                        self.collect_return_types_from_stmt(else_stmt, return_types);
                    }
                }
                Statement::Block(inner_stmts) => {
                    self.collect_return_types(inner_stmts, return_types);
                }
                Statement::Match { arms, .. } => {
                    for arm in arms {
                        self.collect_return_types(&arm.body, return_types);
                    }
                }
                _ => {}
            }
        }
    }

    fn collect_return_types_from_stmt(&mut self, stmt: &Statement, return_types: &mut Vec<Type>) {
        match stmt {
            Statement::Block(stmts) => self.collect_return_types(stmts, return_types),
            Statement::Return(expr) => {
                let ty = expr
                    .as_ref()
                    .map(|e| self.synthesize(e))
                    .unwrap_or(Type::Null);
                return_types.push(ty);
            }
            _ => {}
        }
    }

    fn infer_block_return_type(&mut self, stmts: &[Statement]) -> Type {
        let mut last_ty = Type::Void;
        for stmt in stmts {
            last_ty = self.infer_statement(stmt);
        }
        last_ty
    }

    fn synthesize_expr_fresh(&mut self, expr: &Expr) -> Type {
        self.synthesize(expr)
    }
}

/// Extracts the variable name from a pattern (for declaration purposes).
fn pattern_name(pattern: &Pattern) -> String {
    match pattern {
        Pattern::Identifier(name) => name.clone(),
        Pattern::Wildcard => "_".into(),
        _ => "pattern".into(),
    }
}

/// Extracts a string from a PropertyName.
fn property_name_str(name: &PropertyName) -> String {
    match name {
        PropertyName::Ident(s) => s.clone(),
        PropertyName::String(s) => s.clone(),
        PropertyName::Number(n) => format!("{}", n),
        PropertyName::Computed(_) => "[computed]".into(),
    }
}

impl Default for TypeInference {
    fn default() -> Self {
        Self::new(TraitRegistry::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    fn infer_type(source: &str) -> Type {
        let mut parser = Parser::new(source).expect("lexer should not fail");
        let program = parser.parse().expect("parse should succeed");
        let mut inference = TypeInference::new(TraitRegistry::new());
        let result = inference.infer_program(&program);
        // Return the type of the first declaration's variable
        result
            .typed_env
            .lookup("x")
            .cloned()
            .unwrap_or(Type::Dynamic)
    }

    #[test]
    fn test_infer_int_literal() {
        let ty = infer_type("let x = 42;");
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_infer_float_literal() {
        let ty = infer_type("let x = 3.14;");
        assert_eq!(ty, Type::Float);
    }

    #[test]
    fn test_infer_string_literal() {
        let ty = infer_type("let x = \"hello\";");
        assert_eq!(ty, Type::String);
    }

    #[test]
    fn test_infer_bool_literal() {
        let ty = infer_type("let x = true;");
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_infer_null_literal() {
        let ty = infer_type("let x = null;");
        assert_eq!(ty, Type::Null);
    }

    #[test]
    fn test_infer_typed_annotation() {
        let ty = infer_type("let x: int = 42;");
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_infer_dyn_default() {
        let ty = infer_type("let x;");
        assert_eq!(ty, Type::Dynamic);
    }

    #[test]
    fn test_infer_addition() {
        let ty = infer_type("let x = 1 + 2;");
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_infer_int_float_addition() {
        let ty = infer_type("let x = 1 + 2.0;");
        assert_eq!(ty, Type::Float);
    }

    #[test]
    fn test_infer_string_concat() {
        let ty = infer_type("let x = \"hello\" + \" world\";");
        assert_eq!(ty, Type::String);
    }

    #[test]
    fn test_infer_comparison() {
        let ty = infer_type("let x = 1 === 2;");
        assert_eq!(ty, Type::Bool);
    }
}
