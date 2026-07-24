# string — 字符串工具模块

## 概述

`string` 模块提供独立的字符串工具函数以及 `String` 类的方法定义。
字符串实例方法由 codegen 层映射到运行时函数。

**源文件**: `stdlib/string.ry`

**导入**: `import { ... } from "./string"`

---

## 字符串工具函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `join` | `fn join(array: Array<dyn>, separator: string = ""): string` | 将数组元素以分隔符连接为字符串 |
| `fromCharCode` | `fn fromCharCode(code: int): string` | 从 Unicode 码点创建单字符字符串 |
| `fromCharCodes` | `fn fromCharCodes(codes: Array<int>): string` | 从 Unicode 码点数组创建字符串 |
| `concat` | `fn concat(args: ...string): string` | 拼接所有参数为字符串 |
| `template` | `fn template(templateStr: string, values: Array<dyn>): string` | 使用 `{0}`, `{1}` 占位符格式化模板 |
| `processTemplate` | `fn processTemplate(parts: Array<string>, context: dyn): string` | 处理模板字面量 `${...}` 替换 |

---

## String 类实例方法

> **注意**: 此类作为类型级契约。字符串上的方法调用由 codegen 层 (`expr.rs`) 映射为 `__string_*` 运行时函数。
> 此处定义的函数体仅用于类型检查，实际运行时不执行。

| 方法 | 签名 | 说明 |
|------|------|------|
| `length` | `fn length(self: string): int` | 返回字符串长度 |
| `contains` | `fn contains(self: string, substr: string): bool` | 检查字符串是否包含子串 |
| `startsWith` | `fn startsWith(self: string, prefix: string): bool` | 检查是否以指定前缀开头 |
| `endsWith` | `fn endsWith(self: string, suffix: string): bool` | 检查是否以指定后缀结尾 |
| `indexOf` | `fn indexOf(self: string, substr: string): int` | 返回子串首次出现的位置，未找到返回 -1 |
| `lastIndexOf` | `fn lastIndexOf(self: string, substr: string): int` | 返回子串最后出现的位置，未找到返回 -1 |
| `charAt` | `fn charAt(self: string, index: int): string` | 返回指定位置的字符 |
| `charCodeAt` | `fn charCodeAt(self: string, index: int): int` | 返回指定位置的字符编码 |
| `repeat` | `fn repeat(self: string, count: int): string` | 返回字符串重复 count 次的结果 |
| `substring` | `fn substring(self: string, start: int, end: int): string` | 返回 `[start, end)` 范围的子串 |
| `slice` | `fn slice(self: string, start: int, end: int): string` | 返回子串，支持负索引（从末尾计数） |
| `toUpperCase` | `fn toUpperCase(self: string): string` | 转换为大写 |
| `toLowerCase` | `fn toLowerCase(self: string): string` | 转换为小写 |
| `trim` | `fn trim(self: string): string` | 去除首尾空白 |
| `split` | `fn split(self: string, separator: string): Array<string>` | 按分隔符分割字符串 |
| `isEmpty` | `fn isEmpty(self: string): bool` | 检查字符串是否为空 |

---

## 注意事项

- `slice` 和 `substring` 的区别：`slice` 支持负索引，遇到无效范围返回空字符串
- `join` 函数的默认分隔符为空字符串 `""`
- `template` 函数使用 `{0}`, `{1}` 等位置占位符，而非命名占位符
