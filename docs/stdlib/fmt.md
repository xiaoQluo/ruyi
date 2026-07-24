# fmt — 格式化模块

## 概述

`fmt` 模块提供 `printf` 风格的格式化功能和便捷的 `println` 函数。
使用 `{}` 占位符替换字符串，匹配 JavaScript 的模板字符串风格。

**源文件**: `stdlib/fmt.ry`

**导入**: `import { ... } from "./fmt"`

---

## 函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `format` | `fn format(template: string, args: ...dyn): string` | 替换 `{}` 占位符为参数的字符串形式 |
| `println` | `fn println(template: string, args: ...dyn): void` | 格式化并输出到标准输出，末尾追加换行符 |

---

## 用法示例

```ruyi
format("Hello, {}!", "world")     // "Hello, world!"
format("{} + {} = {}", 1, 2, 3)   // "1 + 2 = 3"
println("Value: {}", 42)           // 输出 "Value: 42\n"
```

---

## 注意事项

- 占位符 `{}` 按顺序消耗参数，第一个 `{}` 使用第一个参数，依此类推
- 每个参数会调用其 `.toString()` 方法获取字符串表示
- 如果占位符数量多于参数数量，多余的占位符保持原样
- 底层委托给 `__string_replace_all_legacy` 运行时 FFI 函数
