# Spec 01: math-stdlib-core — Wire 14 math FFI across runtime/typechecker/codegen

## Overview

`stdlib/math.ry` 在 v0.5.7 末尾被引入工作树，但仅在 runtime 层 (14 个 `ruyi_math_*` C 符号)
完成实现，未在编译器 typechecker 与 codegen 注册。本 spec 将 14 个 FFI 入口接通：

| 入口 | stdlib 用途 | typechecker 类型 | codegen LLVM 签名 |
|------|-----------|----------------|-------------------|
| `__math_pi` | `export const PI: float` 常量 | `() -> Float` | `declare double @__math_pi()` |
| `__math_e` | `export const E: float` 常量 | `() -> Float` | `declare double @__math_e()` |
| `__math_sqrt` | `sqrt(x: float): float` | `(Float) -> Float` | `declare double @__math_sqrt(double)` |
| `__math_pow` | `pow(x, y: float): float` | `(Float, Float) -> Float` | `declare double @__math_pow(double, double)` |
| `__math_abs` | `abs(x: float): float` | `(Float) -> Float` | `declare double @__math_abs(double)` |
| `__math_min` | `min(a, b: float): float` | `(Float, Float) -> Float` | `declare double @__math_min(double, double)` |
| `__math_max` | `max(a, b: float): float` | `(Float, Float) -> Float` | `declare double @__math_max(double, double)` |
| `__math_sin` | `sin(x: float): float` | `(Float) -> Float` | `declare double @__math_sin(double)` |
| `__math_cos` | `cos(x: float): float` | `(Float) -> Float` | `declare double @__math_cos(double)` |
| `__math_tan` | `tan(x: float): float` | `(Float) -> Float` | `declare double @__math_tan(double)` |
| `__math_log` | `log(x: float): float` | `(Float) -> Float` | `declare double @__math_log(double)` |
| `__math_ceil` | `ceil(x: float): float` | `(Float) -> Float` | `declare double @__math_ceil(double)` |
| `__math_floor` | `floor(x: float): float` | `(Float) -> Float` | `declare double @__math_floor(double)` |
| `__math_round` | `round(x: float): float` | `(Float) -> Float` | `declare double @__math_round(double)` |

## Requirements

### REQ-MATH-1: typechecker 注册
**SHALL** 在 `crates/ruyic/src/typechecker/inference.rs::resolve_builtin_name` 中
注册上述 14 个 `__math_*` 字符串到 `Some(Type::Function { ... })`，**param 与 return
均使用 `Type::Float`**（Ruyi `float` ↔ LLVM `double` ↔ Rust `f64`），不沿用
`__builtin_array_*` 的 `Type::Dynamic` 惯例。
- **理由**：stdlib 中 `export const PI: float = __math_pi();` 显式 `float` 类型注解要求
  typechecker 返回 `Type::Float`，否则 gradual typing 报 "cannot assign Dynamic to float"。

### REQ-MATH-2: codegen LLVM 声明
**SHALL** 在 `crates/ruyic/src/codegen/builtins.rs` 末尾追加 14 个
`fn declare_math_*<'ctx>(context, module)` 函数，每个用 `context.f64_type()` +
`fn_type = f64.fn_type(&[..], false)` + `module.add_function("__math_*", fn_type, None)`
构造 LLVM `declare double @__math_*` 指令。
- **理由**：与 `__builtin_array_*` 现有 5 条 `fn declare_*` 实现笔法一致。

### REQ-MATH-3: declare_builtins 调用
**SHALL** 在 `declare_builtins()` 函数末尾追加 14 次 `declare_math_*(context, module)`
调用。
- **顺序**：继现有 18 条 `declare_string_*` 之后、其他新模块之前。

### REQ-MATH-4: runtime extern "C" 16-位
**SHALL** 把 `crates/ruyi_runtime/src/math_ffi.rs` 中 14 个
`pub extern "C" fn ruyi_math_*` 重命名为 `pub extern "C" fn __math_*`（含 8 个单测中的
内部调用）。
- **理由**：A 路径采纳（详见 `proposal.md` D1 与 `design.md` D1），与现有 34 条
  `__builtin_*`/`__string_*` 字面量命名空间一致。
- **替代 B 路**：保留 `ruyi_*` + codegen 通用字面替换——破坏现有 34 条契约；scope 大。

### REQ-MATH-5: builtins re-export
**SHALL** 把 `crates/ruyi_runtime/src/builtins.rs` 中
`pub use crate::math_ffi::{ruyi_math_*: 14 项}` 替换为
`pub use crate::math_ffi::{__math_*: 14 项}`。
- **理由**：同步 runtime 符号改动、保持 `pub use` 与 `pub extern "C"` 一致命名。

## Scenarios

### SCEN-MATH-1: stdlib/math.ry 全部 14 函数 typecheck 通过（end-to-end）
**WHEN** 调用 `./target/release/ruyic --check stdlib/math.ry`
**THEN** 输出 "Type checking passed." 且无 `DiagnosticKind::UnknownVariable` 报错。
- **当前验证**：`./target/release/ruyic --check stdlib/math.ry` → "Type checking passed." ✓

### SCEN-MATH-2: stdlib/math.ry 实编为 LLVM IR
**WHEN** 调用 `./target/release/ruyic --emit-llvm stdlib/math.ry`
**THEN** 输出 `declare double @__math_pi()` `declare double @__math_abs(double)`
等 14 条 LLVM `declare` 指令。
- **验证手段**：`grep '^declare double @__math_' emit.ll | wc -l` = 14。

### SCEN-MATH-3: runtime 17 个 FFI 单测通过（13 个 `__math_*` + 4 个既有）
**WHEN** 运行 `cargo test -p ruyi_runtime --no-default-features --lib math_ffi::`
**THEN** 8 个单测（`test_sqrt` / `test_pow` / `test_abs` / `test_min_max` /
`test_trig` / `test_log` / `test_ceil_floor_round` / `test_constants`）全部通过。
- **验证手段**：`test_result: ok. 8 passed; 0 failed;`。

### SCEN-MATH-4: e2e example `examples/math_demo.ry` 编译通过
**WHEN** 调用 `./target/release/ruyic examples/math_demo.ry -o math_demo`
**THEN** exit 0，二进制文件生成。
- **当前验证**：`./target/release/ruyic examples/math_demo.ry -o math_demo` → exit 0 ✓
- **限制**：runtime archive 异常（见 proposal Scope Fence + design Risks R1）可能
  导致 binary 运行时 panic，而非编译期失败——这是已知问题，记入 Risks R1。

## Out of Scope

- `__math_*` 函数的精度/舍入语义（runtime 实现保留 placeholder，`x.abs()` /
  `x.sqrt()` 等标准库）
- `__math_*` 与 trait 实现（如 `Add for f64`）——属后续 generic trait 集成
- math 性能基准测试
- 反三角函数 `asin` / `acos` / `atan`（stdlib/math.ry 暂未声明——本 spec 只对接
  stdlib/math.ry 已声明的 14 个；新增需求走后续 change）
