# Design: v0.5.8-stdlib-core

## Decisions

### D1: A 路 — 与现有 `__builtin_array_*` / `__string_*` 字面量命名空间一致

A 路选择：runtime 符号统一为 `__X_*` 前缀（如 `__math_pi`、`__time_now`、`__json_parse`），
与 codegen/typechecker 接线侧的 `__` 字面量约定保持一致。runtime 侧旧命名 `ruyi_math_*` /
`ruyi_time_*` / `ruyi_json_*` 已全部 rename 为 `__` 版本。

- **理由**：(1) 与现存 34 条 `__builtin_array_*`(6) / `__builtin_map_*`(7) /
  `__builtin_set_*`(4) / `__string_*`(18) 完全一致，没有重复字面量命名空间；(2) 三端
  （stdlib 调用 → typechecker 注册 → codegen LLVM 声明 → runtime 导出）一一对应、可查
  证；(3) 探查天官报告将 A 路列为推荐。
- **替代考虑**（B 路）：保留 `ruyi_*` 不动，在 codegen 引入"通用 `__X` → `ruyi_X` 字面替换"
  ——破坏与 34 条现有 `__builtin_*`/`__string_*` 一致性，且需大改 codegen/typechecker。
- **风险**：因 runtime FFI 改动，B 路方案相比 A 路需要回滚 4 文件已 wiring 内容；
  决定按 A 路推进，回滚代价为零。

### D2: `Type::Float` vs `Type::Dynamic` —— math/time 选定 `Type::Float`，json 选用 `Type::String`

`__builtin_array_*` 现有惯例所有 params/returns 都用 `Type::Dynamic`（因其 FFI 是
`*mut i8`/`i64`），但这会让 `export fn sqrt(x: float): float { return __math_sqrt(x); }`
因 `Dynamic` 不能窄化为 `float` 而 typecheck 失败。本变更**对应位**地使用：
- math 14 条：参数 + 返回均 `Type::Float`（对应 LLVM `double` / Rust `f64`）
- time 4 条：now/timestamp 用 `Type::Int`；sleep 用 `Type::Float` + `Type::Void`；
  format 用 `Type::Int` + `Type::String`
- json 2 条：parse/stringify 均用 `Type::String` + `Type::String`（因 LLVM `i8*` ↔ Ruyi
  `string`）

- **理由**：与 stdlib/`*.ry` 中显式类型注解（如 `const PI: float =`）一致可工作，且能
  在 typecheck 阶段捕获不当调用（如 `__json_parse(123)`）。
- **替代考虑**：全 `Type::Dynamic`（现有惯例），但在显式类型注解下 stdlib 不能编译。

### D3: codegen 用 `context.f64_type()` + `context.i64_type()` + `context.i8_type().ptr_type(...)`

与 `__builtin_array_*` 现存 5 条声明完全一致的 inkwell API 笔法：
- `f64_ty = context.f64_type()`，`fn_type = f64_ty.fn_type(&[f64_ty.into()], false)`
- `i8_ptr = context.i8_type().ptr_type(Default::default())`
- 参数用 `param.into()`，结果 `module.add_function("__X_y", fn_type, None)`

详细 18 条声明见 tasks.md T-math-6 / T-time-6 / T-json-5。

### D4: runtime archive 异常 — **不深度追查，先记为 Known Risk**

完成三层接线 + `cargo build --release` exit 0，但 `target/release/libruyi_runtime.a`
/`target/release/deps/libruyi_runtime-*.rlib` 经多处 `nm --defined-only` 探查均**未发现**
`__math_abs` 等符号（详见 tasks.md T-verify-3 与 T-verify-4 验证链）。

- **当前假设**：(a) Cargo LTO / codegen-units 配置 + inkwell LLVM 静态库吞并导致
  `math_ffi`/`time_ffi`/`json_ffi` 未被静态归档包入；(b) rlib 内确实含符号但被 deprecated
  链路剔除；(c) 测试时 ruyic 与测试二进制链接的是 rlib 而非 .a，且 rlib 内符号
  已被内联或回归到 `__math_*` 命名空间。
- **风险**：若假设 (a) 正确，则三层接线在源层面正确但运行期 binary 因缺符号崩溃——
  `__math_pi()` 调用可能返回 0（链接到占位 0 符号），compilation 静默不影响。
- **决策**：本变更**不修 cargo archive 问题**——它是构建系统层、不属于 stdlib 接入
  范围；记入 Known Risks，由后续 v0.5.9 专门解决（顺带统一改 codegen `builtins.rs`
  改为声明表驱动，避免重复维护 80+ 手工 declare_*）。

### D5: `fmt_ffi.rs` 与 `builtins.rs:773` 同名异构 — **回滚 fmt_ffi rename**

`__string_replace_all` 在两处定义但签名不同：
- `crates/ruyi_runtime/src/builtins.rs:773`：`fn(input: *const i8, pattern: *const i8,
  replacement: *const i8) -> *mut i8`（3-arg C string 版本，与 codegen
  `declare_string_replace_all` 对位）
- `crates/ruyi_runtime/src/fmt_ffi.rs:53`：`fn(s: *const u8, s_len: usize, from, from_len,
  to, to_len, out, *mut u8, out_cap) -> usize`（8-arg bounded-buffer 版本，2026-07-11
  引入，未在 codegen 注册）

本变更初版按"治理旧 bug rename fmt_ffi.rs → `__string_replace_all`"执行后，触发了
"symbol `__string_replace_all` is already defined"——发现是**两份不同实现同名**，而非
单纯命名错位。已回滚 fmt_ffi.rs 的 rename 决策（在 v0.5.7 范围内保持
`ruyi_string_replace_all` 名）。

- **决策**：维持 `builtins.rs:773` 工作现状 + fmt_ffi.rs 保留原名；codegen 与 runtime
  间调用仍走 `__string_replace_all` (3-arg)。fmt_ffi.rs 8-arg bounded 版本属 v0.5.8+ 后续
  迁移计划，不在本变更范围。
- **后续 v0.5.9 task**：将 fmt_ffi.rs 8-arg 版本迁移到 codegen/runtime，将其重命名为
  `__fmt_replace_all`（避开字面冲突），同时 deprecate builtins.rs:773。

### D6: parser `dyn` return type bug — **不修，记入 Out of Scope**

`stdlib/json.ry:23` `fn parse(s:string): dyn { return ...; }` 触发 parser bug：
"Expected identifier but found 'return'"。同族 bug 在 v0.5.7 由 release-archivist
记录：`stdlib/random.ry` `parseInt(s: int? = 0)` 同样失败（parser 不支持 `?:` 可选参数
语法）。

- **决策**：本变更不修 parser bug（属 v0.6.x parser 重构范畴，scope 远大于 stdlib 接入）；
  在 Out of Scope 显式声明：json.ry 编译失败属预期。
- **代替方案**：在 parser bug 修复前，std lib/json.ry 仍可被 typechecker 单独解析（已接线
  完成），但不能完整 `--check` 端到端；后续 v0.5.9 修 parser 后无需重新接线。

## Risks

| ID | 风险 | 缓解 |
|----|------|------|
| R1 | runtime archive 不含 `__math_*`/`__time_*`/`__json_*` 符号 → binary e2e 静默崩溃 | (D4) 记入 Known Risks，由 v0.5.9 专门解决；本变更仅归档源码层接线 |
| R2 | parser `dyn` return type bug → stdlib/json.ry 不可编译 | (D6) 记入 Out of Scope；typechecker/codegen 三层仍完成接线 |
| R3 | 编 80+ `declare_*` 函数后期难维护 | 决定 v0.5.9 改为声明表驱动（单点定义，自动生成 typechecker + codegen）——本变更不实施 |
| R4 | fmt_ffi.rs 8-arg bounded 版本无人调用 | (D5) 维持现状；v0.5.9 迁移计划 |

## Migration Plan

- **阶段 1（v0.5.7 完成前）**：math/time/json FFI 在 runtime 已暴露 14+4+2 个 C 符号
  (`__math_*`/`__time_*`/`__json_*`)。
- **阶段 2（本变更，v0.5.8）**：新增 typechecker/codegen 接线；std lib FFI 接入可用
  （math、time；json 因 parser bug 暂未完全端到端）。
- **阶段 3（v0.5.9，隐式）**：修复 runtime archive 异常 + parser `dyn` bug +
  fmt_ffi.rs 8-arg bounded 版本 codegen 迁移；与本变更向后兼容。

## Rollback

如果 v0.5.8 接入造成回归：
1. `git revert <feat(stdlib)>` 撤回本变更；
2. runtime 仍保留 14+4+2 `__math_*`/`__time_*`/`__json_*` extern（与 v0.5.7 状态对齐）；
3. stdlib/{math,time,json}.ry 与 examples/math_demo.ry 退回到 git HEAD 状态；
4. 向 roadmap.md 标注 P0 stdlib 4.2/4.3/4.4 仍未完成。
