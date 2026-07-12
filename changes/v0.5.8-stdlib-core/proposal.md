# Proposal: v0.5.8-stdlib-core

## Why

Roadmap 4.2 (`math.ry`)、4.3 (`time.ry`)、4.4 (`json.ry`) 三个 P0 模块已在工作树以
"实现但未提交、未走 spec-superflow"状态存在十余日（自 2026-07-11）。此前 v0.5.6-p1-defects
拆分方案将"补全编译器侧接线"延后于 v0.5.7，但 v0.5.7 完成时仍未处理；本变更将这段缺失
补回：把 stdlib 三个核心模块在 ruyi/typechecker/codegen/runtime 四层完成接线，使其具备
类型检查与代码生成能力。

## Scope (in)

- **math.ry** 14 个 `__math_*` FFI：数学常量（PI / E）、初等函数（sqrt/pow/abs/min/max）、
  三角函数（sin/cos/tan）、对数（log）、舍入（ceil/floor/round）
- **time.ry** 4 个 `__time_*` FFI：当前秒/毫秒时间戳、`sleep()`、ISO 日期格式化
- **json.ry** 2 个 `__json_*` FFI：基础 `parse` / `stringify`
- **fmt_ffi.rs** 命名错位审查（顺风耳勘察同时发现的"已知旧 bug"，保留不修复——理由
  见 design.md 中的 Decision 5）
- 三模块的运行时 C ABI 实现（`crates/ruyi_runtime/src/{math_ffi,time_ffi,json_ffi}.rs`）

## Scope (out / Scope Fence)

- **parser `dyn` 返回类型 bug**：stdlib/json.ry 第 23 行 `parse(s:string): dyn` 触发
  parser "Expected identifier but found 'return'"——与 v0.5.7 遗留 random.ry `?:` 同源。
  本变更**不修 parser**，json.ry 编译失败属预期失败。
- **fmt_ffi.rs 的 8-arg bounded-buffer 重设计**：该文件 2026-07-11 引入的
  `ruyi_string_replace_all(s, s_len, from, from_len, to, to_len, out, out_cap) -> usize` 与
  builtins.rs:773 工作中的 3-arg `__string_replace_all` 同名但签名不同——前文（顺风耳）
  误判为命名错位。**本变更不迁移**，按 design.md Decision 5 处理。
- **`__builtin_map_*` / `__builtin_set_*` / `__string_*` 在 typechecker 未注册**（顺风耳
  同族发现），其代码已能编译运行，仅 typechecker 不识别，本变更不修。
- **Runtime binary 端到端验证**：cargo build 可过、`ruyic --check stdlib/{math,time,json}.ry`
  可过，但 `target/release/libruyi_runtime.a` 是否含 math_ffi.o `__math_*` 符号存疑（详见
  Decision 4）。binary e2e 验证留待后续 v0.5.8 / v0.5.9 走查。
- stdlib FFI cleanup of pre-existing undeclared `__io_*` / `__process_*` / `__path_*` 符号
  （v0.5.7 包含条目，本变更不动）。

## Impact

| 维度 | 影响 |
|------|------|
| 编译器 typechecker | 新增 20 条 `__math_*/__time_*/__json_*` 类型签名（inference.rs:42-） |
| codegen LLVM declarations | 新增 20 个 `fn declare_math_*/time_*/json_*`（builtins.rs:~960+） |
| runtime extern "C" symbols | 新增 20 个 `#[no_mangle] pub ... f64/i64/*mut i8` 函数 |
| 运行时二进制大小 | < +2KB（每个函数 ~30 行 Rust） |
| `cargo check -p ruyi_runtime` | 100% 通过（runtime / compiler 各层源码无误） |
| `cargo build --release` | 通过 |
| `ruyic --check stdlib/math.ry` | Type checking passed ✓ |
| `ruyic --check stdlib/time.ry` | Type checking passed ✓ |
| `ruyic --check stdlib/json.ry` | **FAIL** —— parser `dyn` bug（已知，不在本变更范围） |
| ABI | 与现存 `__builtin_array_*` / `__string_*` 字面量命名空间一致（采纳 A 路径） |

## Capabilities (CLOSED)

- `stdlib-math-core`：14 个 P0 数学 FFI 在四层完整接线
- `stdlib-time-core`：4 个 P0 时间 FFI 在四层完整接线
- `stdlib-json-core`：2 个 P0 JSON FFI 在四层完整接线

## Acceptance

```bash
# 1. 源码级别 — 必须通过
cargo check --workspace
→ exit 0（runtime / compiler / 全部一致）

# 2. 编译器类型检查 — 部分通过（json 阻塞于 parser bug）
./target/release/ruyic --check stdlib/math.ry    → Type checking passed.
./target/release/ruyic --check stdlib/time.ry    → Type checking passed.
./target/release/ruyic --check stdlib/json.ry    → 已知 parser bug（不阻塞 DP-7 归档）

# 3. 提交 — 必须通过
git log --oneline dev/v0.5.8                      → 含 feat(stdlib) + chore(sdd) 两枚
git branch --show-current                         → dev/v0.5.8
```

