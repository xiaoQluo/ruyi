# io — 输入输出模块

## 概述

`io` 模块提供控制台输入和文件系统访问的基本 I/O 操作。
注意：`print` 和 `println` 是编译器内置函数，无需导入。

**源文件**: `stdlib/io.ry`

**导入**: `import { ... } from "./io"`

---

## 控制台输入

| 函数 | 签名 | 说明 |
|------|------|------|
| `readLine` | `fn readLine(): string?` | 从 stdin 读取一行，阻塞到输入完成，EOF 时返回 null |

---

## File 类

### 读操作

| 方法 | 签名 | 说明 |
|------|------|------|
| `readText` | `static fn readText(path: string): string` | 读取文件全部内容为字符串 |
| `readTextAsync` | `static async fn readTextAsync(path: string): Future<string>` | 异步读取文件全部内容 |
| `readLines` | `static fn readLines(path: string): Array<string>` | 读取文件的行数组 |
| `readLinesAsync` | `static async fn readLinesAsync(path: string): Future<Array<string>>` | 异步读取文件的行数组 |

### 写操作

| 方法 | 签名 | 说明 |
|------|------|------|
| `writeText` | `static fn writeText(path: string, content: string): void` | 写入字符串到文件 |
| `writeTextAsync` | `static async fn writeTextAsync(path: string, content: string): Future<void>` | 异步写入字符串到文件 |
| `writeLines` | `static fn writeLines(path: string, lines: Array<string>): void` | 写入行数组到文件（Unix 换行符 `\n`） |
| `writeLinesAsync` | `static async fn writeLinesAsync(path: string, lines: Array<string>): Future<void>` | 异步写入行数组到文件 |

### 文件状态

| 方法 | 签名 | 说明 |
|------|------|------|
| `exists` | `static fn exists(path: string): bool` | 检查文件或目录是否存在 |
| `existsAsync` | `static async fn existsAsync(path: string): Future<bool>` | 异步检查文件或目录是否存在 |
| `isDirectory` | `static fn isDirectory(path: string): bool` | 检查是否为目录 |
| `isDirectoryAsync` | `static async fn isDirectoryAsync(path: string): Future<bool>` | 异步检查是否为目录 |
| `isFile` | `static fn isFile(path: string): bool` | 检查是否为文件 |
| `isFileAsync` | `static async fn isFileAsync(path: string): Future<bool>` | 异步检查是否为文件 |

### 删除与目录操作

| 方法 | 签名 | 说明 |
|------|------|------|
| `delete` | `static fn delete(path: string): void` | 删除文件 |
| `deleteAsync` | `static async fn deleteAsync(path: string): Future<void>` | 异步删除文件 |
| `mkdir` | `static fn mkdir(path: string, recursive: bool = false): void` | 创建目录 |
| `mkdirAsync` | `static async fn mkdirAsync(path: string, recursive: bool = false): Future<void>` | 异步创建目录 |

---

## 注意事项

- `print` 和 `println` 是编译器内置函数，无需导入即可使用
- 文件操作为文本模式，不适合二进制文件
- 异步方法返回 `Future<T>` 类型，需配合 `await` 使用
