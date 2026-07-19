# random — 随机数模块

## 概述

`random` 模块提供基于运行时的伪随机数生成器。
每个 `Random` 实例携带一个不透明 `rng` 令牌，标识运行时中的生成器状态。

**源文件**: `stdlib/random.ry`

**导入**: `import { ... } from "./random"`

---

## Random 类

| 方法 | 签名 | 说明 |
|------|------|------|
| `new` | `fn new(seed?: int): Random` | 创建随机数生成器，可选种子 |
| `nextInt` | `fn nextInt(min: int, max: int): int` | 返回 `[min, max]` 范围内的均匀随机整数 |
| `nextFloat` | `fn nextFloat(): float` | 返回 `[0.0, 1.0)` 范围内的均匀随机浮点数 |
| `nextBool` | `fn nextBool(): bool` | 返回随机布尔值 |
| `nextBytes` | `fn nextBytes(n: int): string` | 返回 n 个伪随机字节（packed 为字符串） |
| `seed` | `fn seed(n: int): void` | 重新为生成器设定种子 |

---

## 自由函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `random_new` | `fn random_new(seed?: int): Random` | 创建新的 Random 实例，可选种子 |

---

## 注意事项

- 不提供种子时，运行时从熵源派生出非确定性种子
- `seed()` 方法会重新初始化底层生成器
- 标准库层不维护本地状态，所有函数转发到 `__random_*` C 函数
- 此 PRNG 为通用伪随机数生成器，**非**密码学安全，需要密码学安全随机数请使用 `crypto` 模块
