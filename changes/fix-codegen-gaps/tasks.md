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

- [ ] `codegen/decl.rs`: `codegen_declaration` 添加 `Declaration::Macro { .. } => Ok(None)`
- [ ] 验证: `examples/macros.ry` 编译通过

**预估**: ~3 行 | **依赖**: 无

## T2: TypeAlias 声明 codegen 跳过

- [ ] `codegen/decl.rs`: `codegen_declaration` 添加 `Declaration::TypeAlias { .. } => Ok(None)`
- [ ] 验证: `examples/type_aliases.ry` 编译通过

**预估**: ~3 行 | **依赖**: 无

## T3: BigInt match 路由修复

- [ ] `codegen/patterns.rs:36`: 从 `Type::Int | Type::BigInt =>` 中移除 `Type::BigInt`
- [ ] 验证: `examples/bigint.ry` 的通配符 match 用例通过编译
- [ ] TDD: 先写一个 `cargo build` 验证,确认 T1+T2+T3 后 `bigint.ry` 至少不再报 Int match error

**预估**: ~1 行 | **依赖**: 无

## T4: 新增 `ruyi_bigint_eq` runtime 函数

- [ ] `crates/ruyi_runtime/src/builtins.rs`: 新增 `pub extern "C" fn ruyi_bigint_eq(a: *mut i8, b: *mut i8) -> i8`
  - Placeholder 实现: 比较两个指针地址或返回 0/1(stub OK)
- [ ] `crates/ruyi_runtime/src/lib.rs`: 在 re-export 列表添加 `ruyi_bigint_eq`
- [ ] TDD: 在 `crates/ruyi_runtime/src/builtins.rs` 内追加 unit test,验证 `ruyi_bigint_eq(same_ptr, same_ptr) == 1` 和 `ruyi_bigint_eq(ptr_a, ptr_b) == 0`(或 placeholder 语义)
- [ ] 验证: `cargo test -p ruyi_runtime --no-default-features --lib` 全部通过

**预估**: ~25 行(含 test) | **依赖**: 无

## T5: codegen 在 BigInt 字面量 match 时调用 `ruyi_bigint_eq`

- [ ] `codegen/patterns.rs`: `compile_generic_match` 在处理 `Pattern::Literal(BigIntLiteral(n))` 时,生成对 `ruyi_bigint_eq(scrutinee, literal_ptr)` 的调用,根据返回值分支跳转
- [ ] 验证: `examples/bigint.ry` 的字面量 match 用例通过编译(本任务不验证 runtime 执行,仅验证 codegen 通过)

**预估**: ~25 行 | **依赖**: T3, T4

## T6: 新增 BigInt 字面量 match example 代码

- [ ] `examples/bigint.ry`: 在 `pattern_matching_demo()` 中追加 `match_literal_demo()` 调用,内部使用 `match (n: bigint) { 42n => ..., _ => ... }`
- [ ] 验证: `bash examples/run_examples.sh` → 33/33 通过

**预估**: ~15 行 | **依赖**: T3, T4, T5

## 整体验证

- [ ] `bash examples/run_examples.sh` → Total: 33, Passed: 33, Failed: 0
- [ ] `cargo build --release` → 零警告
- [ ] `cargo test -p ruyi_runtime --no-default-features --lib` → 全部通过
- [ ] `cargo test -p ruyic --test parser` → 无新增失败
