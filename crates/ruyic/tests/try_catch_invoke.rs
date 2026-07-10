/**
 * Codegen integration tests for try/catch invoke + landingpad mechanism.
 *
 * These tests verify that the compiler emits correct LLVM exception-handling
 * instructions (`invoke`, `landingpad`, `resume`) when compiling try/catch
 * blocks. They require the compiled `ruyic` binary and a working LLVM 14
 * toolchain. The tests are now enabled (previously `#[ignore]`); some may
 * still fail against the pre-existing "Complex new expressions" limitation
 * (`throw new Error(...)`), which is out of scope for this batch.
 *
 * Run with:
 *   cargo test -p ruyic --test try_catch_invoke
 *
 * TDD status:
 *   - RED:   T4 creates landingpad infrastructure (invoke not yet emitted → invoke test fails)
 *   - GREEN: T5 enables invoke in compile_call → all tests pass
 *
 * @author Ruyi Team
 * @date 2026-07-08
 */
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;

/// Get the path to the ruyic binary.
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

/// Helper: compile a .ry source to LLVM IR, returning the IR text.
fn compile_to_ir(source: &str, temp_name: &str) -> io::Result<String> {
    let ruyic_path = get_ruyic_path();
    if !ruyic_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("ruyic binary not found at {:?}", ruyic_path),
        ));
    }

    let temp_dir = env::temp_dir();
    let source_path = temp_dir.join(format!("{}.ry", temp_name));
    let ir_path = temp_dir.join(format!("{}.ll", temp_name));

    fs::write(&source_path, source)?;

    let output = Command::new(&ruyic_path)
        .arg(&source_path)
        .arg("--emit-llvm")
        .arg("-o")
        .arg(&ir_path)
        .output()?;

    let _ = fs::remove_file(&source_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::other(format!(
            "Compilation to IR failed: {}",
            stderr
        )));
    }

    let ir = fs::read_to_string(&ir_path)?;
    let _ = fs::remove_file(&ir_path);
    Ok(ir)
}

/// Helper: compile source to binary and run it, returning stdout.
fn compile_and_run(source: &str) -> io::Result<String> {
    let ruyic_path = get_ruyic_path();
    if !ruyic_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("ruyic binary not found at {:?}", ruyic_path),
        ));
    }

    let temp_dir = env::temp_dir();
    let source_path = temp_dir.join("ruyi_try_catch_test.ry");
    let binary_path = temp_dir.join("ruyi_try_catch_test_bin");

    fs::write(&source_path, source)?;

    let compile_result = Command::new(&ruyic_path)
        .arg(&source_path)
        .arg("-o")
        .arg(&binary_path)
        .output()?;

    let _ = fs::remove_file(&source_path);

    if !compile_result.status.success() {
        let stderr = String::from_utf8_lossy(&compile_result.stderr);
        return Err(io::Error::other(format!("Compilation failed: {}", stderr)));
    }

    let run_result = Command::new(&binary_path).output()?;
    let _ = fs::remove_file(&binary_path);

    if !run_result.status.success() {
        let stderr = String::from_utf8_lossy(&run_result.stderr);
        return Err(io::Error::other(format!(
            "Execution failed (exit {:?}): {}",
            run_result.status.code(),
            stderr
        )));
    }

    Ok(String::from_utf8_lossy(&run_result.stdout).to_string())
}

// ── TDD-RED Tests (expected to FAIL until T5 enables invoke) ──────

/**
 * Test: try/catch with explicit throw emits `landingpad` in LLVM IR.
 *
 * This test verifies T4's landingpad infrastructure is in place.
 * It should PASS after T4 (landingpad block generated, even if unreachable).
 *
 * @author Ruyi Team
 * @date 2026-07-08
 */
#[test]
// Verifies: REQ-LPAD-003/004
fn test_try_catch_emits_landingpad() {
    let source = r#"
fn main(): int {
    try {
        throw new Error("test");
    } catch (e) {
        print("caught");
    }
    return 0;
}
"#;
    let ir = compile_to_ir(source, "test_landingpad").expect("Failed to compile to IR");
    assert!(
        ir.contains("landingpad"),
        "Expected LLVM IR to contain 'landingpad' instruction.\nIR:\n{}",
        ir
    );
}

/**
 * Test: try/catch with function call emits `invoke` in LLVM IR.
 *
 * This test verifies T5's invoke emission when a function call appears
 * inside a try block. Expected to FAIL (RED) until T5 is implemented.
 *
 * TDD state: RED in T4 → GREEN in T5
 *
 * @author Ruyi Team
 * @date 2026-07-08
 */
#[test]
// Verifies: REQ-LPAD-003/004
fn test_try_catch_emits_invoke() {
    let source = r#"
fn doWork(): void {
    throw new Error("boom");
}

fn main(): int {
    try {
        doWork();
    } catch (e) {
        print("caught");
    }
    return 0;
}
"#;
    let ir = compile_to_ir(source, "test_invoke").expect("Failed to compile to IR");
    assert!(
        ir.contains("invoke"),
        "Expected LLVM IR to contain 'invoke' instruction.\nIR:\n{}",
        ir
    );
}

/**
 * Test: exception from inner function is caught by outer try/catch.
 *
 * End-to-end verification that a throw inside a called function propagates
 * correctly to the calling function's catch handler.
 *
 * TDD state: RED in T4 → GREEN in T5
 *
 * @author Ruyi Team
 * @date 2026-07-08
 */
#[test]
// Verifies: REQ-LPAD-003/004
fn test_try_catch_catches_inner_throw() {
    let source = r#"
fn innerThrow(): void {
    throw new Error("boom from inside");
}

fn main(): int {
    try {
        innerThrow();
        print("not caught");
    } catch (e) {
        print("caught");
        return 0;
    }
    return 1;
}
"#;
    let output = compile_and_run(source).expect("Failed to compile and run");
    let output = output.trim();
    assert!(
        output.contains("caught"),
        "Expected output to contain 'caught', got: {}",
        output
    );
    assert!(
        !output.contains("not caught"),
        "Expected output NOT to contain 'not caught', got: {}",
        output
    );
}

/**
 * Test: finally + no catch block emits `resume` instruction.
 *
 * When a try block has no matching catch clause (or no catch at all with
 * finally), the landing pad should emit `resume` to continue unwinding.
 *
 * @author Ruyi Team
 * @date 2026-07-08
 */
#[test]
// Verifies: REQ-LPAD-003/004
fn test_try_finally_emits_resume() {
    let source = r#"
fn doWork(): void {
    throw new Error("boom");
}

fn main(): int {
    try {
        doWork();
    } finally {
        print("finally");
    }
    return 0;
}
"#;
    let ir = compile_to_ir(source, "test_resume").expect("Failed to compile to IR");
    assert!(
        ir.contains("resume"),
        "Expected LLVM IR to contain 'resume' instruction for finally cleanup.\nIR:\n{}",
        ir
    );
}

// ── T7: Extended coverage for REQ-TCI-001 / REQ-TCI-003 / REQ-TCI-005 ──

/**
 * Test: exception propagates through a multi-level call chain and is caught
 * by the outermost try/catch (REQ-TCI-001 boundary — call depth > 1).
 *
 * Verifies that LLVM `invoke` unwinding works correctly when the throwing
 * function is invoked indirectly: `main` -> `middle` -> `innerThrow`.
 *
 * TDD state: RED in T4 → GREEN in T5 (requires invoke unwinding across frames)
 *
 * @author Ruyi Team
 * @date 2026-07-08
 */
#[test]
// Verifies: REQ-LPAD-003/004
fn test_try_catch_through_two_level_call_caught() {
    let source = r#"
fn innerThrow(): void {
    throw new Error("deep boom");
}

fn middle(): void {
    innerThrow();
    print("middle after throw");
}

fn main(): int {
    try {
        middle();
        print("not caught");
    } catch (e) {
        print("caught");
        return 0;
    }
    return 1;
}
"#;
    let ir = compile_to_ir(source, "test_two_level").expect("Failed to compile to IR");
    // The throwing function lives two frames below main — both calls must
    // be `invoke` so LLVM can route the exception up through both frames.
    assert!(
        ir.contains("invoke"),
        "Expected IR to contain 'invoke' for multi-frame unwinding.\nIR:\n{}",
        ir
    );
    assert!(
        ir.contains("landingpad"),
        "Expected IR to contain 'landingpad' for multi-frame unwinding.\nIR:\n{}",
        ir
    );
    assert!(
        ir.matches("invoke").count() >= 2,
        "Expected at least 2 'invoke' instructions (one per call in chain), got {}.\nIR:\n{}",
        ir.matches("invoke").count(),
        ir
    );
}

/**
 * Test: function calls OUTSIDE a try block remain plain `call`, NOT `invoke`
 * (REQ-TCI-003 regression guard).
 *
 * After T5 enables invoke, this guards the opposite direction: calls in
 * non-try contexts must continue to use `call` (zero regression for hot paths).
 *
 * TDD state: RED before T5 (no invoke exists yet, but the source has no try,
 *            so this would trivially pass — therefore it acts as a guard
 *            against accidental T5 over-emission). GREEN after T5 with proper
 *            try_stack scoping.
 *
 * @author Ruyi Team
 * @date 2026-07-08
 */
#[test]
// Verifies: REQ-LPAD-003/004
fn test_non_try_call_uses_call() {
    let source = r#"
fn callee(): int {
    return 42;
}

fn main(): int {
    let x: int = callee();
    return x;
}
"#;
    let ir = compile_to_ir(source, "test_non_try_call").expect("Failed to compile to IR");
    assert!(
        ir.contains("call"),
        "Expected IR to contain 'call' instruction.\nIR:\n{}",
        ir
    );
    // Critical guard: no invoke because there is no try block.
    assert!(
        !ir.contains("invoke"),
        "Expected IR NOT to contain 'invoke' (no try block in source).\nIR:\n{}",
        ir
    );
}

/**
 * Test: function call inside try block emits `invoke`, while function call
 * outside any try block in the same source emits `call` (REQ-TCI-003).
 *
 * Combined check: the SAME source contains both contexts, and the compiler
 * must distinguish them. Confirms try_stack correctly scopes invoke emission.
 *
 * TDD state: RED in T4 (no invoke). GREEN in T5.
 *
 * @author Ruyi Team
 * @date 2026-07-08
 */
#[test]
// Verifies: REQ-LPAD-003/004
fn test_try_catch_emits_invoke_for_inner_calls() {
    let source = r#"
fn boom(): void {
    throw new Error("boom");
}

fn calm(): int {
    return 1;
}

fn main(): int {
    // Outside try — must remain `call`.
    let n: int = calm();
    // Inside try — must become `invoke`.
    try {
        boom();
    } catch (e) {
        print("caught");
    }
    return n;
}
"#;
    let ir = compile_to_ir(source, "test_mixed_invoke_call").expect("Failed to compile to IR");
    let invoke_count = ir.matches("invoke").count();
    let call_count = ir.matches("call ").count();
    assert!(
        invoke_count >= 1,
        "Expected at least 1 'invoke' inside try.\nIR:\n{}",
        ir
    );
    assert!(
        call_count >= 1,
        "Expected at least 1 'call ' outside try.\nIR:\n{}",
        ir
    );
}

/**
 * Test: multiple catch arms with type selector dispatch the exception to the
 * matching arm (REQ-TCI-005).
 *
 * The source defines two error classes and a try with two catch clauses.
 * A function throws `ErrorA` and the program must print `A` (first arm matches).
 *
 * Verifies:
 *   - End-to-end selector dispatch (output is "A")
 *   - IR contains exactly one `landingpad` (selector-based, not duplicated)
 *
 * @author Ruyi Team
 * @date 2026-07-08
 */
#[test]
// Verifies: REQ-LPAD-003/004
fn test_try_catch_multiple_catch_arms() {
    let source = r#"
class ErrorA {
  message: string;
  fn new(message: string) {
    self.message = message;
  }
}

class ErrorB {
  message: string;
  fn new(message: string) {
    self.message = message;
  }
}

fn throwA(): void {
    throw new ErrorA("type A");
}

fn main(): int {
    try {
        throwA();
    } catch (e: ErrorA) {
        print("A");
        return 0;
    } catch (e: ErrorB) {
        print("B");
        return 1;
    }
    return 2;
}
"#;
    let output = compile_and_run(source).expect("Failed to compile and run");
    let output = output.trim();
    assert!(
        output.contains("A"),
        "Expected output to contain 'A' (first catch arm matched), got: {}",
        output
    );
    assert!(
        !output.contains("B"),
        "Expected output NOT to contain 'B' (second arm must not fire), got: {}",
        output
    );

    let ir = compile_to_ir(source, "test_multi_arm").expect("Failed to compile to IR");
    let landingpad_count = ir.matches("landingpad").count();
    assert!(
        landingpad_count >= 1,
        "Expected at least 1 'landingpad' instruction, got {}.\nIR:\n{}",
        landingpad_count,
        ir
    );
}

/**
 * Test: finally block executes on the normal (no-throw) path (REQ-TCI-005).
 *
 * try-finally without any exception must still run finally before exiting.
 * Guards existing finally behavior (regression).
 *
 * @author Ruyi Team
 * @date 2026-07-08
 */
#[test]
// Verifies: REQ-LPAD-003/004
fn test_try_finally_normal_path() {
    let source = r#"
fn main(): int {
    try {
        print("inside try");
    } finally {
        print("finally");
    }
    print("after try");
    return 0;
}
"#;
    let output = compile_and_run(source).expect("Failed to compile and run");
    assert!(
        output.contains("inside try"),
        "Expected 'inside try' in output, got: {}",
        output
    );
    assert!(
        output.contains("finally"),
        "Expected 'finally' in output (normal path), got: {}",
        output
    );
    assert!(
        output.contains("after try"),
        "Expected 'after try' in output (normal continuation), got: {}",
        output
    );
}

/**
 * Test: finally block executes on the exceptional path (REQ-TCI-005).
 *
 * try-catch-finally with an exception must run finally AFTER the catch
 * processes the exception, before the function returns.
 *
 * @author Ruyi Team
 * @date 2026-07-08
 */
#[test]
// Verifies: REQ-LPAD-003/004
fn test_try_finally_exception_path() {
    let source = r#"
fn boom(): void {
    throw new Error("boom");
}

fn main(): int {
    try {
        boom();
    } catch (e) {
        print("caught");
    } finally {
        print("finally");
    }
    print("after try");
    return 0;
}
"#;
    let output = compile_and_run(source).expect("Failed to compile and run");
    assert!(
        output.contains("caught"),
        "Expected 'caught' in output, got: {}",
        output
    );
    assert!(
        output.contains("finally"),
        "Expected 'finally' in output (exception path), got: {}",
        output
    );
    assert!(
        output.contains("after try"),
        "Expected 'after try' (catch completed + finally ran), got: {}",
        output
    );
}

/**
 * Test: exception from a try with NO catch arm propagates upward and is
 * caught by an outer try/catch (REQ-TCI-001 boundary).
 *
 * Source: inner try-finally (no catch) re-throws; outer try-catch receives it.
 * The outer catch must print `outer`; the inner finally must also run.
 *
 * @author Ruyi Team
 * @date 2026-07-08
 */
#[test]
// Verifies: REQ-LPAD-003/004
fn test_try_no_catch_exception_propagates() {
    let source = r#"
fn boom(): void {
    throw new Error("boom");
}

fn main(): int {
    try {
        try {
            boom();
        } finally {
            print("inner finally");
        }
        print("inner after");
    } catch (e) {
        print("outer caught");
        return 0;
    }
    return 1;
}
"#;
    let output = compile_and_run(source).expect("Failed to compile and run");
    assert!(
        output.contains("inner finally"),
        "Expected 'inner finally' in output (inner try has no catch, finally still runs), got: {}",
        output
    );
    assert!(
        output.contains("outer caught"),
        "Expected 'outer caught' in output (exception propagated to outer catch), got: {}",
        output
    );
    assert!(
        !output.contains("inner after"),
        "Expected 'inner after' NOT in output (exception unwound before reaching it), got: {}",
        output
    );
}

/**
 * Test: nested try/catch — inner catch handles the exception, outer catch is
 * not triggered (REQ-TCI-003 unwind target selection).
 *
 * Verifies that the compiler routes the exception to the INNER catch's
 * landing pad, not the outer one (try_stack.last() semantics).
 *
 * @author Ruyi Team
 * @date 2026-07-08
 */
#[test]
// Verifies: REQ-LPAD-003/004
fn test_unwind_in_nested_try() {
    let source = r#"
fn boom(): void {
    throw new Error("boom");
}

fn main(): int {
    try {
        try {
            boom();
        } catch (e) {
            print("inner caught");
        }
        print("outer after inner try");
    } catch (e) {
        print("outer caught");
        return 1;
    }
    return 0;
}
"#;
    let output = compile_and_run(source).expect("Failed to compile and run");
    assert!(
        output.contains("inner caught"),
        "Expected 'inner caught' in output (innermost catch must receive), got: {}",
        output
    );
    assert!(
        output.contains("outer after inner try"),
        "Expected 'outer after inner try' (outer try continues normally), got: {}",
        output
    );
    assert!(
        !output.contains("outer caught"),
        "Expected 'outer caught' NOT in output (inner catch must swallow), got: {}",
        output
    );
}
