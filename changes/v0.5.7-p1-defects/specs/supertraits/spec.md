# supertraits — typechecker 3.2

## ADDED Requirements

### Requirement: Supertrait declaration is parsed and stored

解析器与类型检查器 SHALL 将 `trait A: B` 中 `B` 之后列出的每个标识符视为 `A` 的超 trait，并在 `TraitDecl` 的 `supertraits` 字段中保留其有序列表。

#### Scenario: 单个超 trait 被记录

- **WHEN** 源码中出现 `trait Printable: Comparable { fn to_string(self): string; }`
- **THEN** `Printable` 的 `supertraits` 字段 SHALL 包含一个元素 `Comparable`，且实现 `Printable` 的类型必须同时实现 `Comparable`

#### Scenario: 多个超 trait 按声明顺序保留

- **WHEN** 源码中出现 `trait Foo: Bar + Baz { ... }`
- **THEN** `Foo.supertraits` SHALL 依序为 `[Bar, Baz]`，顺序与源码一致

#### Scenario: 无超 trait 时字段为空列表

- **WHEN** 源码中出现 `trait Standalone { fn run(self): void; }`
- **THEN** `Standalone.supertraits` SHALL 为空列表 `[]`，不应为 `null` 或缺失字段

### Requirement: validate_impls 沿超 trait 链收集全部方法

`validate_impls` SHALL 通过 `collect_all_super_methods` 沿超 trait 链向上回溯，将所有继承的方法视为目标 trait 的一部分，从而允许实现类只显式实现部分方法而通过超 trait 满足其余方法。

#### Scenario: 继承方法判定为已实现

- **WHEN** `trait A { fn a(self); }` 且 `trait B: A { fn b(self); }`，同时类型 `T` 仅显式实现 `B`
- **THEN** 类型检查器 SHALL 判定 `T` 同时满足 `A` 与 `B`，且调用 `t.a()` 与 `t.b()` 均能通过编译

#### Scenario: 缺失必要方法时报错

- **WHEN** `trait B: A { fn b(self); }`，类型 `T` 仅实现 `B` 而未实现 `A`
- **THEN** 类型检查器 SHALL 在 `validate_impls` 阶段报错，指出缺失的方法 `a`，并给出 `B` 通过 `A` 传递依赖该方法的诊断

### Requirement: 超 trait 链中的循环产生编译期错误

类型检查器 SHALL 检测超 trait 链中的循环依赖，并在编译期报错而不是让运行时崩溃或无限递归。

#### Scenario: 直接自循环

- **WHEN** 源码中出现 `trait Loop: Loop { ... }`
- **THEN** 类型检查器 SHALL 输出编译错误，错误信息 SHALL 包含字符串 `cycle` 与重复出现的 trait 名称

#### Scenario: 间接循环 A → B → A

- **WHEN** 源码中出现 `trait A: B { ... }` 与 `trait B: A { ... }`
- **THEN** 类型检查器 SHALL 输出编译错误，错误信息 SHALL 同时列出 `A` 与 `B` 以指出循环参与者

#### Scenario: 循环发生在超 trait 链的更深层

- **WHEN** 源码中出现 `trait P: Q`、`trait Q: R`、`trait R: P` 的三节点循环
- **THEN** 类型检查器 SHALL 仍能识别循环并报错，错误 SHALL 列出全部三个 trait 而不仅报出最早发生冲突的那一对

### Requirement: 缺失超 trait 实现的错误指向具体的 supertrait

当一个 trait 的某个超 trait 未被满足时，错误信息 SHALL 指出是哪一个超 trait 缺失实现，而不是仅说主 trait 未实现。

#### Scenario: 错误信息包含 supertrait 名称

- **WHEN** `trait Composite: Serializable`，类型 `T` 实现 `Composite` 但未实现 `Serializable`
- **THEN** 编译错误信息 SHALL 形如 `trait Serializable not implemented for T (required by Composite)`，并包含 `Serializable` 字面量