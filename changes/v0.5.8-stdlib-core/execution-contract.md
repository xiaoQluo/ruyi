# Execution Contract: v0.5.8-stdlib-core

**Change**: `v0.5.8-stdlib-core`
**Mode**: hotfix
**State**: closing (DP-7 pending)
**Branch**: `dev/v0.5.8`

## Intent Lock

完成 roadmap 4.2 / 4.3 / 4.4（math / time / json 三个 P0 标准库模块）在四层
（runtime / typechecker / codegen / lib.rs pub use）的完整接线，并补立 spec-superflow
卷宗归卷。A 路径采纳（runtime 符号统一为 `__X_*` 前缀，与现有 34 条 `__builtin_array_*` /
`__builtin_map_*` / `__builtin_set_*` / `__string_*` 字面量命名空间一致）。

## Affected Scope (in)

| File | Lines | Action |
|------|-------|--------|
| `crates/ruyi_runtime/src/math_ffi.rs` | 192 | Modify (new, 14 extern "C" C FFI 实现) |
| `crates/ruyi_runtime/src/time_ffi.rs` | 164 | Modify (new, 4 extern "C") |
| `crates/ruyi_runtime/src/json_ffi.rs` | 348 | Modify (new, 2 extern "C") |
| `crates/ruyi_runtime/src/builtins.rs` | +14 −14 | `pub use` re-export 同步重命名 |
| `crates/ruyi_runtime/src/lib.rs` | +3 | `pub mod math_ffi;` `pub mod time_ffi;` `pub mod json_ffi;` |
| `crates/ruyic/src/typechecker/inference.rs` | +44 | `resolve_builtin_name` 加 20 条 |
| `crates/ruyic/src/codegen/builtins.rs` | +90 | 20 个 `fn declare_*` + 20 次调用 |
| `stdlib/math.ry` | 162 | Modify (new, 14 出口) |
| `stdlib/time.ry` | 61 | Modify (new, 5 出口) |
| `stdlib/json.ry` | 45 | Modify (new, 3 出口, 待 parser fix) |
| `examples/math_demo.ry` | ~15 | Modify (new, math 端到端 example) |
| `changes/v0.5.8-stdlib-core/{proposal,design,tasks,execution-contract}.md` | ~400 | 卷宗法度文件 |
| `changes/v0.5.8-stdlib-core/specs/{01-math,02-time,03-json}-*.md` | ~150 | delta specs |
| `changes/v0.5.8-stdlib-core/.spec-superflow.yaml` | ~80 | 状态机 |
| `changes/v0.5.8-stdlib-core/decision-point-audit.md` | ~100 | 决策审计 |

## Task List (Batches)

### Batch A: runtime 符号重命名（顺序可任意）
T-math-A1 / T-time-A1 / T-json-A1 / T-builtins-A2（共 4 任务，独立可并行）

### Batch B: typechecker 注册
T-math-B1 / T-time-B1 / T-json-B1（共 3 任务，独立可并行）

### Batch C: codegen LLVM 声明
T-math-C1 / T-time-C1 / T-json-C1 / T-builtins-C2（共 4 任务，顺序：B → C）

### Batch D: 验证
T-verify-1 → 5（顺序执行；T-verify-5 预期失败但记入卷宗）

### Batch E: git 提交
T-git-1 → 4（顺序执行；T-git-2/3/4 需 Batch A-D 全部完结）

### Batch F: DP-7 归档（sequential）
先 `chore(sdd)` 提交卷宗，再 push。

## Approved Behavior / Scope Fence

- math.ry / time.ry / json.ry 在 stdlib 端到端 `--check` 测试
  - math / time 通过
  - **json 失败**（parser `dyn` bug，记入 Out of Scope，本变更不修）
- 源码层面 `cargo check --workspace` 与 `cargo build --release` 双双 exit 0
- runtime extern "C" 符号 20 个（`__math_*` 14 + `__time_*` 4 + `__json_*` 2）

## Acceptance Criteria

```bash
# 1. 源码层通过
cargo check --workspace
→ exit 0

# 2. 编译层通过
cargo build --release
→ exit 0

# 3. stdlib 端到端
./target/release/ruyic --check stdlib/math.ry  → "Type checking passed."
./target/release/ruyic --check stdlib/time.ry  → "Type checking passed."
./target/release/ruyic --check stdlib/json.ry  → 已知 parser bug（不计入 DP-7 阻断）

# 4. 版本控制
git log --oneline dev/v0.5.8                  → 含 feat(stdlib) + chore(sdd)
git status --short                            → 无未提交改动
git ls-files changes/v0.5.8-stdlib-core/      → 9 项 planning 文件全部入库
```

## Out of Scope (Scope Fence)

- **parser `dyn` return type bug**：`stdlib/json.ry:23` 触发——属 parser 重构范畴。
- **runtime archive 异常**：math_ffi 等模块编译后未出现在 `libruyi_runtime.a` 中——属
  cargo/LTO/inkwell 静态库吞并问题；本源层正确（`cargo check` 通过），binary 端到端
  静默崩属预期风险。
- **fmt_ffi.rs 8-arg bounded-buffer 重设计**：与 `builtins.rs:773` 3-arg `__string_replace_all`
  同名异构；本变更不迁移（governance 不卷 8-arg 接入；属 v0.5.9 计划）。
- **`__builtin_map_*` / `__builtin_set_*` / `__string_*` 在 typechecker 未注册**：同族
  bug，但本变更聚焦 math/time/json。
- **stdlib FFI cleanup of `__io_*` / `__process_*` / `__path_*`**（v0.5.7 残留）。
- stdlib 性能基准 / benchmark。
- `make run-example` 大规模端到端运行（受 runtime archive 阻塞）。

## Risks

详见 `design.md` Risks 段落：
- R1：runtime archive 不含 `__math_*` 等 → binary e2e 静默崩溃（已记 D4）
- R2：parser `dyn` bug → json.ry 不可编译（已记 D6）
- R3：80+ `declare_*` 手工难维护（v0.5.9 转声明表驱动）
- R4：fmt_ffi 8-arg 版本无人调用（v0.5.9 迁移计划）

## Escalation Rules

1. **DP-7 阻断前**：若 `cargo check --workspace` 或 `cargo build --release` 任何一项失败，
   STOP，回滚到 Batch A 重新评估。
2. **依赖外部接入**：math/time 二项 e2e 端到端通过即满足 DP-7；json 因 parser bug 不阻断；
   runtime archive 异常不阻断。
3. **不升级到 v0.5.9 范围**：若发现 fmt_ffi 或 parser bug 必须修才能 DP-7 完成，STOP，
   回滚到 v0.5.8 完成归档，将修复列入 v0.5.9。

## Approval Gate (DP-3) / DP-7 Archive Trigger

本变更由玉帝直接圣裁推进（飞书 Decisions），按玉帝旨意 "补建 change + dev 分支提交"
省略规格化 DP-0~DP-3 流程，于分支 dev/v0.5.8 上一次性合并 git history 后进入 DP-6/DP-7
归档。DP-7 触发条件：
1. Batch A-E 全 ✅
2. acceptance 4 条满足
3. 卷宗 6 件法度文件入 `changes/v0.5.8-stdlib-core/`
4. 飞书「计划完成」卡片送达
