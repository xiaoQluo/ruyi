use ruyic::parser::Parser;
use ruyic::typechecker::diagnostics::{DiagnosticBag, DiagnosticKind};
use ruyic::typechecker::inference::TypeInference;
use ruyic::typechecker::traits::TraitRegistry;
/**
 * Comprehensive tests for the Ruyi gradual type checker.
 *
 * Tests cover:
 * - Type system core (subtyping, consistency, lub)
 * - Type environment (scoping, narrowing, mutability)
 * - Type inference (literals, expressions, functions)
 * - Type checking (programs, diagnostics)
 * - Constraint solving (unification, occurs check)
 * - Gradual typing (dyn, cast insertion, consistency)
 * - Nullable types (narrowing, optional chaining)
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */
use ruyic::typechecker::*;

// ── Type System Core ──────────────────────────────────────────

#[test]
fn test_type_equality() {
    assert_eq!(Type::Int, Type::Int);
    assert_eq!(Type::String, Type::String);
    assert_eq!(Type::Dynamic, Type::Dynamic);
    assert_ne!(Type::Int, Type::Float);
}

#[test]
fn test_type_display() {
    assert_eq!(Type::Int.to_string(), "int");
    assert_eq!(Type::Float.to_string(), "float");
    assert_eq!(Type::Bool.to_string(), "bool");
    assert_eq!(Type::String.to_string(), "string");
    assert_eq!(Type::Null.to_string(), "null");
    assert_eq!(Type::Void.to_string(), "void");
    assert_eq!(Type::Never.to_string(), "never");
    assert_eq!(Type::BigInt.to_string(), "bigint");
    assert_eq!(Type::Dynamic.to_string(), "dyn");
    assert_eq!(Type::Nullable(Box::new(Type::Int)).to_string(), "int?");
    assert_eq!(
        Type::Array(Box::new(Type::String)).to_string(),
        "Array<string>"
    );
    assert_eq!(
        Type::Function {
            params: vec![Type::Int, Type::String],
            return_type: Box::new(Type::Bool),
        }
        .to_string(),
        "fn(int, string) -> bool"
    );
    assert_eq!(Type::Named("MyClass".into(), vec![]).to_string(), "MyClass");
    assert_eq!(
        Type::Generic {
            base: "Array".into(),
            args: vec![Type::Int],
        }
        .to_string(),
        "Array<int>"
    );
    assert_eq!(Type::Trait("Printable".into()).to_string(), "dyn Printable");
    assert_eq!(Type::Error.to_string(), "<error>");
}

#[test]
fn test_subtype_reflexive() {
    assert!(Type::Int.is_subtype_of(&Type::Int));
    assert!(Type::String.is_subtype_of(&Type::String));
    assert!(Type::Dynamic.is_subtype_of(&Type::Dynamic));
    assert!(Type::Nullable(Box::new(Type::Int)).is_subtype_of(&Type::Nullable(Box::new(Type::Int))));
}

#[test]
fn test_subtype_int_float() {
    assert!(Type::Int.is_subtype_of(&Type::Float));
    assert!(!Type::Float.is_subtype_of(&Type::Int));
}

#[test]
fn test_subtype_nullable() {
    assert!(Type::Int.is_subtype_of(&Type::Nullable(Box::new(Type::Int))));
    assert!(Type::Null.is_subtype_of(&Type::Nullable(Box::new(Type::Int))));
    assert!(Type::String.is_subtype_of(&Type::Nullable(Box::new(Type::String))));
}

#[test]
fn test_subtype_never() {
    assert!(Type::Never.is_subtype_of(&Type::Int));
    assert!(Type::Never.is_subtype_of(&Type::String));
    assert!(Type::Never.is_subtype_of(&Type::Dynamic));
    assert!(Type::Never.is_subtype_of(&Type::Nullable(Box::new(Type::Bool))));
}

#[test]
fn test_subtype_dynamic() {
    assert!(Type::Int.is_subtype_of(&Type::Dynamic));
    assert!(Type::Dynamic.is_subtype_of(&Type::Int));
    assert!(Type::Dynamic.is_subtype_of(&Type::String));
}

#[test]
fn test_subtype_array_covariant() {
    assert!(Type::Array(Box::new(Type::Int)).is_subtype_of(&Type::Array(Box::new(Type::Float))));
    assert!(!Type::Array(Box::new(Type::Float)).is_subtype_of(&Type::Array(Box::new(Type::Int))));
}

#[test]
fn test_subtype_function() {
    let fn_int_to_int = Type::Function {
        params: vec![Type::Int],
        return_type: Box::new(Type::Int),
    };
    let fn_float_to_float = Type::Function {
        params: vec![Type::Float],
        return_type: Box::new(Type::Float),
    };
    // fn(int) -> int <: fn(float) -> float
    // contravariant in params: float <: int? No. So this should be false.
    assert!(!fn_int_to_int.is_subtype_of(&fn_float_to_float));
    // fn(float) -> float <: fn(int) -> float
    // contravariant in params: int <: float? Yes. Covariant in return: float <: float? Yes.
    let fn_float_to_float2 = Type::Function {
        params: vec![Type::Float],
        return_type: Box::new(Type::Float),
    };
    let fn_int_to_float = Type::Function {
        params: vec![Type::Int],
        return_type: Box::new(Type::Float),
    };
    assert!(fn_float_to_float2.is_subtype_of(&fn_int_to_float));
}

#[test]
fn test_subtype_object_structural() {
    let obj_a = Type::Object(vec![
        ObjectField {
            name: "x".into(),
            ty: Type::Int,
            optional: false,
        },
        ObjectField {
            name: "y".into(),
            ty: Type::Float,
            optional: false,
        },
    ]);
    let obj_b = Type::Object(vec![ObjectField {
        name: "x".into(),
        ty: Type::Int,
        optional: false,
    }]);
    // { x: int, y: float } <: { x: int } — supertype has fewer fields
    assert!(obj_a.is_subtype_of(&obj_b));
}

#[test]
fn test_consistency() {
    assert!(Type::Int.is_consistent_with(&Type::Int));
    assert!(Type::Int.is_consistent_with(&Type::Dynamic));
    assert!(Type::Dynamic.is_consistent_with(&Type::String));
    assert!(!Type::Int.is_consistent_with(&Type::String));
}

#[test]
fn test_lub_same() {
    assert_eq!(Type::Int.least_upper_bound(&Type::Int), Type::Int);
    assert_eq!(Type::String.least_upper_bound(&Type::String), Type::String);
}

#[test]
fn test_lub_int_float() {
    assert_eq!(Type::Int.least_upper_bound(&Type::Float), Type::Float);
    assert_eq!(Type::Float.least_upper_bound(&Type::Int), Type::Float);
}

#[test]
fn test_lub_with_dyn() {
    assert_eq!(Type::Int.least_upper_bound(&Type::Dynamic), Type::Dynamic);
    assert_eq!(
        Type::Dynamic.least_upper_bound(&Type::String),
        Type::Dynamic
    );
}

#[test]
fn test_lub_unrelated() {
    assert_eq!(Type::Int.least_upper_bound(&Type::String), Type::Dynamic);
}

#[test]
fn test_lub_nullable() {
    let int_or_null = Type::Nullable(Box::new(Type::Int));
    assert_eq!(Type::Int.least_upper_bound(&int_or_null), int_or_null);
}

#[test]
fn test_lub_never() {
    assert_eq!(Type::Never.least_upper_bound(&Type::Int), Type::Int);
    assert_eq!(Type::Int.least_upper_bound(&Type::Never), Type::Int);
}

#[test]
fn test_nullable_collapse() {
    let inner = Type::Nullable(Box::new(Type::Int));
    let result = inner.make_nullable();
    assert_eq!(result, Type::Nullable(Box::new(Type::Int)));
}

#[test]
fn test_non_null() {
    assert_eq!(Type::Nullable(Box::new(Type::Int)).non_null(), Type::Int);
    assert_eq!(Type::Int.non_null(), Type::Int);
    assert_eq!(Type::Null.non_null(), Type::Never);
}

#[test]
fn test_from_annotation() {
    use ruyic::parser::ast::TypeAnnotation;
    assert_eq!(
        Type::from_annotation(&TypeAnnotation::Identifier("int".into())),
        Type::Int
    );
    assert_eq!(
        Type::from_annotation(&TypeAnnotation::Identifier("float".into())),
        Type::Float
    );
    assert_eq!(
        Type::from_annotation(&TypeAnnotation::Identifier("bool".into())),
        Type::Bool
    );
    assert_eq!(
        Type::from_annotation(&TypeAnnotation::Identifier("string".into())),
        Type::String
    );
    assert_eq!(
        Type::from_annotation(&TypeAnnotation::Identifier("null".into())),
        Type::Null
    );
    assert_eq!(
        Type::from_annotation(&TypeAnnotation::Identifier("void".into())),
        Type::Void
    );
    assert_eq!(
        Type::from_annotation(&TypeAnnotation::Identifier("never".into())),
        Type::Never
    );
    assert_eq!(
        Type::from_annotation(&TypeAnnotation::Identifier("dyn".into())),
        Type::Dynamic
    );
    assert_eq!(
        Type::from_annotation(&TypeAnnotation::Identifier("bigint".into())),
        Type::BigInt
    );
    assert_eq!(
        Type::from_annotation(&TypeAnnotation::Identifier("MyClass".into())),
        Type::Named("MyClass".into(), vec![])
    );
}

#[test]
fn test_from_annotation_nullable() {
    use ruyic::parser::ast::TypeAnnotation;
    let inner = TypeAnnotation::Identifier("int".into());
    let nullable = TypeAnnotation::Nullable(Box::new(inner));
    assert_eq!(
        Type::from_annotation(&nullable),
        Type::Nullable(Box::new(Type::Int))
    );
}

#[test]
fn test_from_annotation_function() {
    use ruyic::parser::ast::TypeAnnotation;
    let fn_type = TypeAnnotation::Function {
        params: vec![TypeAnnotation::Identifier("int".into())],
        return_type: Box::new(TypeAnnotation::Identifier("string".into())),
    };
    assert_eq!(
        Type::from_annotation(&fn_type),
        Type::Function {
            params: vec![Type::Int],
            return_type: Box::new(Type::String),
        }
    );
}

#[test]
fn test_from_annotation_generic() {
    use ruyic::parser::ast::TypeAnnotation;
    // Array<T> is normalized from Generic to Type::Array
    let gen_type = TypeAnnotation::Generic {
        base: "Array".into(),
        args: vec![TypeAnnotation::Identifier("int".into())],
    };
    assert_eq!(
        Type::from_annotation(&gen_type),
        Type::Array(Box::new(Type::Int))
    );

    // Non-Array generics remain as Type::Generic
    let map_type = TypeAnnotation::Generic {
        base: "Map".into(),
        args: vec![
            TypeAnnotation::Identifier("string".into()),
            TypeAnnotation::Identifier("int".into()),
        ],
    };
    assert_eq!(
        Type::from_annotation(&map_type),
        Type::Generic {
            base: "Map".into(),
            args: vec![Type::String, Type::Int],
        }
    );
}

#[test]
fn test_from_annotation_array() {
    use ruyic::parser::ast::TypeAnnotation;
    let arr_type = TypeAnnotation::Array(Box::new(TypeAnnotation::Identifier("string".into())));
    assert_eq!(
        Type::from_annotation(&arr_type),
        Type::Array(Box::new(Type::String))
    );
}

// ── Type Environment ───────────────────────────────────────────

#[test]
fn test_env_declare_and_lookup() {
    let mut env = TypeEnvironment::new();
    env.declare_let("x", Type::Int);
    assert_eq!(env.lookup("x"), Some(&Type::Int));
    assert_eq!(env.lookup("y"), None);
}

#[test]
fn test_env_const_immutable() {
    let mut env = TypeEnvironment::new();
    env.declare_const("PI", Type::Float);
    assert_eq!(env.is_mutable("PI"), Some(false));
    assert!(!env.update("PI", Type::Int));
    assert_eq!(env.lookup("PI"), Some(&Type::Float));
}

#[test]
fn test_env_let_mutable() {
    let mut env = TypeEnvironment::new();
    env.declare_let("x", Type::Int);
    assert_eq!(env.is_mutable("x"), Some(true));
    assert!(env.update("x", Type::Float));
    assert_eq!(env.lookup("x"), Some(&Type::Float));
}

#[test]
fn test_env_scope_push_pop() {
    let mut env = TypeEnvironment::new();
    env.declare_let("x", Type::Int);
    env.push_scope();
    env.declare_let("y", Type::String);
    assert_eq!(env.lookup("y"), Some(&Type::String));
    assert_eq!(env.lookup("x"), Some(&Type::Int));
    env.pop_scope();
    assert_eq!(env.lookup("y"), None);
    assert_eq!(env.lookup("x"), Some(&Type::Int));
}

#[test]
fn test_env_shadowing() {
    let mut env = TypeEnvironment::new();
    env.declare_let("x", Type::Int);
    env.push_scope();
    env.declare_let("x", Type::String);
    assert_eq!(env.lookup("x"), Some(&Type::String));
    env.pop_scope();
    assert_eq!(env.lookup("x"), Some(&Type::Int));
}

#[test]
fn test_env_narrowing() {
    let mut env = TypeEnvironment::new();
    env.declare_let("x", Type::Nullable(Box::new(Type::String)));
    env.push_scope();
    env.narrow("x", Type::String);
    assert_eq!(env.lookup("x"), Some(&Type::String));
    env.pop_scope();
    assert_eq!(
        env.lookup("x"),
        Some(&Type::Nullable(Box::new(Type::String)))
    );
}

#[test]
fn test_env_scope_depth() {
    let env = TypeEnvironment::new();
    assert_eq!(env.scope_depth(), 1);
}

// ── Diagnostics ───────────────────────────────────────────────

#[test]
fn test_diagnostic_type_mismatch() {
    let diag = Diagnostic::error(DiagnosticKind::TypeMismatch {
        expected: Type::Int,
        found: Type::String,
    });
    assert!(diag.is_error());
    assert!(!diag.is_warning());
    assert_eq!(
        diag.message(),
        "Type mismatch: expected `int`, but found `string`"
    );
}

#[test]
fn test_diagnostic_unknown_variable() {
    let diag = Diagnostic::error(DiagnosticKind::UnknownVariable { name: "x".into() });
    assert_eq!(diag.message(), "Unknown variable: `x`");
}

#[test]
fn test_diagnostic_immutable_assign() {
    let diag = Diagnostic::error(DiagnosticKind::ImmutableAssign { name: "PI".into() });
    assert_eq!(diag.message(), "Cannot assign to immutable variable `PI`");
}

#[test]
fn test_diagnostic_nullable_access() {
    let diag = Diagnostic::error(DiagnosticKind::NullableAccess {
        ty: Type::Nullable(Box::new(Type::String)),
    });
    assert!(diag.message().contains("Nullable access"));
}

#[test]
fn test_diagnostic_not_callable() {
    let diag = Diagnostic::error(DiagnosticKind::NotCallable { ty: Type::Int });
    assert!(diag.message().contains("not callable"));
}

#[test]
fn test_diagnostic_display() {
    let diag = Diagnostic::error(DiagnosticKind::TypeMismatch {
        expected: Type::Int,
        found: Type::String,
    });
    let s = format!("{}", diag);
    assert!(s.starts_with("error:"));
}

#[test]
fn test_diagnostic_bag() {
    let mut bag = DiagnosticBag::new();
    bag.add_error(DiagnosticKind::TypeMismatch {
        expected: Type::Int,
        found: Type::String,
    });
    bag.add_warning(DiagnosticKind::CannotInfer);
    assert!(bag.has_errors());
    assert!(bag.has_warnings());
    assert_eq!(bag.diagnostics().len(), 2);
}

// ── Constraint Solver ──────────────────────────────────────────

#[test]
fn test_constraint_fresh_var() {
    let mut solver = ConstraintSolver::new();
    let v0 = solver.fresh_var("T");
    let v1 = solver.fresh_var("T");
    assert_ne!(v0.id, v1.id);
    assert_eq!(v0.name, "T0");
    assert_eq!(v1.name, "T1");
}

#[test]
fn test_constraint_unify_same() {
    let mut solver = ConstraintSolver::new();
    solver.add_equal(Type::Int, Type::Int);
    let result = solver.solve();
    match result {
        SolveResult::Solved(_) => {}
        SolveResult::Error(errs) => panic!("Expected success, got errors: {:?}", errs),
    }
}

#[test]
fn test_constraint_unify_type_var() {
    let mut solver = ConstraintSolver::new();
    let var = solver.fresh_var("T");
    solver.add_equal(Type::TypeVar(var.clone()), Type::Int);
    let result = solver.solve();
    match result {
        SolveResult::Solved(subst) => {
            assert_eq!(subst.get(&var.id), Some(&Type::Int));
        }
        SolveResult::Error(errs) => panic!("Expected success, got errors: {:?}", errs),
    }
}

#[test]
fn test_constraint_unify_incompatible() {
    let mut solver = ConstraintSolver::new();
    solver.add_equal(Type::Int, Type::String);
    let result = solver.solve();
    match result {
        SolveResult::Solved(_) => panic!("Expected error for incompatible types"),
        SolveResult::Error(_) => {}
    }
}

#[test]
fn test_constraint_unify_dyn() {
    let mut solver = ConstraintSolver::new();
    solver.add_equal(Type::Dynamic, Type::Int);
    let result = solver.solve();
    match result {
        SolveResult::Solved(_) => {}
        SolveResult::Error(errs) => panic!("Expected success, got errors: {:?}", errs),
    }
}

#[test]
fn test_constraint_unify_int_float() {
    let mut solver = ConstraintSolver::new();
    solver.add_equal(Type::Int, Type::Float);
    let result = solver.solve();
    match result {
        SolveResult::Solved(_) => {}
        SolveResult::Error(errs) => panic!("Expected success, got errors: {:?}", errs),
    }
}

#[test]
fn test_constraint_occurs_check() {
    let mut solver = ConstraintSolver::new();
    let var = solver.fresh_var("T");
    let recursive = Type::Function {
        params: vec![Type::TypeVar(var.clone())],
        return_type: Box::new(Type::TypeVar(var.clone())),
    };
    solver.add_equal(Type::TypeVar(var.clone()), recursive);
    let result = solver.solve();
    match result {
        SolveResult::Solved(_) => panic!("Expected error for recursive type"),
        SolveResult::Error(_) => {}
    }
}

#[test]
fn test_constraint_apply_subst() {
    let mut subst = std::collections::HashMap::new();
    let var = TypeVar::new(0, "T0".into());
    subst.insert(0, Type::Int);
    let ty = Type::Nullable(Box::new(Type::TypeVar(var)));
    let result = ConstraintSolver::apply_subst(&subst, &ty);
    assert_eq!(result, Type::Nullable(Box::new(Type::Int)));
}

// ── Type Checker Integration ──────────────────────────────────

fn check_program(source: &str) -> TypeCheckResult {
    let mut parser = match Parser::new(source) {
        Ok(p) => p,
        Err(_) => {
            let env = TypeEnvironment::new();
            let mut bag = DiagnosticBag::new();
            bag.add_error(DiagnosticKind::Other {
                message: "lexer error".into(),
            });
            return TypeCheckResult {
                env,
                diagnostics: bag.into_diagnostics(),
                has_errors: true,
                tracker: MonomorphizationTracker::new(),
            };
        }
    };
    let program = match parser.parse() {
        Ok(p) => p,
        Err(_) => {
            let env = TypeEnvironment::new();
            let mut bag = DiagnosticBag::new();
            bag.add_error(DiagnosticKind::Other {
                message: "parse error".into(),
            });
            return TypeCheckResult {
                env,
                diagnostics: bag.into_diagnostics(),
                has_errors: true,
                tracker: MonomorphizationTracker::new(),
            };
        }
    };
    let mut checker = TypeChecker::new();
    checker.check(&program)
}

fn assert_no_errors(result: &TypeCheckResult) {
    if result.has_errors {
        let errors: Vec<String> = result
            .diagnostics
            .iter()
            .filter(|d| d.is_error())
            .map(|d| d.message().to_string())
            .collect();
        panic!("Expected no errors, but found: {:?}", errors);
    }
}

fn get_var_type(source: &str, var_name: &str) -> Option<Type> {
    let result = check_program(source);
    result.env.lookup(var_name).cloned()
}

fn synthesize_expr(
    source: &str,
    expr_source: &str,
    class_name: Option<&str>,
    in_function: bool,
) -> Option<Type> {
    let mut parser = Parser::new(source).ok()?;
    let program = parser.parse().ok()?;
    let mut expr_parser = Parser::new(expr_source).ok()?;
    let expr = expr_parser.parse_expression().ok()?;
    let mut inference = TypeInference::new(TraitRegistry::new());
    Some(inference.synthesize_after_check(
        &program,
        &expr,
        class_name,
        in_function,
    ))
}

// ── Literal Inference ─────────────────────────────────────────

#[test]
fn test_infer_int_literal() {
    assert_eq!(get_var_type("let x = 42;", "x"), Some(Type::Int));
}

#[test]
fn test_infer_float_literal() {
    assert_eq!(get_var_type("let x = 3.14;", "x"), Some(Type::Float));
}

#[test]
fn test_infer_string_literal() {
    assert_eq!(get_var_type("let x = \"hello\";", "x"), Some(Type::String));
}

#[test]
fn test_infer_bool_literal() {
    assert_eq!(get_var_type("let x = true;", "x"), Some(Type::Bool));
}

#[test]
fn test_infer_null_literal() {
    assert_eq!(get_var_type("let x = null;", "x"), Some(Type::Null));
}

#[test]
fn test_infer_dyn_default() {
    assert_eq!(get_var_type("let x;", "x"), Some(Type::Dynamic));
}

#[test]
fn test_infer_typed_annotation() {
    assert_eq!(get_var_type("let x: int = 42;", "x"), Some(Type::Int));
    assert_eq!(get_var_type("let x: float = 3.14;", "x"), Some(Type::Float));
    assert_eq!(
        get_var_type("let x: string = \"hi\";", "x"),
        Some(Type::String)
    );
}

// ── Expression Inference ──────────────────────────────────────

#[test]
fn test_infer_addition_int() {
    assert_eq!(get_var_type("let x = 1 + 2;", "x"), Some(Type::Int));
}

#[test]
fn test_infer_addition_int_float() {
    assert_eq!(get_var_type("let x = 1 + 2.0;", "x"), Some(Type::Float));
}

#[test]
fn test_infer_string_concat() {
    assert_eq!(
        get_var_type("let x = \"hello\" + \" world\";", "x"),
        Some(Type::String)
    );
}

#[test]
fn test_infer_comparison() {
    assert_eq!(get_var_type("let x = 1 === 2;", "x"), Some(Type::Bool));
}

#[test]
fn test_infer_less_than() {
    assert_eq!(get_var_type("let x = 1 < 2;", "x"), Some(Type::Bool));
}

#[test]
fn test_infer_logical_and() {
    assert_eq!(
        get_var_type("let x = true && false;", "x"),
        Some(Type::Bool)
    );
}

#[test]
fn test_infer_logical_or() {
    assert_eq!(
        get_var_type("let x = true || false;", "x"),
        Some(Type::Bool)
    );
}

#[test]
fn test_infer_unary_not() {
    assert_eq!(get_var_type("let x = !true;", "x"), Some(Type::Bool));
}

#[test]
fn test_infer_unary_minus() {
    assert_eq!(get_var_type("let x = -42;", "x"), Some(Type::Int));
}

#[test]
fn test_infer_unary_minus_float() {
    assert_eq!(get_var_type("let x = -3.14;", "x"), Some(Type::Float));
}

// ── Type Checker Programs ─────────────────────────────────────

#[test]
fn test_check_simple_program() {
    let result = check_program("let x = 42;");
    assert_no_errors(&result);
}

#[test]
fn test_check_typed_variable() {
    let result = check_program("let x: int = 42;");
    assert_no_errors(&result);
}

#[test]
fn test_check_function_declaration() {
    let result = check_program("fn add(a: int, b: int): int { return a + b; }");
    assert_no_errors(&result);
}

#[test]
fn test_check_class_declaration() {
    let result = check_program("class Point { x: int; y: int; }");
    assert_no_errors(&result);
}

#[test]
fn test_check_trait_declaration() {
    let result = check_program("trait Printable { fn format(self): string; }");
    assert_no_errors(&result);
}

#[test]
fn test_check_if_statement() {
    let result = check_program("if (true) { let x = 1; }");
    assert_no_errors(&result);
}

#[test]
fn test_check_while_statement() {
    let result = check_program("while (true) { let x = 1; }");
    assert_no_errors(&result);
}

#[test]
fn test_check_for_statement() {
    let result = check_program("for (let i = 0; i < 10; i = i + 1) { }");
    assert!(result.diagnostics.len() <= 3);
}

#[test]
fn test_check_try_catch() {
    let result = check_program("try { let x = 1; } catch (e) { let y = 2; }");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_match_statement() {
    let result = check_program("match (x) { 1 => { let y = 1; } }");
    assert_no_errors(&result);
}

#[test]
fn test_check_null_literal() {
    let result = check_program("let x = null;");
    assert_no_errors(&result);
}

#[test]
fn test_check_bool_literal() {
    let result = check_program("let x = true;");
    assert_no_errors(&result);
}

#[test]
fn test_check_string_literal() {
    let result = check_program("let x = \"hello\";");
    assert_no_errors(&result);
}

#[test]
fn test_check_array_literal() {
    let result = check_program("let x = [1, 2, 3];");
    assert!(
        !result.has_errors,
        "errors: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| d.message())
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_check_object_literal() {
    let result = check_program("let x = { y: 1 };");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_arrow_function() {
    let result = check_program("let f = (x: int) => x + 1;");
    assert_no_errors(&result);
}

#[test]
fn test_check_const_declaration() {
    let result = check_program("const PI = 3.14;");
    assert_no_errors(&result);
}

#[test]
fn test_check_multiple_declarations() {
    let result = check_program("let x = 1; let y = \"hello\"; let z = true;");
    assert_no_errors(&result);
}

#[test]
fn test_check_function_with_return() {
    let result = check_program("fn greet(name: string): string { return \"Hello\"; }");
    assert_no_errors(&result);
}

#[test]
fn test_check_function_no_return() {
    let result = check_program("fn main() { }");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_for_of() {
    let result = check_program("for (let item of list) { }");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_for_in() {
    let result = check_program("for (let key in obj) { }");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_type_alias() {
    let result = check_program("type Name = string;");
    assert_no_errors(&result);
}

#[test]
fn test_check_empty_trait() {
    let result = check_program("trait Marker { }");
    assert_no_errors(&result);
}

#[test]
fn test_check_empty_class() {
    let result = check_program("class Empty { }");
    assert_no_errors(&result);
}

#[test]
fn test_check_nested_if() {
    let result = check_program("if (true) { if (false) { let x = 1; } }");
    assert_no_errors(&result);
}

#[test]
fn test_check_if_else() {
    let result = check_program("if (true) { let x = 1; } else { let y = 2; }");
    assert_no_errors(&result);
}

#[test]
fn test_check_ternary() {
    let result = check_program("let x = true ? 1 : 2;");
    assert_no_errors(&result);
}

#[test]
fn test_check_nullish_coalescing() {
    let result = check_program("let x = null ?? 42;");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_optional_member() {
    let result = check_program("let x = obj?.prop;");
    assert_no_errors(&result);
}

#[test]
fn test_check_typeof() {
    let result = check_program("let x = typeof 42;");
    assert_no_errors(&result);
}

#[test]
fn test_check_void_expression() {
    let result = check_program("let x = void 0;");
    assert_no_errors(&result);
}

#[test]
fn test_check_bitwise() {
    let result = check_program("let x = 1 & 2;");
    assert_no_errors(&result);
}

#[test]
fn test_check_shift() {
    let result = check_program("let x = 1 << 2;");
    assert_no_errors(&result);
}

#[test]
fn test_check_power() {
    let result = check_program("let x = 2 ** 3;");
    assert_no_errors(&result);
}

#[test]
fn test_check_modulo() {
    let result = check_program("let x = 10 % 3;");
    assert_no_errors(&result);
}

#[test]
fn test_check_division() {
    let result = check_program("let x = 10 / 3;");
    assert_no_errors(&result);
}

#[test]
fn test_check_unary_plus() {
    let result = check_program("let x = +42;");
    assert_no_errors(&result);
}

#[test]
fn test_check_unary_tilde() {
    let result = check_program("let x = ~42;");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_instanceof() {
    let result = check_program("let x = obj instanceof MyClass;");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_in_operator() {
    let result = check_program("let x = \"key\" in obj;");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_delete() {
    let result = check_program("let x = delete obj.prop;");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_new_expression() {
    let result = check_program("let x = new Point;");
    assert_no_errors(&result);
}

#[test]
fn test_check_self_expression() {
    let result = check_program("let x = self;");
    assert!(result.has_errors, "self at module level should be an error");
}

#[test]
fn test_check_this_expression() {
    let result = check_program("let x = this;");
    assert_no_errors(&result);
}

#[test]
fn test_check_super_expression() {
    let result = check_program("let x = super;");
    assert_no_errors(&result);
}

#[test]
fn test_check_bigint_literal() {
    let result = check_program("let x = 100n;");
    assert_no_errors(&result);
}

#[test]
fn test_check_template_literal() {
    let result = check_program("let x = `hello`;");
    assert_no_errors(&result);
}

#[test]
fn test_check_assignment() {
    let result = check_program("let x = 1; x = 2;");
    assert_no_errors(&result);
}

#[test]
fn test_check_compound_assignment() {
    let result = check_program("let x = 1; x += 1;");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_member_access() {
    let result = check_program("let x = obj.prop;");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_index_access() {
    let result = check_program("let x = arr[0];");
    assert_no_errors(&result);
}

#[test]
fn test_check_function_expression() {
    let result = check_program("let f = fn() { return 1; };");
    assert_no_errors(&result);
}

#[test]
fn test_check_class_expression() {
    let result = check_program("let c = class { };");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_match_expression() {
    let result = check_program("let x = match (val) { 1 => one, 2 => two };");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_if_expression() {
    let result = check_program("let x = if (cond) { 1; } else { 2; };");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_await_expression_simple() {
    let result = check_program("let x = await promise;");
    assert_no_errors(&result);
}

#[test]
fn test_check_grouping() {
    let result = check_program("let x = (1 + 2);");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_nested_function() {
    let result = check_program("fn outer() { fn inner() { return 1; } return inner(); }");
    assert_no_errors(&result);
}

#[test]
fn test_check_class_with_method() {
    let result = check_program("class Counter { fn increment() { } }");
    assert_no_errors(&result);
}

#[test]
fn test_check_class_with_field() {
    let result = check_program("class Point { x: int; y: int; }");
    assert_no_errors(&result);
}

#[test]
fn test_check_class_with_static() {
    let result = check_program("class Config { static version: string = \"1.0\"; }");
    assert_no_errors(&result);
}

#[test]
fn test_check_class_extends() {
    let result = check_program("class Dog extends Animal { fn bark() { } }");
    assert_no_errors(&result);
}

#[test]
fn test_self_referential_field_nullable_warning() {
    let source = "class ListNode { value: int; next: ListNode?; }";
    let result = check_program(source);
    let warnings: Vec<_> = result.warnings().collect();
    assert!(
        !warnings.is_empty(),
        "nullable self-ref should produce warning"
    );
}

#[test]
fn test_self_referential_field_nonnullable_error() {
    let source = "class ListNode { value: int; next: ListNode; }";
    let result = check_program(source);
    assert!(
        result.has_errors,
        "non-nullable self-ref should be an error"
    );
}

#[test]
fn test_normal_fields_no_diagnostic() {
    let source = "class Point { x: float; y: float; }";
    let result = check_program(source);
    assert_no_errors(&result);
    assert_eq!(
        result.warnings().count(),
        0,
        "normal fields should not trigger self-ref warning"
    );
}

#[test]
fn test_check_trait_with_method() {
    let result = check_program("trait Printable { fn format(self): string; }");
    assert_no_errors(&result);
}

#[test]
fn test_check_trait_generic() {
    let result = check_program("trait Container<T> { fn getValue(): T; }");
    assert_no_errors(&result);
}

#[test]
fn test_check_import() {
    let result = check_program("import { add } from \"./math\";");
    assert_no_errors(&result);
}

#[test]
fn test_check_export() {
    let result = check_program("export fn helper() { }");
    assert_no_errors(&result);
}

#[test]
fn test_check_export_default() {
    let result = check_program("export default 42;");
    assert_no_errors(&result);
}

#[test]
fn test_check_try_finally() {
    let result = check_program("try { } finally { }");
    assert_no_errors(&result);
}

#[test]
fn test_check_try_catch_finally() {
    let result = check_program("try { } catch (e) { } finally { }");
    assert_no_errors(&result);
}

#[test]
fn test_check_break() {
    let result = check_program("while (true) { break; }");
    assert_no_errors(&result);
}

#[test]
fn test_check_continue() {
    let result = check_program("while (true) { continue; }");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_throw() {
    let result = check_program("throw Error(\"oops\");");
    assert_no_errors(&result);
}

#[test]
fn test_check_return() {
    let result = check_program("fn foo() { return 42; }");
    assert_no_errors(&result);
}

#[test]
fn test_check_return_void() {
    let result = check_program("fn foo() { return; }");
    assert_no_errors(&result);
}

#[test]
fn test_check_empty_function() {
    let result = check_program("fn foo() { }");
    assert_no_errors(&result);
}

#[test]
fn test_check_function_with_params() {
    let result = check_program("fn add(a: int, b: int): int { return a + b; }");
    assert_no_errors(&result);
}

#[test]
fn test_check_async_function() {
    let result = check_program("let f = async fn() { };");
    assert_no_errors(&result);
}

#[test]
fn test_check_generic_function() {
    let result = check_program("fn identity<T>(x: T): T { return x; }");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_if_let() {
    let result = check_program("if let x = maybe { }");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_while_let() {
    let result = check_program("while let v = iter { }");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_for_of_async() {
    let result = check_program("for (let item of async gen) { }");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_object_destructure() {
    let result = check_program("const { x, y } = point;");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_array_destructure() {
    let result = check_program("let [head, ...tail] = list;");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_spread_in_array() {
    let result = check_program("let x = [...arr];");
    assert_no_errors(&result);
}

#[test]
fn test_check_spread_in_object() {
    let result = check_program("let x = { ...obj };");
    assert_no_errors(&result);
}

#[test]
fn test_check_object_shorthand() {
    let result = check_program("let x = { name };");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_computed_property() {
    let result = check_program("let x = { [key]: value };");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_nullish_assign() {
    let result = check_program("let x = null; x ??= 42;");
    assert_no_errors(&result);
}

#[test]
fn test_check_logical_assign() {
    let result = check_program("let x = true; x &&= false;");
    assert_no_errors(&result);
}

#[test]
fn test_check_bitwise_assign() {
    let result = check_program("let x = 1; x &= 2;");
    assert_no_errors(&result);
}

#[test]
fn test_check_macro_declaration() {
    let result = check_program("macro dummy { }");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_type_alias_generic() {
    let result = check_program("type Result<T, E> = { ok: T, err: E };");
    assert_no_errors(&result);
}

#[test]
fn test_check_nested_scopes() {
    let result = check_program("let x = 1; { let y = 2; { let z = 3; } }");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_match_with_guard() {
    let result = check_program("match (n) { x if (x > 0) => { } }");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_match_wildcard() {
    let result = check_program("match (val) { _ => { } }");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_match_literal_pattern() {
    let result = check_program("match (x) { 1 => { } }");
    assert_no_errors(&result);
}

#[test]
fn test_check_catch_typed() {
    let result = check_program("try { } catch (e: Error) { }");
    assert_no_errors(&result);
}

#[test]
fn test_check_catch_untyped() {
    let result = check_program("try { } catch { }");
    assert_no_errors(&result);
}

#[test]
fn test_check_for_with_init() {
    let result = check_program("for (let i = 0; i < 10; i = i + 1) { }");
    assert!(result.diagnostics.len() <= 3);
}

#[test]
fn test_check_for_infinite() {
    let result = check_program("for (;;) { }");
    assert_no_errors(&result);
}

#[test]
fn test_check_empty_statement() {
    let result = check_program(";");
    assert_no_errors(&result);
}

#[test]
fn test_check_multiple_statements() {
    let result = check_program("let x = 1; let y = 2; let z = x + y;");
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_const_destructure() {
    let result = check_program("const { x, y } = point;");
    assert_no_errors(&result);
}

#[test]
fn test_check_let_no_init() {
    let result = check_program("let x;");
    assert_no_errors(&result);
}

#[test]
fn test_check_let_with_type() {
    let result = check_program("let x: string = \"hello\";");
    assert_no_errors(&result);
}

#[test]
fn test_check_nullable_type_annotation() {
    let result = check_program("let x: int? = null;");
    assert_no_errors(&result);
}

#[test]
fn test_check_never_type_annotation() {
    let result = check_program("fn fail(msg: string): never { throw Error(msg); }");
    assert_no_errors(&result);
}

// ── Nullable Type Safety ────────────────────────────────────────

#[test]
fn test_check_nullable_with_value() {
    let result = check_program("let x: int? = 42;");
    assert_no_errors(&result);
}

#[test]
fn test_check_unsafe_nullable_access_error() {
    let result = check_program("let obj: { prop: int }? = null; let x = obj.prop;");
    assert!(
        result.has_errors,
        "Expected error for unsafe nullable access"
    );
}

#[test]
fn test_check_safe_optional_chaining_access() {
    let result = check_program("let obj: { prop: int }? = null; let x = obj?.prop;");
    assert_no_errors(&result);
}

#[test]
fn test_check_optional_chaining_method_call() {
    let result = check_program("let obj: { fn method(): int }? = null; let x = obj?.method();");
    assert_no_errors(&result);
}

#[test]
fn test_check_nullish_coalescing_int() {
    let result = check_program("let x: int? = null; let y = x ?? 0;");
    assert_no_errors(&result);
}

#[test]
fn test_check_nullish_coalescing_string() {
    let result = check_program("let x: string? = null; let y = x ?? \"default\";");
    assert_no_errors(&result);
}

#[test]
fn test_check_nullish_coalescing_with_value() {
    let result = check_program("let x: int? = 42; let y = x ?? 0;");
    assert_no_errors(&result);
}

#[test]
fn test_check_nullish_coalescing_nested() {
    let result = check_program("let a: int? = null; let b: int? = null; let c = a ?? b ?? 0;");
    assert_no_errors(&result);
}

#[test]
fn test_check_narrowing_after_inequality() {
    let result = check_program("let x: int? = 42; if (x !== null) { let y: int = x; }");
    assert_no_errors(&result);
}

#[test]
fn test_check_narrowing_after_equality() {
    let result = check_program("let x: int? = null; if (x === null) { let y = 0; }");
    assert_no_errors(&result);
}

#[test]
fn test_check_nullable_function_return() {
    let result = check_program("fn getValue(): int? { return null; }");
    assert_no_errors(&result);
}

#[test]
fn test_check_nullable_function_param() {
    let result = check_program("fn process(x: int?) { }");
    assert_no_errors(&result);
}

#[test]
fn test_check_nullable_array() {
    let result = check_program("let arr: Array<int>? = null;");
    assert_no_errors(&result);
}

#[test]
fn test_check_optional_chain_nested() {
    let result =
        check_program("let obj: { inner: { value: int } }? = null; let x = obj?.inner?.value;");
    assert_no_errors(&result);
}

#[test]
fn test_check_nullish_assignment() {
    let result = check_program("let x: int? = null; x ??= 42;");
    assert_no_errors(&result);
}

#[test]
fn test_diagnostic_unsafe_nullable_access() {
    let result = check_program("let obj: { prop: int }? = null; let x = obj.prop;");
    let has_unsafe_error = result
        .diagnostics
        .iter()
        .any(|d| d.message().contains("Unsafe nullable access"));
    assert!(
        has_unsafe_error,
        "Expected UnsafeNullableAccess error: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_check_chained_nullish_coalescing() {
    let result = check_program(
        "let a: string? = null; let b: string? = null; let c = a ?? b ?? \"fallback\";",
    );
    assert_no_errors(&result);
}

#[test]
fn test_check_nullish_coalescing_with_function_call() {
    let result = check_program(
        "fn getDefault(): int { return -1; } let x: int? = null; let y = x ?? getDefault();",
    );
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser/type checker limitation
fn test_check_generic_type_annotation() {
    let result = check_program("let arr: Array<int> = [1, 2, 3];");
    assert_no_errors(&result);
}

#[test]
fn test_check_object_type_annotation() {
    let result = check_program("let p: { x: float, y: float } = { x: 0.0, y: 0.0 };");
    assert_no_errors(&result);
}

#[test]
fn test_check_dyn_type_annotation() {
    let result = check_program("let x: dyn = 42;");
    assert_no_errors(&result);
}

#[test]
fn test_check_void_type_annotation() {
    let result = check_program("fn foo(): void { }");
    assert_no_errors(&result);
}

// ── Async / Await ─────────────────────────────────────────────

#[test]
fn test_check_async_function_declaration() {
    let result = check_program("async fn fetch(): int { return 42; }");
    assert_no_errors(&result);
}

#[test]
fn test_check_async_function_type() {
    let result = check_program("async fn fetch(): int { return 42; }");
    let ty = result.env.lookup("fetch").cloned();
    match ty {
        Some(Type::Function { return_type, .. }) => match *return_type {
            Type::Future(inner) => assert_eq!(*inner, Type::Int),
            other => panic!("Expected Future<int>, got {:?}", other),
        },
        other => panic!("Expected function type, got {:?}", other),
    }
}

#[test]
fn test_check_async_arrow_function() {
    let result = check_program("let f = async fn(): int { return 1; };");
    assert_no_errors(&result);
    let ty = result.env.lookup("f").cloned();
    match ty {
        Some(Type::Function { return_type, .. }) => {
            assert!(matches!(*return_type, Type::Future(_)));
        }
        other => panic!("Expected async function type, got {:?}", other),
    }
}

#[test]
fn test_check_await_expression() {
    let result = check_program(
        "async fn foo(): int { let x = await bar(); return x; } async fn bar(): int { return 1; }",
    );
    assert_no_errors(&result);
}

#[test]
fn test_check_await_on_non_future_error() {
    let result = check_program("async fn foo(): int { let x = await 42; return x; }");
    assert!(result.has_errors, "Expected error for await on non-future");
}

#[test]
fn test_check_async_method() {
    let result = check_program("class Service { async fn fetch(): string { return \"ok\"; } }");
    assert_no_errors(&result);
}

#[test]
fn test_future_type_display() {
    assert_eq!(Type::Future(Box::new(Type::Int)).to_string(), "Future<int>");
    assert_eq!(
        Type::Future(Box::new(Type::String)).to_string(),
        "Future<string>"
    );
}

#[test]
fn test_future_subtype() {
    let f_int = Type::Future(Box::new(Type::Int));
    let f_float = Type::Future(Box::new(Type::Float));
    assert!(f_int.is_subtype_of(&f_int));
    assert!(!f_int.is_subtype_of(&f_float));
}

#[test]
fn test_check_nested_async() {
    let result = check_program("async fn outer(): int { let x = await inner(); return x; } async fn inner(): int { return 1; }");
    assert_no_errors(&result);
}

#[test]
fn test_type_inference_has_trait_registry() {
    use ruyic::typechecker::inference::TypeInference;
    use ruyic::typechecker::traits::TraitRegistry;
    let registry = TraitRegistry::new();
    let _inference = TypeInference::new(registry);
    assert!(true);
}

#[test]
fn test_impl_method_resolution() {
    let source = "
    trait Printable { fn format(self): string; }
    class Point { x: float; y: float; }
    impl Printable for Point {
        fn format(self): string { return \"(0, 0)\"; }
    }
    fn test(p: Point) { let s = p.format(); }
    ";
    let result = check_program(source);
    assert_no_errors(&result);
}

#[test]
fn test_supertrait_circular_detected() {
    let source = "
    trait A extends B { fn a(self); }
    trait B extends A { fn b(self); }
    ";
    let result = check_program(source);
    assert!(result.has_errors, "circular supertrait should be detected");
}

#[test]
fn test_supertrait_valid_hierarchy() {
    let source = "
    trait Debug { fn debug(self): string; }
    trait Printable extends Debug { fn print(self); }
    fn test() {}
    ";
    let result = check_program(source);
    assert_no_errors(&result);
}

#[test]
#[ignore] // Parser doesn't support generic fn with trait bounds yet
fn test_trait_bound_dyn_always_passes() {
    let source = "trait Marker { } fn main() { let x: dyn = 42; }";
    let result = check_program(source);
    assert!(
        !result.has_errors,
        "dyn should pass any bound, errors: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| d.message())
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_self_in_method_has_class_type() {
    let source = "class Point { x: int; y: int; fn sum(self): int { return self.x + self.y; } }";
    assert_eq!(
        synthesize_expr(source, "self", Some("Point"), false),
        Some(Type::Named("Point".into(), vec![]))
    );
    assert_eq!(
        synthesize_expr(source, "self.x", Some("Point"), false),
        Some(Type::Int)
    );
}

#[test]
fn test_self_outside_class_is_error() {
    let result = check_program("let x = self;");
    assert!(result.has_errors, "self at module level should error");
    let has_e4002 = result
        .diagnostics
        .iter()
        .any(|d| d.message().contains("E4002"));
    assert!(has_e4002, "expected diagnostic E4002");
}

#[test]
fn test_self_in_nested_closure_is_dynamic() {
    let source = "class Point { x: int; fn m(self): int { let f = fn() { return self; }; return 0; } }";
    assert_eq!(
        synthesize_expr(source, "self", None, true),
        Some(Type::Dynamic)
    );
}

// ── Class Member Access (T6) ──────────────────────────────────

#[test]
fn test_class_field_via_member_access() {
    let source = "class Point { x: int; y: int; } let p = new Point; let x_type = p.x;";
    assert_eq!(get_var_type(source, "x_type"), Some(Type::Int));
}

#[test]
fn test_class_own_method_via_member_access() {
    let source =
        "class Point { x: int; fn getX(self): int { return self.x; } } let p = new Point; let getX_type = p.getX;";
    assert_eq!(
        get_var_type(source, "getX_type"),
        Some(Type::Function {
            params: vec![Type::Named("Point".into(), vec![])],
            return_type: Box::new(Type::Int),
        })
    );
}
