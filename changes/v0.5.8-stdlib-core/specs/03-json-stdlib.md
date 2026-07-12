# Spec 03: json-stdlib-core — Wire 2 JSON FFI across runtime/typechecker/codegen

## Overview

`stdlib/json.ry` 引入工作树但未接通编译器。本 spec 将 2 个 JSON FFI 入口在 source
层面接通，与 `01-math-stdlib.md` / `02-time-stdlib.md` 同模式。但 JSON 的运行时
端到端**受 parser `dyn` 返回类型 bug 阻塞**（见 SCEN-JSON-3 与 Risks）。

| 入口 | stdlib 用途 | typechecker 类型 | codegen LLVM 签名 |
|------|-----------|----------------|-------------------|
| `__json_parse` | `parse(s: string): dyn` | `(String) -> String` | `declare i8* @__json_parse(i8*)` |
| `__json_stringify` | `stringify(v: dyn): string` | `(String) -> String` | `declare i8* @__json_stringify(i8*)` |

## Requirements

### REQ-JSON-1: typechecker 注册
**SHALL** 在 `crates/ruyic/src/typechecker/inference.rs::resolve_builtin_name` 中
注册 2 条：
- `__json_parse`：`(Type::String) -> Type::String`
- `__json_stringify`：`(Type::String) -> Type::String`
- **不**用 `Type::Dynamic`（因 stdlib 显式声明返回 string）

### REQ-JSON-2: codegen LLVM 声明
**SHALL** 在 `crates/ruyic/src/codegen/builtins.rs` 追加 2 个 `fn declare_json_*<'ctx>`
+ 2 次调用：
- `__json_parse` / `__json_stringify`：用
  `context.i8_type().ptr_type(Default::default())` +
  `fn_type = i8_ptr.fn_type(&[i8_ptr.into()], false)`

### REQ-JSON-3: runtime extern "C" 位
**SHALL** 把 `crates/ruyi_runtime/src/json_ffi.rs` 中 2 个
`pub unsafe extern "C" fn ruyi_json_*` 重命名为 `pub unsafe extern "C" fn __json_*`。
- `__json_parse(json_str: *const i8) -> *mut i8`：占位 JSON 解析器（逐字符状态机）
- `__json_stringify(value: *const i8) -> *mut i8`：占位 JSON 字符串化器
- 错误处理：null 入参返回 null；非法 UTF-8 返回 null；解析失败返回 null

### REQ-JSON-4: builtins re-export
**SHALL** 把 `crates/ruyi_runtime/src/builtins.rs` 中
`pub use crate::json_ffi::{ruyi_json_parse, ruyi_json_stringify}`
替换为 `{__json_parse, __json_stringify}`。

## Scenarios

### SCEN-JSON-1: typecheck 接入（源代码层）
**WHEN** 调用 `./target/release/ruyic --check stdlib/json.ry`
**THEN** 输出 "Type checking passed."（这是预期，但当前因 parser bug 而失败——见 SCEN-JSON-3）

### SCEN-JSON-2: runtime 5 个 FFI 单测通过
**WHEN** `cargo test -p ruyi_runtime --no-default-features --lib json_ffi::`
**THEN** 5 个单测（`test_parse_null` / `test_parse_boolean` / `test_parse_string`
/ `test_parse_number` / `test_stringify`）通过。
- **验证手段**：`test_result: ok. 5 passed; 0 failed;`
- **不受 parser bug 影响**（runtime FFI 单测是 Rust 直测，与 stdlib/json.ry 的
  `dyn` 返回类型无关）

### SCEN-JSON-3: json.ry `--check` 当前已知失败（parking）
**WHEN** 调用 `./target/release/ruyic --check stdlib/json.ry`
**THEN** 输出 `parse error: Expected identifier but found 'keyword 'return'' at line 23, column 5`
**AND** 这是**已知** parser bug，**不计入** v0.5.8 DP-7 阻断项
- **根因**：parser 处理 `fn f(): dyn { return ...; }` 时，将 `dyn {` 误识为
  `dyn` 类型 + `{` 块头，但在 `{ return` 处无法确定返回类型——与 v0.5.7 收尾
  `stdlib/random.ry` `?:` parse error 同源
- **修复范围**：parser 阶段（属 v0.5.9+ 独立 bug 修复），与本 spec 无关

### SCEN-JSON-4: e2e binary 调用（受 R1/R2 双重风险）
**WHEN** 编译一段调用 `parse("null")` 的 Ruyi 程序
**THEN** binary 生成且能运行，输出 `"null"` 字符串
- **限制**：受 R1（runtime archive）与 R2（parser bug）双重影响，本 spec 不强制 e2e
  验证

## Out of Scope

- parser `dyn` 返回类型与 `?:` 可选参数语法 bug（与 v0.5.7 收尾同源）
- 完整 JSON 规范（当前实现为 basic subset：null/boolean/number/string/array/object）
- JSON Path / streaming
- 反序列化到 typed 结构（如 class 实例）
- 与 `dyn` 类型系统的真正整合——当前 JSON 解析结果是 stringified Ruyi 值

## Migration Notes

**v0.5.9 后续**：parser bug 修复后（独立 change），无需修改本 spec 的任何要求——
source-layer 接线已就位，parser 一过 e2e 自动可用。
