# test — 测试模块

## 概述

`test` 模块提供 `@test fn` 声明所使用的断言辅助函数。
每个断言在失败时抛出 `AssertionError`。

**源文件**: `stdlib/test.ry`

**导入**: `import { ... } from "./test"`

---

## 函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `assert_eq` | `fn assert_eq<T>(actual: T, expected: T): void` | 断言两个值严格相等（`===`） |
| `assert_true` | `fn assert_true(value: bool): void` | 断言值为 true |
| `assert_false` | `fn assert_false(value: bool): void` | 断言值为 false |
| `assert_not_null` | `fn assert_not_null<T>(value: T?): void` | 断言值不为 null |

---

## 注意事项

- 所有断言失败时抛出 `AssertionError`，包含描述性错误消息
- `assert_eq` 使用 `===` 严格相等比较
- 这些断言专为 `@test fn` 编写，也可在普通代码中使用
