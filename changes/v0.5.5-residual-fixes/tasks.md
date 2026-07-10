# Tasks: v0.5.5-residual-fixes

> 7 项 P0 修复，4 阶段交付。每批次 TDD 5 步走：RED → GREEN → REFACTOR → VERIFY → COMMIT。

## File Structure

### New Files

| 路径 | 职责 |
|------|------|
| `crates/ruyic/src/codegen/gc_alloc.rs` | GC 调用分发（`--gc=stub` 走 `cc_alloc`，`--gc=real` 走 `ruyi_gc_alloc`） |
| `crates/ruyic/src/cli/gc_mode.rs` | `--gc=<mode>` 标志解析与校验 |
| `crates/ruyic/tests/gc_flag.rs` | GC flag 集成测试（stub/real 切换、非法值拒绝） |
| `crates/ruyic/src/typechecker/impl_table.rs` | `HashMap<(TraitId, TypeId), ImplDef>` 全局表 |
| `crates/ruyic/tests/trait_bounds.rs` | trait 约束检查集成测试（已存在则合并） |
| `crates/ruyi_runtime/src/sched/worker.rs` | 工作窃取 worker（基于 `crossbeam-deque`） |
| `crates/ruyi_runtime/src/sched/injector.rs` | 调度入口 `Injector`（外部提交 future） |
| `crates/ruyi_runtime/src/async_runtime.rs` | `ruyi_await` 真实实现（替换 stub） |
| `crates/ruyi_runtime/tests/spawn.rs` | spawn 集成测试 |
| `crates/ruyic/src/codegen/builtins/spawn.rs` | `spawn` 内建 LLVM IR 生成 |
| `examples/spawn_demo.ry` | spawn 演示示例（3+ 并发任务） |
| `examples/async_sleep.ry` | async/await 演示示例 |
| `examples/trait_bounds.ry` | trait 约束演示示例 |
| `tools/audit-stdlib/src/main.rs` | stdlib 8 模块审计工具 |

### Modified Files

| 路径 | 修改内容 |
|------|---------|
| `crates/ruyic/src/main.rs` | 新增 `--gc=<mode>` CLI 标志 |
| `crates/ruyic/src/driver.rs` | `--gc=real` 时链入 `libruyi_runtime.a`；改用 `cc::Build` |
| `crates/ruyic/src/codegen/mod.rs` | 集成 `gc_alloc.rs` 分发 |
| `crates/ruyic/src/codegen/stmt.rs` | `compile_try` 完整改 `invoke` + landing pad |
| `crates/ruyic/src/codegen/expr.rs` | `compile_call` 在 try 上下文用 `invoke` |
| `crates/ruyic/src/codegen/generator.rs` | `CodegenContext.try_stack` 状态字段 |
| `crates/ruyic/src/codegen/async_codegen.rs` | `ruyi_await` 调用真实实现 |
| `crates/ruyic/src/codegen/builtins/mod.rs` | 注册 `spawn` 内建 |
| `crates/ruyic/src/typechecker/generics.rs` | `check_bounds` 实际查询 `impl_table` |
| `crates/ruyic/src/typechecker/mod.rs` | 集成 `impl_table.rs` |
| `crates/ruyic/tests/codegen.rs` | 移除 21 个测试的 `#[ignore]`（成功后） |
| `crates/ruyic/tests/typechecker.rs` | 移除至少 5 个测试的 `#[ignore]`（成功后） |
| `crates/ruyic/tests/try_catch_invoke.rs` | 移除 13 个测试的 `#[ignore]` |
| `crates/ruyic/tests/compilation_throw_unreachable.rs` | 移除 3 个测试的 `#[ignore]` |
| `stdlib/collections.ry` | `RangeError` / `ArrayIterator` 构造函数补全 |
| `crates/ruyic/Cargo.toml` | 新增 `crossbeam-deque` 依赖 |
| `crates/ruyi_runtime/Cargo.toml` | 新增 `crossbeam-deque` 依赖 |
| `examples/run_examples.sh` | 接入 4 个新 example（async、spawn、trait_bounds、try_catch_invoke） |
| `docs/roadmap-zh.md` | P0 缺陷表更新为 ✅ |

## Interfaces

### Interface: `GcMode` (CLI)

```rust
// crates/ruyic/src/cli/gc_mode.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GcMode {
    Stub,
    Real,
}

impl GcMode {
    /// Parse from "--gc=stub" / "--gc=real" string. Returns Err for invalid.
    pub fn parse(s: &str) -> Result<Self, String>;
}

impl Default for GcMode {
    fn default() -> Self { Self::Stub }
}
```

### Interface: `GcAllocFn` (codegen)

```rust
// crates/ruyic/src/codegen/gc_alloc.rs
pub enum GcAllocFn {
    Stub,   // LLVM IR: call @cc_alloc
    Real,   // LLVM IR: call @ruyi_gc_alloc (declared external)
}

impl GcAllocFn {
    pub fn for_mode(mode: GcMode) -> Self;
    /// Emit LLVM IR calling the allocator. Returns the result value (i8*).
    pub fn emit<'ctx>(
        &self,
        builder: &Builder<'ctx>,
        module: &Module<'ctx>,
        size: IntValue<'ctx>,
    ) -> PointerValue<'ctx>;
}
```

### Interface: `ImplTable` (typechecker)

```rust
// crates/ruyic/src/typechecker/impl_table.rs
pub struct ImplTable {
    map: HashMap<(TraitId, TypeId), ImplDef>,
}

impl ImplTable {
    pub fn new() -> Self;
    /// Register an impl block. Idempotent (later wins).
    pub fn register(&mut self, trait_id: TraitId, type_id: TypeId, def: ImplDef);
    /// Check if impl exists. O(1) lookup.
    pub fn has_impl(&self, trait_id: TraitId, type_id: TypeId) -> bool;
    /// Iterate all impls of a trait (for diagnostic messages).
    pub fn impls_of_trait(&self, trait_id: TraitId) -> Vec<(TypeId, &ImplDef)>;
}
```

### Interface: `Scheduler` (ruyi_runtime)

```rust
// crates/ruyi_runtime/src/sched/mod.rs
pub struct Scheduler {
    workers: Vec<Worker>,
    injector: Injector<Task>,
}

impl Scheduler {
    pub fn new(worker_count: usize) -> Self;
    /// Submit a future to be executed. Returns immediately.
    pub fn spawn(&self, future: Future);
    /// Yield current coroutine and let scheduler pick next.
    pub fn yield_now(&self);
    /// Block current thread until all submitted tasks complete (for tests).
    pub fn join_all(&self);
}
```

### Interface: `compile_spawn` (codegen builtin)

```rust
// crates/ruyic/src/codegen/builtins/spawn.rs
/// Emit LLVM IR for `spawn(fn)`. Generates:
///   - ruyi_future_new(fn_wrapper)
///   - scheduler_spawn(future)
pub fn compile_spawn<'ctx>(
    builder: &Builder<'ctx>,
    module: &Module<'ctx>,
    fn_value: FunctionValue<'ctx>,
) -> Result<()>;
```

### Cross-Batch Consumes/Produces

| Producer | Consumer | Type/Artifact |
|----------|----------|---------------|
| T-1.1.1 (GcMode parse) | T-1.1.2 (codegen 分发) | `GcMode` enum |
| T-1.2.1 (runtime .a 构建) | T-1.2.2 (driver 链接) | `libruyi_runtime.a` 路径 |
| T-1.3.1 (T9 收尾) | T-1.3.2 (启用 21 测试) | `stdlib/collections.ry` 修改 |
| T-1.4.1 (ImplTable) | T-1.4.2 (check_bounds) | `ImplTable` struct |
| T-2.1.1 (Scheduler) | T-2.1.2 (ruyi_await) | `Scheduler` struct |
| T-2.2.1 (CodegenContext.try_stack) | T-2.2.2 (compile_try invoke) | `try_stack` field |
| T-3.1 (Scheduler) | T-3.2 (compile_spawn) | `Scheduler::spawn` |

## Batch 1.1: GC 双模式（#4 P0）

### T-1.1.1: GcMode CLI 解析

**File**: `crates/ruyic/src/cli/gc_mode.rs` (Create)

**TDD Steps**:
1. **RED**: 写测试 `gc_mode::tests::parse_stub_returns_stub` — 期望 `GcMode::parse("stub") == Ok(GcMode::Stub)`
2. **GREEN**: 实现 `GcMode::parse` 匹配字符串 `"stub"` 返回 `Ok(Stub)`
3. **REFACTOR**: 增加 `parse_real`、`parse_invalid_returns_err` 测试，扩 `parse` 逻辑
4. **VERIFY**: `cargo test -p ruyic gc_mode` 全绿
5. **COMMIT**: `feat(cli): add GcMode parser for --gc flag`

**Interfaces**: GcMode enum

---

### T-1.1.2: GcAllocFn codegen 分发

**File**: `crates/ruyic/src/codegen/gc_alloc.rs` (Create)

**Depends on**: T-1.1.1

**TDD Steps**:
1. **RED**: 写测试 `gc_alloc::tests::stub_emits_cc_alloc` — 期望 Stub 模式生成 `call @cc_alloc` 指令
2. **GREEN**: 实现 `GcAllocFn::Stub.emit` 用 inkwell `builder.build_call(cc_alloc_fn, ...)`
3. **REFACTOR**: 增加 `real_emits_ruyi_gc_alloc` 测试，Real 模式声明外部函数 `ruyi_gc_alloc` 并调用
4. **VERIFY**: `cargo test -p ruyic gc_alloc` 全绿；`cargo clippy -p ruyic` 零警告
5. **COMMIT**: `feat(codegen): add GcAllocFn stub/real dispatcher`

**Interfaces**: GcAllocFn enum + emit

---

### T-1.1.3: CLI --gc flag 接入

**File**: `crates/ruyic/src/main.rs` (Modify)

**Depends on**: T-1.1.1

**TDD Steps**:
1. **RED**: 写测试 `cli::tests::default_gc_is_stub` — 期望不传 `--gc` 时 driver 收到 `GcMode::Stub`
2. **GREEN**: 在 clap 中新增 `#[arg(long, default_value = "stub")] gc: String`，调用 `GcMode::parse`
3. **REFACTOR**: 错误处理——`GcMode::parse` 返回 `Err` 时 clap 报错退出
4. **VERIFY**: `cargo run -p ruyic -- --help` 显示 `--gc=<stub|real>`
5. **COMMIT**: `feat(cli): wire --gc=<mode> flag to driver`

---

### T-1.1.4: codegen 全部堆分配点切换

**File**: `crates/ruyic/src/codegen/mod.rs` + 所有子模块 (Modify)

**Depends on**: T-1.1.2, T-1.1.3

**TDD Steps**:
1. **RED**: 写测试 `gc_flag.rs::default_mode_uses_cc_alloc` — 编译 `examples/hello.ry`，断言 IR 含 `call @cc_alloc` 不含 `call @ruyi_gc_alloc`
2. **GREEN**: 在 codegen 所有分配点（`String`、`Array`、`Object`、`Function` 创建）替换为 `GcAllocFn::for_mode(mode).emit(...)`
3. **REFACTOR**: 增加 `real_mode_uses_ruyi_gc_alloc` 测试，断言 `--gc=real` 模式下 IR 含 `declare ... @ruyi_gc_alloc` + `call @ruyi_gc_alloc`
4. **VERIFY**: `cargo test -p ruyic --test gc_flag` 全绿；现有 examples 33/33 通过
5. **COMMIT**: `feat(codegen): route all heap allocations through GcAllocFn dispatcher`

**Interfaces**: GcAllocFn::for_mode

## Batch 1.2: 运行时库静态链接（#5 P0）

### T-1.2.1: ruyi_runtime 产出静态库

**File**: `crates/ruyi_runtime/Cargo.toml` + 入口 `lib.rs` (Modify)

**TDD Steps**:
1. **RED**: 写测试 `build.rs`（临时）— `cargo build -p ruyi_runtime --release` 后断言 `target/release/libruyi_runtime.a` 存在
2. **GREEN**: 在 `ruyi_runtime/Cargo.toml` 加 `[lib]` 段确认 crate-type 含 `staticlib`
3. **REFACTOR**: 加 `cargo build -p ruyi_runtime --release --no-default-features` 测试，无 LLVM 也能产 `.a`
4. **VERIFY**: 两次 build 都产出 `.a`；非空
5. **COMMIT**: `build(runtime): ensure libruyi_runtime.a is produced`

---

### T-1.2.2: driver 链入 libruyi_runtime.a

**File**: `crates/ruyic/src/driver.rs` (Modify)

**Depends on**: T-1.1.3, T-1.2.1

**TDD Steps**:
1. **RED**: 写测试 `driver::tests::real_mode_links_static` — `ruyic --gc=real examples/hello.ry -o hello && ldd ./hello | grep ruyi_runtime`，期望无输出
2. **GREEN**: 改 `driver.rs`：当 `gc == GcMode::Real` 时，`cc::Build::new().flag("-lruyi_runtime").flag(format!("{}/libruyi_runtime.a", path)).link(...)`
3. **REFACTOR**: 增加 `stub_mode_uses_cc` 测试，stub 模式不链 `.a`（保留原行为）
4. **VERIFY**: 两个测试都通过；二进制可执行；examples 33/33
5. **COMMIT**: `feat(driver): link libruyi_runtime.a in --gc=real mode`

**Interfaces**: `GcMode::Real` 触发的链接行为

## Batch 1.3: T9 收尾 + stdlib 审查（#2 P0）

### T-1.3.1: RangeError / ArrayIterator 构造器化

**File**: `stdlib/collections.ry` (Modify)

**Depends on**: T-1.1.4 (codegen 可调用外部函数)

**TDD Steps**:
1. **RED**: 写测试 `codegen.rs::range_error_throws_compiles` — 编译 `throw RangeError("x")`，断言 IR 无 "type not constructible" 错误
2. **GREEN**: 在 `stdlib/collections.ry` 给 `RangeError` 类加构造函数 `fn new(msg: string) { self.message = msg; }`
3. **REFACTOR**: 同样给 `ArrayIterator` 加 `fn new(arr: dyn Array) { self.array = arr; self.index = 0; }`
4. **VERIFY**: 写 `codegen.rs::array_iterator_construct` 测试，编译 `ArrayIterator(myArr)` 期望通过
5. **COMMIT**: `fix(stdlib): make RangeError and ArrayIterator constructible (T9 收尾)`

---

### T-1.3.2: 启用 21 个 codegen 测试

**File**: `crates/ruyic/tests/codegen.rs` (Modify)

**Depends on**: T-1.3.1

**TDD Steps**:
1. **RED**: 暂保留 `#[ignore]`，跑测试，断言至少 21 个 FAIL（基线）
2. **GREEN**: 移除 T-1.3.1 影响范围内的 21 个测试的 `#[ignore]`
3. **REFACTOR**: 整理测试命名（确保每个被启用的测试有清晰 `// Verifies: REQ-COLL-001/002` 注释）
4. **VERIFY**: `cargo test -p ruyic --test codegen -- --ignored --test-threads=1` 21 个全部 PASS
5. **COMMIT**: `test(codegen): enable 21 tests after T9 fix`

---

### T-1.3.3: stdlib 8 模块审计工具

**File**: `tools/audit-stdlib/src/main.rs` (Create)

**Depends on**: T-1.3.1

**TDD Steps**:
1. **RED**: 写测试 `audit_stdlib::tests::report_has_all_modules` — 跑工具，断言输出含 8 个模块名
2. **GREEN**: 工具读 `stdlib/*.ry`，对每个模块：
   - 列函数签名
   - 标记 stub（仅 `TODO` 或 `unimplemented!()`）
   - 输出 `report.md`
3. **REFACTOR**: 增加"未实装函数清单"统计
4. **VERIFY**: 工具产出 `docs/stdlib-audit-v0.5.5.md`，含所有 8 模块评估
5. **COMMIT**: `chore(tooling): add stdlib audit tool and v0.5.5 report`

**注**: 报告输出不阻塞 P0 修复；math/time/json 仅标记"未实装"，不实现

## Batch 1.4: trait 约束检查（#7 P0）

### T-1.4.1: ImplTable 数据结构

**File**: `crates/ruyic/src/typechecker/impl_table.rs` (Create)

**TDD Steps**:
1. **RED**: 写测试 `impl_table::tests::register_and_has_impl` — `register(Printable, int, def); has_impl(Printable, int) == true`
2. **GREEN**: 实现 `ImplTable` 用 `HashMap<(TraitId, TypeId), ImplDef>`
3. **REFACTOR**: 增加 `has_impl_for_unknown_returns_false`、`impls_of_trait_iterates_correctly` 测试
4. **VERIFY**: `cargo test -p ruyic impl_table` 全绿
5. **COMMIT**: `feat(typechecker): add ImplTable for O(1) trait impl lookup`

**Interfaces**: ImplTable

---

### T-1.4.2: check_bounds 实际验证

**File**: `crates/ruyic/src/typechecker/generics.rs` (Modify)

**Depends on**: T-1.4.1

**TDD Steps**:
1. **RED**: 写测试 `typechecker.rs::generic_with_no_impl_fails` — `fn f<T: Printable>(x: T) {} f(42)` 期望编译报错
2. **GREEN**: 改 `check_bounds`：替换 `return true` 为 `impl_table.has_impl(trait_id, type_id)`；若 false 返回 `Err("trait X not implemented for type Y")`
3. **REFACTOR**: 多 bound 测试 `fn f<T: A + B>` 确保两个 bound 都验证
4. **VERIFY**: 新测试通过；现有泛型 examples 不退化（手动回归 `examples/generics*.ry`）
5. **COMMIT**: `fix(typechecker): check_bounds validates impl existence`

**Interfaces**: ImplTable.has_impl

---

### T-1.4.3: 启用 5+ 个 typechecker 测试

**File**: `crates/ruyic/tests/typechecker.rs` (Modify)

**Depends on**: T-1.4.2

**TDD Steps**:
1. **RED**: 跑测试，断言至少 5 个原 `#[ignore]` 测试现在 PASS
2. **GREEN**: 移除 5 个测试的 `#[ignore]`
3. **REFACTOR**: 添加 `// Verifies: REQ-TRAIT-001` 注释
4. **VERIFY**: `cargo test -p ruyic --test typechecker` 通过
5. **COMMIT**: `test(typechecker): enable 5 trait-bounds tests`

## Batch 2.1: ruyi_await 真实化（#1 P0）

### T-2.1.1: Scheduler + Worker

**File**: `crates/ruyi_runtime/src/sched/{mod.rs,worker.rs,injector.rs}` (Create)

**TDD Steps**:
1. **RED**: 写测试 `sched::tests::submit_one_task_runs_it` — `scheduler.spawn(task); scheduler.join_all();` 断言 task 执行
2. **GREEN**: 实现 Scheduler：`workers: Vec<Worker>`（每 worker 一个 `crossbeam_deque::Worker`）+ `Injector`
3. **REFACTOR**: 增加 `test_work_stealing` — 8 个 worker，注入 100 任务，断言全部完成且负载相对均衡
4. **VERIFY**: `cargo test -p ruyi_runtime sched` 全绿；用 `loom` 跑并发测试
5. **COMMIT**: `feat(runtime): add work-stealing scheduler with crossbeam-deque`

**Interfaces**: Scheduler

---

### T-2.1.2: ruyi_await 真实实现

**File**: `crates/ruyi_runtime/src/async_runtime.rs` (Modify)

**Depends on**: T-2.1.1

**TDD Steps**:
1. **RED**: 写测试 `async_runtime::tests::await_ready_future_returns_immediately` — `await ready_future()` 立即返回
2. **GREEN**: 实现 `ruyi_await(future)`：调用 `future.poll()`，若 `Poll::Ready(v)` 返回 `v`；若 `Pending` 调用 `scheduler.yield_now()`
3. **REFACTOR**: 增加 `await_pending_future_resumes_when_ready` 测试
4. **VERIFY**: 测试全绿；runtime 单测不退化
5. **COMMIT**: `feat(runtime): implement real ruyi_await with scheduler yield`

---

### T-2.1.3: codegen 调用 ruyi_await

**File**: `crates/ruyic/src/codegen/async_codegen.rs` (Modify)

**Depends on**: T-2.1.2

**TDD Steps**:
1. **RED**: 写测试 `async_codegen.rs::await_emits_call_to_ruyi_await` — 编译 `await x`，断言 IR 含 `call @ruyi_await`
2. **GREEN**: `compile_await` 生成 `call @ruyi_await(future_value)`
3. **REFACTOR**: 增加 `await_in_real_mode_links_runtime` 测试，验证 `--gc=real` 下能链到 runtime
4. **VERIFY**: examples 含 await 的程序可编译运行（如新建 `examples/async_sleep.ry`）
5. **COMMIT**: `feat(codegen): emit real ruyi_await call in await expressions`

## Batch 2.2: try/catch landing pad（#3 P0）

### T-2.2.1: CodegenContext.try_stack

**File**: `crates/ruyic/src/codegen/generator.rs` (Modify)

**TDD Steps**:
1. **RED**: 写测试 `generator::tests::try_stack_push_pop_balanced` — 进入 try push、离开 try pop，断言 `try_stack.is_empty()`
2. **GREEN**: `CodegenContext` 加 `pub try_stack: Vec<TryFrame>`
3. **REFACTOR**: 增加嵌套 try 测试，外层 try pop 在内层 pop 后
4. **VERIFY**: 单测全绿；不破坏现有 codegen 测试
5. **COMMIT**: `feat(codegen): add try_stack to CodegenContext`

---

### T-2.2.2: compile_try 完整 invoke

**File**: `crates/ruyic/src/codegen/stmt.rs` (Modify)

**Depends on**: T-2.2.1, T-1.2.2 (需要链入 runtime)

**TDD Steps**:
1. **RED**: 写测试 `try_catch_invoke.rs::inner_throw_caught_by_outer` — 编译 `try { innerThrow(); } catch (e) { print("caught"); }`，运行输出含 "caught"
2. **GREEN**: 重写 `compile_try`：try 体所有 `compile_call` 改 `build_invoke`，unwind bb 指向 catch 的 landingpad
3. **REFACTOR**: 增加多 catch arm 测试 `catch (e: A) ... catch (e: B) ...`，验证 selector dispatch
4. **VERIFY**: 13 个 `try_catch_invoke` 测试 + 3 个 `compilation_throw_unreachable` 测试全 PASS
5. **COMMIT**: `fix(codegen): complete compile_try invoke + landing pad (finalizes fix-try-catch-invoke)`

---

### T-2.2.3: 启用 16 个 try/catch 测试

**File**: `crates/ruyic/tests/{try_catch_invoke,compilation_throw_unreachable}.rs` (Modify)

**Depends on**: T-2.2.2

**TDD Steps**:
1. **RED**: 跑测试确认基线（应 16 个 FAIL）
2. **GREEN**: 移除所有 `#[ignore]`
3. **REFACTOR**: 添加 `// Verifies: REQ-LPAD-003/004` 注释
4. **VERIFY**: `cargo test -- --ignored --test-threads=1` 16 个全 PASS
5. **COMMIT**: `test(try-catch): enable all 16 ignored tests`

## Batch 3: spawn 内建（#6 P0）

### T-3.1: spawn 内建 IR 生成

**File**: `crates/ruyic/src/codegen/builtins/spawn.rs` (Create) + `mod.rs` (Modify 注册)

**Depends on**: T-2.1.1 (Scheduler)

**TDD Steps**:
1. **RED**: 写测试 `builtins::spawn::tests::spawn_emits_call_to_scheduler_spawn` — 编译 `spawn(fn)`，断言 IR 含 `call @scheduler_spawn`
2. **GREEN**: 实现 `compile_spawn`：把 `fn` 包成 future，调 `scheduler_spawn(future)`
3. **REFACTOR**: stub 模式编译错误测试：默认模式下编译 `spawn(...)` 报错
4. **VERIFY**: 测试全绿；新 example `examples/spawn_demo.ry` 可编译
5. **COMMIT**: `feat(codegen): implement spawn builtin calling scheduler`

---

### T-3.2: spawn_demo example

**File**: `examples/spawn_demo.ry` (Create) + `run_examples.sh` (Modify)

**Depends on**: T-3.1

**TDD Steps**:
1. **RED**: 写测试 `examples::spawn_demo_runs` — 编译运行 `examples/spawn_demo.ry`，断言输出含 3 个 task 标识
2. **GREEN**: 写 example：3 个 task 各 `await sleep(random)` 然后 `print`；task 间输出应交错
3. **REFACTOR**: `run_examples.sh` 接入新 example；更新总数从 33 → 34
4. **VERIFY**: `bash examples/run_examples.sh` 报告 `Total: 34 | Passed: 34`
5. **COMMIT**: `feat(example): add spawn_demo with 3 concurrent tasks`

---

### T-3.3: spawn 集成测试

**File**: `crates/ruyi_runtime/tests/spawn.rs` (Create)

**Depends on**: T-3.1

**TDD Steps**:
1. **RED**: 写测试 `spawn::tests::spawn_runs_function` — `scheduler.spawn(|| 42)` 运行后返回值正确
2. **GREEN**: 实现 `Scheduler::spawn` 接受 `FnOnce + Send + 'static`
3. **REFACTOR**: 多任务并发测试 `spawn_100_tasks_all_complete`
4. **VERIFY**: `cargo test -p ruyi_runtime --test spawn` 全绿
5. **COMMIT**: `test(runtime): add spawn integration tests`

## Batch 4: 验证与归档

### T-4.1: 整体回归

**TDD Steps**:
1. **RED**: 跑全测试套件，统计 FAIL 数（基线）
2. **GREEN**: 修复所有 FAIL（不应有）
3. **REFACTOR**: 更新 `docs/roadmap-zh.md` P0 表所有项改为 ✅
4. **VERIFY**: `cargo test --workspace` 全绿；`cargo clippy --workspace` 零警告；examples 34/34；91 个 #[ignore] 中至少 42 个已 PASS
5. **COMMIT**: `docs: update roadmap-zh P0 status to all green`

---

### T-4.2: release-archivist 流程

**TDD Steps**:
1. **RED**: 检查 `.spec-superflow.yaml` state 非 `closing`
2. **GREEN**: 走 DP-7 归档确认
3. **REFACTOR**: spec-merger 把 delta specs 合入主 spec
4. **VERIFY**: archive readiness check 通过
5. **COMMIT**: `chore(release): archive v0.5.5-residual-fixes`

---

## Dependency Graph Summary

```
Batch 1.1 (GC flag)
  ├─ T-1.1.1 ──┬─ T-1.1.2 ──┬─ T-1.1.4 ──┬─ T-1.3.1 ─┬─ T-1.3.2
  │            │            │            │           ├─ T-1.3.3
  │            └─ T-1.1.3 ──┘            │           
  │                                      │
  └─ T-1.1.3 ── T-1.2.1 ── T-1.2.2 ──────┘
                                       │
Batch 1.3 (T9 + stdlib)                │
  T-1.3.1 ── T-1.3.2                  │
  T-1.3.1 ── T-1.3.3                  │
                                       │
Batch 1.4 (trait)                      │
  T-1.4.1 ── T-1.4.2 ── T-1.4.3      │
                                       │
Batch 2.1 (await)                      │
  T-2.1.1 ── T-2.1.2 ── T-2.1.3 ──┐
  T-2.1.1 ─────────────────── T-3.1
                                   │
Batch 2.2 (landing pad)             │
  T-2.2.1 ── T-2.2.2 ── T-2.2.3 ──┤
                                   │
Batch 3 (spawn)                    │
  T-3.1 ── T-3.2                  │
  T-3.1 ── T-3.3                  │
                                   ↓
Batch 4 (verify + archive)
  T-4.1 → T-4.2
```

## Validation Checklist

- [x] `## File Structure` — 14 个新文件 + 18 个修改文件
- [x] `## Interfaces` — 6 个跨批次接口契约
- [x] `## Per-task` — 每个任务含文件路径、TDD 5 步、Interfaces、依赖
- [x] `## Granularity` — 每个 TDD 步骤 2-5 分钟可完成
- [x] `## Zero placeholders` — 无 TBD/TODO/"figure out"/"add appropriate"
- [x] `## Dependency ordering` — 显式 "Depends on: Batch N"
- [x] `## Cross-Batch Consumes/Produces` — 7 对接口依赖

完整文卷已存于 `changes/v0.5.5-residual-fixes/tasks.md`。