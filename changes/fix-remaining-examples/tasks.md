# Tasks

## Batch 1: Lexer `$` token 修复

- [ ] **T1.1** 修改 `crates/ruyic/src/lexer/scanner.rs:63` — 移除 `$` 从 ident 起始字符集,新增 `$` → `Token::Dollar` 分支  
      **未实施（已否决）**：`7a04081 feat(lexer): support $ in identifiers (align with spec §2.5)` 选择反向方案——保留 `$` 作为标识符字符。`Token::Dollar` 仍存在于 token 枚举与 macro_expand 消费侧，但 lexer 不再产出该 token。
- [ ] **T1.2** 修改 `crates/ruyic/src/lexer/scanner.rs:433` — 移除 `$` 从 `is_ident_part` 字符集  
      **未实施（已否决）**：同 T1.1，`is_ident_part`（现 scanner.rs:448）仍含 `$`，与 spec §2.5 一致。
- [x] **T1.3** 验证: `cargo test -p ruyic --test macro_expand` → 15/15 通过(包括 2 个修复的测试)
- [x] **T1.4** 验证: `examples/macros.ry` 编译运行成功

## Batch 2: Codegen 顶层 let 全局化

- [x] **T2.1** 修改 `crates/ruyic/src/codegen/generator.rs` — 添加 `globals: HashMap<String, GlobalValue>` 字段
- [x] **T2.2** 修改 `crates/ruyic/src/codegen/decl.rs` 或 `driver.rs` — 在 main 函数生成之前,扫描所有顶层 let/const,创建 LLVM global + initializer
- [x] **T2.3** 修改 `lookup_variable` — 优先查 globals,返回指向 global 的 pointer
- [x] **T2.4** 验证: `examples/type_aliases.ry` 编译运行成功(Exit 0)
- [x] **T2.5** 验证: 已修复的 examples 不退化(31/33 baseline 维持)

## Batch 3: 全局验证

- [x] **T3.1** `cargo build --release` → exit 0,零警告
- [x] **T3.2** `cargo test --workspace` → 全部通过
- [x] **T3.3** `bash examples/run_examples.sh` → **33/33 PASS**
- [x] **T3.4** `make lint` → 零警告

## Batch 4: 归档

- [x] **T4.1** 提交 4 个 commits:
  - `fix(lexer): emit Token::Dollar for standalone $`
  - `fix(codegen): allocate top-level let as LLVM global`
  - (可选)单元测试
  - `docs(changes): close fix-remaining-examples`
- [x] **T4.2** 更新 `.spec-superflow.yaml` 到 `closed`
- [x] **T4.3** 最终报告
---

## 收尾补记（2026-10-18）

- Batch 1 的 `$` 处理方案在实现期被反向决策取代（见 T1.1/T1.2 标注），两项保持未勾选以如实反映；宏能力当前走 `${` 插值 + `$` 合法标识符字符路径，`cargo test -p ruyic --test macro_expand` 2026-10-18 实测 15/15 通过（T1.3）。
- Batch 2 落地证据：`crates/ruyic/src/codegen/generator.rs:83`（`globals` 字段）、`:1254`/`:1570`（`compile_top_level_let_inits`）、`:462`、`:1613`（`globals` 查找）。`examples/types/type_aliases.ry` 与 `examples/basics/macros.ry` 2026-10-18 实测编译运行退出码 0（原清单中的 `examples/macros.ry`、`examples/type_aliases.ry` 路径已随 examples 分级重构迁移）。
- T4.2 原文要求转到 `closed`，本次按状态机实际词汇记为 `archived`。
- T3.3 的 33/33 为 v0.5.5 期口径；当前基线见 docs/roadmap-zh.md（run_examples 65/65、workspace 1280/0，v0.5.11）。
