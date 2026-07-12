# stdlib/random 模块

## ADDED Requirements

### Requirement: random.ry 导出完整的 Random API

`stdlib/random.ry` MUST 导出 `Random` 类与以下 5 个构造与方法：`random_new(seed?)`、`nextInt(min, max)`、`nextFloat()`、`nextBool()`、`nextBytes(n)`，并通过 `import "std::random"` 在用户代码中可见。

#### Scenario: 默认 seed 构造

- **WHEN** 用户调用 `random_new()`（无参）
- **THEN** MUST 返回一个基于熵源（运行时 `RandomState`）初始化的 `Random` 实例，不依赖用户传入 seed

#### Scenario: 指定 seed 构造

- **WHEN** 用户调用 `random_new(seed: int)` 传入固定整数 seed
- **THEN** MUST 返回基于 xorshift64 初始化器派生的 `Random` 实例，同 seed 下产出序列确定性可复现

### Requirement: random 方法经由 ruyi_random_* C FFI 调用

`stdlib/random.ry` 中每个对外函数 MUST 通过 `extern fn` 声明绑定到 runtime 的 C FFI 符号 `ruyi_random_new` / `ruyi_random_next_int` / `ruyi_random_next_float` / `ruyi_random_next_bool` / `ruyi_random_next_bytes`，不得在 `.ry` 内部持有原生逻辑。

#### Scenario: 5 个 FFI 符号在二进制中可链接

- **WHEN** 编译产物链接 `ruyi_runtime`
- **THEN** `nm <binary> | grep ruyi_random_` MUST 命中全部 5 个符号，无 link-time undefined

#### Scenario: stdlib/random.ry 不引入 native 逻辑

- **WHEN** 静态检查 `stdlib/random.ry` 源码
- **THEN** 除 `extern fn` 声明外 MUST 不包含指针运算或运行时状态管理，文件仅做参数转发与边界处理

### Requirement: nextInt 在区间端点上的确定性语义

`Random.nextInt(min, max)` MUST 返回闭区间 `[min, max]` 内的整数；当 `min === max` 时 MUST 返回 `min`；当 `min > max` 时 MUST 产生诊断错误或 panic（不静默回环）。

#### Scenario: min 与 max 相同

- **WHEN** 用户调用 `r.nextInt(5, 5)`
- **THEN** MUST 始终返回 `5`，调用次数与返回值无关

#### Scenario: 多次调用覆盖区间内全部整数

- **WHEN** 测试循环 100000 次调用 `r.nextInt(0, 9)`（seed 固定）
- **THEN** 每个返回值的统计直方图 MUST 落在 `[0, 9]`，且各整数的命中次数偏差不超过统计下限（chi-square 检验通过）

#### Scenario: 非法区间 min > max

- **WHEN** 用户调用 `r.nextInt(10, 5)`
- **THEN** MUST 立即产出错误诊断或运行时 panic，不得返回区间内任何值，不得循环回区间起点