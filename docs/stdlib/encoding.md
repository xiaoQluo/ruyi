# encoding — 编码模块

## 概述

`encoding` 模块提供全面的编码/解码工具，纯 `.ry` 实现，无需外部依赖。
涵盖 Base64、Base64URL、Hex 和 URL 百分比编码。

**源文件**: `stdlib/encoding.ry`

**导入**: `import { ... } from "./encoding"`

---

## 常量

| 常量 | 类型 | 说明 |
|------|------|------|
| `BASE64_ALPHABET` | `string` | 标准 Base64 字母表（`A-Z a-z 0-9 + /`） |
| `BASE64URL_ALPHABET` | `string` | URL 安全 Base64 字母表（`-` 替代 `+`, `_` 替代 `/`） |
| `HEX_CHARS` | `string` | 小写十六进制字符集 |

---

## Base64（RFC 4648 标准字母表）

| 函数 | 签名 | 说明 |
|------|------|------|
| `base64Encode` | `fn base64Encode(data: string): string` | 编码字符串为 Base64 |
| `base64Decode` | `fn base64Decode(encoded: string): string` | 解码 Base64 字符串 |
| `base64EncodeBytes` | `fn base64EncodeBytes(data: Array<int>): string` | 编码字节数组为 Base64 |
| `base64DecodeToBytes` | `fn base64DecodeToBytes(encoded: string): Array<int>` | 解码 Base64 为字节数组 |

---

## Base64URL（RFC 4648 URL 安全字母表）

| 函数 | 签名 | 说明 |
|------|------|------|
| `base64UrlEncode` | `fn base64UrlEncode(data: string): string` | 编码为 Base64URL（无填充） |
| `base64UrlDecode` | `fn base64UrlDecode(encoded: string): string` | 解码 Base64URL |
| `base64UrlEncodeBytes` | `fn base64UrlEncodeBytes(data: Array<int>): string` | 编码字节数组为 Base64URL |
| `base64UrlDecodeToBytes` | `fn base64UrlDecodeToBytes(encoded: string): Array<int>` | 解码 Base64URL 为字节数组 |

---

## 十六进制

| 函数 | 签名 | 说明 |
|------|------|------|
| `hexEncode` | `fn hexEncode(data: string): string` | 将字符串编码为小写十六进制 |
| `hexDecode` | `fn hexDecode(hex: string): string` | 解码十六进制为字符串 |
| `bytesToHex` | `fn bytesToHex(data: Array<int>): string` | 将字节数组编码为小写十六进制 |
| `hexToBytes` | `fn hexToBytes(hex: string): Array<int>` | 解码十六进制为字节数组 |

---

## URL / 百分比编码（RFC 3986）

| 函数 | 签名 | 说明 |
|------|------|------|
| `urlEncode` | `fn urlEncode(data: string): string` | 百分比编码 URL（空格 → `%20`） |
| `urlDecode` | `fn urlDecode(encoded: string): string` | 解码百分比编码（`+` → 空格） |
| `urlEncodeComponent` | `fn urlEncodeComponent(data: string): string` | 编码 URL 查询组件（更激进） |
| `urlDecodeComponent` | `fn urlDecodeComponent(encoded: string): string` | 解码 URL 组件（`+` 保持原样） |

---

## 格式转换工具

| 函数 | 签名 | 说明 |
|------|------|------|
| `base64ToBase64Url` | `fn base64ToBase64Url(base64: string): string` | Base64 转为 Base64URL（`+`→`-`, `/`→`_`, 去除 `=`） |
| `base64UrlToBase64` | `fn base64UrlToBase64(base64url: string): string` | Base64URL 转为 Base64（补充 `=` 填充） |

---

## 验证函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `isBase64` | `fn isBase64(str: string): bool` | 验证是否为有效 Base64 字符串 |
| `isBase64Url` | `fn isBase64Url(str: string): bool` | 验证是否为有效 Base64URL 字符串 |
| `isHex` | `fn isHex(str: string): bool` | 验证是否为有效十六进制字符串 |

---

## 注意事项

- `base64UrlEncode` 输出无填充（padless），适用于 URL 和文件名
- `urlDecode` 将 `+` 解码为空格（兼容 `application/x-www-form-urlencoded`）
- `urlDecodeComponent` 不将 `+` 解码为空格（符合 `decodeURIComponent` 语义）
- `bytesToHex` 和 `hexToBytes` 常用于加密模块的原始字节处理
