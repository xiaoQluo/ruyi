# path — 路径模块

## 概述

`path` 模块提供文件系统路径操作工具。

**源文件**: `stdlib/path.ry`

**导入**: `import { ... } from "./path"`

---

## Path 类

### 路径操作（静态方法）

| 方法 | 签名 | 说明 |
|------|------|------|
| `join` | `static fn join(paths: ...string): string` | 拼接路径段 |
| `basename` | `static fn basename(path: string): string` | 返回路径的最后组件 |
| `basenameNoExt` | `static fn basenameNoExt(path: string): string` | 返回不含扩展名的文件名 |
| `dirname` | `static fn dirname(path: string): string` | 返回目录部分 |
| `extname` | `static fn extname(path: string): string` | 返回文件扩展名（含点号） |
| `isAbsolute` | `static fn isAbsolute(path: string): bool` | 检查是否为绝对路径 |
| `isRelative` | `static fn isRelative(path: string): bool` | 检查是否为相对路径 |
| `resolve` | `static fn resolve(base: string, relative: string): string` | 将相对路径解析为绝对路径 |
| `normalize` | `static fn normalize(path: string): string` | 规范化路径 |
| `withoutExt` | `static fn withoutExt(path: string): string` | 返回去掉扩展名的路径 |
| `changeExt` | `static fn changeExt(path: string, newExt: string): string` | 更改文件扩展名 |
| `compare` | `static fn compare(path1: string, path2: string): int` | 字典序比较两个路径 |
| `equals` | `static fn equals(path1: string, path2: string): bool` | 规范化后比较路径是否相等 |
| `parents` | `static fn parents(path: string): Array<string>` | 列出所有父目录路径 |
| `isChildOf` | `static fn isChildOf(parent: string, child: string): bool` | 检查 child 是否在 parent 下 |
| `relative` | `static fn relative(from: string, to: string): string` | 计算相对路径 |
| `separator` | `static fn separator(): string` | 返回平台路径分隔符 |

---

## 路径扩展函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `hasExt` | `fn hasExt(path: string, ext: string): bool` | 检查路径是否有指定扩展名 |
| `getExts` | `fn getExts(path: string): string` | 获取文件名中的所有扩展名（如 `".tar.gz"`） |

---

## 注意事项

- `Path.equals()` 先规范化再比较，避免 `"/foo/bar"` 和 `"/foo/./bar"` 的差异
- `separator()` 在 Unix 上返回 `"/"`，在 Windows 上返回 `"\\"`
