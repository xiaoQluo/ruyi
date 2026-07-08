# Tasks: Try/Catch Invoke 修复

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `crates/ruyi_exception/Cargo.toml` | Create | shared crate 清单,声明 inkwell feature gate |
| `crates/ruyi_exception/src/lib.rs` | Create | crate root,re-export LandingPadGenerator 与 type id 常量 |
| `crates/ruyi_exception/src/landing_pad.rs` | Create | 搬迁自 ruyi_runtime/exception/landing_pad.rs;改造 TypeId 为整数 |
| `crates/ruyi_runtime/src/exception/landing_pad.rs` | Modify | 替换为 `pub use ruyi_exception::landing_pad::*` |
| `crates/ruyi_runtime/Cargo.toml` | Modify | 添加 ruyi_exception 依赖(默认 features);移除直接 inkwell 依赖(若仅 LandingPadGenerator 使用) |
| `crates/ruyic/Cargo.toml` | Modify | 添加 ruyi_exception 依赖,启用 `llvm14` feature |
| `Cargo.toml` (workspace root) | Modify | 添加 `crates/ruyi_exception` 到 `[workspace.members]` |
| `crates/ruyic/src/codegen/generator.rs` | Modify | `CodegenContext` 新增 `try_stack: Vec<TryFrame>` 字段与 RAII guard |
| `crates/ruyic/src/codegen/stmt.rs` | Modify | `compile_try` 改用 build_invoke + landingpad;`compile_throw` 加 unreachable |
| `crates/ruyic/src/codegen/expr.rs` | Modify | `compile_call` 在 try_stack 非空时改用 build_invoke |
| `crates/ruyic/tests/try_catch_invoke.rs` | Create | codegen 集成测试(全部 `#[ignore]`,需 LLVM 14) |
| `examples/try_catch_invoke.ry` | Create | 端到端 example,内层函数抛出 → 外层 catch 捕获 |
| `examples/run_examples.sh` | Modify | 接入 `try_catch_invoke.ry`,总数 33 → 34 |
| `TRY_CATCH_AUDIT.md` | Modify | §3 无 try_context 的 unreachable 处理 + §5 表格三行更新 |

## Interfaces

### `ruyi_exception::landing_pad::LandingPadGenerator`
- Consumes: `&'ctx Context`, `&'m Module<'ctx>`, `&'b Builder<'ctx>`, `&[CatchTypeId: u32]`, `BasicBlock<'ctx>`, `BasicBlock<'ctx>`, `FunctionValue<'ctx>`
- Produces: `landingpad`, `invoke`, `resume`, `catch_dispatch` LLVM IR 构造
- Consuming crates: ruyic(必启用 `llvm14` feature),ruyi_runtime(默认 `--no-default-features` 路径仍可编译)

### `ruyi_exception::TryTypeId`
- `pub type TryTypeId = u32` — 用整数 type id 替换原 `TypeId`,解除与 ruyi_runtime 特定类型的耦合

### `ruyic::codegen::generator::TryFrame`
- 字段: `landing_pad_bb: BasicBlock<'ctx>`, `catch_bb: Option<BasicBlock<'ctx>>`, `finally_bb: Option<BasicBlock<'ctx>>`, `exception_ptr: PointerValue<'ctx>`
- Consumes: 由 `compile_try` 进入时 push;所有出口(正常完成、catch 跳、finally 跳)统一 RAII pop
- Produces: 给 `compile_call` 查栈顶作为 invoke 的 unwind_bb

### `ruyic::codegen::generator::TryStackGuard<'a>`
- Consumes: `&'a mut CodegenContext`, RAII drop 时自动 pop
- Produces: 自动保证 try_stack 不漏帧

---

## Wave 1: 基础设施(2 项并行)

### T1: 新建 `ruyi_exception` shared crate 并搬迁 LandingPadGenerator

**预估**: ~10 个原子 step,~80 行代码 | **依赖**: 无

#### Step 1: 起草 `crates/ruyi_exception/Cargo.toml`
- [x] 创建 `crates/ruyi_exception/` 目录
- [x] 写入 `Cargo.toml`,包含:
  - `[package] name = "ruyi_exception", version = "0.5.5", edition = "2021"`
  - `[dependencies] inkwell = { workspace = true, default-features = false, features = ["llvm14-0"] }`(feature gate)
  - `[features] llvm14 = ["dep:inkwell"]`
  - `default-features = false`(让 `--no-default-features` 干净)

#### Step 2: 复制现有 `LandingPadGenerator` 实现到 `crates/ruyi_exception/src/landing_pad.rs`
- [x] 创建 `crates/ruyi_exception/src/landing_pad.rs`
- [x] 复制 `crates/ruyi_runtime/src/exception/landing_pad.rs` 全部内容(7 个方法)
- [x] **REFACTOR**: 把 `TypeId` 形参改为 `TryTypeId`(= u32)
- [x] **REFACTOR**: 把所有 `ruyi_runtime::exception::TypeId` 引用改为 `TryTypeId`
- [x] 验证: `cargo check -p ruyi_exception --features llvm14` 编译通过

#### Step 3: 编写 `crates/ruyi_exception/src/lib.rs`
- [x] 在 `lib.rs` 中写 `pub mod landing_pad;`
- [x] 添加 `pub use landing_pad::{LandingPadGenerator, TryTypeId};`

#### Step 4: 注册 workspace member
- [x] 修改根 `Cargo.toml`,在 `[workspace.members]` 添加 `"crates/ruyi_exception"`
- [x] 验证: `cargo check --workspace` (无 LLVM 路径) 编译通过

#### Step 5: 让 ruyi_runtime 改为 re-export
- [x] 修改 `crates/ruyi_runtime/src/exception/landing_pad.rs`: 全文件替换为 `pub use ruyi_exception::landing_pad::*;`
- [x] 验证: `cargo check -p ruyi_runtime` 通过

#### Step 6: 让 ruyic 依赖 ruyi_exception(本 step 暂不调用,只为后续 T3/T4 铺路)
- [x] 修改 `crates/ruyic/Cargo.toml`: 添加 `ruyi_exception = { workspace = true, features = ["llvm14"] }`
- [x] 验证: `cargo check -p ruyic` 通过

#### Step 7: 验证 ruyi_runtime 的 `--no-default-features` 路径不破
- [x] `cargo check -p ruyi_runtime --no-default-features` 通过(确认 inkwell 不被强制引入)

#### Step 8: 单元测试 placeholder (TDD-RED)
- [x] 在 `crates/ruyi_exception/src/landing_pad.rs` 添加 `#[cfg(test)] mod tests`,写一个最小单元测试 `test_landing_pad_types` 验证 `TryTypeId` 的语义(可暂时只是 `assert_eq!(0u32, 0u32)` 占位,核心测试在 T7)

#### Step 9: 最终验证
- [x] `cargo check --workspace` 通过,零警告
- [x] `cargo clippy --workspace` 通过,零警告
- [x] `cargo test -p ruyi_exception --features llvm14` 通过(目前测试 0 个即可,只是验证 build OK)

#### TDD Summary
- RED: step 8 写测试
- GREEN: step 2/3/4/5/6/7 让 build 恢复
- REFACTOR: step 8 测试 placeholder

---

### T2: `CodegenContext` 新增 `try_stack` 与 RAII guard

**预估**: ~5 步,~50 行代码 | **依赖**: 无(与 T1 并行)

#### Step 1: 起草 `TryFrame` 结构体
- [x] `crates/ruyic/src/codegen/generator.rs`:`pub struct TryFrame<'ctx> { landing_pad_bb: BasicBlock<'ctx>, catch_bb: Option<BasicBlock<'ctx>>, finally_bb: Option<BasicBlock<'ctx>>, exception_ptr: PointerValue<'ctx> }`

#### Step 2: 起草 `TryStackGuard`
- [x] `pub struct TryStackGuard<'a, 'ctx> { ctx: &'a mut CodegenContext<'ctx> }`
- [x] `impl Drop` 在 drop 时调用 `ctx.try_stack.pop()`,确保任何提前 return 不漏帧
- [x] 添加 constructor: `impl TryStackGuard { pub fn push(ctx, frame) -> Self { ctx.try_stack.push(frame); Self { ctx } } }`

#### Step 3: 在 `CodegenContext` 中添加 `try_stack: Vec<TryFrame<'ctx>>`
- [x] 在 `CodegenContext` struct 中添加字段
- [x] 在 constructor(`Context::new` 或同等位置)初始化为空 Vec

#### Step 4: 编译验证
- [x] `cargo check -p ruyic` 通过(TDD-GREEN)
- [x] 无新增 clippy 警告

#### Step 5: 单元测试 (TDD-GREEN 验证基础行为)
- [x] 在 `crates/ruyic/src/codegen/generator.rs::tests` 写测试 `test_try_stack_push_pop`,验证 guard RAII 语义:
  - guard 内 `ctx.try_stack.len() == 1`
  - guard 离开 scope 后 `ctx.try_stack.len() == 0`
- [x] 运行 `cargo test -p ruyic --lib codegen::generator::tests::test_try_stack_push_pop` 通过

#### TDD Summary
- RED: step 5 写测试(`ctx.try_stack.push/pop` 的 API 验证)
- GREEN: step 2/3 让 build 恢复,step 5 测试通过
- REFACTOR: step 1 命名清晰化

---

## Wave 2: 核心改造(2 项并行,依赖 Wave 1)

### T3: `compile_throw` 末尾加 unreachable

**预估**: ~3 步,~15 行 | **依赖**: T1, T2(T2 提供 try_stack, T1 提供 LandingPadGenerator 路径)

#### Step 1: 现状分析(RED)
- [x] 阅读 `crates/ruyic/src/codegen/stmt.rs:185-242` 现有 `compile_throw` 实现
- [x] 识别哪些行调用 `try_stack.last()` 与 `build_unconditional_branch`

#### Step 2: 改造 throw 末尾
- [x] 在 `ruyi_throw` 调用后的跳转分支**之后**,添加 `ctx.builder.build_unreachable();`
- [x] 重构: 把"有 try_stack"与"无 try_stack"两个分支末端都加 unreachable
- [x] TDD-RED: 在 `crates/ruyic/tests/try_catch_throw.rs` 添加 codegen 测试(暂时 `#[ignore]`): 检查编译出的 IR 在 throw 调用后含 `unreachable` 指令

#### Step 3: 验证
- [x] `cargo check -p ruyic` 通过
- [x] 现有 try/catch example 编译通过(回归测试)
- [x] 暂不验证 codegen 集成测试(等 T7 一起)

#### TDD Summary
- RED: step 2 codegen 测试(发现 IR 中没有 unreachable)
- GREEN: step 2 实现 unreachable
- REFACTOR: step 2 末端的分支清理

---

### T4: `compile_try` 改用 build_invoke + landingpad

**预估**: ~12 步,~120 行 | **依赖**: T1, T2

#### Step 1: 现状分析
- [x] 阅读 `crates/ruyic/src/codegen/stmt.rs:245-378` 现有 `compile_try`,列出所有 `build_call` 调用位置

#### Step 2: 引入 LandingPadGenerator
- [x] `compile_try` 顶部:`let lp_gen = LandingPadGenerator::new(&ctx.context, &ctx.module, &ctx.builder);`

#### Step 3: 用 try_stack 替代手动 exception_ptr 管理
- [x] 进入 `compile_try`,创建 `TryFrame`:
  - `landing_pad_bb: ctx.context.append_basic_block(ctx.current_function.unwrap(), "try.landingpad")`
  - `catch_bb / finally_bb` 由当前编译的 try_stmt 决定
  - `exception_ptr`: 改为由 landingpad 指令分配(代替手动 alloca)
- [x] 用 `TryStackGuard::push(ctx, frame)` 入栈

#### Step 4: 重写函数调用逻辑
- [x] 在 `compile_block`(try body 内部)的函数调用处,改用 `ctx.try_stack.last()` 决定的 invoke
- [x] 注意: 本步骤需要在 `compile_block` 的参数中加入"(in_try, unwind_bb)"

#### Step 5: 生成 landingpad 指令
- [x] 在 catch block 顶部调用 `lp_gen.build_landing_pad(&catch_type_ids, has_cleanup, "landingpad")`
- [x] 调用 `lp_gen.extract_exception_ptr(landing_pad_val)` 取出异常指针
- [x] 调用 `lp_gen.build_catch_dispatch(...)` 分发到 catch 子句
- [x] 调用 `lp_gen.build_resume(...)` 处理未捕获异常

#### Step 6: try_stack 出口处理
- [x] 正常路径完成后,用 `build_unconditional_branch` 跳到 merge_bb(若存在)
- [x] finally 路径: 在跳转前 finally_block 必须先执行(由 LLVM cleanup 实现保证);guard 自动 pop
- [x] guard 自动 pop 在所有退出 path 上统一发生

#### Step 7: catch 块后续 PHI 处理
- [x] 若 try 体存在 PHI 节点的必要变量,在 catch landingpad 后的合并基本块插入 PHI,与正常路径汇合

#### Step 8: TDD-RED: 现有 throwStmt 仍然工作
- [x] 临时回归测试: 编译 `examples/error.ry` 中已有的 try/catch example,确保不破坏

#### Step 9: TDD-GREEN: 内层 throw 被外层 catch 捕获
- [x] 临时编译验证 `examples/try_catch_invoke.ry`(尚不存在,但可以临时 inline 一段 `fn innerThrow() { throw new Error(); }` 验证)
- [x] 实际 LLVM IR 含 `invoke + landingpad`

#### Step 10: 编译验证
- [x] `cargo check -p ruyic` 通过
- [x] `cargo test -p ruyic --lib` 全部通过(无回归)
- [x] `cargo build --release` 零警告(若 LLVM 14 可用)

#### Step 11: 重构(REFACTOR)
- [x] 提取 `compile_try_blocks`(stmt helper 函数) 拆分 register_bb / invoke 决策 / landingpad 构造
- [x] 提取 `compile_catch_arm_landing` 合并多 catch arm 处理

#### Step 12: 整体 Review
- [x] `cargo clippy -p ruyic` 零警告
- [x] `git diff` 审查,行数控制在 ~120

#### TDD Summary
- RED: step 8/9 端到端验证
- GREEN: step 2/3/4/5/6/7 改造 compile_try
- REFACTOR: step 11

---

## Wave 3: 调用方改造(1 项,依赖 T3 + T4)

### T5: `compile_call` 在 try_stack 非空时改用 build_invoke

**预估**: ~7 步,~40 行 | **依赖**: T4

#### Step 1: 定位 `compile_call`
- [x] 阅读 `crates/ruyic/src/codegen/expr.rs` 中现有 `compile_call` 实现

#### Step 2: 引入 try_stack 判断
- [x] 在 `compile_call` 函数顶部:`let unwind_bb = ctx.try_stack.last().map(|f| f.landing_pad_bb);`

#### Step 3: 修改 call/invoke 决策
- [x] 若 `unwind_bb.is_some()` 且当前 callee 不是 landingpad 自身,则:
  - `use lp_gen = LandingPadGenerator::new(...);`
  - `lp_gen.build_invoke(func_val, &args, then_bb, unwrap(unwind_bb), "invoke")`
- [x] 否则,沿用原 `build_call`

#### Step 4: PHI 处理(若被调函数有返回值)
- [x] invoke 后的返回值通过 `invoke.get_result()`(inkwell API)取
- [x] catch landingpad 后的 PHI 节点收集返回值(本任务先实现基本版本:无返回值的函数优化路径可推迟)

#### Step 5: TDD-RED: try 内 call 用 invoke,try 外 call 仍用 call
- [x] codegen 测试验证场景:
  - case A: `try { foo(); } catch (e) {}` → IR 含 `invoke @foo`
  - case B: `fn main() { foo(); }` → IR 含 `call @foo`(零回归)

#### Step 6: 编译验证
- [x] `cargo check -p ruyic` 通过
- [x] `cargo test -p ruyic --lib` 全部通过(无回归)

#### Step 7: REFACTOR
- [x] 提取 `emit_function_call_invoke_or_call(ctx, func_val, args) -> CallSiteValue` 统一决策点
- [x] 简化 `compile_call` 内部函数

#### TDD Summary
- RED: step 5 codegen 测试
- GREEN: step 3 改造实现
- REFACTOR: step 7

---

## Wave 4: 验证、新 example 与文档(3 项并行)

### T6: 新增 `examples/try_catch_invoke.ry` 并接入 run_examples.sh

**预估**: ~3 步,~30 行 | **依赖**: T4 + T5

#### Step 1: 起草 example 源码
- [x] 创建 `examples/try_catch_invoke.ry`:
```ryui
fn innerThrow(): void {
  throw new Error("boom");
}

fn main(): int {
  try {
    innerThrow();
  } catch (e) {
    print("caught");
    return 0;
  }
  return 1;
}
```

#### Step 2: 接入 run_examples.sh
- [x] 修改 `examples/run_examples.sh`,在 example 列表中添加 `try_catch_invoke`
- [x] 总数 33 → 34

#### Step 3: 端到端验证(若 LLVM 14 可用)
- [x] `ruyic examples/try_catch_invoke.ry -o /tmp/try_catch_invoke`
- [x] `/tmp/try_catch_invoke` → 输出含 "caught",exit 0

#### TDD Summary
- RED: N/A(example 不在测试框架内)
- GREEN: step 1 + 2 写出 example
- REFACTOR: step 3 验证

---

### T7: codegen 集成测试 `crates/ruyic/tests/try_catch_invoke.rs`

**预估**: ~5 步,~50 行 | **依赖**: T4 + T5

#### Step 1: 起草测试 crate
- [x] 创建 `crates/ruyic/tests/try_catch_invoke.rs`
- [x] 顶部 `use std::process::Command;`
- [x] 文件顶部注释注明"所有测试 #[ignore],需 LLVM 14"

#### Step 2: 写测试用例 A: invoke 指令存在
- [x] 测试: `test_try_catch_emits_invoke`
- [x] 编译 `examples/try_catch_invoke.ry --emit-llvm` 到临时 .ll 文件
- [x] 读取 IR,断言含 `invoke`
- [x] 标注 `#[ignore]`

#### Step 3: 写测试用例 B: landingpad 存在
- [x] 测试: `test_try_catch_emits_landingpad`
- [x] 读取同一份 .ll,断言含 `landingpad`

#### Step 4: 写测试用例 C: 端到端执行
- [x] 测试: `test_try_catch_catches_inner_throw`
- [x] 编译为二进制并执行,断言 stdout 含 "caught" 且 exit 0

#### Step 5: 验证
- [x] `cargo test -p ruyic --test try_catch_invoke -- --ignored`(需 LLVM 14)通过
- [x] 不加 `--ignored` 时:`cargo test -p ruyic --test try_catch_invoke` 显示测试被跳过,不失败

#### TDD Summary
- RED: step 2/3/4 测试存在并 fail(skipped)
- GREEN: 实际由 T4+T5 的 codegen 实现支撑
- REFACTOR: N/A

---

### T8: 更新 `TRY_CATCH_AUDIT.md`

**预估**: ~2 步,~10 行 | **依赖**: T4

#### Step 1: §3 末尾加注释
- [x] 在 `## 3. compile_throw Implementation Analysis` 末尾添加:
  ```
  > **UPDATE 2026-07 (post-fix-try-catch-invoke)**: 
  > §3.A/§3.B 已实现 unreachable;§3.C 现在正确: 无 try 上下文时直接 unreachable。
  ```

#### Step 2: §5 表格三行更新
- [x] 把 `Is LandingPadGenerator compatible with codegen?` 答案由 NO 改 YES
- [x] 把 `LandingPadGenerator accessible from ruyic` 一项,由 NO 改 YES(via ruyi_exception shared crate)
- [x] 把 `Does compile_try use invoke?` 一项 NO 改 YES

#### TDD Summary
- N/A (文档更新无 TDD)

---

## 整体验证(完成所有 Wave 后)

- [x] `cargo check --workspace` → 零警告
- [x] `cargo clippy --workspace` → 零警告
- [x] `cargo test --workspace` → 全部通过(无新增失败)
- [x] `cargo test -p ruyi_runtime --no-default-features --lib` → 全部通过(无 LLVM 环境)
- [x] `bash examples/run_examples.sh` → Total: 34 | Passed: 34 | Failed: 0(需 LLVM 14)
- [x] `TRY_CATCH_AUDIT.md` §5 表格三行答案已更新(由 NO/NO/NO → YES/YES/YES)
- [x] `git diff` 审查:无 `as any` / `@ts-ignore` / 空 catch 块 / `unwrap()` on Result
