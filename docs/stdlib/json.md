# json — JSON 模块

## 概述

`json` 模块提供 JSON 解析和字符串化功能。基础实现，支持 JSON 对象、数组、字符串、数字、布尔值和 null。

**源文件**: `stdlib/json.ry`

**导入**: `import { ... } from "./json"`

---

## 函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `parse` | `fn parse(json_string: string): dyn` | 解析 JSON 字符串为 Ruyi 值，失败返回 null |
| `stringify` | `fn stringify(value: dyn): string` | 将 Ruyi 值序列化为 JSON 字符串 |
| `isValid` | `fn isValid(json_string: string): bool` | 检查字符串是否为有效 JSON |

---

## 注意事项

- `parse()` 返回 `dyn` 类型，需根据实际运行时类型进行后续处理
- `isValid()` 内部调用 `parse()`，通过检查返回值是否为 null 判断
- 当前为基础实现，后续可扩展更丰富的功能（自定义缩进、过滤器等）
