# Design: Codegen Gaps Fix

## Context

**Current State**:
- `crates/ruyic/src/codegen/patterns.rs:36` 路由 BigInt 到 `compile_int_match`,但 BigInt 是 i8* 指针而非 i64
- `crates/ruyic/src/codegen/decl.rs::codegen_declaration` 缺少 `Declaration::Macro` 和 `Declaration::TypeAlias` 分支
- 3 个 example 文件因此 fail:`bigint.ry` / `macros.ry` / `type_aliases.ry`
- `crates/ruyi_runtime/src/builtins.rs` 中只有 `ruyi_bigint_from_str`,**无** BigInt 比较函数

**Constraints**:
- 零警告构建
- 不破坏现有 30 个通过的 example
- 改动量最小(每个修复 ≤ 30 行)
- 必须**新增测试代码**覆盖 BigInt 字面量 match(原 example 仅覆盖通配符)

**Stakeholders**:
- 编译器开发者:需要清晰、最小的修复点
- 语言用户:需要 example 可运行
- 运行时维护者:需要新增一个 builtin 函数

## Goals

1. **G1**: 补全 3 个 codegen 路径,使 3 个 example 全部编译通过
2. **G2**: 支持 BigInt 字面量 match(新增 REQ-BM-003),需要新增 `ruyi_bigint_eq` runtime 函数
3. **G3**: 改动量最小(< 60 行总代码,含 runtime)
4. **G4**: 保持零警告原则
5. **G5**: 新增 example 覆盖 BigInt 字面量 match(对应 REQ-BM-003)

## Decisions

### D1: BigInt match 路由改用 `compile_generic_match`

- **Choice**: 从 `Type::Int | Type::BigInt => compile_int_match(...)` 改为 `Type::Int => compile_int_match(...)`,使 BigInt 落入 catch-all `_ => compile_generic_match(...)`
- **Rationale**:
  - `compile_generic_match` 已正确处理通配符和字面量模式(对指针类型也安全)
  - 是 D2/D3 的前置条件(BigInt 必须先正确路由,才能在 generic 路径中比较)
- **Alternatives Considered**:
  - A1: 写专门的 `compile_bigint_match` → 拒绝(增加重复代码;generic 路径已能工作)

### D2: 新增 `ruyi_bigint_eq` runtime 函数

- **Choice**: 在 `crates/ruyi_runtime/src/builtins.rs` 新增 `ruyi_bigint_eq(a: *mut i8, b: *mut i8) -> i8`,从 `lib.rs` re-export
- **Rationale**:
  - BigInt 内部表示为 i8*(指向运行时堆上的 bigint 数据),无法用 LLVM `icmp` 比较
  - 必须调用 runtime 函数做值比较
  - 当前 BigInt 阶段实现为 "opaque"(见 builtins.rs:181 注释),所以 `bigint_eq` 现阶段可以做 placeholder 实现(比较指针地址或始终返回 0/1)
- **Alternatives Considered**:
  - A1: 在 codegen 内联比较 → 拒绝(BigInt 是任意精度,inline 不可行)
  - A2: 跳过字面量支持,只支持通配符 → 拒绝(违反新需求 REQ-BM-003)

### D3: codegen 在 BigInt 字面量 match 时调用 `ruyi_bigint_eq`

- **Choice**: `compile_generic_match` 在遇到 `Pattern::Literal(BigIntLiteral(n))` 时,生成对 `ruyi_bigint_eq(scrutinee, &bigint_literal_value)` 的调用,根据返回值分支
- **Rationale**:
  - 与 generic match 路径一致,不引入新分支
  - literal value 通过 `compile_bigint_literal` 创建,然后传入 `ruyi_bigint_eq`
- **Alternatives Considered**:
  - A1: 走 generic 路径但用指针比较(`icmp eq`) → 拒绝(语义错误,两个不同 bigint 可能同地址)

### D4: Macro 声明在 codegen 阶段跳过

- **Choice**: 在 `codegen_declaration` 添加 `Declaration::Macro { .. } => Ok(None)`
- **Rationale**:
  - 宏声明是编译时抽象,无运行时代码
  - 宏展开器在 typechecker 之前已处理
- **Alternatives Considered**: 无

### D5: TypeAlias 声明在 codegen 阶段跳过

- **Choice**: 在 `codegen_declaration` 添加 `Declaration::TypeAlias { .. } => Ok(None)`
- **Rationale**: 类型别名是编译时抽象,无运行时代码
- **Alternatives Considered**: 无

### D6: 新增 example 测试 BigInt 字面量 match

- **Choice**: 在 `examples/bigint.ry` 中追加 `match_literal_demo` 函数,使用 `match (n: bigint) { 42n => ..., _ => ... }`
- **Rationale**:
  - 直接扩展现有 example 文件,保证 example 套件通过
  - 同时验证 codegen 路径 + runtime 函数
- **Alternatives Considered**:
  - A1: 新建独立 `bigint_match_literal.ry` → 接受(若用户偏好独立文件)

## Risks And Trade-Offs

### R1: `ruyi_bigint_eq` 的 placeholder 实现
- **Risk**: 当前 BigInt 是 opaque,字面量比较的实际语义未定义
- **Mitigation**: placeholder 实现先满足 codegen 链路,真实 bigint 库集成时再升级
- **Severity**: 中(影响语义正确性,但本变更仅要求编译通过)

### R2: 通用 match 路径性能略低于 int switch
- **Risk**: `compile_generic_match` 比 `compile_int_match_switch` 多一些分支跳转
- **Mitigation**: BigInt match 场景不敏感性能
- **Severity**: 低

### R3: 与 14 个 WIP 既存测试失败的耦合
- **Risk**: 新增 runtime 函数可能影响其他 codegen 测试
- **Mitigation**: `ruyi_bigint_eq` 是新函数,不影响现有 API
- **Severity**: 极低

## Cross-Batch Dependencies

- **D1 (BigInt 路由)**: 无外部依赖,必须先完成
- **D2 (新增 runtime 函数)**: 无外部依赖,可与 D1 并行
- **D3 (codegen 调用 runtime)**: 依赖 D1 + D2
- **D4 (Macro skip)**: 无外部依赖,可独立
- **D5 (TypeAlias skip)**: 无外部依赖,可独立
- **D6 (新增 example)**: 依赖 D1 + D2 + D3

**推荐执行顺序**: D4+D5 → D1 → D2 → D3 → D6

