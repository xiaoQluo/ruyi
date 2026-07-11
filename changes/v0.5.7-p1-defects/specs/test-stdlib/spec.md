# 4.8 stdlib/test.ry 与 @test 框架

## ADDED Requirements

### Requirement: parser 识别 @test 属性前缀

parser MUST 在 `fn` 声明前识别 `@test` 属性。`fn name() { ... }` 在 `@test` 前缀下 MUST 注册为测试函数，进入 `TestFunctionRegistry`，而非当作普通函数暴露。

#### Scenario: 无 @test 前缀保持普通函数

- **WHEN** 源码仅含 `fn add(a: int, b: int): int { return a + b; }`
- **THEN** `add` MUST 进入普通函数表，`TestFunctionRegistry` 中无 `add` 条目

#### Scenario: @test 前缀识别

- **WHEN** 源码含 `@test fn add_returns_two() { return 1 + 1; }`
- **THEN** 解析器 MUST 把 `add_returns_two` 加入 `TestFunctionRegistry`，且 MUST NOT 要求函数带 `return`

### Requirement: TestFunctionRegistry 按源位置收集

`TestFunctionRegistry` MUST 按 `file:line:column` 唯一标识每个 `@test fn`，并保留同一模块内的出现顺序。Registry MUST 仅在 parser 阶段写入，运行期只读。

#### Scenario: 同名跨文件隔离

- **WHEN** `a.ry` 与 `b.ry` 各含一个 `@test fn runs() { ... }`
- **THEN** Registry 中含两条独立条目，以不同 `file:line` 区分，运行器 MUST 都执行

#### Scenario: 同文件同名重复

- **WHEN** 同文件出现两次 `@test fn runs() { ... }`
- **THEN** 解析器 MUST 报编译错误 `duplicate test definition`，Registry 不保留任一条目

### Requirement: stdlib/test.ry 导出断言函数集

`stdlib/test.ry` MUST 导出 `assert_eq(actual, expected)`、`assert_true(value)`、`assert_false(value)`、`assert_not_null(value)` 四个函数。断言失败 MUST 抛出 `TestAssertionFailed` 含 `file:line` 与两值字符串；通过 MUST 静默返回。

#### Scenario: assert_eq 不相等抛错

- **WHEN** 调用 `assert_eq(2, 3)`
- **THEN** 抛出 `TestAssertionFailed`，消息 MUST 含 `expected=3 actual=2` 与源 `file:line`

#### Scenario: assert_not_null 通过

- **WHEN** 调用 `assert_not_null("x")` 或 `assert_not_null(0)`
- **THEN** 函数静默返回 `void`，不抛任何异常

### Requirement: ruyic CLI --test 运行测试

`ruyic` CLI MUST 支持 `--test` 标志。CLI 在 `--test` 模式下 MUST 先编译、再执行 `TestFunctionRegistry` 中全部 `@test fn`，最终输出 `passed=N failed=M` 与失败用例的 `file:line`，进程退出码 MUST 等于 `M`（`M=0` 时退出 0）。

#### Scenario: 全部通过

- **WHEN** `--test` 模式下 Registry 内全部用例断言通过
- **THEN** CLI MUST 输出 `passed=N failed=0`，进程退出码 MUST 为 0

#### Scenario: 存在失败用例

- **WHEN** `--test` 模式下任意 `@test fn` 抛出 `TestAssertionFailed`
- **THEN** CLI MUST 输出失败用例 `file:line` 与断言消息，进程退出码 MUST > 0 且等于失败数
