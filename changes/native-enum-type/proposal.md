# Proposal: Native `enum` Type

> **状态**: 已规划 | **日期**: 2026-07-26 | **优先级**: P1
>
> **关联文档**: spec §17（枚举类型语义）

## 背景

Ruyi 当前的 `Option<T>` 和 `Result<T,E>` 使用 `class` + `type` 联合类型别名模拟：

```ruyi
class Some<T> { value: T; ... }
class None    { ... }
type Option<T> = Some<T> | None;
```

这种方式存在四个核心问题：

1. **大量重复代码**：`option.ry` 约 175 行，每个方法（isSome/unwrap/map/andThen 等）需在两个 class 中各实现一遍
2. **无穷尽性保证**：联合类型别名是开放的，编译器无法保证 match 覆盖所有变体
3. **match 无法自动解构**：不支持构造器模式 `Some(value) => ...`，只能用 `if-else` + `isSome()`/`unwrap()`
4. **内存布局不优**：每个 `Some(42)` 都需要 GC 堆分配，而原生 tagged union 可在栈上分配

## 目标

引入原生 `enum` 关键字和语义，实现：

- 封闭的变体集合（tagged union）
- 构造器模式匹配（`match opt { Some(v) => ... None => ... }`）
- 基于封闭变体的穷尽性检查
- 高效的 tagged union 内存布局（1 字节 tag + payload，无需堆分配）

## 语法设计

详见 spec-zh.md §17 枚举类型语义。

```ruyi
enum Option<T> {
    Some(T),
    None,
}

enum Result<T, E> {
    Ok(T),
    Err(E),
}

enum Json {
    Null,
    Bool(bool),
    Number(float),
    Str(string),
    Array(Array<Json>),
    Object(Map<string, Json>),
}
```

## 实现路线

### Phase 1：解析器

- 新增 `enum` 声明解析（变体列表 + 泛型参数）
- `match` 中新增构造器模式 `Variant(bindings...)`
- AST 节点：`Declaration::Enum { name, type_params, variants }`
- AST 模式：`Pattern::Constructor { name, args }`

**涉及文件**：
- `crates/ruyic/src/parser/ast.rs`
- `crates/ruyic/src/parser/parser.rs`

### Phase 2：类型检查器

- 枚举变体注册为类型构造器
- `match` 穷尽性检查基于 enum 的封闭变体集
- `if let` / `while let` 对 enum 构造器的类型缩窄
- 泛型单态化对 enum 的支持

**涉及文件**：
- `crates/ruyic/src/typechecker/inference.rs`
- `crates/ruyic/src/typechecker/generics.rs`

### Phase 3：代码生成

- 简单 enum → LLVM tagged union：`{ i8 tag, union payload }`
- 无数据变体（如 `None`）→ 仅 tag，零额外分配
- `match` → 基于 tag 的 `switch` + payload 解构
- 构造器调用 → 写入 tag + payload 到 struct

**涉及文件**：
- `crates/ruyic/src/codegen/stmt.rs`
- `crates/ruyic/src/codegen/expr.rs`
- `crates/ruyic/src/codegen/types.rs`

### Phase 4：stdlib 迁移

- `stdlib/option.ry`：`Some`/`None` class → `enum Option<T> { Some(T), None }`
- `stdlib/result.ry`：同上
- 验证所有使用 Option/Result 的 stdlib 模块仍然工作
- 更新相关示例文件

**涉及文件**：
- `stdlib/option.ry`
- `stdlib/result.ry`
- 其他引用 Option/Result 的 stdlib 模块

## 预期收益

| 指标 | 当前（class 模拟） | 引入 enum 后 |
|------|-------------------|------------|
| `option.ry` 行数 | ~175 行 | ~80 行（减少 ~54%） |
| `result.ry` 行数 | ~190 行 | ~80 行（减少 ~58%） |
| 构造器模式 match | 不支持 | 原生支持 |
| 穷尽性保证 | 无（开放联合） | 有（封闭 enum） |
| 内存布局 | 堆分配（GC） | tagged union（栈上） |
| 类型安全 | 中（任何 class 可混入） | 高（变体封闭） |

## 前置条件

- 解析器已接受 `Token::Enum`（已完成：`enum` 关键字已保留）
- spec §17 语法设计已定义（已完成）
- 现有 `Statement::Yield` 已移除（已完成：yield 特性已清理，不冲突）

## 依赖关系

- 不依赖其他未完成的语言特性
- 与 `delete` 限制、`yield` 移除正交
- 需要在代码生成模块有较深的改动（新增 tagged union 类型、构造器编译、match switch 编译）

## 风险

| 风险 | 影响 | 缓解 |
|------|------|------|
| 现有 Option/Result 用户代码破坏 | 中 | Phase 4 迁移时保持 API 兼容，方法签名不变 |
| LLVM tagged union 对齐问题 | 低 | 使用 LLVM `i8` tag + max-aligned payload，各平台对齐一致 |
| 与泛型单态化的交互复杂度 | 中 | 先实现非泛型 enum，再扩展泛型支持 |
| match 穷尽性检查的正确性 | 高 | 充分的单元测试覆盖各种遗漏模式 |
