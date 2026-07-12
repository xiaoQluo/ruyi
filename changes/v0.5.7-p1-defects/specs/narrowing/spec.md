# narrowing — typechecker 3.4

## ADDED Requirements

### Requirement: if 分支正向收窄为非空类型

当一个值为 `T?`（可空类型）且类型检查器在 `if (x !== null)` / `if (x !== null)` 形式判断后进入 `if` 分支时，系统 SHALL 将该值的类型在 `if` 分支内收窄为 `T`，允许直接调用 `T` 的非空方法而无需 `!` 操作符。

#### Scenario: if 分支中可空字符串收窄为 string

- **WHEN** 源码中出现 `let s: string? = maybe(); if (s !== null) { print(s.length); }`
- **THEN** 在 `if` 分支内 `s` SHALL 被收窄为 `string`，访问 `.length` SHALL 通过编译，不要求 `s!`

#### Scenario: 收窄仅在条件成立的分支内生效

- **WHEN** 源码中可空值 `x` 在 `if (x !== null)` 分支内调用 `x.method()`，同一作用域的 `else` 分支同时存在
- **THEN** 仅 `if` 分支内 `x` 的类型 SHALL 被收窄；`else` 分支内 `x` SHALL 仍为 `T?` 类型

### Requirement: else 分支反向收窄为可空类型

当一个值在 `if` 分支中被正向收窄为非空类型 `T` 时，系统 SHALL 在 `else` 分支中将其类型反向收窄为 `T?`，从而使 `else` 分支可以安全地观测该值的可空性并使用 `!` 等操作符显式处理。

#### Scenario: else 分支中可空值回归可空类型

- **WHEN** 源码中出现 `if (x !== null) { use(x); } else { x!.method(); }`
- **THEN** 在 `else` 分支中 `x` SHALL 被识别为 `T?` 类型，允许 `!` 操作符再次断言

#### Scenario: 反向收窄不与正向收窄冲突

- **WHEN** 同一可空变量在 `if`/`else` 两侧均被使用
- **THEN** `if` 分支中 `x` SHALL 为 `T`，`else` 分支中 `x` SHALL 为 `T?`，两侧独立收窄 SHALL 不互相污染

### Requirement: 收窄在 instanceof / typeof / match 之后生效

除可空检查外，系统 SHALL 在 `instanceof` 类型测试、`typeof` 字面量类型检查以及 `match` 模式匹配成功之后同样进行类型收窄，使对应分支内可访问子类型专有成员。

#### Scenario: instanceof 收窄到具体类

- **WHEN** 源码中出现 `if (animal instanceof Dog) { animal.bark(); }`
- **THEN** 在 `if` 分支内 `animal` SHALL 被收窄为 `Dog`，`bark()` SHALL 通过编译

#### Scenario: match 模式收窄到联合分支

- **WHEN** 源码中出现 `match (shape) { Circle(c) => ..., Rect(r) => ... }`
- **THEN** 在 `Circle(c)` 分支中 `c` SHALL 被收窄为 `Circle` 类型，可访问 `Circle` 字段

#### Scenario: 收窄失败时不静默回退

- **WHEN** 收窄尝试中类型推断失败（例如 `dyn` 值上的 instanceof 不可判定）
- **THEN** 系统 SHALL 报类型错误而不是将变量静默回退到 `dyn`，错误 SHALL 指出无法收窄的具体原因

### Requirement: 收窄在循环体内单次有效

收窄 SHALL 仅在产生收窄判断的作用域内有效；进入新循环或新作用域后 SHALL 不携带上一分支的收窄结论。

#### Scenario: 循环外收窄不会泄漏

- **WHEN** 源码中 `if (x !== null) { ... }` 之后出现 `for (...) { use(x); }`
- **THEN** 循环体内 `x` SHALL 仍为原始声明类型 `T?`，不应沿用 `if` 分支的 `T` 收窄