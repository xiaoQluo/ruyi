# Tasks: v0.5.8-stdlib-core

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `crates/ruyi_runtime/src/math_ffi.rs` | Modify (new) | 14 个 `__math_*` C FFI 实现 |
| `crates/ruyi_runtime/src/time_ffi.rs` | Modify (new) | 4 个 `__time_*` C FFI 实现 |
| `crates/ruyi_runtime/src/json_ffi.rs` | Modify (new) | 2 个 `__json_*` C FFI 实现 |
| `crates/ruyi_runtime/src/builtins.rs` | Modify | `pub use` 把 `__math_*`/`__time_*`/`__json_*` re-export |
| `crates/ruyi_runtime/src/lib.rs` | Modify | `pub mod math_ffi;`/`time_ffi;`/`json_ffi;` |
| `crates/ruyic/src/typechecker/inference.rs` | Modify | resolve_builtin_name 加 14+4+2=20 条注册 |
| `crates/ruyic/src/codegen/builtins.rs` | Modify | 14+4+2=20 个 `fn declare_*` + 20 次调用 |
| `stdlib/math.ry` | Modify (new) | 14 函数调用 `__math_*` |
| `stdlib/time.ry` | Modify (new) | 4 函数调用 `__time_*` |
| `stdlib/json.ry` | Modify (new) | 2 函数调用 `__json_*`（含 `dyn` 暂未生效） |
| `examples/math_demo.ry` | Modify (new) | math 端到端验证 example（compile-only 受 parser/arch 限制） |

---

## A. 共享前缀重命名 runtime 符号（A 路）

### T-math-A1: `math_ffi.rs` 重命名 14 个
- `pub extern "C" fn ruyi_math_pi/e/sqrt/pow/abs/min/max/sin/cos/tan/log/ceil/floor/round`
  → `pub extern "C" fn __math_*` (`replaceAll`)
- 验证: `nm math_ffi.o | grep -c ruyi_math` → 0

### T-time-A1: `time_ffi.rs` 重命名 4 个
- `ruyi_time_now/timestamp/sleep/format` → `__time_*`
- 验证: `grep -c ruyi_time time_ffi.rs` → 0

### T-json-A1: `json_ffi.rs` 重命名 2 个
- `ruyi_json_parse/stringify` → `__json_*`
- 验证: `grep -c ruyi_json json_ffi.rs` → 0

### T-builtins-A2: `builtins.rs` pub use 同步重命名
- `pub use crate::math_ffi::{ruyi_math_*}` → `pub use crate::math_ffi::{__math_*}`
- `pub use crate::time_ffi::{ruyi_time_*}` → `pub use crate::time_ffi::{__time_*}`
- `pub use crate::json_ffi::{ruyi_json_*}` → `pub use crate::json_ffi::{__json_*}`
- 验证: `cargo check -p ruyi_runtime` → exit 0

---

## B. typechecker 注册（`crates/ruyic/src/typechecker/inference.rs::resolve_builtin_name`）

### T-math-B1: 14 条 `__math_*` 类型签名
```rust
"__math_pi"  => Some(Type::Function { params: vec![],           return_type: Box::new(Type::Float) }),
"__math_e"   => Some(Type::Function { params: vec![],           return_type: Box::new(Type::Float) }),
"__math_sqrt"   => Some(Type::Function { params: vec![Type::Float], return_type: Box::new(Type::Float) }),
"__math_pow"    => Some(Type::Function { params: vec![Type::Float, Type::Float], return_type: Box::new(Type::Float) }),
"__math_abs"    => Some(Type::Function { params: vec![Type::Float], return_type: Box::new(Type::Float) }),
"__math_min"    => Some(Type::Function { params: vec![Type::Float, Type::Float], return_type: Box::new(Type::Float) }),
"__math_max"    => Some(Type::Function { params: vec![Type::Float, Type::Float], return_type: Box::new(Type::Float) }),
"__math_sin"    => Some(Type::Function { params: vec![Type::Float], return_type: Box::new(Type::Float) }),
"__math_cos"    => Some(Type::Function { params: vec![Type::Float], return_type: Box::new(Type::Float) }),
"__math_tan"    => Some(Type::Function { params: vec![Type::Float], return_type: Box::new(Type::Float) }),
"__math_log"    => Some(Type::Function { params: vec![Type::Float], return_type: Box::new(Type::Float) }),
"__math_ceil"   => Some(Type::Function { params: vec![Type::Float], return_type: Box::new(Type::Float) }),
"__math_floor"  => Some(Type::Function { params: vec![Type::Float], return_type: Box::new(Type::Float) }),
"__math_round"  => Some(Type::Function { params: vec![Type::Float], return_type: Box::new(Type::Float) }),
```
验证: `cargo check -p ruyic` → exit 0

### T-time-B1: 4 条 `__time_*` 类型签名
```rust
"__time_now"       => Some(Type::Function { params: vec![], return_type: Box::new(Type::Int) }),
"__time_timestamp" => Some(Type::Function { params: vec![], return_type: Box::new(Type::Int) }),
"__time_sleep"     => Some(Type::Function { params: vec![Type::Float], return_type: Box::new(Type::Void) }),
"__time_format"    => Some(Type::Function { params: vec![Type::Int], return_type: Box::new(Type::String) }),
```
验证: 同

### T-json-B1: 2 条 `__json_*` 类型签名
```rust
"__json_parse"     => Some(Type::Function { params: vec![Type::String], return_type: Box::new(Type::String) }),
"__json_stringify" => Some(Type::Function { params: vec![Type::String], return_type: Box::new(Type::String) }),
```
验证: 同

---

## C. codegen LLVM 声明（`crates/ruyic/src/codegen/builtins.rs`）

### T-math-C1: 14 个 `fn declare_math_*` + 14 次调用

每条形如：
```rust
fn declare_math_pi<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let f64_ty = context.f64_type();
    let fn_type = f64_ty.fn_type(&[], false);
    module.add_function("__math_pi", fn_type, None);
}
```

并在 `declare_builtins` 末尾追加 14 次调用。

### T-time-C1: 4 个 `fn declare_time_*` + 4 次调用
```rust
fn declare_time_now<'ctx>(...) {
    let i64_ty = context.i64_type();
    let fn_type = i64_ty.fn_type(&[], false);
    module.add_function("__time_now", fn_type, None);
}
// declare_time_timestamp → i64 @__time_timestamp()
// declare_time_sleep     → void @__time_sleep(double)
// declare_time_format    → i8* @__time_format(i64)
```

### T-json-C1: 2 个 `fn declare_json_*` + 2 次调用
```rust
fn declare_json_parse<'ctx>(...) {
    let i8_ptr = context.i8_type().ptr_type(Default::default());
    let fn_type = i8_ptr.fn_type(&[i8_ptr.into()], false);
    module.add_function("__json_parse", fn_type, None);
}
// declare_json_stringify → i8* @__json_stringify(i8*)
```

### T-builtins-C2: `declare_builtins` 末尾追加 20 次调用
```rust
declare_math_pi(context, module);
declare_math_e(context, module);
declare_math_sqrt(context, module);
declare_math_pow(context, module);
declare_math_abs(context, module);
declare_math_min(context, module);
declare_math_max(context, module);
declare_math_sin(context, module);
declare_math_cos(context, module);
declare_math_tan(context, module);
declare_math_log(context, module);
declare_math_ceil(context, module);
declare_math_floor(context, module);
declare_math_round(context, module);

declare_time_now(context, module);
declare_time_timestamp(context, module);
declare_time_sleep(context, module);
declare_time_format(context, module);

declare_json_parse(context, module);
declare_json_stringify(context, module);
```

### T-builtins-C3: fmt_ffi 命名错位 — **未处理**
按 design.md D5，回滚 fmt_ffi.rs rename，仅在 v0.5.9 迁移计划中处理。

---

## D. stdlib `.ry` 与 example 验证

### T-math-D1: `stdlib/math.ry` — 已存在（2026-07-11 引入），保持现状
### T-time-D1: `stdlib/time.ry` — 已存在，保持现状
### T-json-D1: `stdlib/json.ry` — 已存在，**端到端不可用**（parser `dyn` bug）
### T-math-D2: `examples/math_demo.ry` — 已存在，math slice 端到端验证 example
```ry
import { PI, E, sqrt, pow, abs, min, max, floor, ceil, round, sin, cos } from "math";

fn main() {
    print("PI = " + PI);
    print("sqrt(16) = " + sqrt(16.0));
    print("pow(2,10) = " + pow(2.0, 10.0));
    print("min(3,5) = " + min(3.0, 5.0));
    print("max(3,5) = " + max(3.0, 5.0));
    print("abs(-3.5) = " + abs(0.0 - 3.5));
    print("floor(2.7) = " + floor(2.7));
    print("ceil(2.3) = " + ceil(2.3));
    print("round(2.5) = " + round(2.5));
    print("sin(0) = " + sin(0.0));
    print("cos(0) = " + cos(0.0));
}
```

### T-verify-1: `cargo check --workspace` → exit 0
### T-verify-2: `cargo build --release` → exit 0
### T-verify-3: `./target/release/ruyic --check stdlib/math.ry` → "Type checking passed."
### T-verify-4: `./target/release/ruyic --check stdlib/time.ry` → "Type checking passed."
### T-verify-5: `./target/release/ruyic --check stdlib/json.ry` → 已知 parser bug（记入 Out of Scope）

---

## E. git 提交与分支

### T-git-1: 工作树整理
- 已确认 `dev/v0.5.8` 分支
- 已确认 main 上两枚 `chore(sdd)` 落定（commit 3cd0d4c、684ad78）

### T-git-2: `feat(stdlib): add math/time/json core FFI wiring (v0.5.8)`
- 含 3 个 untracked `*_ffi.rs` + 3 个 untracked `*.ry` + 4 个 modified 文件

### T-git-3: `chore(sdd): add v0.5.8-stdlib-core卷宗 artifacts`
- 含 proposal/specs/design/tasks/execution-contract/.yaml/decision-point-audit.md

### T-git-4: `git push origin dev/v0.5.8`
- 推送至远程端（需要远程存在 `dev/v0.5.8` 分支）

---

## F. 已完成进度（reflective）

| 任务 | 状态 | 证据 |
|------|------|------|
| T-math-A1 | ✅ | `grep -c ruyi_math math_ffi.rs` = 0 |
| T-time-A1 | ✅ | `grep -c ruyi_time time_ffi.rs` = 0 |
| T-json-A1 | ✅ | `grep -c ruyi_json json_ffi.rs` = 0 |
| T-builtins-A2 | ✅ | `cargo check -p ruyi_runtime` exit 0 |
| T-math-B1 14 entries | ✅ | `grep -c __math_ inference.rs` = 14 |
| T-time-B1 4 entries | ✅ | 同 |
| T-json-B1 2 entries | ✅ | 同 |
| T-math-C1 14 declares + 14 calls | ✅ | `grep -cE 'declare_math_\|fn declare_math_' builtins.rs` = 28 |
| T-time-C1 4 declares + 4 calls | ✅ | 类似 |
| T-json-C1 2 declares + 2 calls | ✅ | 类似 |
| T-builtins-C2 | ✅ | declare_builtins 末尾追加完毕 |
| T-builtins-C3（fmt_ffi rename） | ⚠️ 回滚 | 发现 duplicate 重复源后回滚至 lib.rs:54 = `ruyi_string_replace_all` |
| T-math-D2 example | ✅ | examples/math_demo.ry 存在 |
| T-verify-1 cargo check --workspace | ✅ | exit 0 |
| T-verify-2 cargo build --release | ✅ | exit 0 |
| T-verify-3 --check math | ✅ | "Type checking passed." |
| T-verify-4 --check time | ✅ | "Type checking passed." |
| T-verify-5 --check json | ⚠️ | parser `dyn` bug（不修） |

