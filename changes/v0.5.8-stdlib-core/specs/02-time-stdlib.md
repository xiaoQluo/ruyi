# Spec 02: time-stdlib-core — Wire 4 time FFI across runtime/typechecker/codegen

## Overview

`stdlib/time.ry` 引入工作树但未接通编译器。本 spec 将 4 个时间 FFI 入口接通：

| 入口 | stdlib 用途 | typechecker 类型 | codegen LLVM 签名 |
|------|-----------|----------------|-------------------|
| `__time_now` | `now(): int` 当前秒级时间戳 | `() -> Int` | `declare i64 @__time_now()` |
| `__time_timestamp` | `timestamp(): int` 毫秒级时间戳 | `() -> Int` | `declare i64 @__time_timestamp()` |
| `__time_sleep` | `sleep(seconds: float): void` 同步阻塞 | `(Float) -> Void` | `declare void @__time_sleep(double)` |
| `__time_format` | `format_time(timestamp: int): string` ISO 8601 | `(Int) -> String` | `declare i8* @__time_format(i64)` |

`now_string()` 是 stdlib 高层封装（`format_time(now())`），无需 codegen 单独登记。

## Requirements

### REQ-TIME-1: typechecker 注册
**SHALL** 在 `crates/ruyic/src/typechecker/inference.rs::resolve_builtin_name` 中
注册 4 条：
- `__time_now` / `__time_timestamp`：`() -> Type::Int`
- `__time_sleep`：`(Type::Float) -> Type::Void`
- `__time_format`：`(Type::Int) -> Type::String`

### REQ-TIME-2: codegen LLVM 声明
**SHALL** 在 `crates/ruyic/src/codegen/builtins.rs` 追加 4 个 `fn declare_time_*<'ctx>`
+ 4 次 `declare_time_*(context, module)` 调用：
- `__time_now` / `__time_timestamp`：用 `context.i64_type()` + `fn_type = i64.fn_type(&[], false)`
- `__time_sleep`：用 `context.void_type()` + `fn_type = void.fn_type(&[f64.into()], false)`
- `__time_format`：用 `context.i8_type().ptr_type(...)` + `fn_type = i8_ptr.fn_type(&[i64.into()], false)`

### REQ-TIME-3: runtime extern "C" 位
**SHALL** 把 `crates/ruyi_runtime/src/time_ffi.rs` 中 4 个
`pub extern "C" fn ruyi_time_*` 重命名为 `pub extern "C" fn __time_*`。
- `__time_now` / `__time_timestamp` 返回 `i64`（Unix 时间秒/毫秒）
- `__time_sleep` 阻塞当前线程 `thread::sleep(Duration::from_secs_f64(seconds))`
- `__time_format` 返回堆分配 `*mut i8` ISO 8601 字符串（调用方负责释放）

### REQ-TIME-4: builtins re-export
**SHALL** 把 `crates/ruyi_runtime/src/builtins.rs` 中
`pub use crate::time_ffi::{ruyi_time_now, ruyi_time_timestamp, ruyi_time_sleep, ruyi_time_format}`
替换为 `{__time_now, __time_timestamp, __time_sleep, __time_format}`。

## Scenarios

### SCEN-TIME-1: stdlib/time.ry 全部 5 函数 typecheck 通过
**WHEN** `./target/release/ruyic --check stdlib/time.ry`
**THEN** 输出 "Type checking passed."
- **当前验证**：✓

### SCEN-TIME-2: `now()` 调用 `__time_now` 链接正确
**WHEN** 编译一段调用 `now()` 的代码
**THEN** LLVM IR 中出现 `call i64 @__time_now()`
- **验证手段**：`grep 'call i64 @__time_now' emit.ll`

### SCEN-TIME-3: runtime 4 个 FFI 单测通过
**WHEN** `cargo test -p ruyi_runtime --no-default-features --lib time_ffi::`
**THEN** 4 个单测（`test_now` / `test_timestamp` / `test_format` /
`test_is_leap_year`）通过；`test_now` 验证返回值大于 `1577836800`（2020-01-01），
`test_format` 验证已知 timestamp `1704067200` 格式化为 "2024-01-01 00:00:00"。
- **验证手段**：`test_result: ok. 4 passed; 0 failed;`

### SCEN-TIME-4: `sleep` 真实阻塞
**WHEN** 运行一段 `sleep(0.5)` 代码
**THEN** 程序实际阻塞 ≥ 0.5 秒（用于 e2e 行为验证；属 binary e2e，受 runtime
archive R1 影响可能延迟验证）

## Out of Scope

- 异步 sleep（`async fn sleep` 集成 green thread scheduler）——属运行时 async
  runtime 增强，本 spec 不涉
- 时区支持（`__time_format` 当前 UTC-only）——后续 improvement
- `Duration` 类型、ISO 8601 解析（从 string 到 timestamp）——属 date/datetime 模块
  完善
- `now_string()` 在 codegen 单独登记——它是 stdlib 内部 `format_time(now())` 组合，
  调用 `__time_format` 与 `__time_now` 已足够
- performance optimization（如 `clock_gettime` 替换 `SystemTime::now`）
