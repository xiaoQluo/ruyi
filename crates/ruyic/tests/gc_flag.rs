/**
 * CLI 与 IR 集成测试，覆盖 Batch 1.1 GC 双模式。
 *
 * 测试覆盖：
 * 1. `--gc` flag 注册与合法/非法值
 * 2. 默认 stub 模式编译 `examples/hello.ry`，IR 含 `call @cc_alloc`
 * 3. `--gc=real` 模式编译示例，IR 含 `declare ... @ruyi_gc_alloc`
 *
 * @author luozegang
 * @date 2026-07-10
 */
use std::path::PathBuf;
use std::process::Command;

fn ruyic_bin() -> &'static str {
    env!("CARGO_BIN_EXE_ruyic")
}

/// 项目根目录（`tests/gc_flag.rs` 在 `crates/ruyic/` 下）。
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

/// `--help` 输出包含 `--gc` flag。
#[test]
fn ruyic_help_documents_gc_flag() {
    let out = Command::new(ruyic_bin())
        .arg("--help")
        .output()
        .expect("failed to spawn ruyic --help");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("--gc"),
        "ruyic --help should document --gc flag; got:\n{}",
        stdout
    );
}

/// `--gc=stub` 是合法值，不会被 clap 拒绝。
#[test]
fn ruyic_accepts_gc_stub_value() {
    let out = Command::new(ruyic_bin())
        .arg("--gc=stub")
        .output()
        .expect("failed to spawn ruyic --gc=stub");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "--gc=stub should be accepted by clap; got:\n{}",
        stderr
    );
}

/// `--gc=real` 是合法值，不会被 clap 拒绝。
#[test]
fn ruyic_accepts_gc_real_value() {
    let out = Command::new(ruyic_bin())
        .arg("--gc=real")
        .output()
        .expect("failed to spawn ruyic --gc=real");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "--gc=real should be accepted by clap; got:\n{}",
        stderr
    );
}

/// `--gc=bogus <input>` 退出码非零 + 友好错误信息。
#[test]
fn ruyic_rejects_gc_bogus_value() {
    let out = Command::new(ruyic_bin())
        .args(["--gc=bogus", "some_input.ry"])
        .output()
        .expect("failed to spawn ruyic --gc=bogus some_input.ry");
    assert!(
        !out.status.success(),
        "--gc=bogus should produce a non-zero exit; status: {:?}",
        out.status
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.to_lowercase().contains("gc")
            && (stderr.contains("bogus") || stderr.contains("expected")),
        "stderr should mention GC mode error; got:\n{}",
        stderr
    );
}

/// 默认 stub 模式：编译 `examples/hello.ry`，IR 含 `call @cc_alloc`。
#[test]
#[ignore = "requires LLVM 14 (run with --ignored)"]
fn hello_ir_default_mode_emits_cc_alloc() {
    let workspace = workspace_root();
    let mut cmd = Command::new(ruyic_bin());
    cmd.current_dir(&workspace);
    cmd.arg("examples/hello.ry");
    cmd.arg("--emit-llvm");
    let out = cmd
        .output()
        .expect("failed to spawn ruyic hello.ry --emit-llvm");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "ruyic hello.ry --emit-llvm should succeed; stderr:\n{}",
        stderr
    );

    // ruyic emits IR to examples/target/hello.ll
    let ll_path = workspace.join("examples/target/hello.ll");
    let ir = std::fs::read_to_string(&ll_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {}", ll_path.display(), e));

    assert!(
        ir.contains("@cc_alloc"),
        "default GC mode (stub) should reference @cc_alloc; IR:\n{}",
        pick_alloc_lines(&ir)
    );
    assert!(
        ir.contains("call i8* @cc_alloc("),
        "default GC mode (stub) should emit `call i8* @cc_alloc(...)`; IR:\n{}",
        pick_alloc_lines(&ir)
    );
}

/// `--gc=real` 模式：编译 `examples/fibonacci.ry`，IR 含
/// `declare ... @ruyi_gc_alloc` （即使本程序未必真分配）。
#[test]
#[ignore = "requires LLVM 14 (run with --ignored)"]
fn fibonacci_ir_real_mode_declares_ruyi_gc_alloc() {
    let workspace = workspace_root();
    let mut cmd = Command::new(ruyic_bin());
    cmd.current_dir(&workspace);
    cmd.arg("examples/fibonacci.ry");
    cmd.arg("--gc=real");
    cmd.arg("--emit-llvm");
    let out = cmd
        .output()
        .expect("failed to spawn ruyic fibonacci.ry --gc=real --emit-llvm");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "ruyic fibonacci.ry --gc=real --emit-llvm should succeed; stderr:\n{}",
        stderr
    );

    let ll_path = workspace.join("examples/target/fibonacci.ll");
    let ir = std::fs::read_to_string(&ll_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {}", ll_path.display(), e));

    assert!(
        ir.contains("declare i8* @ruyi_gc_alloc("),
        "--gc=real should declare @ruyi_gc_alloc in IR; alloc lines:\n{}",
        pick_alloc_lines(&ir)
    );
}

/// 从 IR 中抽取含 `alloc` / `@cc_alloc` 的行（避免喷大量 IR）。
fn pick_alloc_lines(ir: &str) -> String {
    ir.lines()
        .filter(|l| l.contains("alloc") || l.contains("cc_alloc"))
        .take(20)
        .collect::<Vec<_>>()
        .join("\n")
}
