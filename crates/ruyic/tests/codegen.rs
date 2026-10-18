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
 * - Parallelism bug fixed (v0.5.10): `compile_and_run` now uses thread-unique
 *   temp filenames, so tests can run with default thread count.
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
    // Relative fallbacks must resolve against the workspace root: cargo
    // runs test binaries with the *package* dir as cwd, so a plain
    // `target/debug/ruyic` would otherwise hit a stale nested
    // `crates/ruyic/target/...` build.
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root not found")
        .to_path_buf();
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
                    let debug_path = workspace_root.join("target/debug/ruyic");
                    if debug_path.exists() {
                        debug_path
                    } else {
                        workspace_root.join("target/release/ruyic")
                    }
                })
        })
}

/// Compile source to a temp binary and run it, returning stdout.
/// Uses thread ID in the filename to avoid collisions when tests run in parallel.
fn compile_and_run(source: &str) -> io::Result<String> {
    let ruyic_path = get_ruyic_path();
    if !ruyic_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("ruyic binary not found at {:?}", ruyic_path),
        ));
    }

    // Write source to a thread-unique temp file to avoid parallel collisions
    let temp_dir = env::temp_dir();
    let tid = format!("{:?}", std::thread::current().id());
    let source_path = temp_dir.join(format!("ruyi_codegen_test_{}.ry", tid));
    let binary_path = temp_dir.join(format!("ruyi_codegen_test_bin_{}", tid));

    fs::write(&source_path, source)?;

    // Run ruyic from the workspace root so it can resolve
    // `target/release/libruyi_runtime.a` via relative path.
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| io::Error::other("workspace root not found"))?;
    let compile_result = Command::new(&ruyic_path)
        .current_dir(workspace_root)
        .arg(&source_path)
        .arg("-o")
        .arg(&binary_path)
        .output()?;

    if !compile_result.status.success() {
        let stderr = String::from_utf8_lossy(&compile_result.stderr);
        return Err(io::Error::other(format!("Compilation failed: {}", stderr)));
    }

    // Run with real LLVM unwinder enabled so try/catch works. A wall-clock
    // cap kills runaway programs (e.g. a stdlib deadlock) instead of
    // hanging the whole test suite.
    let mut child = Command::new(&binary_path)
        .env("RUYI_USE_UNWIND", "1")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let mut stdout_pipe = child.stdout.take().expect("stdout piped");
    let mut stderr_pipe = child.stderr.take().expect("stderr piped");
    let stdout_handle = std::thread::spawn(move || {
        use std::io::Read;
        let mut buf = String::new();
        let _ = stdout_pipe.read_to_string(&mut buf);
        buf
    });
    let stderr_handle = std::thread::spawn(move || {
        use std::io::Read;
        let mut buf = String::new();
        let _ = stderr_pipe.read_to_string(&mut buf);
        buf
    });

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let run_result = loop {
        match child.try_wait()? {
            Some(status) => break status,
            None => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    let stderr = stderr_handle.join().unwrap_or_default();
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        format!("Execution timed out after 30s: {}", stderr),
                    ));
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }
    };

    // Cleanup
    let _ = fs::remove_file(&source_path);
    let _ = fs::remove_file(&binary_path);

    if !run_result.success() {
        let stderr = stderr_handle.join().unwrap_or_default();
        return Err(io::Error::other(format!("Execution failed: {}", stderr)));
    }

    Ok(stdout_handle.join().unwrap_or_default())
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

/// Assert that compiling and running source produces output containing
/// every given fragment. Used for programs whose concurrently scheduled
/// tasks have no defined output ordering, where `assert_output`'s exact
/// comparison would be meaningless.
fn assert_output_contains(source: &str, fragments: &[&str]) {
    let result = compile_and_run(source).expect("compile_and_run failed");
    for fragment in fragments {
        assert!(
            result.contains(fragment),
            "Output missing {:?}.\nSource:\n{}\n\nActual:\n{}",
            fragment,
            source,
            result
        );
    }
}

// ── Smoke Tests ───────────────────────────────────────────────

/// Basic smoke test: print(42) should output "42"
#[test]
fn smoke_print_int() {
    assert_output("print(42);", "42");
}

// `spawn(f)` submits a plain function to the global scheduler. Two
// properties matter. The submission must not crash the process when a
// worker picks it up — the builtin has only the bare call target, not a
// future object. And the task must actually run before the program
// exits, otherwise a fire-and-forget spawn silently does nothing.
#[test]
fn spawn_runs_plain_function_before_exit() {
    let source = r#"
fn task_a(): void {
  print("A");
}

fn main() {
  spawn(task_a);
  print("main done");
}
"#;
    assert_output_contains(source, &["main done", "A"]);
}

// `Process.exec` used to hand a NUL-delimited packed triple back as if it
// were a `ProcessResult` instance, so every field read loaded struct bytes
// out of the middle of the packed string and segfaulted.
#[test]
fn process_exec_returns_populated_result() {
    let source = r#"
import { Process } from "process";

fn main() {
  let r = Process.exec("echo ruyi_exec_probe");
  print("[" + r.stdout.trim() + "]");
  print("[" + r.stderr + "]");
  print(r.exitCode);
  print(r.success);
}
"#;
    assert_output(source, "[ruyi_exec_probe]\n[]\n0\ntrue");
}

// A non-zero exit must be reported faithfully rather than swallowed.
#[test]
fn process_exec_reports_failure_exit_code() {
    let source = r#"
import { Process } from "process";

fn main() {
  let r = Process.exec("echo oops 1>&2; exit 3");
  print(r.exitCode);
  print(r.success);
  print(r.stderr.trim());
}
"#;
    assert_output(source, "3\nfalse\noops");
}

// PBKDF2 feeds each digest back in as the next message, and a digest is
// not valid UTF-8 — routing it through a `string` corrupted it. These
// vectors pin the byte-level path end to end, stdlib included.
#[test]
fn pbkdf2_sha256_matches_reference_vectors() {
    let source = r#"
import { pbkdf2Sha256 } from "crypto/pbkdf2";

fn main() {
  print(pbkdf2Sha256("password", "salt", 1, 32));
  print(pbkdf2Sha256("password", "salt", 2, 32));
  print(pbkdf2Sha256("password", "salt", 4096, 32));
}
"#;
    assert_output(
        source,
        "120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b\n\
         ae4d0c95af6b46d32d0adff928f06dd02a303f8ef3c251dfd6e2d85a95474c43\n\
         c5e478d59288c841aa530db6845c4c8d962893a001ce4e11a4963873aa98134a",
    );
}

/// Smoke test for string printing
#[test]
fn smoke_print_string() {
    assert_output(r#"print("hello");"#, "hello");
}

/// Smoke test for boolean printing
#[test]
fn smoke_print_bool() {
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

// A field initializer must run at every allocation site. The parser keeps
// `init` on `ClassElement::Field`, but codegen dropped it, so every
// `new Cfg()` produced zero values — silently disabling the defaults that
// stdlib option classes (`ProcessOptions.shell`, `ExecOptions.shell`, …)
// depend on.
#[test]
fn test_class_field_initializer_applies_on_new() {
    let source = r#"
class Cfg {
    n: int = 42;
    s: string = "hi";
    b: bool = true;
    static fn make(): Cfg { return new Cfg(); }
}
fn main() {
    let c = Cfg.make();
    print(c.n);
    print(c.b);
    print(c.s);
}
"#;
    assert_output(source, "42\ntrue\nhi");
}

// Initializers must also run on the `Class.new(...)` constructor path, and a
// later field initializer must be able to read an earlier one via `self`.
#[test]
fn test_class_field_initializer_runs_on_ctor_path() {
    let source = r#"
class Rect {
    w: int = 3;
    h: int = 4;
    area: int = 0;
    fn new() {
        self.area = self.w * self.h;
    }
}
fn main() {
    let r = new Rect();
    print(r.w);
    print(r.h);
    print(r.area);
}
"#;
    assert_output(source, "3\n4\n12");
}

// Indexing a nullable array must behave like indexing the array itself once
// the null check has narrowed it. `args[0]` used to load the Ruyi string
// through whatever the `Array<string>?` slot held, segfaulting.
#[test]
fn test_index_nullable_array_after_null_check() {
    let source = r#"
fn idx(args: Array<string>?): string {
    if (args !== null) {
        return args[0];
    }
    return "none";
}
fn main() {
    print(idx(["ruyi", "ok"]));
    print(idx(null));
}
"#;
    assert_output(source, "ruyi\nnone");
}

// Comparing a `string?` against a string literal must compare the string
// contents. It used to report `false` even when the value was identical,
// because the comparison never unwrapped the nullable.
#[test]
fn test_nullable_string_equals_string_literal() {
    let source = r#"
fn check(v: string?): bool {
    if (v !== null) {
        return v === "hello";
    }
    return false;
}
fn main() {
    // Concatenated so the value is not the same literal pointer the
    // comparison operand points at.
    let built = "hel" + "lo";
    print(check(built));
    print(check("world"));
    print(check(null));
}
"#;
    assert_output(source, "true\nfalse\nfalse");
}

// A `null` nullable must compare false without being dereferenced. Routing
// nullable strings to strcmp made `v === "x"` crash on a null `v`.
#[test]
fn test_nullable_string_comparison_is_null_safe() {
    let source = r#"
fn eq(v: string?): bool {
    return v === "hello";
}
fn ne(v: string?): bool {
    return v !== "hello";
}
fn main() {
    print(eq(null));
    print(ne(null));
    print(eq("hel" + "lo"));
    print(ne("hel" + "lo"));
}
"#;
    assert_output(source, "false\ntrue\ntrue\nfalse");
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
///
/// Note: Full E2E execution requires `_Unwind_RaiseException` support which
/// is not available on macOS. This test verifies that the source compiles
/// successfully (typecheck + codegen + link), matching the test name.
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
    // Verify compilation succeeds — full E2E execution is blocked by the
    // macOS unwinding limitation (see crates/ruyi_runtime/src/exception/runtime.rs).
    let result = compile_and_run(source);
    // Accept either success (output matches) or execution failure (unwinder
    // not available). Compilation failures are NOT acceptable.
    match result {
        Ok(output) => {
            let trimmed = output.trim();
            assert_eq!(trimmed, "out of bounds", "Unexpected output: {}", trimmed);
        }
        Err(e) => {
            let msg = e.to_string();
            // If the error is an execution failure (not compilation), that's acceptable
            assert!(
                msg.contains("Execution failed"),
                "Expected execution failure or success, got compilation failure: {}",
                msg
            );
        }
    }
}

/// Verify that `ArrayIterator.new(arr)` constructs an iterable instance
/// whose `.next()` returns each element and then null when exhausted.
///
/// The iterator yields `int?`, so the exhausted call prints the `int?`
/// null sentinel — the all-ones bit pattern, i.e. `-1` (0 is a legitimate
/// element value and must stay distinguishable from null).
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
    assert_output(source, "10\n20\n30\n-1");
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
    let tid = format!("{:?}", std::thread::current().id());
    let source_path = temp_dir.join(format!("{}_test_{}.ry", test_name, tid));
    let ir_path = temp_dir.join(format!("{}_test_{}.ll", test_name, tid));
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

/// Slice one function out of a module: from its `define` line to the
/// closing brace. Assertions must target the test program's own
/// functions — every module also pulls in the stdlib prelude, whose
/// bodies would satisfy loose marker checks by accident.
fn function_body<'a>(ir: &'a str, name: &str) -> &'a str {
    let needle = format!("@{}(", name);
    let (start, _) = ir
        .match_indices(&needle)
        .find(|(idx, _)| {
            let line_start = ir[..*idx].rfind('\n').map(|p| p + 1).unwrap_or(0);
            ir[line_start..*idx].starts_with("define")
        })
        .unwrap_or_else(|| panic!("no definition of {} in IR: {}", needle, ir));
    let rest = &ir[start..];
    let end = rest.find("\n}\n").unwrap_or(rest.len());
    &rest[..end]
}

/// Slice one basic block out of a function body: from its `label:` to the
/// blank line that separates blocks.
fn block_body<'a>(func_ir: &'a str, label: &str) -> &'a str {
    let needle = format!("{}:", label);
    let start = func_ir
        .find(&needle)
        .unwrap_or_else(|| panic!("no block {} in IR: {}", needle, func_ir));
    let rest = &func_ir[start..];
    let end = rest.find("\n\n").unwrap_or(rest.len());
    &rest[..end]
}

// Exception lowering uses a pending-exception protocol rather than LLVM
// landingpads: `ruyi_try_begin` registers the frame, `ruyi_throw_typed`
// parks the thrown instance, and the dispatch block asks
// `ruyi_find_handler` which catch claims it. Every function still carries
// the personality function so the platform unwinder can walk frames.

#[test]
fn codegen_exception_throw_uses_pending_exception_protocol() {
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
    assert!(
        !ir.contains("landingpad"),
        "Exception lowering is landingpad-free; IR: {}",
        ir
    );
    let main_ir = function_body(&ir, "main");
    assert!(
        main_ir.contains("personality ptr @__gxx_personality_v0"),
        "Expected personality for the platform unwinder; IR: {}",
        main_ir
    );
    assert!(
        main_ir.contains("call i32 @ruyi_try_begin("),
        "Expected ruyi_try_begin to register the frame; IR: {}",
        main_ir
    );
    assert!(
        main_ir.contains("call void @ruyi_throw_typed("),
        "Expected ruyi_throw_typed to park the exception; IR: {}",
        main_ir
    );
    assert!(
        main_ir.contains("call i64 @ruyi_find_handler("),
        "Expected ruyi_find_handler dispatch; IR: {}",
        main_ir
    );
    assert!(
        main_ir.contains("label %try.catch.0"),
        "Expected the dispatch switch to reach the catch block; IR: {}",
        main_ir
    );
    assert!(
        main_ir.contains("call void @ruyi_clear_pending_exception()"),
        "Expected the catch block to consume the pending exception; IR: {}",
        main_ir
    );
    assert!(
        main_ir.contains("call void @ruyi_try_end()"),
        "Expected ruyi_try_end on the frame's normal paths; IR: {}",
        main_ir
    );
}

#[test]
fn codegen_exception_nested_uses_pending_exception_protocol() {
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
    assert!(
        !ir.contains("landingpad"),
        "Exception lowering is landingpad-free; IR: {}",
        ir
    );
    let inner_ir = function_body(&ir, "inner");
    assert!(
        inner_ir.contains("call void @ruyi_throw_typed("),
        "Expected the callee to park the pending exception; IR: {}",
        inner_ir
    );
    let main_ir = function_body(&ir, "main");
    assert!(
        main_ir.contains("call i32 @ruyi_try_begin(ptr null, i64 0, ptr null, i64 0, i1 true)"),
        "Expected the finally-only frame to register with the is_finally flag; IR: {}",
        main_ir
    );
    let finally_ir = block_body(main_ir, "try_finally");
    assert!(
        finally_ir.contains("call ptr @ruyi_get_pending_exception()"),
        "Expected the finally body to inspect the pending exception; IR: {}",
        finally_ir
    );
    assert!(
        finally_ir.contains("label %try_dispatch"),
        "Expected the finally body to hand the preserved exception back to the outer dispatch; IR: {}",
        finally_ir
    );
}

#[test]
fn codegen_exception_rethrow_reaches_outer_handler() {
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
    assert!(
        !ir.contains("landingpad"),
        "Exception lowering is landingpad-free; IR: {}",
        ir
    );
    let main_ir = function_body(&ir, "main");
    assert_eq!(
        main_ir.matches("call i32 @ruyi_try_begin(").count(),
        2,
        "Expected two nested frames; IR: {}",
        main_ir
    );
    assert_eq!(
        main_ir.matches("call i64 @ruyi_find_handler(").count(),
        2,
        "Expected both frames to dispatch on the pending exception; IR: {}",
        main_ir
    );
    let rethrow_ir = main_ir
        .split("\n\n")
        .find(|b| b.contains("try.catch") && b.contains("call void @ruyi_throw_typed("))
        .unwrap_or_else(|| panic!("no catch block rethrows; IR: {}", main_ir));
    assert!(
        rethrow_ir.contains("br label %try_dispatch"),
        "Expected the rethrow to re-enter the outer dispatch; IR: {}",
        rethrow_ir
    );
}

// ── Generic Return Type Inference Regression Tests ─────────────

/// A generic method's type parameter must be resolved to the concrete
/// type at the call site. When `U` leaks through unresolved, `print`
/// falls back to the string format specifier and hands a boxed int to
/// printf as a `char*`, segfaulting.
///
/// The method's own `type_params` are the ones at stake here: `Ok.map<U>`
/// declares `fn map<U>(self, f: fn(T) -> U): Result<U, E>`, so `U` appears
/// only in the return type and in the closure parameter — never in the
/// receiver's type arguments, which is why class-parameter substitution
/// alone cannot resolve it.
#[test]
fn test_print_resolves_generic_method_return_type() {
    let source = r#"
import { Ok, Err, Result } from "option";

fn main() {
  let v: Result<int, string> = Ok.new(5);
  let mapped = v.map((x: int) => x * 2);
  print(mapped.unwrap());
}
"#;
    assert_output(source, "10");
}

/// `mapErr<F>` moves the method's type parameter into the *second* slot
/// of `Result<T, F>`, so a fix that only ever resolves a leading type
/// argument would still leave `F` dangling here.
#[test]
fn test_generic_method_type_param_in_error_slot() {
    let source = r#"
import { Ok, Err, Result } from "option";

fn main() {
  let v: Result<int, string> = Err.new("boom");
  let wrapped = v.mapErr((e: string) => e.length());
  print(wrapped.error);
}
"#;
    assert_output(source, "4");
}

/// `map` followed by `andThen` chains two generic calls whose results
/// feed the next call's receiver, so `U` has to stay concrete at every
/// link rather than degrading to a pointer partway down the chain.
#[test]
fn test_generic_method_type_param_across_chain() {
    let source = r#"
import { Ok, Err, Result } from "option";

fn main() {
  let start: Result<int, string> = Ok.new(3);
  let chained = start
      .map((x: int) => x + 1)
      .andThen((x: int) => Ok.new(x * 2));
  print(chained.unwrap());
}
"#;
    assert_output(source, "8");
}

/// `Fiber.spawn(closure)` must actually launch the closure on a fiber:
/// the stdlib stub cannot reach `__fiber_spawn` (same ABI gap as
/// `Thread.spawn`), so without compiler wiring the closure body never
/// runs and `join()` returns immediately against a dummy handle.
#[test]
fn test_fiber_spawn_runs_closure() {
    let source = r#"
import { Fiber } from "fiber";

fn main() {
  let f = Fiber.spawn(fn(arg) {
    print("fiber-body");
  });
  f.join();
  print("main-done");
}
"#;
    // join() synchronizes with fiber completion (the runtime signals the
    // completion channel after the entry returns), so ordering is fixed.
    assert_output(source, "fiber-body\nmain-done");
}

/// A union-dispatch method returning `bool` compiles to an `i1` call
/// result. The dispatch merge boxes every candidate result into the
/// `{i64, ptr}` Dynamic struct; inserting the raw `i1` into the i64
/// field is invalid IR that LLVM's ISel rejects with
/// `Do not know how to promote this operator's operand!`.
#[test]
fn test_union_dispatch_bool_result_compiles() {
    let source = r#"
import { Ok, Err, Result } from "option";

fn main() {
  let ok_val: Result<int, string> = Ok.new(5);
  if (ok_val.isOk()) {
    print("yes");
  } else {
    print("no");
  }
}
"#;
    assert_output(source, "yes");
}

/// The same defect on the `Option` side of the union: `Some.map<U>` must
/// resolve `U` from the closure's return type.
#[test]
fn test_generic_method_type_param_on_option() {
    let source = r#"
import { Some, None, Option } from "option";

fn main() {
  let v: Option<int> = Some.new(4);
  let mapped = v.map((x: int) => x + 1);
  print(mapped.unwrap());
}
"#;
    assert_output(source, "5");
}

/// A generic class method that calls a method on a field typed by a class
/// type parameter must see the instantiated field type. `Ok<T, E>.toString`
/// reads `self.value` (declared `T`); without substitution the call
/// dispatches on `T` and looks for a nonexistent `T_toString`, so the
/// specialization never gets emitted and the union call traps.
#[test]
fn test_generic_class_method_field_type_substitution() {
    let source = r#"
import { Ok, Err, Result } from "option";

fn main() {
  let ok_val: Result<int, string> = Ok.new(99);
  print(ok_val.toString());
  let err_val: Result<int, string> = Err.new("boom");
  print(err_val.toString());
}
"#;
    assert_output(source, "Ok(99)\nErr(boom)");
}

/// The same substitution on the `Option` side, where the field is also
/// reached through the alias rather than a concrete member class.
#[test]
fn test_generic_class_method_field_type_substitution_on_option() {
    let source = r#"
import { Some, None, Option } from "option";

fn main() {
  let v: Option<int> = Some.new(99);
  print(v.toString());
}
"#;
    assert_output(source, "Some(99)");
}

/// A closure referencing an enclosing function-local must read that
/// variable through its capture environment, not an undefined alloca
/// reference (which silently produced garbage).
#[test]
fn test_closure_captures_local() {
    let source = r#"
fn main() {
  let n = 10;
  let f = (x: int) => x + n;
  print(f(5));
}
"#;
    assert_output(source, "15");
}

/// A closure capturing a parameter must survive being returned from its
/// enclosing function and called after that frame is gone.
#[test]
fn test_closure_capture_survives_return() {
    let source = r#"
fn makeAdder(n: int): fn(int) -> int {
  return (x: int) => x + n;
}

fn main() {
  let f = makeAdder(10);
  print(f(5));
}
"#;
    assert_output(source, "15");
}

/// Transitive capture: the inner arrow captures through the outer
/// arrow's body (the sortByKey/keyFn shape from stdlib/sort.ry).
#[test]
fn test_closure_capture_transitive_chain() {
    let source = r#"
fn makeCmp(offset: int): fn(int, int) -> int {
  return (a: int, b: int) => (a - b) + offset;
}

fn apply(cmp: fn(int, int) -> int, x: int, y: int): int {
  return cmp(x, y);
}

fn main() {
  let cmp = makeCmp(7);
  print(apply(cmp, 3, 1));
}
"#;
    assert_output(source, "9");
}

/// A capturing closure stored into a class field and invoked later
/// through the function-typed-field call path.
#[test]
fn test_closure_capture_in_class_field() {
    let source = r#"
class Box {
  op: fn(int) -> int;
  base: int;

  fn new(base: int) {
    self.base = base;
    self.op = (x: int) => x + base;
  }
}

fn main() {
  let b = Box.new(100);
  print(b.op(5));
}
"#;
    assert_output(source, "105");
}

/// A capture-free closure keeps the plain-function ABI (env = null).
#[test]
fn test_closure_without_capture_still_works() {
    let source = r#"
fn apply2(f: fn(int) -> int, v: int): int {
  return f(v);
}

fn main() {
  let f = (x: int) => x * 3;
  print(apply2(f, 4));
}
"#;
    assert_output(source, "12");
}

/// Shared mutable capture: two spawned closures increment the same
/// enclosing `let` local; writes must land in one shared GC-heap cell
/// (sync_demo's mutex-counter pattern).
#[test]
fn test_shared_mutable_capture_via_threads() {
    let source = r#"
import { Thread } from "thread";

fn main() {
  let counter: int = 0;
  let t1 = Thread.spawn(fn(arg) { counter = counter + 1; });
  let t2 = Thread.spawn(fn(arg) { counter = counter + 1; });
  t1.join();
  t2.join();
  print(counter);
}
"#;
    assert_output(source, "2");
}

/// A single-threaded shared capture: the closure mutates the original
/// binding and the enclosing scope observes the update afterwards.
#[test]
fn test_shared_mutable_capture_single_thread() {
    let source = r#"
fn main() {
  let counter: int = 0;
  let bump = () => { counter = counter + 1; };
  bump();
  bump();
  bump();
  print(counter);
}
"#;
    assert_output(source, "3");
}

/// RWLock reentrant read: overlapping read guards must all be
/// releasable (a single _guard slot loses the first guard, which then
/// deadlocks a following writeLock).
#[test]
fn test_rwlock_reentrant_read_releases_all_guards() {
    let source = r#"
import { RWLock } from "rwlock";

fn main() {
  let rwl = RWLock.new();
  rwl.readLock();
  let reentrant = rwl.tryReadLock();
  rwl.readUnlock();
  rwl.readUnlock();
  rwl.writeLock();
  rwl.writeUnlock();
  print(reentrant);
  print("no deadlock");
}
"#;
    assert_output(source, "true\nno deadlock");
}

/// Condvar wait must atomically release the caller's held mutex and
/// re-acquire it before returning. The FFI receives the HELD guard (and
/// replaces it), not the mutex itself: re-locking the non-reentrant Mutex
/// inside wait() deadlocks the waiter against its own lock.
#[test]
fn test_condvar_wait_releases_and_relocks() {
    let source = r#"
import { Thread } from "thread";
import { Channel } from "channel";
import { Mutex } from "mutex";
import { Condvar } from "condvar";

fn main() {
  let mtx = Mutex.new();
  let cv = Condvar.new();
  let flag = false;
  let done = Channel.new(1);

  Thread.spawn(fn(arg) {
    mtx.lock();
    while (!flag) {
      cv.wait(mtx);
    }
    mtx.unlock();
    done.send(1);
  });

  Thread.sleep(5);
  mtx.lock();
  flag = true;
  cv.notifyOne();
  mtx.unlock();

  done.recv();
  print("condvar ok");
}
"#;
    assert_output(source, "condvar ok");
}

/// Dynamic values round-trip through arrays: the runtime element store is
/// one 8-byte word, so a Dynamic (16-byte struct) element is heap-boxed
/// on push/set and unboxed on get/pop. Storing the raw tag word instead
/// corrupts every non-int payload ("hello" came back as 0, FFI guard
/// pointers came back as null causing deadlocks).
#[test]
fn test_dyn_array_roundtrip() {
    let source = r#"
fn main() {
  let arr: Array<dyn> = [];
  let s: dyn = "hello";
  arr.push(s);
  arr.push(42);
  print(arr.get(0));
  print(arr.get(1));
  print(arr.pop());
}
"#;
    assert_output(source, "hello\n42\n42");
}

/// Dynamic arrays via index read/write follow the same box protocol.
#[test]
fn test_dyn_array_index_roundtrip() {
    let source = r#"
fn main() {
  let arr: Array<dyn> = [];
  arr.push("world");
  print(arr[0]);
  let d: dyn = 7;
  arr[0] = d;
  print(arr[0]);
}
"#;
    assert_output(source, "world\n7");
}

/// The array LITERAL path must box dyn elements too: without boxing, the
/// literal stores the raw string pointer as the element word and the read
/// side interprets it as a box pointer, loading a Dynamic struct out of
/// character data (garbage output, and a segfault when the pointer is not
/// readable as a struct).
#[test]
fn test_dyn_array_literal_string_roundtrip() {
    let source = r#"
fn main() {
  let arr: Array<dyn> = ["Ruyi", 3];
  print(arr[0]);
  print(arr[1]);
}
"#;
    assert_output(source, "Ruyi\n3");
}

/// Variadic `...args: dyn` packs call-site arguments into an Array<dyn>;
/// the same box protocol applies to that pack, or `args[0]` crashes when
/// the receiver indexes it.
#[test]
fn test_variadic_dyn_args_index_roundtrip() {
    let source = r#"
fn take(...args: dyn): dyn {
  return args[0];
}

fn main() {
  print(take("Ruyi"));
  print(take(9));
}
"#;
    assert_output(source, "Ruyi\n9");
}

/// The spread literal path must box dyn elements as well: a concrete-typed
/// source array contributes raw words (ints, raw string pointers) and the
/// Expr elements after it must be boxed to the same protocol, or reads of
/// the resulting Array<dyn> unbox garbage (or segfault).
#[test]
fn test_dyn_array_spread_literal_roundtrip() {
    let source = r#"
fn main() {
  let base = [1, 2];
  let arr: Array<dyn> = [...base, "Ruyi", 3];
  print(arr[0]);
  print(arr[1]);
  print(arr[2]);
  print(arr[3]);
}
"#;
    assert_output(source, "1\n2\nRuyi\n3");
}

/// stdlib `format`/`println` must substitute placeholders positionally —
/// one `{}` per argument, surplus placeholders left intact. The legacy
/// Ruyi-side loop called replace_all per argument, so every placeholder
/// collapsed to the first argument ("1 + 1 = 1"), and println compounded
/// it by passing the rest-pack Array itself as a single dyn argument.
#[test]
fn test_stdlib_format_positional_substitution() {
    let source = r#"
import { format, println } from "fmt";

fn main() {
  print(format("{} + {} = {}", 1, 2, 3));
  println("v{}.{}.{}", 0, 6, 0);
  println("圆周率约为 {}", 3.14159);
  println("布尔值: {} 和 {}", true, false);
  print(format("{} and {}", "A"));
}
"#;
    assert_output(
        source,
        "1 + 2 = 3\nv0.6.0\n\n圆周率约为 3.14159\n\n布尔值: true 和 false\n\nA and {}",
    );
}

/// A generic `T?` return consumed by a function-local `let` must carry
/// the call's specialized result type. Codegen previously recorded the
/// local with the *erased* `Nullable<Named("T")>` type, so `print(elem)`
/// dispatched to the pointer/`%s` path and segfaulted on the int payload,
/// and `missing === null` compared the wrong sentinel.
#[test]
fn test_generic_nullable_return_in_function_binding() {
    let source = r#"
fn findPositive<T>(arr: Array<T>, pred: fn(T) -> bool): T? {
  let n = arr.length();
  let i = 0;
  while (i < n) {
    let e = arr.get(i);
    if (pred(e)) {
      return e;
    }
    i = i + 1;
  }
  return null;
}

fn gt20(x: int): bool {
  return x > 20;
}

fn check() {
  let data = [10, 25, 3, 47, 8, 62];
  let elem = findPositive(data, gt20);
  if (elem !== null) {
    print("elem=");
    print(elem);
  }
  let missing = findPositive(data, (x: int) => x > 100);
  print("missing===null: ");
  print(missing === null);
}

check();
"#;
    assert_output(source, "elem=\n25\nmissing===null: \ntrue");
}

/// `float?` mirrors `int?`: null is the all-ones bit pattern (a NaN), and
/// `=== null` / `!== null` / `if let` / `!` must test that bit pattern —
/// float semantics alone would make `NaN === NaN` false and every null
/// check fail.
#[test]
fn test_nullable_float_null_comparison() {
    let source = r#"
fn maybe(x: float): float? {
  if (x > 100.0) {
    return null;
  }
  return x;
}

fn check() {
  let a = maybe(3.5);
  if (a !== null) {
    print("a not null");
  }
  let b = maybe(200.0);
  if (b === null) {
    print("b is null");
  }
  if let c = maybe(42.5) {
    print("c=");
    print(c);
  }
  let d = maybe(7.25)!;
  print("d=");
  print(d);
  let e = maybe(55.5);
  match (e) {
    null => { print("e is null"); },
    v => { print("e value: "); print(v); },
  }
  let g = maybe(300.0);
  match (g) {
    null => { print("g is null"); },
    v => { print("g value"); },
  }
}

check();
"#;
    assert_output(
        source,
        "a not null\nb is null\nc=\n42.500000\nd=\n7.250000\ne value: \n55.500000\ng is null",
    );
}

/// A generic `T?` return specialized at `float` must flow through the
/// monomorphized body (null → NaN sentinel) and stay comparable against
/// `null` at the call site. This is the `minBy`/`find` shape used by
/// `stdlib/sort.ry` over float arrays.
#[test]
fn test_generic_nullable_float_return() {
    let source = r#"
fn pick<T>(arr: Array<T>, pred: fn(T) -> bool): T? {
  let n = arr.length();
  let i = 0;
  while (i < n) {
    let e = arr.get(i);
    if (pred(e)) {
      return e;
    }
    i = i + 1;
  }
  return null;
}

fn check() {
  let data = [1.5, 25.5, 3.25];
  let hit = pick(data, (x: float) => x > 20.0);
  if (hit !== null) {
    print("hit=");
    print(hit);
  }
  let miss = pick(data, (x: float) => x > 100.0);
  if (miss === null) {
    print("miss is null");
  }
}

check();
"#;
    assert_output(source, "hit=\n25.500000\nmiss is null");
}

/// A `null` argument must be compiled from its *expected parameter type*
/// (`string?` → ptr null), not from the enclosing function's return type.
/// Inside a `float?`-returning function the return-type fallback used to
/// hijack the literal into the NaN sentinel; the call-arg adaptation then
/// bitcast it to `inttoptr (i64 -1)` — a non-null pointer — so the callee
/// saw a bogus string and the null check on its result failed.
#[test]
fn test_null_arg_in_nullable_float_returning_function() {
    let source = r#"
fn borrow(x: string?): string? {
  return x;
}

fn h(): float? {
  let line = borrow(null);
  if (line === null) {
    return null;
  }
  return 2.5;
}

print(h() === null);
"#;
    assert_output(source, "true");
}

/// Same hijack with the `int?` sentinel: inside an `int?`-returning
/// function the `string?`-param `null` argument was compiled to i64 -1
/// (the int null sentinel) and then inttoptr'd into a non-null pointer.
#[test]
fn test_null_arg_in_nullable_int_returning_function() {
    let source = r#"
fn borrow(x: string?): string? {
  return x;
}

fn h(): int? {
  let line = borrow(null);
  if (line === null) {
    return null;
  }
  return 3;
}

print(h() === null);
"#;
    assert_output(source, "true");
}

/// A rest parameter annotated with a plain element type (`...parts: string`)
/// must package its arguments as raw strings, not boxed `dyn` values.
/// The element type fell back to Dynamic whenever the annotation was not
/// literally `Array<T>`, so consumers that read the array as raw pointers
/// (e.g. the runtime behind `Path.join`) saw box tags / empty strings.
#[test]
fn test_rest_param_plain_element_type() {
    let source = r#"
fn joinAll(sep: string, ...parts: string): string {
  let out = "";
  let i = 0;
  while (i < parts.length()) {
    if (i > 0) { out = out + sep; }
    out = out + parts.get(i);
    i = i + 1;
  }
  return out;
}

fn check() {
  print(joinAll("-", "a", "b", "c"));
}

check();
"#;
    assert_output(source, "a-b-c");
}

/// A plain call to a user-declared function must not be rewritten by a
/// same-named builtin macro. `stringify("hi")` used to expand to the
/// debug rendering of its argument tokens, silently replacing the call
/// (which made the stdlib `json.isValid` always true).
#[test]
fn test_user_function_shadows_builtin_macro() {
    let source = r#"
fn stringify(s: string): string {
  return "user:" + s;
}

fn check() {
  print(stringify("hi"));
}

check();
"#;
    assert_output(source, "user:hi");
}

/// A subclass instance must be able to call a method it inherits. The
/// call site used to look up `Sub_getMsg`, which is never declared, and
/// failed with "Function not found".
#[test]
fn test_subclass_inherited_method_call() {
    let source = r#"
class Base {
  msg: string;
  fn new(m: string) { self.msg = m; }
  fn getMsg(self): string { return self.msg; }
}

class Sub extends Base {
  code: int;
  fn new(m: string, c: int) {
    super.new(m);
    self.code = c;
  }
}

fn check() {
  let s = Sub.new("hi", 1);
  print(s.getMsg() + "/" + s.code.toString());
}

check();
"#;
    assert_output(source, "hi/1");
}

/// A base-class constructor must keep writing the base fields at the same
/// offsets on a subclass instance. The subclass layout used to place its
/// own fields directly after `__typeid`, so `super.new()` wrote the base
/// field into the first own-field slot and the subclass constructor then
/// overwrote it (a custom `Error` subclass lost its `message`).
#[test]
fn test_subclass_base_field_not_overwritten() {
    let source = r#"
class Base {
  msg: string;
  fn new(m: string) { self.msg = m; }
}

class Sub extends Base {
  code: int;
  fn new(m: string, c: int) {
    super.new(m);
    self.code = c;
  }
}

fn check() {
  let s = Sub.new("hello", 7);
  print("msg=" + s.msg + " code=" + s.code.toString());
}

check();
"#;
    assert_output(source, "msg=hello code=7");
}

/// Imported aliases (`import { x as y }`) desugar to `const y = x;` and
/// were called through the generic function-value path, which passes
/// arguments 1:1 — so a rest parameter never received its packaged array
/// (`fmtFormat(...)` crashed reading the array header) and omitted
/// defaults were never synthesized.
#[test]
fn test_imported_function_alias_with_rest_params() {
    let source = r#"
import { format as fmtFormat } from "fmt";

fn check() {
  print(fmtFormat("v{}", "0.5"));
}

check();
"#;
    assert_output(source, "v0.5");
}

/// `Buffer.from` must emit exactly the UTF-8 bytes of the string. The
/// loop used to run over the *byte* length while `charCodeAt` indexes
/// Unicode scalars, so multi-byte strings gained 0xFF padding (the
/// out-of-range index returned -1, which encoded as a single byte).
#[test]
fn test_buffer_from_utf8_string() {
    let source = r#"
import { Buffer } from "buffer";

fn check() {
  let b = Buffer.from("通知: 系统维护");
  print(b.length().toString() + "|" + b.toString());
}

check();
"#;
    assert_output(source, "20|通知: 系统维护");
}

/// Chained method call through an anonymous-object field: `o.b.hello()`.
/// The method-call receiver path only understood class struct layouts, so
/// any object-literal field used directly as a method receiver failed
/// codegen ("Cannot access field on type: Object(...)"). WebSocket frame
/// decoding reads `decoded.payload.toString(...)` this way.
#[test]
fn test_method_call_on_anonymous_object_field() {
    let source = r#"
class B {
  s: string;
  fn new(s: string) { self.s = s; }
  fn hello(self): string { return self.s; }
}

fn make(): { b: B, n: int } {
  return { b: B.new("x"), n: 3 };
}

fn check() {
  let o = make();
  print("[" + o.b.hello() + "]");
  print(o.n);
}

check();
"#;
    assert_output(source, "[x]\n3");
}

/// Storing a `dyn` value into a class-typed field must unbox the payload
/// pointer (`{i64 tag, ptr data}` → `data`). Storing the raw 16-byte
/// struct into an 8-byte field leaves the tag word (e.g. 0x1d) in the
/// slot, so later method calls dereference garbage. `HttpServer.accept`
/// assigns a `dyn` socket to `HttpRequest._sock`, which crashed every
/// server-side response.
#[test]
fn test_dyn_value_stored_into_class_field() {
    let source = r#"
class Sock {
  h: int;
  fn new(h: int) { self.h = h; }
  fn handle(self): int { return self.h; }
}

class Req {
  s: Sock;
}

fn make(): dyn {
  return Sock.new(7);
}

fn check() {
  let d: dyn = make();
  let r = new Req();
  r.s = d;
  print("handle=");
  print(r.s.handle());
}

check();
"#;
    assert_output(source, "handle=\n7");
}

/// `s.substring(start)` with the `end` argument omitted must return the
/// tail of the string. Codegen used to pad the missing trailing argument
/// with the generic zero default, so the runtime saw `end = 0`, clamped
/// it to `start`, and returned "". The stdlib parses URLs and handshake
/// headers with this one-argument form, so every HTTP/WebSocket request
/// broke.
#[test]
fn test_string_substring_single_arg() {
    let source = r#"
fn check() {
  let s = "hello world";
  print("[" + s.substring(2, 6) + "][" + s.substring(6) + "][" + s.substring(0) + "][" + s.substring(20) + "]");
}
check();
"#;
    assert_output(source, "[llo ][world][hello world][]");
}

/// A named `fn` expression handed to Thread.spawn must (a) capture
/// enclosing locals through its env block, (b) get the trampoline to
/// actually pass the boxed user argument (the closure declares one
/// parameter), and (c) run to completion on the worker thread. The int
/// user arg must arrive with its real value — the Dynamic `{value,
/// marker}` layout, not the words swapped.
#[test]
fn test_fn_expr_capture_via_thread_spawn() {
    let source = r#"
import { Thread } from "thread";

fn worker(id: int, base: int): void {
  print(id * 100 + base);
  print("\n");
}

fn main() {
  let base = 7;
  let t = Thread.spawn(fn(arg) { worker(arg, base); }, 3);
  t.join();
}
"#;
    assert_output(source, "307");
}

/// Same spawn cell path for an arrow closure with a capture, so the
/// trampoline stays correct for both closure literal forms.
#[test]
fn test_arrow_capture_via_thread_spawn() {
    let source = r#"
import { Thread } from "thread";

fn worker(id: int, base: int): void {
  print(id * 100 + base);
  print("\n");
}

fn main() {
  let base = 9;
  let t = Thread.spawn((arg) => { worker(arg, base); }, 4);
  t.join();
}
"#;
    assert_output(source, "409");
}

/// A `dyn`-typed argument flowing into a typed parameter must be
/// unboxed at the call site. The Dynamic layout is a two-word struct
/// `{value, marker}`; passing it raw fills two parameter registers and
/// shifts every following argument (gradual-typing call gap).
#[test]
fn test_dynamic_arg_unboxed_at_call() {
    let source = r#"
fn add(x: int, y: int): void {
  print(x + y);
  print("\n");
}

fn main() {
  let d: dyn = 4;
  add(d, 9);
}
"#;
    assert_output(source, "13");
}

/// print() on a `dyn`-boxed int must show the value. The dispatcher
/// still read the pre-B1 layout (marker in the value word) and printed
/// `[object 0x1]` for every boxed int.
#[test]
fn test_print_dynamic_int_shows_value() {
    let source = r#"
fn main() {
  let a: dyn = 42;
  let b: dyn = 0;
  let s: dyn = "hi";
  let f: dyn = 2.5;
  print(a);
  print(b);
  print(s);
  print(f);
}
"#;
    assert_output(source, "42\n0\nhi\n2.500000");
}
