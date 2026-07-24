# buffer — 二进制缓冲区模块

## 概述

`buffer` 模块提供全面的二进制缓冲区实现，支持字节级读写、大小端控制、字符串编码和格式转换。
使用 `Array<int>` 作为后端存储，所有字节值限制在 0-255 范围内。

**源文件**: `stdlib/buffer.ry`

**导入**: `import { ... } from "./buffer"`

---

## Buffer 类

### 构造函数

| 方法 | 签名 | 说明 |
|------|------|------|
| `new` | `fn new(data: Array<int>)` | 从现有字节数组创建 Buffer |

### 属性

| 方法 | 签名 | 说明 |
|------|------|------|
| `length` | `fn length(): int` | 返回缓冲区字节数 |

### 工厂方法 (static)

| 方法 | 签名 | 说明 |
|------|------|------|
| `alloc` | `static fn alloc(size: int): Buffer` | 分配指定大小的零填充缓冲区 |
| `fromArray` | `static fn fromArray(arr: Array<int>): Buffer` | 从整型数组创建缓冲区，值掩码到 0-255 |
| `fromString` | `static fn fromString(s: string, encoding: string = "utf8"): Buffer` | 从字符串创建缓冲区，支持 `"utf8"`, `"ascii"`, `"hex"` 编码 |
| `fromBase64` | `static fn fromBase64(s: string): Buffer` | 从 Base64 字符串解码创建缓冲区 |
| `fromHex` | `static fn fromHex(s: string): Buffer` | 从十六进制字符串解码创建缓冲区 |
| `concat` | `static fn concat(buffers: Array<Buffer>): Buffer` | 拼接多个缓冲区 |

### 基本访问

| 方法 | 签名 | 说明 |
|------|------|------|
| `get` | `fn get(index: int): int` | 获取指定索引的字节值（越界返回 0） |
| `set` | `fn set(index: int, value: int): void` | 设置指定索引的字节值（越界忽略） |
| `fill` | `fn fill(value: int, start: int = 0, end: int = -1): Buffer` | 填充字节范围，返回 self 支持链式调用 |
| `slice` | `fn slice(start: int = 0, end: int = -1): Buffer` | 返回字节切片（复制语义） |
| `copy` | `fn copy(target: Buffer, targetStart: int = 0, sourceStart: int = 0, sourceEnd: int = -1): int` | 复制字节到目标缓冲区，返回实际写入字节数 |
| `toArray` | `fn toArray(): Array<int>` | 返回内部字节数组的副本 |

### 比较与搜索

| 方法 | 签名 | 说明 |
|------|------|------|
| `equals` | `fn equals(other: Buffer): bool` | 逐字节比较是否相等 |
| `indexOf` | `fn indexOf(value: int, start: int = 0): int` | 返回首次出现的索引，未找到返回 -1 |
| `lastIndexOf` | `fn lastIndexOf(value: int, start: int = -1): int` | 返回最后出现的索引，未找到返回 -1 |
| `includes` | `fn includes(value: int): bool` | 检查是否包含指定字节值 |
| `startsWith` | `fn startsWith(prefix: Buffer): bool` | 检查是否以指定前缀开始 |
| `endsWith` | `fn endsWith(suffix: Buffer): bool` | 检查是否以指定后缀结束 |

### 整型读取（支持大端/小端）

| 方法 | 签名 | 说明 |
|------|------|------|
| `readUInt8` | `fn readUInt8(offset: int): int` | 读取无符号 8 位整数 |
| `readInt16BE` | `fn readInt16BE(offset: int): int` | 读取大端有符号 16 位整数 |
| `readInt16LE` | `fn readInt16LE(offset: int): int` | 读取小端有符号 16 位整数 |
| `readInt32BE` | `fn readInt32BE(offset: int): int` | 读取大端有符号 32 位整数 |
| `readInt32LE` | `fn readInt32LE(offset: int): int` | 读取小端有符号 32 位整数 |
| `readUInt32BE` | `fn readUInt32BE(offset: int): int` | 读取大端无符号 32 位整数 |
| `readUInt32LE` | `fn readUInt32LE(offset: int): int` | 读取小端无符号 32 位整数 |

### 整型写入（支持大端/小端）

| 方法 | 签名 | 说明 |
|------|------|------|
| `writeUInt8` | `fn writeUInt8(offset: int, value: int): void` | 写入无符号 8 位整数 |
| `writeInt16BE` | `fn writeInt16BE(offset: int, value: int): void` | 写入大端有符号 16 位整数 |
| `writeInt16LE` | `fn writeInt16LE(offset: int, value: int): void` | 写入小端有符号 16 位整数 |
| `writeInt32BE` | `fn writeInt32BE(offset: int, value: int): void` | 写入大端有符号 32 位整数 |
| `writeInt32LE` | `fn writeInt32LE(offset: int, value: int): void` | 写入小端有符号 32 位整数 |

### 浮点读写（支持大端/小端）

| 方法 | 签名 | 说明 |
|------|------|------|
| `readFloat64BE` | `fn readFloat64BE(offset: int): float` | 读取大端 64 位浮点数（IEEE 754） |
| `readFloat64LE` | `fn readFloat64LE(offset: int): float` | 读取小端 64 位浮点数（IEEE 754） |
| `writeFloat64BE` | `fn writeFloat64BE(offset: int, value: float): void` | 写入大端 64 位浮点数 |
| `writeFloat64LE` | `fn writeFloat64LE(offset: int, value: float): void` | 写入小端 64 位浮点数 |

### 字符串读写

| 方法 | 签名 | 说明 |
|------|------|------|
| `readString` | `fn readString(offset: int, length: int): string` | 读取 UTF-8 编码字符串，无效序列替换为 U+FFFD |
| `writeString` | `fn writeString(offset: int, value: string, encoding: string = "utf8"): int` | 写入字符串，返回写入字节数 |

### 编码转换

| 方法 | 签名 | 说明 |
|------|------|------|
| `toString` | `fn toString(encoding: string = "utf8"): string` | 整体转换为字符串，支持 `"utf8"`, `"ascii"`, `"hex"` |
| `toBase64` | `fn toBase64(): string` | Base64 编码（RFC 4648） |
| `toHex` | `fn toHex(): string` | 小写十六进制编码 |
| `toBase64Url` | `fn toBase64Url(): string` | Base64URL 编码（RFC 4648，无填充） |

---

## 注意事项

- 所有字节值自动掩码到 0-255 范围
- 浮点数读写使用简化实现，基于符号/指数/尾数分解，精度有限
- `readString` 支持 1/2/3/4 字节 UTF-8 序列
- `writeString` 支持 `"utf8"` 和 `"ascii"` 编码
- `toBase64Url` 输出无填充，适用于 URL 安全场景
