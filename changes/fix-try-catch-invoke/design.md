# Design: Try/Catch Invoke 修复与 LandingPad 架构重构

## Context

**Current State**:
- `crates/ruyic/src/codegen/stmt.rs:245-378` 的 `compile_try` 用 `build_call` 调用 try 体内部函数,不生成 LLVM `invoke` 指令
- `crates/ruyic/src/codegen/stmt.rs:185-242` 的 `compile_throw` 在调用 `ruyi_throw` 后未追加 `unreachable`,可能造成 IR 不一致
- `crates/ruyic/src/codegen/expr.rs:889` 的 `compile_call` 始终用 `build_call`,不知道 try 上下文
- `LandingPadGenerator` 仅存在于 `crates/ruyi_runtime/src/exception/landing_pad.rs`,藏在 `#[cfg(feature = "inkwell")]`,`ruyic` 无法访问
- `CodegenContext` 当前没有 `try_stack`/`in_try_block` 字段,无法让 callee 函数判断当前 try 上下文
- CI commit `72fd843` (2026-07) 临时移除了 ci.yml 因为 LLVM 14 runner 不可用;codegen 集成测试长期 `#[ignore]`
- 已有 `TRY_CATCH_AUDIT.md` (2026-05-04) 锁定根因,但尚未落地修复

**Constraints**:
- 零警告编译(AGENTS.md §零警告原则)
- 不破坏现有 33/34 通过的 example 套件,逆向回归测试
- LLVM 14 inkwell API 在 `compile_try`/`compile_throw`/`compile_call` 处须正确调用
- `ruyi_await` 空操作、`allow_partial_codegen` 等其他 P0 遗留项**不在本次范围**(per DP-0)
- codegen 集成测试用 `#[ignore]`,CI 不需 LLVM runner
- 无 LLVM 环境下 `cargo check -p ruyi_runtime --no-default-features` 仍须通过

**Stakeholders**:
- 编译器开发者:需要清晰的 try/catch 实现,架构合理、不引入 unsafe 滥用
- 语言用户:try/catch 必须能捕获实际函数抛出的异常,examples/try_catch_invoke.ry 端到端验证
- 运行时维护者:LandingPadGenerator 位置变更影响 `ruyi_runtime::exception` re-export

## Goals

1. **G1**: try/catch 端到端正确捕获被调用函数抛出的异常(兑现 roadmap §阶段一成功标准第 2 条)
2. **G2**: try_stack 正确追踪嵌套 try 块,unwind 目标定位至最内层 catch
3. **G3**: LandingPadGenerator 通过 shared crate `ruyi_exception` 同时服务 ruyic 与 ruyi_runtime,单一实现
4. **G4**: `compile_throw` 加 unreachable,基本块正确终止,避免错误 PHI
5. **G5**: 零警告原则,所有 cargo check/clippy 干净
6. **G6**: examples 套件 33 → 34 通过,无回归

## Decisions

### D1: `compile_try` 改用 `build_invoke` 并由 `LandingPadGenerator::build_catch_dispatch` 处理 dispatch

- **Choice**: 重写 `compile_try`(stmt.rs:245-378),在 try 体函数调用处改用 `build_invoke`,catch 块起始处构造 `landingpad` 指令,selector 匹配由 `LandingPadGenerator::build_catch_dispatch` 统一完成。
- **Rationale**:
  - LLVM 异常处理语义要求 try 内函数调用用 `invoke` 才能路由到 landingpad
  - `LandingPadGenerator` 已实现完整接口(7 个方法),只需把它搬到 ruyic 可达位置
  - 单点改造,try/catch 上下文语义统一由 `LandingPadGenerator` 负责
- **Alternatives Considered**:
  - A1: 在 ruyic 内重写 invoke 生成逻辑 → 拒绝(代码重复,LandingPadGenerator 已存在)
  - A2: 仅手动管理 try_stack(沿用旧实现) → 拒绝(本变更目标即淘汰手动 try_stack)

### D2: 新建 `crates/ruyi_exception` shared crate 承载 `LandingPadGenerator`

- **Choice**: 新建 `crates/ruyi_exception/src/{lib.rs, landing_pad.rs}`,把 `crates/ruyi_runtime/src/exception/landing_pad.rs` 的内容搬入(并改造 `TypeId` 为泛型或整数型,断绝与 ruyi_runtime 特定类型的依赖)
- **Rationale**:
  - ruyic 与 ruyi_runtime 都需访问 `LandingPadGenerator`
  - shared crate 是 workspace 成员的标准做法(Rust 2021 edition 兼容)
  - 单一实现,无重复代码
  - inkwell 依赖仍在 shared crate 内,通过 feature gate 管理(参考 fix-codegen-gaps 的 R1 处理模式)
- **Alternatives Considered**:
  - A1: 把 `LandingPadGenerator` 移到 `ruyic/src/codegen/landing_pad.rs`(ruyic 独占) → 拒绝(让 ruyi_runtime 反向依赖 ruyic,违反分层)
  - A2: 让 `ruyi_runtime` 反向导出 `LandingPadGenerator` 给 `ruyic` → 拒绝(违反依赖方向,编译顺序将 ruyi_runtime 排在 ruyic 之前)
  - A3: 拆分为独立 module path 但仍位于 ruyi_runtime(#[cfg] 切换) → 拒绝(脆弱,易破坏 feature 配置)

### D3: `CodegenContext` 用 `try_stack: Vec<TryFrame>` 而非单 `in_try_block: bool`

- **Choice**: `CodegenContext` 新增字段 `try_stack: Vec<TryFrame>`,其中 `TryFrame { catch_bb: Option<BasicBlock>, finally_bb: Option<BasicBlock>, exception_ptr: Option<PointerValue> }`。`compile_try` 进入时 push,所有出口(normally exit、catch 跳、finally 跳)统一 pop。
- **Rationale**:
  - 嵌套 try 需要栈式结构(REQ-TCI-004 强制要求)
  - catch_bb/finally_bb 信息为 `compile_call` 在 try 内生成 invoke 时所需的 unwind 目标
  - 一次性入栈,callee 不需感知上下文深度,语义最简
- **Alternatives Considered**:
  - A1: `in_try_block: bool` + 当前 try 帧独立字段 → 拒绝(嵌套场景下丢失外层 try 信息)
  - A2: LLVM `personality` 函数 + 语言级 try 标记 → 拒绝(改动面过大,超出本次范围)

### D4: `compile_call` 在 try 内对**所有**函数调用生成 `invoke`,而非限制于 throw 标记的函数

- **Choice**: `compile_call`(expr.rs:889) 改写为: 若 `ctx.try_stack` 非空且栈顶 `catch_bb`/`landing_bb` 存在,则用 `build_invoke`,unwrap 目标设置为栈顶 catch 的 landingpad；否则用 `build_call`(零回归)。
- **Rationale**:
  - 调用方无需提前知道被调用函数是否会抛,LLVM 在运行时通过 personality 函数决定
  - 简化逻辑:不需在函数声明上标注 `can_throw`
  - 性能开销仅出现在 try 范围内(常见程序极小)
  - 与 REQ-TCI-003、REQ-TCI-004 一致
- **Alternatives Considered**:
  - A1: 仅对 `throw` 标记的函数生成 `invoke` → 拒绝(语义不全,运行时无 throw 标记)
  - A2: 仅对 RValue context 的 call 生成 invoke → 拒绝(语义混乱,失去 catch 能力)

### D5: `compile_throw` 加 `unreachable` 后**继续**执行现有 `try_stack` 跳转

- **Choice**: 在 `ctx.builder.build_call(throw_fn, ...)` 之后,若 `ctx.try_stack.last().is_some()`,则继续执行原 try_stack 跳转逻辑(catch/finally/merge);但在跳转之后立即 `build_unreachable()`。若无 try 上下文,直接 `build_unreachable()`。
- **Rationale**:
  - noreturn 函数(`ruyi_throw`)调用后必须有 unreachable 终止基本块,避免 PHI 错乱
  - 现有 try_stack 跳转实现正确,无需重写
  - 改动最小,行为清晰
- **Alternatives Considered**:
  - A1: 完全移除 try_stack 跳转,改用 LLVM landingpad 异常处理 → 拒绝(超出本次范围,会改动 panic/catch 行为)

### D6: codegen 集成测试用 `#[ignore]`,文档说明 LLVM 依赖

- **Choice**: 新增 `crates/ruyic/tests/try_catch_invoke.rs`,所有测试函数标注 `#[ignore]`,运行命令 `cargo test -p ruyic --test try_catch_invoke -- --ignored`(需 LLVM 14)
- **Rationale**:
  - 与 fix-codegen-gaps 的 codegen 测试一致(`#[ignore]` 因 CI 无 LLVM)
  - 保留测试代码,本地有 LLVM 环境时可手动验证
  - 不破坏现有 CI(已临时移除 ci.yml)
- **Alternatives Considered**:
  - A1: 让测试不 `#[ignore]`,在 CI 安装 LLVM 14 → 接受为后续工作(超出本变更范围)
  - A2: 跳过 codegen 测试,只写 unit test → 拒绝(unit test 无法验证 invoke/landingpad LLVM IR)

### D7: 不重写整个 `LandingPadGenerator`,而是迁移并做最小适配

- **Choice**: 把 `crates/ruyi_runtime/src/exception/landing_pad.rs` 整体搬到 `ruyi_exception::landing_pad`,并把 `TypeId` 参数改为整数 type id(原本是 `ruyi_runtime` 内部定义的),避免 circular dep。仅在必要时微调 API 签名
- **Rationale**:
  - LandingPadGenerator 是已审计的成熟实现,7 个方法覆盖 build_landing_pad / build_invoke / build_resume / extract_exception_ptr / extract_selector / build_eh_typeid_for / build_catch_dispatch
  - 避免重新设计 API,降低风险
  - 适配 type id 为 i32/integer 是 trivial 改动
- **Alternatives Considered**:
  - A1: 重写 LandingPadGenerator → 拒绝(已存在代码,本变更专注调用方,不在 helper)
  - A2: 在编译期宏生成 wrapper → 拒绝(过度工程)

## Risks And Trade-Offs

### R1: LLVM invoke 指令生成错误的潜在面

- **Risk**: `build_invoke` 要求指定 normal_bb 与 unwind_bb,任何不一致都会导致编译失败或运行时崩溃
- **Mitigation**: 仅在 `compile_try` 边界内调用 `compile_call`,确保 unwind_bb = 当前 try 栈顶的 catch landing pad;为 `compile_try` 增加单元测试覆盖嵌套、finally、rethrow 场景
- **Severity**: 中(影响异常处理正确性,需严格测试)

### R2: ruyi_exception shared crate 的 LLVM feature gate 复杂度

- **Risk**: ruyi_runtime 的 `--no-default-features` 路径不应要求 LLVM;若 ruyi_exception 默认启用 inkwell 会破坏此路径
- **Mitigation**: ruyi_exception 用 `default-features = false` + `features = ["llvm14"]` 显式启用,ruyic 启用 `llvm14` feature,ruyi_runtime 不启用(继续走 opaque 调用)
- **Severity**: 中(影响多模式构建)

### R3: 与现有 try/catch example 的兼容性回归

- **Risk**: 切换到 invoke 后,LLVM IR 体积增加 2-5%,旧 binary 行为细微变化
- **Mitigation**: examples 套件全量回归(33 → 34),所有现有 try/catch example 行为保持(examples/io.ry, examples/error.ry 等)
- **Severity**: 低

### R4: TRY_CATCH_AUDIT.md 中"已知限制"的边界

- **Risk**: 本变更后,`compile_throw` 无 try_stack 上下文时仍有 `unreachable`,原"A. noreturn function should use unreachable"的限制已自动消除
- **Mitigation**: 更新 TRY_CATCH_AUDIT.md §3 (noreturn/return 处理),反映新行为
- **Severity**: 低

### R5: try_stack 内存泄漏或不一致 pop

- **Risk**: `compile_try` 在错误路径(expect failure 或 panic 之前的某分支)漏 pop try_stack,造成后续函数调用误判 try 上下文
- **Mitigation**: 用 RAII guard `struct TryStackGuard<'a>(&'a mut CodegenContext)` 包装 `push/pop`,确保任何早期返回都正确清理(借鉴现有 `push_gc_root_scope` 模式)
- **Severity**: 中(影响后续 codegen 正确性)

## Cross-Batch Dependencies

- **D4 (in-stack call → invoke)** 依赖 **D3** 的 `try_stack` 字段
- **D1 (compile_try 改写)** 依赖 **D3** + **D2** (LandingPadGenerator 可达)
- **D5 (compile_throw 加 unreachable)** 无外部依赖,可与 D1 并行
- **D6 (codegen 集成测试)** 依赖 **D1** + **D4** 完成
- **D7 (LandingPadGenerator 迁移)** 是 **D1** + **D2** 的前置

**推荐执行顺序**:
- **Wave 1 (基础设施)**: D7 → D3 (LandingPadGenerator 到位 + try_stack 数据结构)
- **Wave 2 (核心改造)**: D1 + D5 并行 (compile_try 重写 + compile_throw unreachable)
- **Wave 3 (调用方改造)**: D4 (compile_call try 上下文判断)
- **Wave 4 (验证)**: D6 (codegen 集成测试) + new example + regression check

**Critical Path**: D7 → D1 → D4 → D6
