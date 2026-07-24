# Design: fix-batch-low-risk-defects

## Context

**Current State**: Ruyi compiler v0.5.9。缺陷审查识别出 4 类低风险问题：allow_partial_codegen 全局覆盖、3 个测试断言错误、12 条 codegen "not yet supported" 路径、路线图文档过时。

**Stakeholders**: Ruyi 编译器开发者、使用 v0.5.9 编译器的用户。

**Constraints**:
- LLVM 14 绑定 (inkwell llvm14-0 feature)
- Rust 2021 edition, workspace resolver = "2"
- clippy 零警告原则
- Javadoc 风格注释保留
- 不引入新的外部依赖

**Related Changes**: `fix-exception-unwinder`（独立 Change A）处理异常 unwinder（优先级 1）。

## Goals

1. `allow_partial_codegen` 对用户代码的 codegen 错误不再静默吞没
2. 3 个类型检查器测试断言正确，测试通过
3. 12 条 codegen 路径从 `Err("...not yet supported")` 升级为功能实现
4. 路线图文档反映 v0.5.9 真实状态

## Decisions

### D1: allow_partial_codegen Scoping

**Choice**: 在 driver 层记录 stdlib 项数量，传递给 CodeGenerator，codegen 时按项索引判断是否为 stdlib 项。

**Rationale**: 
- 驱动已明确知道哪些 ModuleItem 来自 stdlib（`auto_load_stdlib` 将它们 prepend 到程序项之前）
- 不修改 AST 类型或模块系统——仅在 codegen 入口传递一个 `usize` 计数值
- 非侵入式，实现成本最低

**Alternatives considered**:
- *Per-AST-node source tracking*: 需要修改所有 AST 类型添加 `source_path` 字段，侵入性太强，对于一个标志来说过度设计
- *Set to `false` and fix stdlib source*: 理论上最清洁，但无法保证 stdlib 在所有 edge case 下都不会触发 codegen 错误（例如 future stdlib 扩展），引入技术债务
- *Module-level tracking via resolver*: 需要在 ModuleResolver 中维护来源信息，增加模块系统复杂度

### D2: Anonymous Function Codegen

**Choice**: 复用现有箭头函数编译路径。匿名函数（`fn(x) { body }`）编译为自动命名的函数 `__anon_{counter}`，与箭头函数 `__arrow_{counter}` 平行。

**Rationale**:
- 箭头函数 codegen 已完整实现，匿名函数语义上等价于非 async 的箭头函数
- 命名约定（`__anon_` vs `__arrow_`）区分来源，便于调试
- 无新增 LLVM IR 模式，风险低

**Alternatives considered**:
- *独立编译路径*: 与箭头函数分叉实现，但语义相同会导致代码重复
- *lambda lifting*: 过早优化，当前无闭包捕获场景

### D3: Async Arrow Function Codegen

**Choice**: 编译为具名异步函数 `__async_arrow_{counter}`，复用现有 async function codegen 基础设施。

**Rationale**:
- `async fn` codegen 已存在（`async_codegen.rs`、work-stealing scheduler）
- 关键差异：异步箭头函数无显式名称 → 自动命名

**Alternatives considered**:
- *降级为同步 + spawn*: hack，破坏 async 语义
- *直接返回 Future 字面量*: 当前无 Future 字面量运行时支持

### D4: Nested Member Access

**Choice**: 递归编译成员访问链。将 `a.b.c` 展开为连续 GEP + load 操作：先编译 `a.b` 得到 ptr + type，再编译 `.c`。

**Rationale**:
- LLVM 的 GEP（GetElementPtr）本就支持链式偏移计算
- 已有单层成员访问的完整实现，扩展为递归即可

**Alternatives considered**:
- *展平为单次 GEP*: 需要 typechecker 传递完整的嵌套类型信息，跨层耦合

### D5: Indirect Calls

**Choice**: 在 `compile_call` 的非直接调用路径中，对非 `Expr::MemberCall` 的 callee 表达式先编译得到函数指针，然后通过 `build_indirect_call` 调用。

**Rationale**:
- 整体设计思路已在函数调用路径中有雏形（`:2334` 处的 `_ => return Err("Indirect calls not yet supported")`）
- 直接调用的函数指针 (`func_ptr`) 与间接调用的位转换模式完全兼容

**Alternatives considered**:
- *引入 vtable dispatch*: 不适用于函数变量场景，过度设计

### D6: Spread Arguments

**Choice**: 对于 4 处 spread arguments 路径（函数调用 ×2 + 构造器 + super 构造器），统一实现：识别 `Argument::Spread(array_expr)`，编译后获取数组指针，调用 `__builtin_array_length` + `__builtin_array_get` 逐元素解包追加到 args 列表。

**Rationale**:
- `__builtin_array_length` / `__builtin_array_get` 已在 codegen 中声明和可用
- 4 处路径的展开逻辑完全一致，可提取公共函数 `unpack_spread_args`

**Alternatives considered**:
- *Runtime spread*: 需要修改函数调用 ABI，影响范围大
- *限制为字面量数组*: 过于限制，用户无法使用变量数组展开

### D7: Compound Assignment

**Choice**: 读-运算-写 (load-operate-store) 模式。对于 `x += expr`：先 load `x` 的值，编译 `expr`，执行运算（使用现有二元运算 codegen），store 结果回 `x`。

**Rationale**:
- 现有二元运算 codegen（`compile_binary_op`）已覆盖 `+`、`-`、`*`、`/`、`%`
- 完全复用现有逻辑，仅需在赋值入门前插入 load + op + store 序列

**Alternatives considered**:
- *LLVM atomicrmw*: 适合并发场景，但 Ruyi 单线程执行无需 atomic 语义

### D8: Complex Assignments

**Choice**: 扩展 `compile_assign` 的 `left` 匹配分支。当前仅支持 `Identifier` 和 `MemberAccess`，扩展到 `IndexAccess`（`arr[i] = val`）。

**Rationale**:
- IndexAccess 的 codegen 已在读取路径实现（`compile_expr` 中 IndexAccess 分支）
- 写入仅需反向操作：获取数组 ptr + 索引 + 值 → `__builtin_array_set`

**Alternatives considered**:
- *全量左值 (lvalue) 系统*: 需要重构 codegen 的类型系统，scope 过大

### D9: Complex New Expressions

**Choice**: 扩展 `compile_new` 以支持非标识符 callee。先编译 callee 表达式得到类类型，再按标准 new 流程分配+构造。

**Rationale**:
- 解除了 `throw Error.new(...)` 的编译阻断（此前需 `RangeError.new` 这种简单标识符形式）
- 与 stdlib 的 `Error.new("msg")` 模式对齐

**Alternatives considered**:
- *仅支持 MemberAccess 形式*: 能覆盖 80% 场景，但限制灵活性

### D10: Complex Pattern Binding

**Choice**: 在 `compile_binding` 中为 `Pattern::Array` 和 `Pattern::Object` 新增分支。数组解构：编译右侧表达式，迭代子模式，生成 `__builtin_array_get` 调用。对象解构：编译右侧表达式，为每个字段生成 class_field_access + load。

**Rationale**:
- 数组/对象 codegen 已有成熟实现，解构是其自然的写入侧扩展
- 不引入新的 LLVM IR 模式

**Alternatives considered**:
- *Defer to typechecker rewrite*: typechecker 已完整支持模式匹配，codegen 缺位是遗留问题，不应继续延期

### D11: Roadmap Update Strategy

**Choice**: 同步更新 `roadmap.md` 和 `roadmap-zh.md`，标记以下已完成项：
- v0.5.5: try-catch/finally codegen (1.7)、throw expression (1.8)、模板字面量 (1.10)、RangeError/ArrayIterator 构造化、21 codegen 测试启用
- v0.5.6: 代码生成文档漂移修复
- v0.5.7: 12 项 P1 缺陷（typechecker 3.2/3.4/3.5/3.6 + runtime 2.6 + stdlib 4.5/4.6/4.8/4.9）
- v0.5.8: stdlib core（math/time/json FFI wiring）
- v0.5.9: stdlib cleanup (R1-R5)、class codegen

更新 Current State Assessment 中各模块完成度（codegen 从 75% 提升至 82% 反映 12 条新路径）。

**Rationale**: 路线图是用户和开发者了解项目状态的第一入口，过时的路线图产生误导。

### D12: Test Fix Strategy

**Choice**: 直接应用 `pending-issues.md` 中记录的已知修复方案：
- `test_check_match_statement`: 更新断言匹配 v0.5 type checker 行为（具体行为需实地验证后确定修复）
- `test_bool_patterns_with_wildcard`: `assert!(result.has_redundancy)` → `assert!(!result.has_redundancy)`
- `test_from_annotation_generic`: 期望值从 `Generic{...}` → `Type::Array(Box::new(Type::Int))`

**Rationale**: 修复方案已由 Oracle Phase 1 审查确认，直接执行。

## Risks And Trade-Offs

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| 12 条 codegen 路径中某条触发 LLVM 类型不匹配 | Medium | 编译 panic | 每个路径用 `cargo test -p ruyic --test codegen` 逐一验证 |
| Spread arguments 对运行时数组操作依赖未声明函数 | Medium | 链接错误 | 验证 `__builtin_array_*` 在所有目标平台已声明 |
| allow_partial_codegen 条件化后用户代码报错增加 | High | 用户体验 | 这是预期行为——此前被隐藏的错误现在正确暴露；需确保错误信息清晰含文件位置 |
| 间接调用函数指针类型不匹配 | Low | SIGSEGV | 调用前 `build_bitcast` 对齐函数指针类型 |
| 路线图更新遗漏某项 | Low | 文档不完整 | 对照 git log v0.5.4..HEAD 中所有 feat/fix commit 逐一确认 |

### Trade-off: 实现深度 vs 优雅性

复合赋值和 spread 参数的"读-运算-写"和"解包-追加"模式在 LLVM IR 层面是正确的，但在语言语义层面有简化：
- 复合赋值的左值只求值一次（符合 JS 语义，因为 Ruyi 无 getter/setter）
- Spread 参数的求值顺序为从左到右（符合 JS 语义）

这些简化在当前版本是可接受的，因为 Ruyi 无副作用 getter/Proxy 等机制。
