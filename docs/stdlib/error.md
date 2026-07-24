# error — 错误类型层次结构

## 概述

`error` 模块提供完整的错误类型层次结构，包含基类 `Error` 和多种特定错误类型。

**源文件**: `stdlib/error.ry`

**导入**: `import { ... } from "./error"`

---

## 错误类型层次

```
Error
├── TypeError
├── RuntimeError
├── LogicError
│   ├── AssertionError
│   └── ArgumentError
├── RangeError
├── NullError
├── ArithmeticError
├── IteratorError
├── ParseError
├── NullAssertionError
└── IOError
```

---

## 基类: `Error`

| 方法 | 签名 | 说明 |
|------|------|------|
| `new` | `fn new(message: string)` | 创建错误实例 |
| `getMessage` | `fn getMessage(self): string` | 返回错误消息 |
| `getStackTrace` | `fn getStackTrace(self): Array<string>` | 返回堆栈跟踪 |
| `toString` | `fn toString(self): string` | 返回错误消息字符串 |

---

## 子类

| 类 | 继承自 | 说明 |
|------|------|------|
| `TypeError` | `Error` | 类型错误 |
| `RuntimeError` | `Error` | 运行时错误 |
| `LogicError` | `Error` | 逻辑/编程错误基类 |
| `AssertionError` | `LogicError` | 断言失败 |
| `ArgumentError` | `LogicError` | 无效参数 |
| `RangeError` | `Error` | 索引越界 |
| `NullError` | `Error` | 空值错误 |
| `ArithmeticError` | `Error` | 算术错误 |
| `IteratorError` | `Error` | 迭代器错误 |
| `ParseError` | `Error` | 解析错误 |
| `NullAssertionError` | `Error` | 空值断言失败 |
| `IOError` | `Error` | 输入输出错误 |

所有子类均提供:
| 方法 | 签名 | 说明 |
|------|------|------|
| `new` | `fn new(message: string)` | 创建错误实例 |

---

## 辅助函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `assert` | `fn assert(condition: bool, message: string): void` | 断言条件为真，否则抛出 `AssertionError` |
| `assertNotNull` | `fn assertNotNull<T>(value: T?, message: string): T` | 断言值不为 null，否则抛出 `NullAssertionError` |

---

## 注意事项

- 所有错误类都有 `message: string` 属性和 `stackTrace: Array<string>` 属性
- `stackTrace` 在 `throw` 时由运行时填充
- `assert()` 和 `assertNotNull()` 用于防御性编程
