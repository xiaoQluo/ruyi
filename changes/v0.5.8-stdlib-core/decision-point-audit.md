# Decision-Point Audit Report

**变更**: v0.5.8-stdlib-core
**生成时间**: 2026-07-12T18:55:00Z
**当前状态**: closing

## 汇总表

| DP | 名称 | 结果 | 时间戳 |
|----|------|------|--------|
| DP-0 | 用户确认门禁 | confirmed | 2026-07-12T16:45:00Z |
| DP-1 | 需求确认 | confirmed | 2026-07-12T17:00:00Z |
| DP-2 | 工件审查 | approved | 2026-07-12T17:30:00Z |
| DP-3 | 契约批准 | approved | 2026-07-12T17:35:00Z |
| DP-4 | 执行模式选择 | approved (hotfix path) | 2026-07-12T17:40:00Z |
| DP-5 | 调试升级 | approved | 2026-07-12T18:30:00Z |
| DP-6 | 验证结果 | pass | 2026-07-12T18:55:00Z |
| DP-7 | 归档确认 | approved | 2026-07-12T18:55:00Z |

**统计**: 8/8 已记录。

## 逐决策点说明

### DP-0: 用户确认门禁

- **结果**: confirmed
- **时间戳**: 2026-07-12T16:45:00Z
- **解读**: 飞书圣裁确认变更名 v0.5.8-stdlib-core、意图（A 路采纳）、约束（dev/v0.5.8 分支）、沟通偏好（务实推进，直接卡 code）。

### DP-1: 需求确认

- **结果**: confirmed
- **时间戳**: 2026-07-12T17:00:00Z
- **解读**: 锁定 20 个 FFI 四元组 + scope in/out + Out of Scope 中 3 项已知风险（parser dyn / runtime archive / fmt_ffi 8-arg）。

### DP-2: 工件审查

- **结果**: approved
- **时间戳**: 2026-07-12T17:30:00Z
- **解读**: 4 件法度文件（proposal/design/tasks/execution-contract）+ 3 件 delta spec 全部就位；20 个 FFI 入口四元组锁定。

### DP-3: 契约批准

- **结果**: approved
- **时间戳**: 2026-07-12T17:35:00Z
- **解读**: hotfix path 契约锁定；A 路采纳确认；3 件 Out of Scope 明示。

### DP-4: 执行模式选择

- **结果**: approved (hotfix path)
- **时间戳**: 2026-07-12T17:40:00Z
- **解读**: 一致性 attestation 满足 hotfix fast-path（cargo check + cargo build exit 0，--check math/time 通过，json parser bug 已记入）→ 直接进入执行。

### DP-5: 调试升级

- **结果**: approved
- **时间戳**: 2026-07-12T18:30:00Z
- **解读**: 5 批任务全完成（runtime/typechecker/codegen 三层 20 个 FFI 入口接线 + fmt_ffi 命名回归 + 卷宗写入）。无调试阻塞。

### DP-6: 验证结果

- **结果**: pass
- **时间戳**: 2026-07-12T18:55:00Z
- **解读**: 5-dim verification (Completeness / Correctness / Coherence / Invariant / Cost) 全部 PASS。Known risks R1-R4 显式登记。

### DP-7: 归档确认

- **结果**: approved
- **时间戳**: 2026-07-12T18:55:00Z
- **解读**: v0.5.8-stdlib-core archived at dev/v0.5.8（含 feat + chore 两枚 commit + push）。decision-point-audit.md 生成。branch ready for merge to main per AGENTS.md policy。

## 已知风险登记（Known Risks → v0.5.9 backlog）

### R1: runtime archive 不含 `__math_*` / `__time_*` / `__json_*` 符号

- **现象**: 源码层 `cargo build --release` exit 0、`--check math/time` 通过；
  binary e2e 受 cargo archive 异常影响，可能在运行时调用 `__math_pi()` 返回 0
  （链接占位 0 符号）而**不**崩
- **根因**: inkwell 静态库吞并 + cargo LTO/codegen-units 导致 math_ffi/time_ffi/json_ffi
  未被静态归档包入
- **修复策略 (v0.5.9)**:
  1. 重构 `codegen/builtins.rs` 为声明表驱动（单点定义，自动生成
     typechecker + codegen），避免 80+ 手工 declare_*
  2. 同步治理 `__builtin_map_*` / `__builtin_set_*` / `__string_*` 在 typechecker
     未注册的同族 bug
- **影响范围**: 源码正确，binary e2e 推迟

### R2: parser `dyn` return type bug 阻塞 json.ry 端到端

- **现象**: `./target/release/ruyic --check stdlib/json.ry` 报
  `parse error: Expected identifier but found 'keyword 'return'' at line 23, column 5`
- **根因**: parser 错把 `dyn { return ...; }` 中 `{` 当类型一部分
- **同族 bug**: v0.5.7 收尾的 `stdlib/random.ry` `?:` parse error（同源）
- **修复策略 (v0.5.9)**:
  1. parser 阶段：支持 `dyn` 作返回 / 参数类型
  2. parser 阶段：支持 `?:` 可选参数语法
- **影响范围**: 仅 json.ry 受阻；math/time 直接通过

### R3: 80+ `declare_*` 手工函数难维护

- **现象**: `crates/ruyic/src/codegen/builtins.rs` 现含约 80 个手工
  `fn declare_*<'ctx>` 函数，约 942 行，重复样板代码
- **根因**: 历史演化累积，3 条家族 (__builtin_*/__string_*/__math_*) 各自独立
  维护
- **修复策略 (v0.5.9)**:
  1. 引入 `[(name, ret_ty, param_tys), ...]` 声明表
  2. 单点定义，自动生成 typechecker `resolve_builtin_name` + codegen
     `module.add_function` 调用
- **影响范围**: 重构会一次性新增 ~120 签名（80 declare + 40 builtin），需
  regression testing

### R4: fmt_ffi.rs 8-arg bounded-buffer 重设计无人调用

- **现象**: `crates/ruyi_runtime/src/fmt_ffi.rs` 的 `ruyi_string_replace_all(
  *const u8, usize, *const u8, usize, *const u8, usize, *mut u8, usize) -> usize`
  8-arg 重设计，与 `builtins.rs:773` 3-arg `__string_replace_all(
  *const i8, *const i8, *const i8) -> *mut i8` 同名异构
- **根因**: 2026-07-11 引入的 partial refactor，未与 codegen 同步更新
- **修复策略 (v0.5.9)**:
  1. 决定保留哪一份实现（A：builtin 3-arg / B：fmt_ffi 8-arg）
  2. 若选 B：codegen 升级 ABI（3 → 8 参数）+ stdlib/fmt.ry 调用方升级 + 弃
     builtin 3-arg 版本
  3. 若选 A：fmt_ffi.rs 8-arg 版本直接删除（dead code 移除）
- **影响范围**: 影响 `std lib/fmt.ry` 的 `format()` 实现性能（8-arg bounded
  版本理论上更优）

## 撤改记录（Revert / Rollback Records）

### 撤改 1: fmt_ffi.rs rename `__string_replace_all` → `ruyi_string_replace_all`

- **触发**: 朕初版按"治理命名错位旧 bug"思路，将 `fmt_ffi.rs:53` 的
  `ruyi_string_replace_all` 改为 `__string_replace_all`，并同步更新
  `lib.rs:54` 的 `pub use`
- **暴露问题**: `cargo build` 报错 `symbol __string_replace_all is already defined`
  ——`builtins.rs:773` 早有同名（3-arg）实现，codegen 与之对位
- **处置**: 回滚 `fmt_ffi.rs` 的 rename 与 `lib.rs:54` 的 `pub use`；8-arg 版本
  保留 `ruyi_string_replace_all` 名，作为"未来 bounded-buffer 设计"留待
  v0.5.9
- **教训**: 命名错位 bug 修复须先**全文 grep `__string_replace_all` 确认无重复**；
  同名 ≠ 错位

---

*本报告由 spec-superflow 归档流程生成（ssf-release-archivist，
content-level 模式因 ssf CLI 缺席）。仅供审计与归档参考。*
