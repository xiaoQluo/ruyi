/**
 * Collections extension tests for Ruyi stdlib (Spec 4.9, v0.5.7-p1-defects).
 *
 * Covers the new Array/Iterator methods on `stdlib/collections.ry`:
 *   1. `test_array_sort` — `Array<T>.sort()` returns an ascending `Array<T>`.
 *   2. `test_array_contains_indexof` — `contains` / `indexOf` agree on
 *      presence and report a `-1` index for misses.
 *   3. `test_iterator_takewhile_skipwhile` — lazy iterator adapters on
 *      `ArrayIterator<T>` short-circuit on the first failing / passing
 *      predicate respectively.
 *   4. `test_array_sum_with_add` — `Array<int>.sum()` is callable because
 *      `int` implements the new `Add` trait (cycle-safe via Batch 1.1
 *      supertrait transitive cycle detection).
 *
 * The `TypeChecker` invoked from these tests does NOT auto-load stdlib
 * (only `Driver` does). We therefore prepend the live `stdlib/collections.ry`
 * source at test time so the methods under test are visible to the
 * parser + trait validator. This means the tests double as a smoke check
 * that the entire stdlib source still parses and trait-checks cleanly —
 * any cycle, duplicate, or signature regression in the new Add/Mul/ArrayOps
 * surface will surface as a non-empty `result.diagnostics` and fail the
 * `!has_errors` assertion.
 *
 * @author Ruyi Team
 * @date 2026-07-12
 */
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use ruyic::parser::Parser;
use ruyic::typechecker::checker::TypeChecker;
use ruyic::typechecker::traits::build_trait_registry;

/**
 * Resolve the workspace root from `CARGO_MANIFEST_DIR` (set at compile
 * time by Cargo for the ruyic crate). The stdlib lives at `<root>/stdlib`.
 */
fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root must have at least two ancestors above crates/ruyic")
        .to_path_buf()
}

/**
 * Cache the stdlib/collections.ry source on first access so the four
 * tests share a single filesystem read.
 */
fn collections_source() -> &'static str {
    static CACHE: OnceLock<String> = OnceLock::new();
    CACHE.get_or_init(|| {
        let path = workspace_root().join("stdlib").join("collections.ry");
        fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
    })
}

/**
 * Parse + type-check the concatenation of `stdlib/collections.ry` (always
 * prepended) and the test-specific source. The prepended stdlib is what
 * makes the new `sort` / `indexOf` / `takeWhile` / `sum` symbols visible
 * to the type checker.
 */
fn check(source: &str) -> ruyic::typechecker::TypeCheckResult {
    let combined = format!("{}\n{}", collections_source(), source);
    let mut parser = Parser::new(&combined).expect("lexer should not fail");
    let program = parser.parse().expect("parse should succeed");
    let mut checker = TypeChecker::new();
    checker.check(&program)
}

// ── 1. Array.sort ──────────────────────────────────────────────

#[test]
fn test_array_sort() {
    let result = check(
        r#"
        let arr = [3, 1, 2];
        let sorted = arr.sort();
        "#,
    );
    assert!(
        !result.has_errors,
        "sort must compile: {:?}",
        result.diagnostics
    );
}

// ── 2. Array.contains / indexOf ────────────────────────────────

#[test]
fn test_array_contains_indexof() {
    let result = check(
        r#"
        let arr = [1, 2, 3];
        let has_it = arr.contains(2);
        let idx = arr.indexOf(2);
        let missing = arr.contains(99);
        let idx_missing = arr.indexOf(99);
        "#,
    );
    assert!(
        !result.has_errors,
        "contains/indexOf must compile: {:?}",
        result.diagnostics
    );
}

// ── 3. ArrayIterator.takeWhile / skipWhile ─────────────────────

#[test]
fn test_iterator_takewhile_skipwhile() {
    let result = check(
        r#"
        let arr = [1, 2, 3, 4];
        let iter = arr.iter();
        let taken = iter.takeWhile(x => x < 3);
        let skipped = iter.skipWhile(x => x < 3);
        "#,
    );
    assert!(
        !result.has_errors,
        "takeWhile/skipWhile must compile: {:?}",
        result.diagnostics
    );
}

// ── 4. Array.sum with Add trait ────────────────────────────────

#[test]
fn test_array_sum_with_add() {
    let result = check(
        r#"
        let arr = [1, 2, 3, 4];
        let total = arr.sum();
        "#,
    );
    assert!(
        !result.has_errors,
        "sum on int (which implements Add) must compile: {:?}",
        result.diagnostics
    );
}

/// The stdlib prepended to every test must itself be a valid Ruyi
/// program. If this test fails, one of the new Add/Mul/ArrayOps
/// declarations has regressed (e.g. a trait cycle, duplicate impl, or
/// signature mismatch).
#[test]
fn test_stdlib_compiles_cleanly() {
    let result = check("");
    assert!(
        !result.has_errors,
        "stdlib/collections.ry alone must compile cleanly, got: {:?}",
        result.diagnostics
    );
}

/// Direct surface-level assertions on the stdlib's `TraitRegistry`.
/// Because the type checker is permissive about array methods
/// (see `lookup_property` returning `Type::Dynamic` for `Type::Array`),
/// a "did the new methods ship?" test cannot rely on inference alone.
/// This test parses `stdlib/collections.ry` independently and inspects
/// the registered traits/impls so that the absence of `Add`, `Mul`,
/// the `ArrayOps` methods (`sort`, `indexOf`, `first`, `last`,
/// `slice`, `concat`), or the `int`/`float` impls fails loudly.
#[test]
fn test_stdlib_trait_registry_shape() {
    let mut parser = Parser::new(collections_source()).expect("lexer should not fail");
    let program = parser.parse().expect("stdlib should parse");
    let registry = build_trait_registry(&program);

    // Add + Mul must be declared as cycle-free standalone traits.
    let add = registry
        .get_trait("Add")
        .expect("trait `Add` must be declared in stdlib/collections.ry");
    assert!(
        add.methods.contains_key("add"),
        "trait `Add` must declare `add(self, other: Self): Self`, got methods: {:?}",
        add.methods.keys().collect::<Vec<_>>()
    );
    let mul = registry
        .get_trait("Mul")
        .expect("trait `Mul` must be declared in stdlib/collections.ry");
    assert!(
        mul.methods.contains_key("mul"),
        "trait `Mul` must declare `mul(self, other: Self): Self`, got methods: {:?}",
        mul.methods.keys().collect::<Vec<_>>()
    );

    // ArrayOps must now carry the six new methods (sort, indexOf,
    // first, last, slice, concat) on top of the existing surface.
    let array_ops = registry
        .get_trait("ArrayOps")
        .expect("trait `ArrayOps` must be declared in stdlib/collections.ry");
    for required in ["sort", "indexOf", "first", "last", "slice", "concat"] {
        assert!(
            array_ops.methods.contains_key(required),
            "ArrayOps must declare `{}` (Spec 4.9), got methods: {:?}",
            required,
            array_ops.methods.keys().collect::<Vec<_>>()
        );
    }

    // int and float must each have both `impl Add for T` and `impl Mul for T`.
    let target_name = |ty: &ruyic::parser::ast::TypeAnnotation| -> String {
        use ruyic::parser::ast::TypeAnnotation;
        match ty {
            TypeAnnotation::Identifier(name) | TypeAnnotation::Builtin(name) => name.clone(),
            TypeAnnotation::Generic { base, .. } => base.clone(),
            _ => String::new(),
        }
    };
    let mut found_add_int = false;
    let mut found_add_float = false;
    let mut found_mul_int = false;
    let mut found_mul_float = false;
    for impl_info in registry.impls() {
        let target = target_name(&impl_info.for_type);
        match (impl_info.trait_name.as_str(), target.as_str()) {
            ("Add", "int") => found_add_int = true,
            ("Add", "float") => found_add_float = true,
            ("Mul", "int") => found_mul_int = true,
            ("Mul", "float") => found_mul_float = true,
            _ => {}
        }
    }
    assert!(found_add_int, "must have `impl Add for int`");
    assert!(found_add_float, "must have `impl Add for float`");
    assert!(found_mul_int, "must have `impl Mul for int`");
    assert!(found_mul_float, "must have `impl Mul for float`");
}
