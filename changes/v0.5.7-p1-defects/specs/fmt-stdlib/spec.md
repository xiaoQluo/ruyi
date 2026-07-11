# 4.6 stdlib/fmt.ry 模板格式化

## ADDED Requirements

### Requirement: fmt.format 导出与实现底座

`stdlib/fmt.ry` MUST 导出 `format(template: string, ...args): string` 与 `println(template, ...args)`。`format` 内部 MUST 调用 runtime 内建 `__string_replace_all` 完成占位符展开，并复用 `core.ry` 已有的 `string_concat` 与各类型的 `toString` 实现，避免重写。

#### Scenario: 模板无占位符

- **WHEN** 调用 `fmt.format("hello world")` 且模板不含 `{}` / `{name}`
- **THEN** 返回 `"hello world"` 原样，且 args 可为空

#### Scenario: 实现依赖 __string_replace_all

- **WHEN** 任意一次占位符替换发生于 `format` 内部
- **THEN** 调用链 MUST 经过 `__string_replace_all`，且 MUST NOT 引入正则依赖

### Requirement: 位置占位符 {} 与 {n}

占位符 `{}` MUST 按参数出现顺序替换下一个位置参数；`{0}` / `{1}` / `{n}` MUST 按下标替换对应位置参数。下标越界 MUST 抛出运行时错误，错误信息 MUST 含调用源文件的 `file:line`。

#### Scenario: 顺序占位符匹配

- **WHEN** 调用 `fmt.format("{} is {}", "age", 30)`
- **THEN** 返回 `"age is 30"`，两个 `{}` 分别消费第 0、第 1 参数

#### Scenario: 下标越界报错

- **WHEN** 调用 `fmt.format("{5}", 1)` 下标超出 args 长度
- **THEN** 抛出 `IndexOutOfBoundsError`，错误信息 MUST 含模板串与调用点 `file:line`

### Requirement: 命名占位符 {name} 与类型处理

占位符 `{name}` MUST 接受命名参数集合（命名参数走 `Map<string, dyn>` 风格或具名参数语法），按 `name` 键替换字符串化后的值。未命中的键 MUST 保留原 `{name}` 字面量。`fmt.ry` MUST 通过 `Int.toString` / `Float.toString` / `Bool.toString` 把非字符串类型先转字符串再插入。

#### Scenario: 命名命中

- **WHEN** 调用 `fmt.format("Hello, {user}", {"user": "world"})`
- **THEN** 返回 `"Hello, world"`，且 `user` 不在模板其余位置歧义

#### Scenario: 未命中的命名占位符保留

- **WHEN** 调用 `fmt.format("Hi {missing}", {"other": 1})`
- **THEN** 返回 `"Hi {missing}"`，占位符原样保留而不抛错
