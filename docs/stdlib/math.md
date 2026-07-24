# math — 数学模块

## 概述

`math` 模块提供完整的数学函数和常量，所有函数操作于 `float`（f64）值。

**源文件**: `stdlib/math.ry`

**导入**: `import { ... } from "./math"`

---

## 常量

| 常量 | 类型 | 说明 |
|------|------|------|
| `PI` | `float` | 圆周率 π |
| `E` | `float` | 自然对数的底数 e |

---

## 基本数学函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `sqrt` | `fn sqrt(x: float): float` | 平方根 |
| `pow` | `fn pow(x: float, y: float): float` | x 的 y 次幂 |
| `abs` | `fn abs(x: float): float` | 绝对值 |
| `min` | `fn min(a: float, b: float): float` | 两者中的较小值 |
| `max` | `fn max(a: float, b: float): float` | 两者中的较大值 |

---

## 三角函数（弧度制）

| 函数 | 签名 | 说明 |
|------|------|------|
| `sin` | `fn sin(x: float): float` | 正弦 |
| `cos` | `fn cos(x: float): float` | 余弦 |
| `tan` | `fn tan(x: float): float` | 正切 |
| `asin` | `fn asin(x: float): float` | 反正弦，返回值范围 `[-π/2, π/2]` |
| `acos` | `fn acos(x: float): float` | 反余弦，返回值范围 `[0, π]` |
| `atan` | `fn atan(x: float): float` | 反正切，返回值范围 `[-π/2, π/2]` |
| `atan2` | `fn atan2(y: float, x: float): float` | 四象限反正切，返回值范围 `[-π, π]` |

---

## 对数函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `log` | `fn log(x: float): float` | 自然对数 |
| `log2` | `fn log2(x: float): float` | 以 2 为底的对数 |
| `log10` | `fn log10(x: float): float` | 以 10 为底的对数 |

---

## 指数函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `exp` | `fn exp(x: float): float` | e 的 x 次幂 |

---

## 舍入函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `ceil` | `fn ceil(x: float): float` | 向上取整 |
| `floor` | `fn floor(x: float): float` | 向下取整 |
| `round` | `fn round(x: float): float` | 四舍五入 |
| `trunc` | `fn trunc(x: float): float` | 截断取整 |

---

## 符号函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `sign` | `fn sign(x: float): float` | 返回 -1（负数）、1（正数）或 0（零） |

---

## 双曲函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `sinh` | `fn sinh(x: float): float` | 双曲正弦 |
| `cosh` | `fn cosh(x: float): float` | 双曲余弦 |
| `tanh` | `fn tanh(x: float): float` | 双曲正切 |

---

## 杂项

| 函数 | 签名 | 说明 |
|------|------|------|
| `hypot` | `fn hypot(x: float, y: float): float` | 计算 `sqrt(x² + y²)`（斜边） |
| `cbrt` | `fn cbrt(x: float): float` | 立方根 |

---

## 注意事项

- 所有函数操作于 `float`（f64）类型
- 三角函数使用弧度制，非角度制
- 内部委托给 `__math_*` 运行时 FFI 函数
