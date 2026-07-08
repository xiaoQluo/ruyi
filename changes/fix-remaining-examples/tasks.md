# Tasks

## Batch 1: Lexer `$` token 修复

- [ ] **T1.1** 修改 `crates/ruyic/src/lexer/scanner.rs:63` — 移除 `$` 从 ident 起始字符集,新增 `$` → `Token::Dollar` 分支
- [ ] **T1.2** 修改 `crates/ruyic/src/lexer/scanner.rs:433` — 移除 `$` 从 `is_ident_part` 字符集
- [ ] **T1.3** 验证: `cargo test -p ruyic --test macro_expand` → 15/15 通过(包括 2 个修复的测试)
- [ ] **T1.4** 验证: `examples/macros.ry` 编译运行成功

## Batch 2: Codegen 顶层 let 全局化

- [ ] **T2.1** 修改 `crates/ruyic/src/codegen/generator.rs` — 添加 `globals: HashMap<String, GlobalValue>` 字段
- [ ] **T2.2** 修改 `crates/ruyic/src/codegen/decl.rs` 或 `driver.rs` — 在 main 函数生成之前,扫描所有顶层 let/const,创建 LLVM global + initializer
- [ ] **T2.3** 修改 `lookup_variable` — 优先查 globals,返回指向 global 的 pointer
- [ ] **T2.4** 验证: `examples/type_aliases.ry` 编译运行成功(Exit 0)
- [ ] **T2.5** 验证: 已修复的 examples 不退化(31/33 baseline 维持)

## Batch 3: 全局验证

- [ ] **T3.1** `cargo build --release` → exit 0,零警告
- [ ] **T3.2** `cargo test --workspace` → 全部通过
- [ ] **T3.3** `bash examples/run_examples.sh` → **33/33 PASS**
- [ ] **T3.4** `make lint` → 零警告

## Batch 4: 归档

- [ ] **T4.1** 提交 4 个 commits:
  - `fix(lexer): emit Token::Dollar for standalone $`
  - `fix(codegen): allocate top-level let as LLVM global`
  - (可选)单元测试
  - `docs(changes): close fix-remaining-examples`
- [ ] **T4.2** 更新 `.spec-superflow.yaml` 到 `closed`
- [ ] **T4.3** 最终报告