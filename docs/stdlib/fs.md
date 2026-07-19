# fs — 文件系统模块

## 概述

`fs` 模块提供全面的文件系统操作，涵盖路径工具、文件元数据、读写、目录操作、递归遍历和文件操作工具。

**源文件**: `stdlib/fs.ry`

**导入**: `import { ... } from "./fs"`

---

## 常量

| 常量 | 类型 | 值 | 说明 |
|------|------|-----|------|
| `SEPARATOR` | `string` | `"/"` | 平台路径分隔符 |

---

## 路径工具

| 函数 | 签名 | 说明 |
|------|------|------|
| `isAbsolute` | `fn isAbsolute(path: string): bool` | 检查路径是否为绝对路径 |
| `isRelative` | `fn isRelative(path: string): bool` | 检查路径是否为相对路径 |
| `join` | `fn join(segments: ...string): string` | 拼接多个路径段 |
| `basename` | `fn basename(path: string): string` | 返回路径的最后组件（文件名） |
| `dirname` | `fn dirname(path: string): string` | 返回目录部分 |
| `extname` | `fn extname(path: string): string` | 返回文件扩展名（含点号） |
| `normalize` | `fn normalize(path: string): string` | 规范化路径（解析 `..` 和 `.`） |
| `resolve` | `fn resolve(base: string, relative: string): string` | 将相对路径解析为绝对路径 |
| `relative` | `fn relative(from: string, to: string): string` | 计算从 `from` 到 `to` 的相对路径 |

---

## 文件元数据

| 函数 | 签名 | 说明 |
|------|------|------|
| `exists` | `fn exists(path: string): bool` | 检查路径是否存在 |
| `existsAsync` | `async fn existsAsync(path: string): Future<bool>` | 异步检查路径是否存在 |
| `isFile` | `fn isFile(path: string): bool` | 检查是否为文件 |
| `isFileAsync` | `async fn isFileAsync(path: string): Future<bool>` | 异步检查是否为文件 |
| `isDir` | `fn isDir(path: string): bool` | 检查是否为目录 |
| `isDirAsync` | `async fn isDirAsync(path: string): Future<bool>` | 异步检查是否为目录 |
| `size` | `fn size(path: string): int` | 返回文件大小（字节），失败返回 -1 |
| `sizeAsync` | `async fn sizeAsync(path: string): Future<int>` | 异步返回文件大小 |
| `mtime` | `fn mtime(path: string): int` | 返回最后修改时间（纪元毫秒），失败返回 -1 |
| `mtimeAsync` | `async fn mtimeAsync(path: string): Future<int>` | 异步返回最后修改时间 |

---

## 文件读写

| 函数 | 签名 | 说明 |
|------|------|------|
| `readFile` | `fn readFile(path: string): string` | 读取文件全部内容 |
| `readFileAsync` | `async fn readFileAsync(path: string): Future<string>` | 异步读取文件全部内容 |
| `writeFile` | `fn writeFile(path: string, content: string): void` | 写入文件（覆盖已有内容） |
| `writeFileAsync` | `async fn writeFileAsync(path: string, content: string): Future<void>` | 异步写入文件 |
| `appendFile` | `fn appendFile(path: string, content: string): void` | 追加内容到文件末尾 |
| `appendFileAsync` | `async fn appendFileAsync(path: string, content: string): Future<void>` | 异步追加内容到文件 |
| `copyFile` | `fn copyFile(src: string, dst: string): void` | 复制文件 |
| `copyFileAsync` | `async fn copyFileAsync(src: string, dst: string): Future<void>` | 异步复制文件 |
| `readLines` | `fn readLines(path: string): Array<string>` | 按行读取文件 |
| `readLinesAsync` | `async fn readLinesAsync(path: string): Future<Array<string>>` | 异步按行读取文件 |
| `writeLines` | `fn writeLines(path: string, lines: Array<string>): void` | 写入行数组到文件（Unix 换行符） |

---

## 目录操作

| 函数 | 签名 | 说明 |
|------|------|------|
| `readDir` | `fn readDir(path: string): Array<string>` | 列出目录中的条目名称 |
| `readDirAsync` | `async fn readDirAsync(path: string): Future<Array<string>>` | 异步列出目录条目 |
| `mkdir` | `fn mkdir(path: string, recursive: bool = false): void` | 创建目录 |
| `mkdirAsync` | `async fn mkdirAsync(path: string, recursive: bool = false): Future<void>` | 异步创建目录 |
| `removeDir` | `fn removeDir(path: string, recursive: bool = false): bool` | 删除目录 |
| `removeDirAsync` | `async fn removeDirAsync(path: string, recursive: bool = false): Future<bool>` | 异步删除目录 |
| `ensureDir` | `fn ensureDir(path: string): void` | 确保目录存在（等价于 `mkdir -p`） |
| `ensureDirAsync` | `async fn ensureDirAsync(path: string): Future<void>` | 异步确保目录存在 |

---

## 文件操作

| 函数 | 签名 | 说明 |
|------|------|------|
| `rename` | `fn rename(oldPath: string, newPath: string): bool` | 重命名/移动文件或目录 |
| `renameAsync` | `async fn renameAsync(oldPath: string, newPath: string): Future<bool>` | 异步重命名 |
| `deleteFile` | `fn deleteFile(path: string): void` | 删除文件（不存在时静默忽略） |
| `deleteFileAsync` | `async fn deleteFileAsync(path: string): Future<void>` | 异步删除文件 |
| `truncate` | `fn truncate(path: string, size: int): void` | 截断文件到指定大小 |
| `touch` | `fn touch(path: string): void` | 创建空文件或更新 mtime |

---

## 递归操作

| 函数 | 签名 | 说明 |
|------|------|------|
| `walkDir` | `fn walkDir(dir: string): Array<string>` | 递归遍历目录，返回所有路径（含文件和目录） |
| `listAllFiles` | `fn listAllFiles(dir: string): Array<string>` | 递归列出所有文件（不含目录） |
| `listAllDirs` | `fn listAllDirs(dir: string): Array<string>` | 递归列出所有子目录 |
| `copyDir` | `fn copyDir(src: string, dst: string): void` | 递归复制目录 |

---

## FileInfo 类

| 方法 | 签名 | 说明 |
|------|------|------|
| `new` | `fn new()` | 创建空的 FileInfo |
| `fromPath` | `static fn fromPath(path: string): FileInfo` | 通过检查文件系统创建 FileInfo |

**属性**:

| 属性 | 类型 | 说明 |
|------|------|------|
| `path` | `string` | 完整路径 |
| `size` | `int` | 文件大小（-1 表示未知） |
| `isDir` | `bool` | 是否为目录 |
| `isFile` | `bool` | 是否为文件 |
| `mtime` | `int` | 最后修改时间 |

---

## 高级工具函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `isBinary` | `fn isBinary(path: string): bool?` | 启发式检查文件是否为二进制 |
| `isEmptyFile` | `fn isEmptyFile(path: string): bool` | 检查文件是否为空 |
| `isSymlink` | `fn isSymlink(path: string): bool` | 检查是否为符号链接（暂未实现） |
| `readFileAsBase64` | `fn readFileAsBase64(path: string): string` | 读取文件并 Base64 编码 |
| `readFileAsHex` | `fn readFileAsHex(path: string): string` | 读取文件并十六进制编码 |
| `writeFileFromBase64` | `fn writeFileFromBase64(path: string, b64: string): void` | 解码 Base64 内容并写入文件 |
| `writeFileFromHex` | `fn writeFileFromHex(path: string, hex: string): void` | 解码十六进制内容并写入文件 |
| `fileExtension` | `fn fileExtension(path: string): string` | 返回文件扩展名 |
| `hasExtension` | `fn hasExtension(path: string, ext: string): bool` | 检查是否有指定扩展名 |
| `transformFile` | `fn transformFile(path: string, transform: fn(string) -> string): void` | 读取、变换、写回文件 |
| `filesEqual` | `fn filesEqual(path1: string, path2: string): bool` | 比较两文件内容是否相同 |
| `concatFiles` | `fn concatFiles(sources: Array<string>, dest: string): void` | 连接多个文件 |
| `moveFile` | `fn moveFile(src: string, dst: string): bool` | 移动文件 |
| `parentDir` | `fn parentDir(path: string): string` | 返回父目录路径 |
| `fileNameWithoutExt` | `fn fileNameWithoutExt(path: string): string` | 返回不含扩展名的文件名 |
| `changeExtension` | `fn changeExtension(path: string, newExt: string): string` | 更改文件扩展名 |
| `tempDir` | `fn tempDir(): string` | 返回系统临时目录 |
| `tempFilePath` | `fn tempFilePath(prefix: string = "ruyi_"): string` | 生成临时文件路径 |
| `tempDirPath` | `fn tempDirPath(prefix: string = "ruyi_dir_"): string` | 生成临时目录路径 |
| `isTextExtension` | `fn isTextExtension(path: string): bool` | 检查扩展名是否为已知文本类型 |

---

## 注意事项

- 文件操作基于文本模式，二进制文件操作可能会损坏数据
- `isSymlink()` 因平台特定 FFI 尚未就绪，始终返回 false
- `walkDir` 支持循环符号链接检测（通过已访问路径追踪）
- `copyFile` 不保留文件元数据（权限、时间戳）
- `truncate` 操作于文本内容，二进制文件可能损坏
