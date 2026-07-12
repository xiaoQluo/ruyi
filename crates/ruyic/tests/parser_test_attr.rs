/**
 * Test framework + `@test` attribute parser tests (Spec 4.8, v0.5.7-p1-defects).
 *
 * Covers three scenarios:
 *   1. `@test fn foo() {}` parses into `Declaration::Function` with the
 *      `test` annotation captured in the new `annotations: Vec<String>`
 *      field, and is registered in `TestFunctionRegistry`.
 *   2. Two declarations named `foo` in different source files both land in
 *      the registry under their own `file:line` keys — the registry MUST
 *      NOT deduplicate by function name alone.
 *   3. A non-annotated `fn bar() {}` is NOT registered.
 *
 * @author Ruyi Team
 * @date 2026-07-12
 */
use ruyic::parser::ast::{Declaration, ModuleItem};
use ruyic::parser::Parser;
use ruyic::runtime::test_registry::{TestFnEntry, TestFunctionRegistry};

/** Helper: parse a source string into a `Program`. */
fn parse(source: &str) -> ruyic::parser::ast::Program {
    let mut parser = Parser::new(source).expect("lexer should not fail");
    parser.parse().expect("parse should succeed")
}

/** Extract every `Declaration::Function` from a parsed program, in order. */
fn fn_decls(program: &ruyic::parser::ast::Program) -> Vec<(String, Vec<String>)> {
    program
        .items
        .iter()
        .filter_map(|item| match item {
            ModuleItem::Declaration(Declaration::Function {
                name, annotations, ..
            }) => Some((name.clone(), annotations.clone())),
            _ => None,
        })
        .collect()
}

/** Collect every top-level declaration from a program. */
fn all_decls(program: &ruyic::parser::ast::Program) -> Vec<Declaration> {
    program
        .items
        .iter()
        .filter_map(|item| match item {
            ModuleItem::Declaration(d) => Some(d.clone()),
            _ => None,
        })
        .collect()
}

// ── 1. `@test` is captured on the AST node ─────────────────────────────

#[test]
fn test_parse_test_attribute() {
    let program = parse(
        r#"
        @test
        fn foo(): void {}
        "#,
    );

    let fns = fn_decls(&program);
    let (name, annotations) = fns
        .into_iter()
        .next()
        .expect("expected exactly one function declaration");

    assert_eq!(name, "foo");
    assert!(
        annotations.iter().any(|a| a == "test"),
        "expected `@test` annotation to be captured, got {:?}",
        annotations
    );
}

// ── 2. Same name in two different files — both registered ───────────────

#[test]
fn test_registry_dedup_by_location() {
    let program_a = parse(
        r#"
        @test
        fn runs(): void {}
        "#,
    );
    let program_b = parse(
        r#"
        @test
        fn runs(): void {}
        "#,
    );

    let mut registry = TestFunctionRegistry::new();
    registry.collect_from_program(&all_decls(&program_a), "a.ry", "module_a");
    registry.collect_from_program(&all_decls(&program_b), "b.ry", "module_b");

    assert_eq!(
        registry.count(),
        2,
        "two @test fns in different files must register as two distinct entries"
    );

    let files: Vec<&str> = registry.all().iter().map(|e| e.file.as_str()).collect();
    assert!(
        files.contains(&"a.ry") && files.contains(&"b.ry"),
        "registry must preserve the originating file for each entry, got: {:?}",
        files
    );

    let names: Vec<&str> = registry.all().iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["runs", "runs"]);
}

// ── 3. Non-annotated function is NOT registered ─────────────────────────

#[test]
fn test_no_attribute_no_register() {
    let program = parse(
        r#"
        fn bar(): int { return 1; }
        "#,
    );

    let mut registry = TestFunctionRegistry::new();
    registry.collect_from_program(&all_decls(&program), "no_test.ry", "module");

    assert_eq!(
        registry.count(),
        0,
        "function without @test annotation must not be registered"
    );
    assert!(
        registry.all().is_empty(),
        "registry must be empty for sources without @test"
    );
}

// ── 4. Sanity: TestFnEntry preserves all four location/name fields ─────

#[test]
fn test_entry_fields_are_preserved() {
    let mut registry = TestFunctionRegistry::new();
    registry.register(TestFnEntry {
        name: "my_check".to_string(),
        file: "tests/check.ry".to_string(),
        line: 7,
        module: "tests".to_string(),
    });

    let entries = registry.all();
    assert_eq!(entries.len(), 1);
    let entry = entries[0];
    assert_eq!(entry.name, "my_check");
    assert_eq!(entry.file, "tests/check.ry");
    assert_eq!(entry.line, 7);
    assert_eq!(entry.module, "tests");
}
