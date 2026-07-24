# io — 输入输出模块

## 概述

`io` 模块提供控制台输入操作。
注意：`print` 和 `println` 是编译器内置函数，无需导入。
文件系统操作（读、写、目录操作、路径工具等）请使用 [`fs` 模块](./fs.md)。

**源文件**: `stdlib/io.ry`

**导入**: `import { readLine } from "./io"`

---

## 控制台输入

| 函数 | 签名 | 说明 |
|------|------|------|
| `readLine` | `fn readLine(): string?` | 从 stdin 读取一行，阻塞到输入完成，EOF 时返回 null |

---

## 文件系统操作

文件系统操作已迁移至 [`fs` 模块](./fs.md)，包括：

- **路径操作**: `join`, `basename`, `dirname`, `extname`, `normalize`, `resolve`, `relative`
- **文件读写**: `readFile`, `writeFile`, `appendFile`, `readLines`, `writeLines`
- **文件元数据**: `exists`, `isFile`, `isDir`, `size`, `mtime`
- **目录操作**: `readDir`, `mkdir`, `removeDir`, `ensureDir`
- **递归操作**: `walkDir`, `listAllFiles`, `listAllDirs`, `copyDir`
- **高级工具**: `isBinary`, `isEmptyFile`, `readFileAsBase64`, `FileInfo` 类等
- **全部操作均提供同步和异步（`*Async`）两套 API**

```
import { readFile, writeFile, exists } from "./fs";
```

## 注意事项

- `print` 和 `println` 是编译器内置函数，无需导入即可使用
