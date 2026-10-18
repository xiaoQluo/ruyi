# Tasks: Fix Codegen Gaps

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `crates/ruyi_runtime/src/builtins.rs` | Modify | Add `ruyi_bigint_eq` runtime function |
| `crates/ruyi_runtime/src/lib.rs` | Modify | Re-export `ruyi_bigint_eq` |
| `crates/ruyic/src/codegen/patterns.rs` | Modify | Remove `BigInt` from `compile_int_match` route; add `Pattern::Literal(BigIntLiteral)` dispatch via `ruyi_bigint_eq` |
| `crates/ruyic/src/codegen/decl.rs` | Modify | Add `Macro` and `TypeAlias` skip arms in `codegen_declaration` |
| `examples/bigint.ry` | Modify | Add `match_literal_demo` function with literal-pattern match |

## Interfaces

- `ruyi_runtime::builtins::ruyi_bigint_eq(a: *mut i8, b: *mut i8) -> i8` — equality predicate for BigInt
  - Consumes: two BigInt opaque pointers
  - Produces: `i8` (0 = false, non-zero = true)
  - Placeholder implementation acceptable

---

## T1: Macro 声明 codegen 跳过

- [x] `codegen/decl.rs`: `codegen_declaration` 添加 `Declaration::Macro { .. } => Ok(None)`
- [x] 验证: `examples/macros.ry` 编译通过

**预估**: ~3 行 | **依赖**: 无

## T2: TypeAlias 声明 codegen 跳过

- [x] `codegen/decl.rs`: `codegen_declaration` 添加 `Declaration::TypeAlias { .. } => Ok(None)`
- [x] 验证: `examples/type_aliases.ry` 编译通过

**预估**: ~3 行 | **依赖**: 无

## T3: BigInt match 路由修复

- [x] `codegen/patterns.rs:36`: 从 `Type::Int | Type::BigInt =>` 中移除 `Type::BigInt`
- [x] 验证: `examples/bigint.ry` 的通配符 match 用例通过编译
- [x] TDD: 先写一个 `cargo build` 验证,确认 T1+T2+T3 后 `bigint.ry` 至少不再报 Int match error

**预估**: ~1 行 | **依赖**: 无

## T4: 新增 `ruyi_bigint_eq` runtime 函数

- [x] `crates/ruyi_runtime/src/builtins.rs`: 新增 `pub extern "C" fn ruyi_bigint_eq(a: *mut i8, b: *mut i8) -> i8`
  - Placeholder 实现: 比较两个指针地址或返回 0/1(stub OK)
- [x] `crates/ruyi_runtime/src/lib.rs`: 在 re-export 列表添加 `ruyi_bigint_eq`
- [x] TDD: 在 `crates/ruyi_runtime/src/builtins.rs` 内追加 unit test,验证 `ruyi_bigint_eq(same_ptr, same_ptr) == 1` 和 `ruyi_bigint_eq(ptr_a, ptr_b) == 0`(或 placeholder 语义)
- [x] 验证: `cargo test -p ruyi_runtime --no-default-features --lib` 全部通过

**预估**: ~25 行(含 test) | **依赖**: 无

## T5: codegen 在 BigInt 字面量 match 时调用 `ruyi_bigint_eq`

- [x] `codegen/patterns.rs`: `compile_generic_match` 在处理 `Pattern::Literal(BigIntLiteral(n))` 时,生成对 `ruyi_bigint_eq(scrutinee, literal_ptr)` 的调用,根据返回值分支跳转
- [x] 验证: `examples/bigint.ry` 的字面量 match 用例通过编译(本任务不验证 runtime 执行,仅验证 codegen 通过)

**预估**: ~25 行 | **依赖**: T3, T4

## T6: 新增 BigInt 字面量 match example 代码

- [x] `examples/bigint.ry`: 在 `pattern_matching_demo()` 中追加 `match_literal_demo()` 调用,内部使用 `match (n: bigint) { 42n => ..., _ => ... }`
- [x] 验证: `bash examples/run_examples.sh` → 33/33 通过

**预估**: ~15 行 | **依赖**: T3, T4, T5

## 整体验证

- [x] `bash examples/run_examples.sh` → Total: 33, Passed: 33, Failed: 0
- [x] `cargo build --release` → 零警告
- [x] `cargo test -p ruyi_runtime --no-default-features --lib` → 全部通过
- [x] `cargo test -p ruyic --test parser` → 无新增失败

---

## 收尾补记（2026-10-18）

清单此前长期停留未勾选，本次按当前代码逐项核实后补齐（非新增验证结论，实现证据在 DP-6/DP-7 已记录）：

- T1/T2：`crates/ruyic/src/codegen/decl.rs:86-87` 已有 `Declaration::Macro` / `Declaration::TypeAlias` 跳过分支（实际签名为 `Ok(())`，与清单里写的 `Ok(None)` 不同，意图一致）。
- T3：`crates/ruyic/src/codegen/patterns.rs:36` 未采用「从 int 路由里移除 BigInt」，而是改为 `compile_bigint_match` 专用路由，覆盖更全。
- T4/T5：`crates/ruyi_runtime/src/builtins.rs:313` + `lib.rs:61` re-export + `patterns.rs` 经 `build_ruyi_bigint_eq` 调用，链路与单测均在。
- T6：示例已随 examples 分级重构迁移到 `examples/stdlib/bigint_demo.ry`（字面量 match 在 :129），2026-10-18 编译运行退出码 0。`examples/macros.ry` → `examples/basics/macros.ry`、`examples/type_aliases.ry` → `examples/types/type_aliases.ry`，两者实测退出码 0。
- 整体验证：当前基线为 `./examples/run_examples.sh` 65/65、`cargo test --workspace` 1280/0（见 docs/roadmap-zh.md 版本表 v0.5.11）。清单中的 33/33 为当时（v0.5.5）口径。
