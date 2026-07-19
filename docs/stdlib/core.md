# core — 核心类型与操作

## 概述

`core` 模块提供基础类型的扩展方法、数值解析函数以及常用数学常量和函数。
该模块**自动可用**，无需显式导入。

**源文件**: `stdlib/core.ry`

---

## 类型类

### `Int` — 整数操作

| 方法 | 签名 | 说明 |
|------|------|------|
| `toString` | `fn toString(self: int): string` | 将整数转为字符串 |

### `Float` — 浮点数操作

| 方法 | 签名 | 说明 |
|------|------|------|
| `toString` | `fn toString(self: float): string` | 将浮点数转为字符串 |

### `Bool` — 布尔操作

| 方法 | 签名 | 说明 |
|------|------|------|
| `toString` | `fn toString(self: bool): string` | 将布尔值转为字符串 |

---

## 数值解析

| 函数 | 签名 | 说明 |
|------|------|------|
| `parseInt` | `fn parseInt(s: string): int` | 将字符串解析为整数，失败抛出 `ParseError` |
| `parseFloat` | `fn parseFloat(s: string): float` | 将字符串解析为浮点数，失败抛出 `ParseError` |

---

## 数学常量

| 常量 | 类型 | 值 | 说明 |
|------|------|----|------|
| `PI` | `float` | `3.141592653589793` | 圆周率 π |
| `E` | `float` | `2.718281828459045` | 自然对数的底数 e |

---

## 数学函数

### 整数函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `min` | `fn min(a: int, b: int): int` | 返回两个整数中较小者 |
| `max` | `fn max(a: int, b: int): int` | 返回两个整数中较大者 |
| `abs` | `fn abs(x: int): int` | 返回整数的绝对值 |

### 浮点数函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `minFloat` | `fn minFloat(a: float, b: float): float` | 返回两个浮点数中较小者 |
| `maxFloat` | `fn maxFloat(a: float, b: float): float` | 返回两个浮点数中较大者 |
| `absFloat` | `fn absFloat(x: float): float` | 返回浮点数的绝对值 |

---

## 注意事项

- 该模块自动导入，无需 `import` 语句
- 数值解析函数在输入无效时会抛出 `ParseError`
- `Int`、`Float`、`Bool` 类型的 `toString` 方法在字符串模板中自动调用
