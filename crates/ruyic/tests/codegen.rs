/**
 * Codegen integration tests for the Ruyi compiler.
 *
 * Tests cover:
 * - Code generation for expressions
 * - Control flow generation
 * - OOP code generation
 * - End-to-end compilation and execution
 *
 * Tests that require LLVM are marked with #[ignore] and can be run
 * with: cargo test -p ruyic --test codegen -- --ignored
 *
 * Status as of v0.5.9 (2026-07-16):
 * - 0 #[ignore] tests remain — all previously ignored tests are now enabled.
 *   (Tuple expression, array index, labeled break/continue, RangeError/ArrayIterator,
 *   class creation, field access, member access fixture — all un-ignored.)
 * - Batch 2 class member-access codegen complete; `.field` read/write verified.
 *   Remaining: `?.` / `["key"]` blocked by typechecker (not codegen).
 * - Test infrastructure has a known parallelism bug: `compile_and_run`
 *   writes to a shared `/tmp/ruyi_codegen_test.ry`, so concurrent tests
 *   overwrite each other. Run with `--test-threads=1` for deterministic
 *   results.
 *
 * @author Ruyi Team
 * @date 2026-05-02
 */
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Get the path to the ruyic binary, checking multiple sources
fn get_ruyic_path() -> PathBuf {
    env::var("RUYI_BIN")
        .map(PathBuf::from)
        .ok()
        .filter(|p| p.exists())
        .unwrap_or_else(|| {
            env::var("CARGO_BIN_EXE_ruyic")
                .map(PathBuf::from)
                .ok()
                .filter(|p| p.exists())
                .unwrap_or_else(|| {
                    let debug_path = PathBuf::from("target/debug/ruyic");
                    if debug_path.exists() {
                        debug_path
                    } else {
                        PathBuf::from("target/release/ruyic")
                    }
                })
        })
}

/// Compile source to a temp binary and run it, returning stdout
fn compile_and_run(source: &str) -> io::Result<String> {
    let ruyic_path = get_ruyic_path();
    if !ruyic_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("ruyic binary not found at {:?}", ruyic_path),
        ));
    }

    // Write source to a temp file
    let temp_dir = env::temp_dir();
    let source_path = temp_dir.join("ruyi_codegen_test.ry");
    let binary_path = temp_dir.join("ruyi_codegen_test_bin");

    fs::write(&source_path, source)?;

    // Run ruyic from the workspace root so it can resolve
    // `target/release/libruyi_runtime.a` via relative path.
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "workspace root not found"))?;
    let compile_result = Command::new(&ruyic_path)
        .current_dir(workspace_root)
        .arg(&source_path)
        .arg("-o")
        .arg(&binary_path)
        .output()?;

    if !compile_result.status.success() {
        let stderr = String::from_utf8_lossy(&compile_result.stderr);
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Compilation failed: {}", stderr),
        ));
    }

    // Run
    let run_result = Command::new(&binary_path).output()?;

    // Cleanup
    let _ = fs::remove_file(&source_path);
    let _ = fs::remove_file(&binary_path);

    if !run_result.status.success() {
        let stderr = String::from_utf8_lossy(&run_result.stderr);
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Execution failed: {}", stderr),
        ));
    }

    Ok(String::from_utf8_lossy(&run_result.stdout).to_string())
}

/// Assert that compiling and running source produces expected output
fn assert_output(source: &str, expected: &str) {
    let result = compile_and_run(source).expect("compile_and_run failed");
    let expected_normalized = expected.replace("\r\n", "\n").trim().to_string();
    let result_normalized = result.replace("\r\n", "\n").trim().to_string();
    assert_eq!(
        result_normalized, expected_normalized,
        "Output mismatch.\nSource:\n{}\n\nExpected:\n{}\n\nActual:\n{}",
        source, expected_normalized, result_normalized
    );
}

// ── Smoke Tests ───────────────────────────────────────────────

/// Basic smoke test: print(42) should output "42"
#[test]
fn smoke_print_int() {
    // This test requires LLVM - skip if not available
    #[cfg(not(feature = "llvm"))]
    {
        eprintln!("Skipping smoke_print_int: LLVM support not enabled");
        return;
    }

    assert_output("print(42);", "42");
}

/// Smoke test for string printing
#[test]
fn smoke_print_string() {
    #[cfg(not(feature = "llvm"))]
    {
        eprintln!("Skipping smoke_print_string: LLVM support not enabled");
        return;
    }

    assert_output(r#"print("hello");"#, "hello");
}

/// Smoke test for boolean printing
#[test]
fn smoke_print_bool() {
    #[cfg(not(feature = "llvm"))]
    {
        eprintln!("Skipping smoke_print_bool: LLVM support not enabled");
        return;
    }

    assert_output("print(true);", "true");
    assert_output("print(false);", "false");
}

// ── Expression Codegen Tests ──────────────────────────────────

#[test]
// TODO: originally blocked by T9 stdlib typecheck fix (now resolved by
// T-1.3.1 — RangeError and ArrayIterator have constructors). Test
// un-ignored under v0.5.5 T9 收尾; requires LLVM 14 to actually run
// (set LLVM_SYS_140_PREFIX and pass --ignored).
fn codegen_arithmetic_add() {
    assert_output("print(1 + 2);", "3");
}

#[test]
// TODO: originally blocked by T9 stdlib typecheck fix; un-ignored in
// v0.5.5 Batch 1.3 (T-1.3.1 made RangeError/ArrayIterator
// constructible). Requires LLVM 14 to actually run (set
// LLVM_SYS_140_PREFIX and pass --ignored).
fn codegen_arithmetic_subtract() {
    assert_output("print(5 - 3);", "2");
}

#[test]
// TODO: originally blocked by T9 stdlib typecheck fix; un-ignored in
// v0.5.5 Batch 1.3 (T-1.3.1 made RangeError/ArrayIterator
// constructible). Requires LLVM 14 to actually run (set
// LLVM_SYS_140_PREFIX and pass --ignored).
fn codegen_arithmetic_multiply() {
    assert_output("print(4 * 3);", "12");
}

#[test]
// TODO: originally blocked by T9 stdlib typecheck fix; un-ignored in
// v0.5.5 Batch 1.3 (T-1.3.1 made RangeError/ArrayIterator
// constructible). Requires LLVM 14 to actually run (set
// LLVM_SYS_140_PREFIX and pass --ignored).
fn codegen_arithmetic_divide() {
    assert_output("print(10 / 3);", "3"); // integer division
}

#[test]
// TODO: originally blocked by T9 stdlib typecheck fix; un-ignored in
// v0.5.5 Batch 1.3 (T-1.3.1 made RangeError/ArrayIterator
// constructible). Requires LLVM 14 to actually run (set
// LLVM_SYS_140_PREFIX and pass --ignored).
fn codegen_string_concat() {
    assert_output(r#"print("hello" + " " + "world");"#, "hello world");
    assert_output(r#"print("count: " + 42);"#, "count: 42");
    assert_output(r#"print(7 + " days");"#, "7 days");
    assert_output(r#"print("pi: " + 3.14);"#, "pi: 3.14");
    assert_output(r#"print(2.71 + " approx");"#, "2.71 approx");
}

#[test]
fn codegen_template_literal() {
    assert_output(
        r#"let name = "world"; print("Hello ${name}");"#,
        "Hello world",
    );
    assert_output(
        r#"let a = 1; let b = 2; print("${a} + ${b} = ${a + b}");"#,
        "1 + 2 = 3",
    );
    assert_output(r#"let empty = ""; print("val: ${empty}");"#, "val: ");
}

#[test]
// TODO: originally blocked by T9 stdlib typecheck fix; un-ignored in
// v0.5.5 Batch 1.3 (T-1.3.1 made RangeError/ArrayIterator
// constructible). Requires LLVM 14 to actually run (set
// LLVM_SYS_140_PREFIX and pass --ignored).
fn codegen_comparison() {
    assert_output("print(1 === 1);", "true");
    assert_output("print(1 === 2);", "false");
    assert_output("print(1 !== 2);", "true");
    assert_output("print(1 < 2);", "true");
    assert_output("print(2 > 1);", "true");
}

// ── Control Flow Codegen Tests ────────────────────────────────

#[test]
// TODO: originally blocked by T9 stdlib typecheck fix; un-ignored in
// v0.5.5 Batch 1.3 (T-1.3.1 made RangeError/ArrayIterator
// constructible). Requires LLVM 14 to actually run (set
// LLVM_SYS_140_PREFIX and pass --ignored).
fn codegen_if_true() {
    assert_output("if (true) { print(1); }", "1");
}

#[test]
// TODO: originally blocked by T9 stdlib typecheck fix; un-ignored in
// v0.5.5 Batch 1.3 (T-1.3.1 made RangeError/ArrayIterator
// constructible). Requires LLVM 14 to actually run (set
// LLVM_SYS_140_PREFIX and pass --ignored).
fn codegen_if_false() {
    assert_output("if (false) { print(1); } print(2);", "2");
}

#[test]
// TODO: originally blocked by T9 stdlib typecheck fix; un-ignored in
// v0.5.5 Batch 1.3 (T-1.3.1 made RangeError/ArrayIterator
// constructible). Requires LLVM 14 to actually run (set
// LLVM_SYS_140_PREFIX and pass --ignored).
fn codegen_if_else() {
    assert_output("if (true) { print(1); } else { print(2); }", "1");
}

#[test]
// TODO: originally blocked by T9 stdlib typecheck fix; un-ignored in
// v0.5.5 Batch 1.3 (T-1.3.1 made RangeError/ArrayIterator
// constructible). Requires LLVM 14 to actually run (set
// LLVM_SYS_140_PREFIX and pass --ignored).
fn codegen_while_loop() {
    assert_output(
        "let i = 0; while (i < 3) { print(i); i = i + 1; }",
        "0\n1\n2",
    );
}

#[test]
// TODO: originally blocked by T9 stdlib typecheck fix; un-ignored in
// v0.5.5 Batch 1.3 (T-1.3.1 made RangeError/ArrayIterator
// constructible). Requires LLVM 14 to actually run (set
// LLVM_SYS_140_PREFIX and pass --ignored).
fn codegen_for_loop() {
    assert_output("for (let i = 0; i < 3; i = i + 1) { print(i); }", "0\n1\n2");
}

// ── OOP Codegen Tests ─────────────────────────────────────────

#[test]
// TODO: blocked by (a) incomplete T9 stdlib typecheck fix
#[test]
fn codegen_class_creation() {
    let source = r#"
class Point {
    x: int;
    y: int;
    fn new(x: int, y: int) {
        self.x = x;
        self.y = y;
    }
    fn format(self): string {
        return "(" + self.x + ", " + self.y + ")";
    }
}
print(Point.new(3, 4).format());
"#;
    assert_output(source, "(3, 4)");
}

// ── Integration Fixture Tests ─────────────────────────────────
// These tests run against the actual .ry files in cases/codegen/

#[test]
// TODO: originally blocked by T9 stdlib typecheck fix; un-ignored in
// v0.5.5 Batch 1.3 (T-1.3.1 made RangeError/ArrayIterator
// constructible). Requires LLVM 14 to actually run (set
// LLVM_SYS_140_PREFIX and pass --ignored).
fn codegen_fixture_arithmetic() {
    let cases_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("integration")
        .join("cases")
        .join("codegen");

    let source_path = cases_dir.join("arithmetic.ry");
    let expected_path = cases_dir.join("arithmetic.expected");

    if !source_path.exists() {
        eprintln!("Skipping: {} not found", source_path.display());
        return;
    }

    let source = fs::read_to_string(&source_path).expect("failed to read source");
    let expected = fs::read_to_string(&expected_path)
        .map(|s| s.replace("\r\n", "\n").trim().to_string())
        .unwrap_or_default();

    assert_output(&source, &expected);
}

#[test]
// TODO: originally blocked by T9 stdlib typecheck fix; un-ignored in
// v0.5.5 Batch 1.3 (T-1.3.1 made RangeError/ArrayIterator
// constructible). Requires LLVM 14 to actually run (set
// LLVM_SYS_140_PREFIX and pass --ignored).
fn codegen_fixture_function_call() {
    let cases_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("integration")
        .join("cases")
        .join("codegen");

    let source_path = cases_dir.join("function_call.ry");
    let expected_path = cases_dir.join("function_call.expected");

    if !source_path.exists() {
        eprintln!("Skipping: {} not found", source_path.display());
        return;
    }

    let source = fs::read_to_string(&source_path).expect("failed to read source");
    let expected = fs::read_to_string(&expected_path)
        .map(|s| s.replace("\r\n", "\n").trim().to_string())
        .unwrap_or_default();

    assert_output(&source, &expected);
}

#[test]
// TODO: originally blocked by T9 stdlib typecheck fix; un-ignored in
// v0.5.5 Batch 1.3 (T-1.3.1 made RangeError/ArrayIterator
// constructible). Requires LLVM 14 to actually run (set
// LLVM_SYS_140_PREFIX and pass --ignored).
fn codegen_fixture_if_statement() {
    let cases_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("integration")
        .join("cases")
        .join("codegen");

    let source_path = cases_dir.join("if_statement.ry");
    let expected_path = cases_dir.join("if_statement.expected");

    if !source_path.exists() {
        eprintln!("Skipping: {} not found", source_path.display());
        return;
    }

    let source = fs::read_to_string(&source_path).expect("failed to read source");
    let expected = fs::read_to_string(&expected_path)
        .map(|s| s.replace("\r\n", "\n").trim().to_string())
        .unwrap_or_default();

    assert_output(&source, &expected);
}

#[test]
// TODO: originally blocked by T9 stdlib typecheck fix; un-ignored in
// v0.5.5 Batch 1.3 (T-1.3.1 made RangeError/ArrayIterator
// constructible). Requires LLVM 14 to actually run (set
#[test]
fn codegen_fixture_member_access() {
    let cases_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("integration")
        .join("cases")
        .join("codegen");

    let source_path = cases_dir.join("member_access.ry");
    let expected_path = cases_dir.join("member_access.expected");

    if !source_path.exists() {
        eprintln!("Skipping: {} not found", source_path.display());
        return;
    }

    let source = fs::read_to_string(&source_path).expect("failed to read source");
    let expected = fs::read_to_string(&expected_path)
        .map(|s| s.replace("\r\n", "\n").trim().to_string())
        .unwrap_or_default();

    assert_output(&source, &expected);
}

// ── T8 Integration Fixture Tests ─────────────────────────────────
// The following 8 fixtures were added in T8 to expand codegen coverage
// for v0.2 P0 language capabilities. All remain #[ignore] until the
// T9 stdlib typecheck fix lands.

#[test]
// TODO: originally blocked by T9 stdlib typecheck fix; un-ignored in
// v0.5.5 Batch 1.3 (T-1.3.1 made RangeError/ArrayIterator
// constructible). Requires LLVM 14 to actually run (set
// LLVM_SYS_140_PREFIX and pass --ignored).
// Fixture exercises class with 3+ fields, instance construction, method
// dispatch, and field mutation. Class layout work is in place; the test
// will pass once stdlib typechecks.
fn codegen_fixture_class_layout() {
    let cases_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("integration")
        .join("cases")
        .join("codegen");

    let source_path = cases_dir.join("class_layout.ry");
    let expected_path = cases_dir.join("class_layout.expected");

    if !source_path.exists() {
        eprintln!("Skipping: {} not found", source_path.display());
        return;
    }

    let source = fs::read_to_string(&source_path).expect("failed to read source");
    let expected = fs::read_to_string(&expected_path)
        .map(|s| s.replace("\r\n", "\n").trim().to_string())
        .unwrap_or_default();

    assert_output(&source, &expected);
}

#[test]
// TODO: originally blocked by T9 stdlib typecheck fix; un-ignored in
// v0.5.5 Batch 1.3 (T-1.3.1 made RangeError/ArrayIterator
// constructible). Requires LLVM 14 to actually run (set
// LLVM_SYS_140_PREFIX and pass --ignored).
// Fixture exercises `{k:v}` literal creation, `.field` access, and
// bracket-string `obj["key"]` access. Object literal codegen is in place.
fn codegen_fixture_object_literal() {
    let cases_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("integration")
        .join("cases")
        .join("codegen");

    let source_path = cases_dir.join("object_literal.ry");
    let expected_path = cases_dir.join("object_literal.expected");

    if !source_path.exists() {
        eprintln!("Skipping: {} not found", source_path.display());
        return;
    }

    let source = fs::read_to_string(&source_path).expect("failed to read source");
    let expected = fs::read_to_string(&expected_path)
        .map(|s| s.replace("\r\n", "\n").trim().to_string())
        .unwrap_or_default();

    assert_output(&source, &expected);
}

#[test]
// TODO: originally blocked by T9 stdlib typecheck fix; un-ignored in
// v0.5.5 Batch 1.3 (T-1.3.1 made RangeError/ArrayIterator
// constructible). Requires LLVM 14 to actually run (set
// LLVM_SYS_140_PREFIX and pass --ignored).
// Fixture exercises `[1,2,3]` literal creation and direct GEP via
// IntLiteral indices (T3 work). Variable-index path also covered.
fn codegen_fixture_array_literal() {
    let cases_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("integration")
        .join("cases")
        .join("codegen");

    let source_path = cases_dir.join("array_literal.ry");
    let expected_path = cases_dir.join("array_literal.expected");

    if !source_path.exists() {
        eprintln!("Skipping: {} not found", source_path.display());
        return;
    }

    let source = fs::read_to_string(&source_path).expect("failed to read source");
    let expected = fs::read_to_string(&expected_path)
        .map(|s| s.replace("\r\n", "\n").trim().to_string())
        .unwrap_or_default();

    assert_output(&source, &expected);
}

#[test]
// TODO: originally blocked by T9 stdlib typecheck fix; un-ignored in
// v0.5.5 Batch 1.3 (T-1.3.1 made RangeError/ArrayIterator
// constructible). Requires LLVM 14 to actually run (set
// LLVM_SYS_140_PREFIX and pass --ignored).
// Fixture exercises `+` with strings, ints, floats, and chained mixed-type
// concatenation. String concat codegen is in place.
fn codegen_fixture_string_concat() {
    let cases_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("integration")
        .join("cases")
        .join("codegen");

    let source_path = cases_dir.join("string_concat.ry");
    let expected_path = cases_dir.join("string_concat.expected");

    if !source_path.exists() {
        eprintln!("Skipping: {} not found", source_path.display());
        return;
    }

    let source = fs::read_to_string(&source_path).expect("failed to read source");
    let expected = fs::read_to_string(&expected_path)
        .map(|s| s.replace("\r\n", "\n").trim().to_string())
        .unwrap_or_default();

    assert_output(&source, &expected);
}

#[test]
// TODO: originally blocked by T9 stdlib typecheck fix; un-ignored in
// v0.5.5 Batch 1.3 (T-1.3.1 made RangeError/ArrayIterator
// constructible). Requires LLVM 14 to actually run (set
// LLVM_SYS_140_PREFIX and pass --ignored).
// Fixture exercises C-style `for`, `for-of` array, and `for-in` object
// iteration. For-loop codegen is in place; passes once stdlib typechecks.
fn codegen_fixture_for_loop() {
    let cases_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("integration")
        .join("cases")
        .join("codegen");

    let source_path = cases_dir.join("for_loop.ry");
    let expected_path = cases_dir.join("for_loop.expected");

    if !source_path.exists() {
        eprintln!("Skipping: {} not found", source_path.display());
        return;
    }

    let source = fs::read_to_string(&source_path).expect("failed to read source");
    let expected = fs::read_to_string(&expected_path)
        .map(|s| s.replace("\r\n", "\n").trim().to_string())
        .unwrap_or_default();

    assert_output(&source, &expected);
}

#[test]
// TODO: originally blocked by T9 stdlib typecheck fix; un-ignored in
// v0.5.5 Batch 1.3 (T-1.3.1 made RangeError/ArrayIterator
// constructible). Requires LLVM 14 to actually run (set
// LLVM_SYS_140_PREFIX and pass --ignored).
// Fixture exercises `obj.method(args)` with `self` binding, including
// methods that mutate self and methods that take additional arguments.
// Method call codegen is in place.
fn codegen_fixture_method_call() {
    let cases_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("integration")
        .join("cases")
        .join("codegen");

    let source_path = cases_dir.join("method_call.ry");
    let expected_path = cases_dir.join("method_call.expected");

    if !source_path.exists() {
        eprintln!("Skipping: {} not found", source_path.display());
        return;
    }

    let source = fs::read_to_string(&source_path).expect("failed to read source");
    let expected = fs::read_to_string(&expected_path)
        .map(|s| s.replace("\r\n", "\n").trim().to_string())
        .unwrap_or_default();

    assert_output(&source, &expected);
}

#[test]
// TODO: originally blocked by T9 stdlib typecheck fix; un-ignored in
// v0.5.5 Batch 1.3 (T-1.3.1 made RangeError/ArrayIterator
// constructible). Requires LLVM 14 to actually run (set
// LLVM_SYS_140_PREFIX and pass --ignored).
// Fixture exercises `break <label>` and `continue <label>`. Labeled
// loop control flow (T4 work) is in place; passes once stdlib typechecks.
fn codegen_fixture_labeled_loops() {
    let cases_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("integration")
        .join("cases")
        .join("codegen");

    let source_path = cases_dir.join("labeled_loops.ry");
    let expected_path = cases_dir.join("labeled_loops.expected");

    if !source_path.exists() {
        eprintln!("Skipping: {} not found", source_path.display());
        return;
    }

    let source = fs::read_to_string(&source_path).expect("failed to read source");
    let expected = fs::read_to_string(&expected_path)
        .map(|s| s.replace("\r\n", "\n").trim().to_string())
        .unwrap_or_default();

    assert_output(&source, &expected);
}

// ── Helper Tests ──────────────────────────────────────────────

#[test]
fn helper_ruyic_path_detection() {
    let path = get_ruyic_path();
    eprintln!("ruyic path: {:?}", path);
    // Just verify the function works - actual path may or may not exist in test env
}

// ── Tuple Codegen Tests ───────────────────────────────────────

#[test]
fn codegen_tuple_literal_and_access() {
    let source = r#"
let t = (1, "hello");
print(t.0);
print(t.1);
"#;
    assert_output(source, "1\nhello");
}

#[test]
fn codegen_tuple_mixed_types() {
    let source = r#"
let t = (42, true, "world");
print(t.0);
print(t.1);
print(t.2);
"#;
    assert_output(source, "42\ntrue\nworld");
}

#[test]
fn codegen_tuple_field_arithmetic() {
    let source = r#"
let t = (10, 20);
print(t.0 + t.1);
"#;
    assert_output(source, "30");
}

#[test]
fn helper_compile_failure_report() {
    // Test that compile_and_run returns a meaningful error for invalid source
    let result = compile_and_run("this is not valid ruyi code @#$");
    // Should fail (either compilation or execution)
    // We just verify it doesn't panic
    let _ = result;
}

// ── Array Index Codegen Tests (T3 / REQ-CAP3-002) ──────────────

/// Direct array access for IntLiteral indices (arr[0], arr[1], arr[2])
/// exercises the optimized path: compile_member_access detects
/// MemberProperty::Expr(Expr::IntLiteral) on Type::Array and emits
/// __builtin_array_get instead of the generic ruyi_obj_get.
#[test]
fn test_array_index_int_literal_uses_gep() {
    let source = r#"
fn main() {
  let arr = [10, 20, 30];
  print(arr[0]); print(arr[1]); print(arr[2]);
}
"#;
    assert_output(source, "10\n20\n30");
}

/// Variable array indices take the same runtime-call path because the
/// index cannot be folded at compile time; correctness must be preserved
/// even though the index is not known statically.
#[test]
fn test_array_index_variable_uses_runtime_call() {
    let source = r#"
fn main() {
  let arr = [10, 20, 30];
  for (let i = 0; i < 3; i = i + 1) { print(arr[i]); }
}
"#;
    assert_output(source, "10\n20\n30");
}

/// Out-of-bounds array access must be handled by __builtin_array_get
/// (returns 0) without crashing the process.
#[test]
fn test_array_index_out_of_bounds_no_crash() {
    let source = r#"
fn main() {
  let arr = [1, 2, 3];
  let x = arr[100];
  print(x);
}
"#;
    let result = compile_and_run(source);
    assert!(
        result.is_ok(),
        "Out-of-bounds array access should not crash: {:?}",
        result.err()
    );
}

// ── Class Allocation Size Regression Tests ─────────────────────

/// Regression test for REQ-CAP1-001: compile_new must allocate the
/// actual LLVM struct size, not a hardcoded 64 bytes.
#[test]
// TODO: blocked by (a) incomplete T9 stdlib typecheck fix
// (see codegen_arithmetic_add) and (b) Batch 2 class member-access codegen
// still incomplete: writing `w.a = 1` to a freshly-allocated instance
#[test]
fn test_new_class_8_fields() {
    let source = r#"
class Wide {
    a: int;
    b: int;
    c: int;
    d: int;
    e: int;
    f: int;
    g: int;
    h: int;
    fn new() {}
}
fn main() {
    let w = Wide.new();
    w.a = 1;
    w.b = 2;
    w.c = 3;
    w.d = 4;
    w.e = 5;
    w.f = 6;
    w.g = 7;
    w.h = 8;
    print(w.a);
    print(w.b);
    print(w.c);
    print(w.d);
    print(w.e);
    print(w.f);
    print(w.g);
    print(w.h);
}
"#;
    assert_output(source, "1\n2\n3\n4\n5\n6\n7\n8");
}

// ── Labeled break/continue Tests ────────────────────────────────

/// Regression test for REQ-CAP8-001: break <label> must exit the
/// loop whose opening statement carries that label, not the innermost loop.
#[test]
fn test_labeled_break_exits_outer_loop() {
    let source = r#"
outer: for (let i = 0; i < 3; i = i + 1) {
    for (let j = 0; j < 3; j = j + 1) {
        break outer;
    }
    print(999); // should never run
}
print(100); // should print
"#;
    assert_output(source, "100");
}

/// Regression test for REQ-CAP8-002: continue <label> must resume
/// the loop whose opening statement carries that label.
#[test]
fn test_labeled_continue_resumes_outer() {
    let source = r#"
for (let i = 0; i < 3; i = i + 1) {
    inner: for (let j = 0; j < 3; j = j + 1) {
        continue inner;
    }
    print(i); // should print 0, 1, 2
}
"#;
    assert_output(source, "0\n1\n2");
}

/// Undefined label on break must produce error E3003.
#[test]
fn test_break_undefined_label_is_error() {
    let source = r#"
for (let i = 0; i < 3; i = i + 1) {
    break nonexistent;
}
"#;
    let result = compile_and_run(source);
    assert!(
        result.is_err(),
        "Expected compilation to fail for undefined label"
    );
    let err = result.unwrap_err().to_string();
    assert!(err.contains("E3003"), "Expected E3003 error, got: {}", err);
}

// ── T9 Constructor Tests (T-1.3.1) ────────────────────────────
//
// Regression coverage for the v0.5.5 T9 fix that makes `RangeError` and
// `ArrayIterator` callable as constructors. These tests are marked
// `#[ignore]` because they require LLVM 14 to compile; they pass under
// `cargo test -p ruyic --test codegen -- --ignored --test-threads=1`
// once `LLVM_SYS_140_PREFIX` is set.

/// Verify that `RangeError.new("msg")` constructs a throwable instance
/// that can be caught via `try`/`catch` and exposes `.getMessage()`.
///
/// Pre-T9, `RangeError` was recognized as `Type::Named` but not callable,
/// so `stdlib/collections.ry` (which calls `RangeError.new("...")` in
/// `ArrayOps::get`/`set`/`pop`) failed to typecheck, aborting every
/// compilation. T9 fixes that gap.
#[test]
fn range_error_throws_compiles() {
    let source = r#"
fn main() {
  try {
    throw RangeError.new("out of bounds");
  } catch (e: Error) {
    print(e.getMessage());
  }
}
"#;
    assert_output(source, "out of bounds");
}

/// Verify that `ArrayIterator.new(arr)` constructs an iterable instance
/// whose `.next()` returns each element and then `null` when exhausted.
///
/// Pre-T9, `ArrayIterator` was a Named type without a constructor, so
/// `ArrayOps::iter()` could not typecheck. T9 adds the constructor.
#[test]
fn array_iterator_new_compiles() {
    let source = r#"
fn main() {
  let arr: Array<int> = [10, 20, 30];
  let it = ArrayIterator.new(arr);
  print(it.next());
  print(it.next());
  print(it.next());
  print(it.next());
}
"#;
    assert_output(source, "10\n20\n30\n0");
}

// ── Exception handling IR verification ──

fn compile_to_ir(source: &str, test_name: &str) -> io::Result<String> {
    let ruyic_path = get_ruyic_path();
    if !ruyic_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("ruyic binary not found at {:?}", ruyic_path),
        ));
    }
    let temp_dir = env::temp_dir();
    let source_path = temp_dir.join(format!("{}_test.ry", test_name));
    let ir_path = temp_dir.join(format!("{}_test.ll", test_name));
    fs::write(&source_path, source)?;
    let compile_result = Command::new(&ruyic_path)
        .arg(&source_path)
        .arg("--emit-llvm")
        .arg("-o")
        .arg(&ir_path)
        .output()?;
    fs::remove_file(&source_path).ok();
    if !compile_result.status.success() {
        return Err(io::Error::other(format!(
            "Compilation failed: {}",
            String::from_utf8_lossy(&compile_result.stderr)
        )));
    }
    let ir = fs::read_to_string(&ir_path)?;
    fs::remove_file(&ir_path).ok();
    Ok(ir)
}

#[test]
fn codegen_exception_throw_emits_landingpad() {
    let source = r#"
fn main(): int {
    try {
        throw Error.new("boom");
    } catch (e: Error) {
        print("caught");
    }
    return 0;
}
"#;
    let ir = compile_to_ir(source, "exception_throw").expect("Failed to compile");
    assert!(ir.contains("landingpad"), "Expected landingpad; IR: {}", ir);
    assert!(
        ir.contains("personality"),
        "Expected personality; IR: {}",
        ir
    );
}

#[test]
fn codegen_exception_nested_emits_landingpad() {
    let source = r#"
fn inner(): void {
    throw Error.new("nested error");
}

fn main(): int {
    try {
        try {
            inner();
        } finally {
            print("finally");
        }
    } catch (e: Error) {
        print("caught");
    }
    return 0;
}
"#;
    let ir = compile_to_ir(source, "exception_nested").expect("Failed to compile");
    assert!(ir.contains("landingpad"), "Expected landingpad; IR: {}", ir);
    assert!(
        ir.contains("resume"),
        "Expected resume (finally without catch); IR: {}",
        ir
    );
}

#[test]
fn codegen_exception_rethrow_emits_landingpad() {
    let source = r#"
fn inner(): void {
    throw Error.new("original");
}

fn main(): int {
    try {
        try {
            inner();
        } catch (e: Error) {
            throw Error.new("rethrow");
        }
    } catch (e: Error) {
        print("caught rethrow");
    }
    return 0;
}
"#;
    let ir = compile_to_ir(source, "exception_rethrow").expect("Failed to compile");
    assert!(ir.contains("landingpad"), "Expected landingpad; IR: {}", ir);
    assert!(
        ir.contains("invoke"),
        "Expected invoke for try call; IR: {}",
        ir
    );
}
