# exhaustiveness — typechecker 3.5

## ADDED Requirements

### Requirement: 联合类型 match 缺臂时产出诊断

当 `Expr::Match` 的目标类型是 `Type::Union` 而某些 Variant 缺少对应 arm 时，类型检查器 SHALL 为每一个缺失的 Variant 产出诊断，文本 SHALL 列出未覆盖的构造子名称。

#### Scenario: 三变体联合只匹配两支

- **WHEN** 源码中出现 `match (color: Red|Green|Blue) { Red => ..., Green => ... }`
- **THEN** 类型检查器 SHALL 产出诊断，文本 SHALL 包含缺失的 `Blue`

#### Scenario: 嵌套 Variant 在缺臂诊断中保留路径

- **WHEN** 源码中出现 `match (msg: Ok(int)|Err(string)|Pending)` 且仅覆盖 `Ok` 与 `Err`
- **THEN** 类型检查器 SHALL 产出诊断，文本 SHALL 列出 `Pending`

### Requirement: 通配 arm 抑制缺臂诊断

当 `match` 包含 `_` 形如通配 arm 时，类型检查器 SHALL 不再为剩余 Variant 产出缺臂诊断，因为 `_` 已表达对未列举情况的兜底意图。

#### Scenario: 通配 arm 完全抑制诊断

- **WHEN** 源码中出现 `match (color: Red|Green|Blue) { Red => ..., _ => ... }`
- **THEN** 类型检查器 SHALL 不为 `Green` 与 `Blue` 产出任何缺臂诊断

#### Scenario: 命名变体仍触发诊断但 _ 不触发

- **WHEN** 源码中出现 `match (color: Red|Green|Blue) { Red => ..., Green => ..., _ => ... }`
- **THEN** 类型检查器 SHALL 不产出任何缺臂诊断，`_` 已覆盖剩余的 `Blue`

### Requirement: missing_arms 返回缺失 Variant 列表

`Type::Union::missing_arms()` SHALL 返回一个有序的 Variant 名称列表，列出对给定 match 表达式而言未覆盖的 Variant，供诊断文本与未来 IDE 集成复用。

#### Scenario: 缺两支返回两元素列表

- **WHEN** 类型检查器在 `match (color: Red|Green|Blue) { Red => ... }` 上调用 `Type::Union::missing_arms()`
- **THEN** 该方法 SHALL 返回 `["Green", "Blue"]`，顺序与 `Type::Union` 中 Variant 声明顺序一致

#### Scenario: 完全覆盖时返回空列表

- **WHEN** 类型检查器在覆盖所有 Variant 的 match 表达式上调用 `Type::Union::missing_arms()`
- **THEN** 该方法 SHALL 返回空列表 `[]`，调用方据此判定无需产出诊断

### Requirement: 诊断保持 warning 级以维持向后兼容

为不破坏现有依赖不完整 match 的代码，本变更 SHALL 将缺臂诊断设为 warning 级而非 error 级，编译器 SHALL 仍生成可执行产物。

#### Scenario: warning 不阻塞产物生成

- **WHEN** 源码中存在缺臂 match 表达式
- **THEN** 编译器 SHALL 继续生成可执行二进制，stderr SHALL 输出 warning 行，不返回非零 exit code

#### Scenario: warning 文本稳定可解析

- **WHEN** 编译器产出缺臂 warning
- **THEN** warning 文本 SHALL 形如 `warning: non-exhaustive match: missing variants Blue`，便于 CI 与工具解析