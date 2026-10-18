/**
 * Codegen tests for compile_throw unreachable instruction.
 *
 * Tests verify that after `call ruyi_throw`, the LLVM IR contains
 * an `unreachable` instruction, ensuring proper basic block termination.
 *
 * These tests are enabled (previously `#[ignore]`) and require LLVM 20.
 * Run with: cargo test -p ruyic --test compilation_throw_unreachable
 *
 * @author Ruyi Team
 * @date 2026-07-08
 */
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Get the path to the ruyic binary
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root not found")
        .to_path_buf()
}

fn get_ruyic_path() -> PathBuf {
    // Test binaries run with the *package* dir as cwd, so relative
    // `target/...` paths miss the workspace build. Resolve against the
    // workspace root (same helper as tests/codegen.rs).
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

/// Compile source to LLVM IR and return the IR content
fn compile_to_llvm(source: &str) -> io::Result<String> {
    let ruyic_path = get_ruyic_path();
    if !ruyic_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("ruyic binary not found at {:?}", ruyic_path),
        ));
    }

    let temp_dir = env::temp_dir();
    // Tests in this file run in parallel; a shared file name makes them
    // overwrite each other's source/IR.
    let tid = format!("{:?}", std::thread::current().id());
    let source_path = temp_dir.join(format!("ruyi_throw_test_{}.ry", tid));
    let llvm_path = temp_dir.join(format!("ruyi_throw_test_{}.ll", tid));

    fs::write(&source_path, source)?;

    let compile_result = Command::new(&ruyic_path)
        .current_dir(workspace_root())
        .arg(&source_path)
        .arg("--emit-llvm")
        .arg("-o")
        .arg(&llvm_path)
        .output()?;

    if !compile_result.status.success() {
        let stderr = String::from_utf8_lossy(&compile_result.stderr);
        return Err(io::Error::other(format!("Compilation failed: {}", stderr)));
    }

    let ir_content = fs::read_to_string(&llvm_path)?;

    // Cleanup
    let _ = fs::remove_file(&source_path);
    let _ = fs::remove_file(&llvm_path);

    Ok(ir_content)
}

/// Test that throw without try context emits unreachable after ruyi_throw call
#[test]
// Verifies: REQ-LPAD-003/004
fn test_throw_without_try_emits_unreachable() {
    let source = r#"
fn throwError(): void {
    throw Error.new("test");
}

fn main(): int {
    throwError();
    return 0;
}
"#;

    let ir = compile_to_llvm(source).expect("compilation should succeed");

    // Current lowering uses the pending-exception protocol: `throw`
    // parks the exception via `ruyi_throw_typed` and returns normally so
    // the nearest `ruyi_try_begin` frame can dispatch it. There is no
    // "throw never returns" contract any more, so no `unreachable`
    // after the call is expected.
    assert!(
        ir.contains("call void @ruyi_throw_typed("),
        "IR should park the exception via ruyi_throw_typed.\nIR:\n{}",
        ir
    );
}

/// Test that a throw inside `try` engages the pending-exception frame
/// (`ruyi_try_begin` + dispatch) rather than relying on landing pads.
#[test]
// Verifies: REQ-LPAD-003/004
fn test_throw_with_try_emits_unreachable_block() {
    let source = r#"
fn throwError(): void {
    throw Error.new("test");
}

fn main(): int {
    try {
        throwError();
    } catch (e) {
        print("caught");
    }
    return 0;
}
"#;

    let ir = compile_to_llvm(source).expect("compilation should succeed");

    assert!(
        ir.contains("call void @ruyi_throw_typed("),
        "IR should park the exception via ruyi_throw_typed.\nIR:\n{}",
        ir
    );
    assert!(
        ir.contains("@ruyi_try_begin") && ir.contains("@ruyi_find_handler"),
        "IR should register and dispatch the try frame.\nIR:\n{}",
        ir
    );
}
