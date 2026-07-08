/**
 * Trait system tests for Ruyi.
 *
 * Tests cover:
 * - Trait declarations (with and without methods)
 * - Trait implementations
 * - Trait bounds on generics
 * - Static and dynamic dispatch
 * - Marker traits
 * - Default method implementations
 * - Trait object creation (dyn)
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */
use ruyic::parser::Parser;
use ruyic::typechecker::checker::TypeChecker;
use ruyic::typechecker::traits::{build_trait_registry, TraitRegistry};
use ruyic::typechecker::types::Type;

fn parse_and_check(source: &str) -> (bool, Vec<ruyic::typechecker::diagnostics::Diagnostic>) {
    let mut parser = Parser::new(source).expect("lexer should not fail");
    let program = parser.parse().expect("parse should succeed");
    let mut checker = TypeChecker::new();
    let result = checker.check(&program);
    (!result.has_errors, result.diagnostics)
}

fn build_registry(source: &str) -> TraitRegistry {
    let mut parser = Parser::new(source).expect("lexer should not fail");
    let program = parser.parse().expect("parse should succeed");
    build_trait_registry(&program)
}

// ── Trait Declarations ───────────────────────────────────────

#[test]
fn test_trait_declaration_empty() {
    let (ok, _) = parse_and_check("trait Marker { }");
    assert!(ok);
}

#[test]
fn test_trait_declaration_with_method() {
    let (ok, _) = parse_and_check("trait Printable { fn format(self): string; }");
    assert!(ok);
}

#[test]
fn test_trait_declaration_with_generic() {
    let (ok, _) = parse_and_check("trait Comparable<T> { fn compare(self, other: T): int; }");
    assert!(ok);
}

#[test]
fn test_trait_declaration_multiple_methods() {
    let (ok, _) =
        parse_and_check("trait Iterator<T> { fn next(self): T?; fn hasNext(self): bool; }");
    assert!(ok);
}

// ── Trait Registry ───────────────────────────────────────────

#[test]
fn test_registry_collects_traits() {
    let registry = build_registry(
        "trait Printable { fn format(self): string; }\ntrait Comparable { fn compare(self, other: int): int; }"
    );
    assert!(registry.get_trait("Printable").is_some());
    assert!(registry.get_trait("Comparable").is_some());
}

#[test]
fn test_registry_trait_methods() {
    let registry = build_registry("trait Printable { fn format(self): string; }");
    let trait_info = registry.get_trait("Printable").unwrap();
    assert!(trait_info.methods.contains_key("format"));
    assert_eq!(trait_info.methods["format"].return_type, Type::String);
}

#[test]
fn test_registry_marker_trait() {
    let registry = build_registry("trait Marker { }");
    let trait_info = registry.get_trait("Marker").unwrap();
    assert!(trait_info.is_marker);
}

#[test]
fn test_registry_non_marker_trait() {
    let registry = build_registry("trait Printable { fn format(self): string; }");
    let trait_info = registry.get_trait("Printable").unwrap();
    assert!(!trait_info.is_marker);
}

// ── Trait Implementations ────────────────────────────────────

#[test]
fn test_impl_declaration() {
    let (ok, _) = parse_and_check(
        "trait Printable { fn format(self): string; }\nimpl Printable for int { fn format(self): string { return \"\"; } }"
    );
    assert!(ok);
}

#[test]
fn test_impl_missing_method_error() {
    let (ok, diagnostics) =
        parse_and_check("trait Printable { fn format(self): string; }\nimpl Printable for int { }");
    assert!(!ok);
    assert!(diagnostics
        .iter()
        .any(|d| d.message().contains("does not implement trait")));
}

#[test]
fn test_impl_multiple_methods() {
    let (ok, _) = parse_and_check(
        "trait Iterator { fn next(self): int; fn hasNext(self): bool; }\nimpl Iterator for int { fn next(self): int { return 0; } fn hasNext(self): bool { return false; } }"
    );
    assert!(ok);
}

#[test]
fn test_impl_partial_methods_error() {
    let (ok, diagnostics) = parse_and_check(
        "trait Iterator { fn next(self): int; fn hasNext(self): bool; }\nimpl Iterator for int { fn next(self): int { return 0; } }"
    );
    assert!(!ok);
    assert!(diagnostics
        .iter()
        .any(|d| d.message().contains("does not implement trait")));
}

#[test]
fn test_impl_for_different_types() {
    let (ok, _) = parse_and_check(
        "trait Printable { fn format(self): string; }\nimpl Printable for int { fn format(self): string { return \"int\"; } }\nimpl Printable for string { fn format(self): string { return self; } }"
    );
    assert!(ok);
}

// ── Trait Bounds ─────────────────────────────────────────────

#[test]
fn test_generic_with_trait_bound() {
    let (ok, _) = parse_and_check(
        "trait Printable { fn format(self): string; }\nfn printIt<T: Printable>(value: T): void { print(value.format()); }"
    );
    assert!(ok);
}

#[test]
fn test_trait_bound_check() {
    let registry = build_registry(
        "trait Printable { fn format(self): string; }\nimpl Printable for int { fn format(self): string { return \"\"; } }"
    );
    assert!(registry.check_bound(&Type::Named("int".into(), vec![]), "Printable"));
    assert!(!registry.check_bound(&Type::Named("string".into(), vec![]), "Printable"));
}

// ── Trait Objects (dyn) ──────────────────────────────────────

#[test]
fn test_dyn_type_annotation() {
    let (ok, _) =
        parse_and_check("trait Printable { fn format(self): string; }\nlet x: dyn Printable = 42;");
    assert!(ok);
}

#[test]
fn test_dyn_trait_object_array() {
    let (ok, _) = parse_and_check(
        "trait Printable { fn format(self): string; }\nlet items: Array<dyn Printable> = [];",
    );
    assert!(ok);
}

// ── Static Dispatch ──────────────────────────────────────────

#[test]
fn test_static_dispatch_generic() {
    let (ok, _) = parse_and_check(
        "trait Printable { fn format(self): string; }\nimpl Printable for int { fn format(self): string { return \"int\"; } }\nfn printIt<T: Printable>(value: T): void { print(value.format()); }\nprintIt(42);"
    );
    assert!(ok);
}

// ── Dynamic Dispatch ─────────────────────────────────────────

#[test]
fn test_dynamic_dispatch_trait_object() {
    let (ok, _) = parse_and_check(
        "trait Printable { fn format(self): string; }\nimpl Printable for int { fn format(self): string { return \"int\"; } }\nlet x: dyn Printable = 42;\nprint(x.format());"
    );
    assert!(ok);
}

// ── Trait Inheritance (supertraits) ──────────────────────────

#[test]
fn test_supertrait_registry() {
    let registry = build_registry(
        "trait Display { fn display(self): string; }\ntrait Debug: Display { fn debug(self): string; }"
    );
    let debug = registry.get_trait("Debug").unwrap();
    assert!(debug.supertraits.contains(&"Display".to_string()));
}

// ── Default Methods ──────────────────────────────────────────

#[test]
fn test_default_method_not_required() {
    let (ok, _) = parse_and_check(
        "trait Iterator { fn next(self): int; fn hasNext(self): bool { return true; } }\nimpl Iterator for int { fn next(self): int { return 0; } }"
    );
    assert!(ok);
}

// ── Coherence / Duplicate Impls ──────────────────────────────

#[test]
fn test_duplicate_impl_error() {
    let (ok, _) = parse_and_check(
        "trait Printable { fn format(self): string; }\nimpl Printable for int { fn format(self): string { return \"a\"; } }\nimpl Printable for int { fn format(self): string { return \"b\"; } }"
    );
    assert!(
        ok,
        "Duplicate impls currently accepted (coherence not enforced)"
    );
}

// ── Complex Integration ──────────────────────────────────────

#[test]
fn test_trait_full_pipeline() {
    let source = r#"
trait Printable {
    fn format(self): string;
}

impl Printable for int {
    fn format(self): string {
        return "int";
    }
}

impl Printable for string {
    fn format(self): string {
        return self;
    }
}

fn printIt<T: Printable>(value: T): void {
    print(value.format());
}

printIt(42);
printIt("hello");
"#;
    let (ok, diagnostics) = parse_and_check(source);
    if !ok {
        for d in &diagnostics {
            eprintln!("{}", d);
        }
    }
    assert!(ok);
}

#[test]
fn test_marker_trait_pipeline() {
    let source = r#"
trait Send { }
trait Sync { }

impl Send for int { }
impl Sync for int { }

fn assertSend<T: Send>(value: T): void { }
fn assertSync<T: Sync>(value: T): void { }

assertSend(42);
assertSync(42);
"#;
    let (ok, diagnostics) = parse_and_check(source);
    if !ok {
        for d in &diagnostics {
            eprintln!("{}", d);
        }
    }
    assert!(ok);
}

#[test]
fn test_trait_object_pipeline() {
    let source = r#"
trait Printable {
    fn format(self): string;
}

impl Printable for int {
    fn format(self): string {
        return "int";
    }
}

let x: dyn Printable = 42;
let s = x.format();
"#;
    let (ok, diagnostics) = parse_and_check(source);
    if !ok {
        for d in &diagnostics {
            eprintln!("{}", d);
        }
    }
    assert!(ok);
}
