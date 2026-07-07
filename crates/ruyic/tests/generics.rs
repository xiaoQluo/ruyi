use ruyic::parser::Parser;
use ruyic::typechecker::constraints::ConstraintSolver;
use ruyic::typechecker::diagnostics::DiagnosticBag;
use ruyic::typechecker::generics::*;
use ruyic::typechecker::types::{Type, TypeVar};
/**
 * Comprehensive tests for the Ruyi generics system.
 *
 * Tests cover:
 * - Type parameter declarations
 * - Generic function definitions
 * - Generic class definitions
 * - Type argument inference
 * - Explicit type arguments
 * - Trait bounds
 * - Constraint checking
 * - Monomorphization tracking
 * - Generics + nullable interaction
 * - Generics + dyn interaction
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */
use ruyic::typechecker::*;

// ── Type Parameter Info ──────────────────────────────────────────

#[test]
fn test_type_param_info_new() {
    let info = TypeParamInfo::new("T".to_string(), 0);
    assert_eq!(info.name, "T");
    assert_eq!(info.var_id, 0);
    assert!(info.bounds.is_empty());
}

#[test]
fn test_type_param_info_with_bounds() {
    let info = TypeParamInfo::with_bounds("T".to_string(), 0, vec!["Comparable".to_string()]);
    assert_eq!(info.name, "T");
    assert_eq!(info.bounds, vec!["Comparable"]);
}

#[test]
fn test_type_param_info_to_type_var() {
    let info = TypeParamInfo::new("T".to_string(), 42);
    let var = info.to_type_var();
    assert_eq!(var.id, 42);
    assert_eq!(var.name, "T");
}

#[test]
fn test_type_param_info_to_type() {
    let info = TypeParamInfo::new("T".to_string(), 42);
    let ty = info.to_type();
    assert_eq!(ty, Type::TypeVar(TypeVar::new(42, "T".to_string())));
}

// ── Name Mangling ────────────────────────────────────────────────

#[test]
fn test_mangle_name_single_arg() {
    assert_eq!(mangle_name("identity", &[Type::Int]), "identity__int");
}

#[test]
fn test_mangle_name_multiple_args() {
    assert_eq!(
        mangle_name("map", &[Type::Int, Type::String]),
        "map__int__string"
    );
}

#[test]
fn test_mangle_name_no_args() {
    assert_eq!(mangle_name("foo", &[]), "foo");
}

#[test]
fn test_mangle_name_nullable() {
    assert_eq!(
        mangle_name("wrap", &[Type::Nullable(Box::new(Type::Int))]),
        "wrap__int__opt"
    );
}

#[test]
fn test_mangle_name_array() {
    assert_eq!(
        mangle_name("process", &[Type::Array(Box::new(Type::Float))]),
        "process__Array__float"
    );
}

#[test]
fn test_mangle_name_nested_generic() {
    assert_eq!(
        mangle_name(
            "nested",
            &[Type::Generic {
                base: "Array".to_string(),
                args: vec![Type::Int],
            }]
        ),
        "nested__Array__int"
    );
}

#[test]
fn test_mangle_name_function_type() {
    assert_eq!(
        mangle_name(
            "apply",
            &[Type::Function {
                params: vec![Type::Int],
                return_type: Box::new(Type::String),
            }]
        ),
        "apply__fn_int__string"
    );
}

#[test]
fn test_mangle_name_dyn() {
    assert_eq!(mangle_name("identity", &[Type::Dynamic]), "identity__dyn");
}

#[test]
fn test_mangle_name_float() {
    assert_eq!(mangle_name("process", &[Type::Float]), "process__float");
}

#[test]
fn test_mangle_name_bool() {
    assert_eq!(mangle_name("check", &[Type::Bool]), "check__bool");
}

// ── Specialization ──────────────────────────────────────────────

#[test]
fn test_specialization_new() {
    let spec = Specialization::new(
        "identity",
        vec![Type::Int],
        Type::Function {
            params: vec![Type::Int],
            return_type: Box::new(Type::Int),
        },
    );
    assert_eq!(spec.generic_name, "identity");
    assert_eq!(spec.type_args, vec![Type::Int]);
    assert_eq!(spec.mangled_name, "identity__int");
}

#[test]
fn test_specialization_multiple_type_args() {
    let spec = Specialization::new(
        "map",
        vec![Type::Int, Type::String],
        Type::Function {
            params: vec![Type::Array(Box::new(Type::Int))],
            return_type: Box::new(Type::Array(Box::new(Type::String))),
        },
    );
    assert_eq!(spec.mangled_name, "map__int__string");
}

// ── Monomorphization Tracker ─────────────────────────────────────

#[test]
fn test_tracker_register_and_lookup() {
    let mut tracker = MonomorphizationTracker::new();
    let var_id = tracker.fresh_var_id();
    let def = GenericDefinition {
        name: "identity".to_string(),
        type_params: vec![TypeParamInfo::new("T".to_string(), var_id)],
        body_type: Type::Function {
            params: vec![Type::TypeVar(TypeVar::new(var_id, "T".to_string()))],
            return_type: Box::new(Type::TypeVar(TypeVar::new(var_id, "T".to_string()))),
        },
    };
    tracker.register_generic(def);
    assert!(tracker.is_generic("identity"));
    assert!(!tracker.is_generic("unknown"));
    assert!(tracker.get_generic("identity").is_some());
}

#[test]
fn test_tracker_specialize_identity_int() {
    let mut tracker = MonomorphizationTracker::new();
    let var_id = tracker.fresh_var_id();
    let type_var = TypeVar::new(var_id, "T".to_string());
    let def = GenericDefinition {
        name: "identity".to_string(),
        type_params: vec![TypeParamInfo::new("T".to_string(), var_id)],
        body_type: Type::Function {
            params: vec![Type::TypeVar(type_var.clone())],
            return_type: Box::new(Type::TypeVar(type_var)),
        },
    };
    tracker.register_generic(def);

    let mut diagnostics = DiagnosticBag::new();
    let spec = tracker
        .specialize("identity", vec![Type::Int], &mut diagnostics)
        .unwrap();
    assert_eq!(spec.mangled_name, "identity__int");
    assert_eq!(spec.type_args, vec![Type::Int]);
    match &spec.specialized_type {
        Type::Function {
            params,
            return_type,
        } => {
            assert_eq!(params, &vec![Type::Int]);
            assert_eq!(**return_type, Type::Int);
        }
        _ => panic!("Expected function type"),
    }
}

#[test]
fn test_tracker_specialize_identity_string() {
    let mut tracker = MonomorphizationTracker::new();
    let var_id = tracker.fresh_var_id();
    let type_var = TypeVar::new(var_id, "T".to_string());
    let def = GenericDefinition {
        name: "identity".to_string(),
        type_params: vec![TypeParamInfo::new("T".to_string(), var_id)],
        body_type: Type::Function {
            params: vec![Type::TypeVar(type_var.clone())],
            return_type: Box::new(Type::TypeVar(type_var)),
        },
    };
    tracker.register_generic(def);

    let mut diagnostics = DiagnosticBag::new();
    let spec = tracker
        .specialize("identity", vec![Type::String], &mut diagnostics)
        .unwrap();
    assert_eq!(spec.mangled_name, "identity__string");
    match &spec.specialized_type {
        Type::Function {
            params,
            return_type,
        } => {
            assert_eq!(params, &vec![Type::String]);
            assert_eq!(**return_type, Type::String);
        }
        _ => panic!("Expected function type"),
    }
}

#[test]
fn test_tracker_specialize_arity_error() {
    let mut tracker = MonomorphizationTracker::new();
    let var_id = tracker.fresh_var_id();
    let def = GenericDefinition {
        name: "identity".to_string(),
        type_params: vec![TypeParamInfo::new("T".to_string(), var_id)],
        body_type: Type::Function {
            params: vec![Type::TypeVar(TypeVar::new(var_id, "T".to_string()))],
            return_type: Box::new(Type::TypeVar(TypeVar::new(var_id, "T".to_string()))),
        },
    };
    tracker.register_generic(def);

    let mut diagnostics = DiagnosticBag::new();
    let result = tracker.specialize("identity", vec![Type::Int, Type::String], &mut diagnostics);
    assert!(result.is_none());
    assert!(diagnostics.has_errors());
}

#[test]
fn test_tracker_specialize_deduplication() {
    let mut tracker = MonomorphizationTracker::new();
    let var_id = tracker.fresh_var_id();
    let type_var = TypeVar::new(var_id, "T".to_string());
    let def = GenericDefinition {
        name: "identity".to_string(),
        type_params: vec![TypeParamInfo::new("T".to_string(), var_id)],
        body_type: Type::Function {
            params: vec![Type::TypeVar(type_var.clone())],
            return_type: Box::new(Type::TypeVar(type_var)),
        },
    };
    tracker.register_generic(def);

    let mut diagnostics = DiagnosticBag::new();
    let spec1 = tracker
        .specialize("identity", vec![Type::Int], &mut diagnostics)
        .unwrap();
    let spec2 = tracker
        .specialize("identity", vec![Type::Int], &mut diagnostics)
        .unwrap();
    assert_eq!(spec1.mangled_name, spec2.mangled_name);
}

#[test]
fn test_tracker_specialize_with_nullable() {
    let mut tracker = MonomorphizationTracker::new();
    let var_id = tracker.fresh_var_id();
    let type_var = TypeVar::new(var_id, "T".to_string());
    let def = GenericDefinition {
        name: "wrap".to_string(),
        type_params: vec![TypeParamInfo::new("T".to_string(), var_id)],
        body_type: Type::Function {
            params: vec![Type::TypeVar(type_var.clone())],
            return_type: Box::new(Type::Nullable(Box::new(Type::TypeVar(type_var)))),
        },
    };
    tracker.register_generic(def);

    let mut diagnostics = DiagnosticBag::new();
    let spec = tracker
        .specialize("wrap", vec![Type::Int], &mut diagnostics)
        .unwrap();
    assert_eq!(
        spec.specialized_type,
        Type::Function {
            params: vec![Type::Int],
            return_type: Box::new(Type::Nullable(Box::new(Type::Int))),
        }
    );
}

#[test]
fn test_tracker_specialize_with_dyn() {
    let mut tracker = MonomorphizationTracker::new();
    let var_id = tracker.fresh_var_id();
    let type_var = TypeVar::new(var_id, "T".to_string());
    let def = GenericDefinition {
        name: "identity".to_string(),
        type_params: vec![TypeParamInfo::new("T".to_string(), var_id)],
        body_type: Type::Function {
            params: vec![Type::TypeVar(type_var.clone())],
            return_type: Box::new(Type::TypeVar(type_var)),
        },
    };
    tracker.register_generic(def);

    let mut diagnostics = DiagnosticBag::new();
    let spec = tracker
        .specialize("identity", vec![Type::Dynamic], &mut diagnostics)
        .unwrap();
    assert_eq!(
        spec.specialized_type,
        Type::Function {
            params: vec![Type::Dynamic],
            return_type: Box::new(Type::Dynamic),
        }
    );
}

#[test]
fn test_tracker_specialize_two_params() {
    let mut tracker = MonomorphizationTracker::new();
    let var_id_t = tracker.fresh_var_id();
    let var_id_u = tracker.fresh_var_id();
    let type_var_t = TypeVar::new(var_id_t, "T".to_string());
    let type_var_u = TypeVar::new(var_id_u, "U".to_string());
    let def = GenericDefinition {
        name: "map".to_string(),
        type_params: vec![
            TypeParamInfo::new("T".to_string(), var_id_t),
            TypeParamInfo::new("U".to_string(), var_id_u),
        ],
        body_type: Type::Function {
            params: vec![
                Type::Array(Box::new(Type::TypeVar(type_var_t))),
                Type::Function {
                    params: vec![Type::TypeVar(TypeVar::new(var_id_t, "T".to_string()))],
                    return_type: Box::new(Type::TypeVar(type_var_u)),
                },
            ],
            return_type: Box::new(Type::Array(Box::new(Type::TypeVar(TypeVar::new(
                var_id_u,
                "U".to_string(),
            ))))),
        },
    };
    tracker.register_generic(def);

    let mut diagnostics = DiagnosticBag::new();
    let spec = tracker
        .specialize("map", vec![Type::Int, Type::String], &mut diagnostics)
        .unwrap();
    assert_eq!(spec.mangled_name, "map__int__string");
}

// ── Type Argument Inference ──────────────────────────────────────

#[test]
fn test_infer_type_args_identity() {
    let mut tracker = MonomorphizationTracker::new();
    let var_id = tracker.fresh_var_id();
    let type_var = TypeVar::new(var_id, "T".to_string());
    let def = GenericDefinition {
        name: "identity".to_string(),
        type_params: vec![TypeParamInfo::new("T".to_string(), var_id)],
        body_type: Type::Function {
            params: vec![Type::TypeVar(type_var.clone())],
            return_type: Box::new(Type::TypeVar(type_var)),
        },
    };
    tracker.register_generic(def);

    let mut diagnostics = DiagnosticBag::new();
    let inferred = tracker
        .infer_type_args("identity", &[Type::Int], &mut diagnostics)
        .unwrap();
    assert_eq!(inferred.len(), 1);
    assert_eq!(inferred[0], Type::Int);
}

#[test]
fn test_infer_type_args_map() {
    let mut tracker = MonomorphizationTracker::new();
    let var_id_t = tracker.fresh_var_id();
    let var_id_u = tracker.fresh_var_id();
    let type_var_t = TypeVar::new(var_id_t, "T".to_string());
    let type_var_u = TypeVar::new(var_id_u, "U".to_string());
    let def = GenericDefinition {
        name: "map".to_string(),
        type_params: vec![
            TypeParamInfo::new("T".to_string(), var_id_t),
            TypeParamInfo::new("U".to_string(), var_id_u),
        ],
        body_type: Type::Function {
            params: vec![
                Type::Array(Box::new(Type::TypeVar(type_var_t))),
                Type::Function {
                    params: vec![Type::TypeVar(TypeVar::new(var_id_t, "T".to_string()))],
                    return_type: Box::new(Type::TypeVar(type_var_u)),
                },
            ],
            return_type: Box::new(Type::Array(Box::new(Type::TypeVar(TypeVar::new(
                var_id_u,
                "U".to_string(),
            ))))),
        },
    };
    tracker.register_generic(def);

    let mut diagnostics = DiagnosticBag::new();
    let inferred = tracker
        .infer_type_args(
            "map",
            &[
                Type::Array(Box::new(Type::Int)),
                Type::Function {
                    params: vec![Type::Int],
                    return_type: Box::new(Type::String),
                },
            ],
            &mut diagnostics,
        )
        .unwrap();
    assert_eq!(inferred.len(), 2);
    assert_eq!(inferred[0], Type::Int);
    assert_eq!(inferred[1], Type::String);
}

#[test]
fn test_infer_type_args_with_dyn() {
    let mut tracker = MonomorphizationTracker::new();
    let var_id = tracker.fresh_var_id();
    let type_var = TypeVar::new(var_id, "T".to_string());
    let def = GenericDefinition {
        name: "identity".to_string(),
        type_params: vec![TypeParamInfo::new("T".to_string(), var_id)],
        body_type: Type::Function {
            params: vec![Type::TypeVar(type_var.clone())],
            return_type: Box::new(Type::TypeVar(type_var)),
        },
    };
    tracker.register_generic(def);

    let mut diagnostics = DiagnosticBag::new();
    let inferred = tracker
        .infer_type_args("identity", &[Type::Dynamic], &mut diagnostics)
        .unwrap();
    assert_eq!(inferred[0], Type::Dynamic);
}

// ── Trait Bounds ─────────────────────────────────────────────────

#[test]
fn test_trait_bounds_empty() {
    let info = TypeParamInfo::new("T".to_string(), 0);
    assert!(info.bounds.is_empty());
}

#[test]
fn test_trait_bounds_with_comparable() {
    let info = TypeParamInfo::with_bounds("T".to_string(), 0, vec!["Comparable".to_string()]);
    assert_eq!(info.bounds, vec!["Comparable"]);
}

#[test]
fn test_trait_bounds_multiple() {
    let info = TypeParamInfo::with_bounds(
        "T".to_string(),
        0,
        vec!["Comparable".to_string(), "Clone".to_string()],
    );
    assert_eq!(info.bounds, vec!["Comparable", "Clone"]);
}

#[test]
fn test_specialize_with_trait_bounds() {
    let mut tracker = MonomorphizationTracker::new();
    let var_id = tracker.fresh_var_id();
    let type_var = TypeVar::new(var_id, "T".to_string());
    let def = GenericDefinition {
        name: "max".to_string(),
        type_params: vec![TypeParamInfo::with_bounds(
            "T".to_string(),
            var_id,
            vec!["Comparable".to_string()],
        )],
        body_type: Type::Function {
            params: vec![
                Type::TypeVar(type_var.clone()),
                Type::TypeVar(type_var.clone()),
            ],
            return_type: Box::new(Type::TypeVar(type_var)),
        },
    };
    tracker.register_generic(def);

    let mut diagnostics = DiagnosticBag::new();
    let spec = tracker
        .specialize("max", vec![Type::Int], &mut diagnostics)
        .unwrap();
    assert_eq!(spec.mangled_name, "max__int");
}

// ── Type Substitution ────────────────────────────────────────────

#[test]
fn test_substitute_simple() {
    let mut subst = std::collections::HashMap::new();
    subst.insert(0, Type::Int);
    let ty = Type::TypeVar(TypeVar::new(0, "T".to_string()));
    let result = MonomorphizationTracker::substitute_type(&subst, &ty);
    assert_eq!(result, Type::Int);
}

#[test]
fn test_substitute_nullable() {
    let mut subst = std::collections::HashMap::new();
    subst.insert(0, Type::Int);
    let ty = Type::Nullable(Box::new(Type::TypeVar(TypeVar::new(0, "T".to_string()))));
    let result = MonomorphizationTracker::substitute_type(&subst, &ty);
    assert_eq!(result, Type::Nullable(Box::new(Type::Int)));
}

#[test]
fn test_substitute_function() {
    let mut subst = std::collections::HashMap::new();
    subst.insert(0, Type::Int);
    subst.insert(1, Type::String);
    let ty = Type::Function {
        params: vec![Type::TypeVar(TypeVar::new(0, "T".to_string()))],
        return_type: Box::new(Type::TypeVar(TypeVar::new(1, "U".to_string()))),
    };
    let result = MonomorphizationTracker::substitute_type(&subst, &ty);
    assert_eq!(
        result,
        Type::Function {
            params: vec![Type::Int],
            return_type: Box::new(Type::String),
        }
    );
}

#[test]
fn test_substitute_array() {
    let mut subst = std::collections::HashMap::new();
    subst.insert(0, Type::Float);
    let ty = Type::Array(Box::new(Type::TypeVar(TypeVar::new(0, "T".to_string()))));
    let result = MonomorphizationTracker::substitute_type(&subst, &ty);
    assert_eq!(result, Type::Array(Box::new(Type::Float)));
}

#[test]
fn test_substitute_nested_generic() {
    let mut subst = std::collections::HashMap::new();
    subst.insert(0, Type::Int);
    let ty = Type::Generic {
        base: "Array".to_string(),
        args: vec![Type::TypeVar(TypeVar::new(0, "T".to_string()))],
    };
    let result = MonomorphizationTracker::substitute_type(&subst, &ty);
    assert_eq!(
        result,
        Type::Generic {
            base: "Array".to_string(),
            args: vec![Type::Int],
        }
    );
}

#[test]
fn test_substitute_no_change() {
    let subst = std::collections::HashMap::new();
    let ty = Type::Int;
    let result = MonomorphizationTracker::substitute_type(&subst, &ty);
    assert_eq!(result, Type::Int);
}

// ── Generic Function Type Checking ────────────────────────────────

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

#[test]
fn test_check_generic_function() {
    let result = check_program("fn identity<T>(x: T): T { return x; }");
    assert_no_errors(&result);
}

#[test]
fn test_check_generic_function_two_params() {
    let result =
        check_program("fn apply<T, U>(arr: Array<T>, f: fn(T) -> U): Array<T> { return arr; }");
    assert_no_errors(&result);
}

#[test]
fn test_check_generic_class() {
    let result = check_program("class Box<T> { value: T; }");
    assert_no_errors(&result);
}

#[test]
fn test_check_generic_trait() {
    let result = check_program("trait Container<T> { fn getValue(): T; }");
    assert_no_errors(&result);
}

#[test]
fn test_check_generic_function_with_bounds() {
    let result = check_program("fn max<T: Comparable>(a: T, b: T): T { return a; }");
    assert_no_errors(&result);
}

#[test]
fn test_check_generic_function_multiple_bounds() {
    let result =
        check_program("fn sort<T: Comparable + Clone>(arr: Array<T>): Array<T> { return arr; }");
    assert_no_errors(&result);
}

#[test]
fn test_check_generic_class_with_method() {
    let result = check_program("class Option<T> { value: T?; fn new(value: T?) { } fn unwrap(): T { return self.value; } }");
    assert_no_errors(&result);
}

#[test]
fn test_check_type_alias_generic() {
    let result = check_program("type Result<T, E> = { ok: T, err: E };");
    assert_no_errors(&result);
}

// ── Monomorphization Context (Codegen) ───────────────────────────

#[test]
fn test_monomorphization_context_register() {
    use ruyic::codegen::monomorph::{MonomorphizationContext, MonomorphizedFunction};

    let mut ctx = MonomorphizationContext::new();
    let func = MonomorphizedFunction {
        mangled_name: "identity__int".to_string(),
        generic_name: "identity".to_string(),
        type_args: vec![Type::Int],
        param_types: vec![Type::Int],
        return_type: Type::Int,
    };
    ctx.register(func);
    assert!(ctx.get_function("identity__int").is_some());
    assert!(!ctx.is_generated("identity__int"));
}

#[test]
fn test_monomorphization_context_mark_generated() {
    use ruyic::codegen::monomorph::{MonomorphizationContext, MonomorphizedFunction};

    let mut ctx = MonomorphizationContext::new();
    let func = MonomorphizedFunction {
        mangled_name: "identity__int".to_string(),
        generic_name: "identity".to_string(),
        type_args: vec![Type::Int],
        param_types: vec![Type::Int],
        return_type: Type::Int,
    };
    ctx.register(func);
    ctx.mark_generated("identity__int");
    assert!(ctx.is_generated("identity__int"));
}

#[test]
fn test_collect_monomorphizations() {
    use ruyic::codegen::monomorph::collect_monomorphizations;

    let mut tracker = MonomorphizationTracker::new();
    let var_id = tracker.fresh_var_id();
    let type_var = TypeVar::new(var_id, "T".to_string());
    let def = GenericDefinition {
        name: "identity".to_string(),
        type_params: vec![TypeParamInfo::new("T".to_string(), var_id)],
        body_type: Type::Function {
            params: vec![Type::TypeVar(type_var.clone())],
            return_type: Box::new(Type::TypeVar(type_var)),
        },
    };
    tracker.register_generic(def);

    let mut diagnostics = DiagnosticBag::new();
    tracker.specialize("identity", vec![Type::Int], &mut diagnostics);

    let monomorphizations = collect_monomorphizations(&tracker);
    assert_eq!(monomorphizations.len(), 1);
    assert_eq!(monomorphizations[0].mangled_name, "identity__int");
    assert_eq!(monomorphizations[0].param_types, vec![Type::Int]);
    assert_eq!(monomorphizations[0].return_type, Type::Int);
}

#[test]
fn test_populate_from_tracker() {
    use ruyic::codegen::monomorph::{collect_monomorphizations, MonomorphizationContext};

    let mut tracker = MonomorphizationTracker::new();
    let var_id = tracker.fresh_var_id();
    let type_var = TypeVar::new(var_id, "T".to_string());
    let def = GenericDefinition {
        name: "identity".to_string(),
        type_params: vec![TypeParamInfo::new("T".to_string(), var_id)],
        body_type: Type::Function {
            params: vec![Type::TypeVar(type_var.clone())],
            return_type: Box::new(Type::TypeVar(type_var)),
        },
    };
    tracker.register_generic(def);

    let mut diagnostics = DiagnosticBag::new();
    tracker.specialize("identity", vec![Type::String], &mut diagnostics);

    let mut ctx = MonomorphizationContext::new();
    ctx.populate_from_tracker(&tracker);
    assert!(ctx.get_function("identity__string").is_some());
}

// ── Constraint Solver Integration ────────────────────────────────

#[test]
fn test_constraint_solver_generic_substitution() {
    let mut solver = ConstraintSolver::new();
    let t_var = solver.fresh_var("T");
    let u_var = solver.fresh_var("U");

    // T = int, U = string
    solver.add_equal(Type::TypeVar(t_var.clone()), Type::Int);
    solver.add_equal(Type::TypeVar(u_var.clone()), Type::String);

    let result = solver.solve();
    match result {
        ruyic::typechecker::constraints::SolveResult::Solved(subst) => {
            assert_eq!(subst.get(&t_var.id), Some(&Type::Int));
            assert_eq!(subst.get(&u_var.id), Some(&Type::String));
        }
        ruyic::typechecker::constraints::SolveResult::Error(errs) => {
            panic!("Expected success, got errors: {:?}", errs);
        }
    }
}

#[test]
fn test_constraint_solver_generic_function_inference() {
    let mut solver = ConstraintSolver::new();
    let t_var = solver.fresh_var("T");

    // fn identity<T>(x: T): T called with identity(42)
    // Constraint: T = int
    solver.add_equal(Type::TypeVar(t_var.clone()), Type::Int);

    let result = solver.solve();
    match result {
        ruyic::typechecker::constraints::SolveResult::Solved(subst) => {
            assert_eq!(subst.get(&t_var.id), Some(&Type::Int));
        }
        ruyic::typechecker::constraints::SolveResult::Error(errs) => {
            panic!("Expected success, got errors: {:?}", errs);
        }
    }
}

#[test]
fn test_constraint_solver_trait_bound() {
    let mut solver = ConstraintSolver::new();
    let t_var = solver.fresh_var("T");

    // T: Comparable
    solver.add_trait_bound(t_var.clone(), "Comparable".to_string());

    let result = solver.solve();
    match result {
        ruyic::typechecker::constraints::SolveResult::Solved(_) => {}
        ruyic::typechecker::constraints::SolveResult::Error(errs) => {
            panic!("Expected success, got errors: {:?}", errs);
        }
    }
}

// ── Generics + Nullable Interaction ───────────────────────────────

#[test]
fn test_generic_nullable_return() {
    let mut tracker = MonomorphizationTracker::new();
    let var_id = tracker.fresh_var_id();
    let type_var = TypeVar::new(var_id, "T".to_string());
    let def = GenericDefinition {
        name: "wrap".to_string(),
        type_params: vec![TypeParamInfo::new("T".to_string(), var_id)],
        body_type: Type::Function {
            params: vec![Type::TypeVar(type_var.clone())],
            return_type: Box::new(Type::Nullable(Box::new(Type::TypeVar(type_var)))),
        },
    };
    tracker.register_generic(def);

    let mut diagnostics = DiagnosticBag::new();
    let spec = tracker
        .specialize("wrap", vec![Type::Int], &mut diagnostics)
        .unwrap();
    // wrap<int> should return int?
    assert_eq!(
        spec.specialized_type,
        Type::Function {
            params: vec![Type::Int],
            return_type: Box::new(Type::Nullable(Box::new(Type::Int))),
        }
    );
}

#[test]
fn test_generic_nullable_param() {
    let mut tracker = MonomorphizationTracker::new();
    let var_id = tracker.fresh_var_id();
    let type_var = TypeVar::new(var_id, "T".to_string());
    let def = GenericDefinition {
        name: "unwrap_or".to_string(),
        type_params: vec![TypeParamInfo::new("T".to_string(), var_id)],
        body_type: Type::Function {
            params: vec![
                Type::Nullable(Box::new(Type::TypeVar(type_var.clone()))),
                Type::TypeVar(type_var.clone()),
            ],
            return_type: Box::new(Type::TypeVar(type_var)),
        },
    };
    tracker.register_generic(def);

    let mut diagnostics = DiagnosticBag::new();
    let spec = tracker
        .specialize("unwrap_or", vec![Type::String], &mut diagnostics)
        .unwrap();
    assert_eq!(
        spec.specialized_type,
        Type::Function {
            params: vec![Type::Nullable(Box::new(Type::String)), Type::String,],
            return_type: Box::new(Type::String),
        }
    );
}

// ── Generics + Dyn Interaction ─────────────────────────────────────

#[test]
fn test_generic_with_dyn_arg() {
    let mut tracker = MonomorphizationTracker::new();
    let var_id = tracker.fresh_var_id();
    let type_var = TypeVar::new(var_id, "T".to_string());
    let def = GenericDefinition {
        name: "identity".to_string(),
        type_params: vec![TypeParamInfo::new("T".to_string(), var_id)],
        body_type: Type::Function {
            params: vec![Type::TypeVar(type_var.clone())],
            return_type: Box::new(Type::TypeVar(type_var)),
        },
    };
    tracker.register_generic(def);

    let mut diagnostics = DiagnosticBag::new();
    // Per spec Section 10.4: when called with dyn, T = dyn
    let spec = tracker
        .specialize("identity", vec![Type::Dynamic], &mut diagnostics)
        .unwrap();
    assert_eq!(
        spec.specialized_type,
        Type::Function {
            params: vec![Type::Dynamic],
            return_type: Box::new(Type::Dynamic),
        }
    );
}

#[test]
fn test_generic_inference_with_dyn() {
    let mut tracker = MonomorphizationTracker::new();
    let var_id = tracker.fresh_var_id();
    let type_var = TypeVar::new(var_id, "T".to_string());
    let def = GenericDefinition {
        name: "identity".to_string(),
        type_params: vec![TypeParamInfo::new("T".to_string(), var_id)],
        body_type: Type::Function {
            params: vec![Type::TypeVar(type_var.clone())],
            return_type: Box::new(Type::TypeVar(type_var)),
        },
    };
    tracker.register_generic(def);

    let mut diagnostics = DiagnosticBag::new();
    let inferred = tracker
        .infer_type_args("identity", &[Type::Dynamic], &mut diagnostics)
        .unwrap();
    assert_eq!(inferred[0], Type::Dynamic);
}

// ── Generic Type Alias ───────────────────────────────────────────

#[test]
fn test_generic_type_alias() {
    let result = check_program("type Result<T, E> = { ok: T, err: E };");
    assert_no_errors(&result);
}

#[test]
fn test_generic_type_alias_single_param() {
    let result = check_program("type Callback<T> = fn(T) -> void;");
    assert_no_errors(&result);
}

// ── Make Generic Definition Helpers ───────────────────────────────

#[test]
fn test_make_generic_function_def() {
    let mut tracker = MonomorphizationTracker::new();
    let type_params = vec![ruyic::parser::ast::TypeParam {
        name: "T".to_string(),
        bounds: vec![],
    }];
    let def = make_generic_function_def(
        "identity",
        &type_params,
        &[Type::TypeVar(TypeVar::new(0, "T".to_string()))],
        &Type::TypeVar(TypeVar::new(0, "T".to_string())),
        &mut tracker,
    );
    assert_eq!(def.name, "identity");
    assert_eq!(def.type_params.len(), 1);
    assert_eq!(def.type_params[0].name, "T");
}

#[test]
fn test_make_generic_class_def() {
    let mut tracker = MonomorphizationTracker::new();
    let type_params = vec![ruyic::parser::ast::TypeParam {
        name: "T".to_string(),
        bounds: vec![],
    }];
    let def = make_generic_class_def("Box", &type_params, &mut tracker);
    assert_eq!(def.name, "Box");
    assert_eq!(def.type_params.len(), 1);
    assert_eq!(def.type_params[0].name, "T");
}

#[test]
fn test_make_generic_trait_def() {
    let mut tracker = MonomorphizationTracker::new();
    let type_params = vec![ruyic::parser::ast::TypeParam {
        name: "T".to_string(),
        bounds: vec![],
    }];
    let def = make_generic_trait_def("Comparable", &type_params, &mut tracker);
    assert_eq!(def.name, "Comparable");
    assert_eq!(def.type_params.len(), 1);
    assert_eq!(def.type_params[0].name, "T");
    assert_eq!(def.type_params[0].bounds, vec!["Comparable"]);
}
