# bigint — 大整数模块

## 概述

`bigint` 模块包装内置的 `bigint` 类型，提供实用构造函数和操作。
`bigint` 类型是任意精度整数，运行时值表示为由 GC 管理的堆上指针。

**源文件**: `stdlib/bigint.ry`

**导入**: `import { ... } from "./bigint"`

---

## BigInt 类

| 方法 | 签名 | 说明 |
|------|------|------|
| `fromString` | `static fn fromString(s: string): BigInt` | 从十进制字符串创建 BigInt，无效字符串抛出 `RangeError` |
| `eq` | `fn eq(other: BigInt): bool` | 检查两个 BigInt 是否相等 |
| `toString` | `fn toString(): string` | 转换为十进制字符串 |

---

## 工具函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `parseBigInt` | `fn parseBigInt(s: string): BigInt` | 从十进制字符串解析 BigInt |
| `bigIntFromInt` | `fn bigIntFromInt(n: int): BigInt` | 从普通 int 创建 BigInt |

---

## 注意事项

- BigInt 内部使用不透明句柄 (`handle: int`) 由运行时管理
- `toString()` 当前返回占位符形式 `"BigInt(handle)"`，待后续 FFI 绑定完善
- 未来计划添加加/减/乘/除/模/幂/比较的 FFI 支持
