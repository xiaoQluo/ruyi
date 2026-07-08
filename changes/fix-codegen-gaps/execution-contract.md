# Execution Contract: fix-codegen-gaps

**Change**: `fix-codegen-gaps`
**Mode**: hotfix
**State**: specifying → bridging (DP-2 ✅ approved, pending DP-3 approval)

## Intent Lock

补全 codegen 对 BigInt 模式匹配、宏声明、类型别名三类声明的 LLVM IR 生成支持,并新增 BigInt 字面量比较的 runtime 支持,使 3 个失败的 example 全部通过编译,且新增 BigInt 字面量 match 测试覆盖。

## Affected Scope

- `crates/ruyic/src/codegen/patterns.rs` — BigInt match 路由 + 字面量 codegen
- `crates/ruyic/src/codegen/decl.rs` — Macro / TypeAlias 声明跳过
- `crates/ruyi_runtime/src/builtins.rs` — 新增 `ruyi_bigint_eq` 函数
- `crates/ruyi_runtime/src/lib.rs` — re-export `ruyi_bigint_eq`
- `examples/bigint.ry` — 新增字面量 match 测试代码

## Task List (Batches)

### Batch 1: Macro & TypeAlias skip (低风险、可独立完成)

| ID | Action | Files | LOC | Verify |
|----|--------|-------|-----|--------|
| T1 | `codegen_declaration` 添加 `Declaration::Macro { .. } => Ok(None)` | `codegen/decl.rs` | ~3 | `examples/macros.ry` 编译通过 |
| T2 | `codegen_declaration` 添加 `Declaration::TypeAlias { .. } => Ok(None)` | `codegen/decl.rs` | ~3 | `examples/type_aliases.ry` 编译通过 |

### Batch 2: BigInt match 路由修复

| ID | Action | Files | LOC | Verify |
|----|--------|-------|-----|--------|
| T3 | `patterns.rs:36` 从 `Type::Int \| Type::BigInt` 移除 BigInt,落入 generic 路径 | `codegen/patterns.rs` | ~1 | `examples/bigint.ry` 通配符 match 用例编译通过 |

### Batch 3: 新增 `ruyi_bigint_eq` runtime 函数

| ID | Action | Files | LOC | Verify |
|----|--------|-------|-----|--------|
| T4a | `builtins.rs` 新增 `ruyi_bigint_eq(a: *mut i8, b: *mut i8) -> i8` (placeholder) | `ruyi_runtime/src/builtins.rs` | ~15 | unit test: 同/异值返回正确 |
| T4b | `lib.rs` re-export `ruyi_bigint_eq` | `ruyi_runtime/src/lib.rs` | ~1 | 无编译错误 |
| T4c | unit test 验证 `ruyi_bigint_eq` | `ruyi_runtime/src/builtins.rs` | ~10 | `cargo test -p ruyi_runtime --no-default-features --lib` 通过 |

### Batch 4: codegen 在 BigInt 字面量 match 时调用 `ruyi_bigint_eq`

| ID | Action | Files | LOC | Verify |
|----|--------|-------|-----|--------|
| T5 | `compile_generic_match` 处理 `Pattern::Literal(BigIntLiteral(_))`,生成 `ruyi_bigint_eq` 调用 | `codegen/patterns.rs` | ~25 | codegen 不再报错 |

### Batch 5: 新增 example + 整体验证

| ID | Action | Files | LOC | Verify |
|----|--------|-------|-----|--------|
| T6 | `examples/bigint.ry` 追加 `match_literal_demo` 函数 | `examples/bigint.ry` | ~15 | 编译并运行通过 |
| T7 | 整体验证:examples + build + test | — | — | 见下方验收标准 |

## Acceptance Criteria

```bash
# 1. 全 example 通过
bash examples/run_examples.sh
→ Total: 33 | Passed: 33 | Failed: 0

# 2. 零警告构建
cargo build --release
→ 零警告

# 3. runtime 测试通过
cargo test -p ruyi_runtime --no-default-features --lib
→ 全部通过

# 4. parser 测试无新增失败(允许 14 个 WIP 既存失败)
cargo test -p ruyic --test parser
→ 与基线一致,无新增失败
```

## Out of Scope (Scope Fence)

- bigint 四则运算 codegen 优化(已有兜底路径,不属于本变更)
- 宏运行时展开(展开器在 typechecker 前已处理)
- 类型别名语义验证(typechecker 已处理)
- BigInt 真实数值比较语义(`ruyi_bigint_eq` 现阶段用 placeholder,真实库集成时再升级)
- 14 个 WIP 既存 parser 测试失败(`Builtin` vs `Identifier` 不匹配,与本变更无关)

## Handoff Rules

- Batch 1 → Batch 2 → Batch 3 → Batch 4 → Batch 5 顺序执行(Batch 1 内部 T1/T2 可并行)
- 任一 Batch 失败:停下,回退到 `specifying` 重新评估
- 风险点:BigInt 字面量 codegen 涉及 LLVM API 调用,首次写可能耗时

## Approval Gate (DP-3)

需用户明确批准后才进入 `approved-for-build` 状态。批准后:

```bash
ssf state set changes/fix-codegen-gaps dp_3_result "approved: scope extends to ruyi_runtime; 5 batches, 7 tasks"
ssf state set changes/fix-codegen-gaps dp_3_timestamp $(date -u +%Y-%m-%dT%H:%M:%SZ)
```

## Ambiguity Flags (Resolved)

- ✅ `bigint_eq` 运行时函数**不存在**于 runtime 库,需要新增(已在 Batch 3 处理)
- ✅ `examples/bigint.ry` 需新增字面量 match 测试代码(已在 Batch 5 处理)
- ✅ 修复 `bigint` 路由已采用 `compile_generic_match` 路径(已写 D1)

---

**请确认 (DP-3)**:以上契约是否符合预期?批准后立即进入执行阶段 (`approved-for-build`)。
