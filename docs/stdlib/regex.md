# regex — 正则表达式模块

## 概述

`regex` 模块提供基于 Thompson NFA 的正则引擎，纯 `.ry` 实现，无 FFI 依赖。

**源文件**: `stdlib/regex.ry`

**导入**: `import { ... } from "./regex"`

---

## 支持的正则语法

| 语法 | 说明 |
|------|------|
| 字面量 | 直接字符匹配 |
| `.` | 匹配除换行符外的任意字符 |
| `*` | 前一个元素零次或多次 |
| `+` | 前一个元素一次或多次 |
| `?` | 前一个元素零次或一次 |
| `|` | 或运算 |
| `()` | 分组 |
| `^` | 行首 |
| `$` | 行尾 |
| `[]` | 字符类（支持 `[a-z]` 范围和 `[^...]` 否定） |
| `\d` | 数字 `[0-9]` |
| `\w` | 单词字符 `[a-zA-Z0-9_]` |
| `\s` | 空白字符 |
| `\D`, `\W`, `\S` | 对应的大写形式为否定 |
| `{n,m}` | 重复次数范围 |
| `\n` | 换行符 |
| `\t` | 制表符 |
| `\r` | 回车符 |
| `\xNN` | 十六进制字符 |

---

## MatchResult 类

| 属性 | 类型 | 说明 |
|------|------|------|
| `index` | `int` | 匹配的起始位置 |
| `text` | `string` | 匹配的文本 |
| `end` | `int` | 匹配的结束位置 |

---

## Regex 类

| 方法 | 签名 | 说明 |
|------|------|------|
| `new` | `fn new(pattern: string): Regex` | 创建正则表达式 |
| `compile` | `fn compile(): bool` | 编译模式，成功返回 true |
| `test` | `fn test(text: string): bool` | 检查模式是否与整个字符串匹配 |
| `exec` | `fn exec(text: string): MatchResult?` | 查找第一个匹配，未匹配返回 null |
| `matchAll` | `fn matchAll(text: string): MatchResult[]` | 查找所有不重叠的匹配 |
| `replace` | `fn replace(text: string, replacement: string): string` | 替换第一个匹配项 |
| `replaceAll` | `fn replaceAll(text: string, replacement: string): string` | 替换所有匹配项 |
| `split` | `fn split(text: string, maxParts: int): string[]` | 按模式分割字符串 |

---

## 注意事项

- 使用 Thompson NFA 构造，避免 catastrophic backtracking
- `compile()` 必须在 `exec`/`test` 等方法之前调用
- `test()` 要求模式匹配整个字符串（包含起始和结束）
- `exec()` 查找第一个匹配（可在字符串任意位置）
- `split()` 的 `maxParts` 参数可限制分割次数，`-1` 表示不限
