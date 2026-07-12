# 4.9 collections.ry Array 与 Iterator 扩展

## ADDED Requirements

### Requirement: Array 扩展 ≥7 实例方法

`collections.ry::Array` MUST 新增 ≥7 个实例方法：`sort()`、`contains(x)`、`indexOf(x)`、`first()`、`last()`、`slice(begin, end?)`、`concat(other)`。`sort` 默认升序、可选比较器 `(T, T) -> int`；`slice` 的 `end` 缺省时 MUST 取到末尾；`concat` MUST 返回新数组且 MUST NOT 修改原数组。

#### Scenario: sort 默认升序

- **WHEN** 调用 `arr.sort()` 于 `[3, 1, 2]`
- **THEN** 返回 `[1, 2, 3]` 且 `arr` 自身 MUST NOT 被原地修改（参考实现细节由类型决定，关键语义为返回升序数组）

#### Scenario: contains/indexOf 一致性

- **WHEN** 对 `[1, 2, 3]` 分别调用 `arr.contains(2)` 与 `arr.indexOf(2)`
- **THEN** MUST 分别返回 `true` 与 `1`；调用 `contains(99)` 与 `indexOf(99)` MUST 分别返回 `false` 与 `-1`

### Requirement: Iterator 扩展 ≥8 实例方法

`collections.ry::Iterator` MUST 新增 ≥8 个实例方法：`takeWhile(p)`、`skipWhile(p)`、`chain(other)`、`enumerate()`、`zip(other)`、`sum()`、`product()`、`any(p)`、`all(p)`。`takeWhile` / `skipWhile` MUST 接受谓词 `T -> bool` 并返回惰性迭代器；`any` / `all` MUST 短路求值；`zip` MUST 在较短序列耗尽时停止。

#### Scenario: takeWhile 与 skipWhile

- **WHEN** 在 `[1, 2, 3, 4]` 上分别调用 `iter.takeWhile(x => x < 3)` 与 `iter.skipWhile(x => x < 3)`
- **THEN** `takeWhile` MUST 产生 `[1, 2]`，`skipWhile` MUST 产生 `[3, 4]`

#### Scenario: any/all 短路求值

- **WHEN** 在 `[true, false, true]` 上调用 `iter.any(x => x)` 与 `iter.all(x => x)`
- **THEN** MUST 分别返回 `true` 与 `false`；实现 MUST 在结果确定时立即返回，不再消费后续元素

### Requirement: sum/product 受 Add supertrait 约束

`Array.sum()` / `Iterator.sum()` 与 `Array.product()` / `Iterator.product()` MUST 元素实现 `Add`/`Mul` supertrait 的类型才允许调用。类型检查器 MUST 在缺失 supertrait 边界时报类型错误并定位到调用点行号。**This requirement depends on completion of supertraits (3.2).**

#### Scenario: int 元素 sum 通过

- **WHEN** 调用 `[1, 2, 3].sum()` 与 `iter.product()` 于 `[1, 2, 3, 4]`
- **THEN** 编译通过，分别在运行期返回 `6` 与 `24`

#### Scenario: 缺失 supertrait 拒绝

- **WHEN** 对类型参数 `T` 调用 `.sum()` 而 `T` 缺少 `Add` supertrait
- **THEN** 类型检查器 MUST 产出 `type error: T does not implement Add supertrait`，诊断定位 MUST 在 `.sum()` 调用处行号而非模板行号
