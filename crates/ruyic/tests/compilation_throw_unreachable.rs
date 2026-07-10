/**
 * Codegen tests for compile_throw unreachable instruction.
 *
 * Tests verify that after `call ruyi_throw`, the LLVM IR contains
 * an `unreachable` instruction, ensuring proper basic block termination.
 *
 * These tests are enabled (previously `#[ignore]`) and require LLVM 14.
 * Run with: cargo test -p ruyic --test compilation_throw_unreachable
 *
 * @author Ruyi Team
 * @date 2026-07-08
 */
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;

/// Get the path to the ruyic binary
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
    let source_path = temp_dir.join("ruyi_throw_test.ry");
    let llvm_path = temp_dir.join("ruyi_throw_test.ll");

    fs::write(&source_path, source)?;

    let compile_result = Command::new(&ruyic_path)
        .arg(&source_path)
        .arg("--emit-llvm")
        .arg("-o")
        .arg(&llvm_path)
        .output()?;

    if !compile_result.status.success() {
        let stderr = String::from_utf8_lossy(&compile_result.stderr);
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Compilation failed: {}", stderr),
        ));
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
    throw new Error("test");
}

fn main(): int {
    throwError();
    return 0;
}
"#;

    let ir = compile_to_llvm(source).expect("compilation should succeed");

    // Check that after call to ruyi_throw, there's an unreachable instruction
    // The pattern should be: call void @ruyi_throw(...) followed by unreachable
    assert!(
        ir.contains("call void @ruyi_throw") || ir.contains("call i32 @ruyi_throw"),
        "IR should contain call to ruyi_throw.\nIR:\n{}",
        ir
    );

    // Check for unreachable instruction after the throw call
    // We look for the pattern where ruyi_throw is followed by unreachable
    let lines: Vec<&str> = ir.lines().collect();
    let mut found_throw = false;
    let mut found_unreachable = false;

    for (i, line) in lines.iter().enumerate() {
        if line.contains("@ruyi_throw") && line.contains("call") {
            found_throw = true;
            // Check the next non-empty line for unreachable
            for j in (i + 1)..lines.len() {
                let next_line = lines[j].trim();
                if !next_line.is_empty() {
                    if next_line == "unreachable" {
                        found_unreachable = true;
                    }
                    break;
                }
            }
            break;
        }
    }

    assert!(found_throw, "Should find call to ruyi_throw in IR");
    assert!(
        found_unreachable,
        "After call to ruyi_throw, should find unreachable instruction.\nIR:\n{}",
        ir
    );
}

/// Test that throw with try context emits unreachable in a separate block
#[test]
// Verifies: REQ-LPAD-003/004
fn test_throw_with_try_emits_unreachable_block() {
    let source = r#"
fn throwError(): void {
    throw new Error("test");
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

    // Check that IR contains ruyi_throw call
    assert!(
        ir.contains("@ruyi_throw"),
        "IR should contain reference to ruyi_throw.\nIR:\n{}",
        ir
    );

    // Check that there's an unreachable instruction somewhere in the IR
    // (it should be in a block after the throw's branch)
    assert!(
        ir.contains("unreachable"),
        "IR should contain unreachable instruction.\nIR:\n{}",
        ir
    );
}
