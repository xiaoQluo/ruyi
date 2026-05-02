# Ruyi 发展路线图

> **愿景**: 让 Ruyi 成为高性能、类型安全、开发者友好的原生编译语言，从编译器核心走向完整生态。

---

## 目录

- [当前状态总览](#当前状态总览)
- [三大阶段](#三大阶段)
- [第一阶段：基础库实现](#第一阶段基础库实现)
  - [1.1 编译器关键补全](#11-编译器关键补全)
  - [1.2 运行时与代码生成集成](#12-运行时与代码生成集成)
  - [1.3 标准库扩展](#13-标准库扩展)
- [第二阶段：生态建设与包管理](#第二阶段生态建设与包管理)
  - [2.1 包管理器 ruyipkg](#21-包管理器-ruyipkg)
  - [2.2 包注册中心](#22-包注册中心)
  - [2.3 构建系统](#23-构建系统)
  - [2.4 第三方生态加速](#24-第三方生态加速)
- [第三阶段：开发工具支持](#第三阶段开发工具支持)
  - [3.1 LSP 语言服务器](#31-lsp-语言服务器)
  - [3.2 格式化工具 ruyifmt](#32-格式化工具-ruyifmt)
  - [3.3 代码检查工具 ruyilint](#33-代码检查工具-ruyilint)
  - [3.4 调试器支持](#34-调试器支持)
  - [3.5 文档生成器 ruyidoc](#35-文档生成器-ruyidoc)
  - [3.6 编辑器集成](#36-编辑器集成)
- [里程碑与时间线](#里程碑与时间线)

---

## 当前状态总览

### 编译器各模块完成度

| 模块 | 完成度 | 关键差距 |
|------|--------|----------|
| **词法分析器 (Lexer)** | ✅ 完成 | 全部 spec token 已实现 |
| **语法分析器 (Parser)** | ⚠️ 85% | 部分测试 `#[ignore]`；macro 规则、type alias、`new(参数)` 等待支持 |
| **类型检查器 (TypeChecker)** | ⚠️ 80% | trait bound 检查始终返回 true（unsound）；class 继承深度类型检查不足 |
| **宏展开 (Macro Expand)** | ⚠️ 60% | `$()` 重复模式未实现；模板仅支持 `$Identifier` |
| **代码生成 (CodeGen)** | ❌ 40% | **类编译为空操作**；数组/对象/模板字符串无法编译；try/catch/throw 未实现；match 未实现 |
| **垃圾回收 (GC)** | ⚠️ 70% | 运行时 GC 完整，但 codegen 不生成 GC 安全代码 |
| **驱动器 (Driver)** | ✅ 90% | 流水线完整，缺少增量编译 |
| **运行时 (Runtime)** | ⚠️ 75% | 异步调度器、异常机制均在运行时实现，但未集成到 codegen |

### 标准库完成度

| 模块 | 完成度 | 说明 |
|------|--------|------|
| core.ry | ✅ 完成 | String/Int/Float/Bool 内建方法 |
| string.ry | ✅ 完成 | 完整字符串操作（30+ 方法） |
| io.ry | ✅ 完成 | print/println/readLine + File 类（含异步版） |
| error.ry | ✅ 完成 | 9 种错误子类型 + assert/assertNotNull |
| option.ry | ✅ 完成 | Option\<T> 枚举 + 13 个方法 |
| result.ry | ✅ 完成 | Result\<T,E> 枚举 + 15 个方法 |
| process.ry | ✅ 完成 | Process 类 + 环境变量 + 信号常量 |
| path.ry | ✅ 完成 | Path 类（15+ 静态方法） |
| collections.ry | ⚠️ 80% | Array/Map/Set + Iterator，但 SetIterator 为 stub |
| **math.ry** | ❌ 缺失 | 无三角函数、sqrt、pow、log 等 |
| **json.ry** | ❌ 缺失 | 无 JSON 解析/序列化 |
| **time.ry** | ❌ 缺失 | 无 Date/DateTime/sleep |
| **random.ry** | ❌ 缺失 | 无随机数生成 |
| **regex.ry** | ❌ 缺失 | 无正则表达式 |
| **fmt.ry** | ❌ 缺失 | 无格式化输出 |
| **test.ry** | ❌ 缺失 | 无测试框架 |

### 集成测试覆盖

- **35 个集成测试用例**，覆盖 basic, async, codegen, control_flow, errors, functions, stdlib, types
- **spec 特性覆盖率约 30-40%**
- **无覆盖的关键特性**: trait/impl、class 继承、module import/export、macro、for-in/for-of、break/continue 标签、模板字符串、可选链 `?.`

---

## 三大阶段

```
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│  Phase 1          Phase 2           Phase 3                         │
│  基础库实现        生态建设与包管理     开发工具支持                    │
│  (0-6 月)         (6-12 月)         (12-18 月)                     │
│                                                                     │
│  ┌───────────┐   ┌──────────────┐   ┌──────────────┐               │
│  │ 编译器补全 │   │ 包管理器      │   │ LSP 服务器   │               │
│  │ 运行时集成 │   │ 包注册中心    │   │ ruyifmt     │               │
│  │ 标准库扩展 │   │ 构建系统      │   │ ruyilint    │               │
│  │ 测试覆盖   │   │ 第三方生态    │   │ 调试器       │               │
│  └───────────┘   └──────────────┘   │ ruyidoc     │                │
│                                      └──────────────┘               │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 第一阶段：基础库实现

> **目标**: 让 Ruyi 从"能解析和类型检查"变为"能编译运行真实程序"

### 1.1 编译器关键补全

按依赖顺序排列，前项是后项的基础。

#### P0 — 代码生成核心（阻塞一切）

| 任务 | 优先级 | 依赖 | 说明 |
|------|--------|------|------|
| 运行时函数调用集成 | 🔴 关键 | 无 | codegen 中调用 `ruyi_alloc`/`ruyi_dealloc` 等运行时函数，使字符串和数组可分配 |
| 字符串运行时 | 🔴 关键 | 运行时集成 | 生成调用 `string_length`、`string_concat` 等运行时函数的 LLVM IR |
| 数组运行时 | 🔴 关键 | 运行时集成 | 生成调用 `array_new`、`array_push`、`array_get` 等运行时函数的 LLVM IR |
| 类代码生成 | 🔴 关键 | 运行时集成 | 实现 `compile_class()`——当前为空操作，class 声明不产生任何 LLVM IR |
| 对象字面量代码生成 | 🔴 关键 | 类代码生成 | `{ key: value }` 语法编译为堆分配结构体 |

#### P1 — 控制流与模式匹配

| 任务 | 优先级 | 依赖 | 说明 |
|------|--------|------|------|
| try/catch/finally 代码生成 | 🟠 高 | 运行时集成 | 使用 LLVM landing pad 机制，运行时已有 `LandingPadGenerator` |
| throw 语句代码生成 | 🟠 高 | try/catch | 生成异常抛出调用 |
| match 语句代码生成 | 🟠 高 | 无 | 模式匹配编译为条件分支链 |
| for-in / for-of 循环代码生成 | 🟠 高 | 迭代器运行时 | `for (let k in obj)` 和 `for (let v of arr)` |
| 复合赋值运算符 | 🟡 中 | 无 | `+=`、`-=`、`*=` 等 |
| break/continue 标签 | 🟡 中 | 无 | `break outer;`、`continue outer;` |

#### P2 — 类型系统完善

| 任务 | 优先级 | 依赖 | 说明 |
|------|--------|------|------|
| trait bound 实际检查 | 🟠 高 | 无 | 当前 `check_bounds()` 始终返回 true——需要真正验证 impl 是否满足 trait 约束 |
| class 继承深度类型检查 | 🟡 中 | 无 | `extends` 继承的完整类型检查（方法签名兼容性、字段覆盖） |
| 可选链 `?.` 代码生成 | 🟡 中 | 无 | `obj?.method()` 的 null-short-circuit 编译 |
| null 断言 `!` 代码生成 | 🟡 中 | 无 | `value!` 运行时空值检查 |
| 宏重复模式 `$()*` `$()+` | 🟡 中 | 无 | macro_expand/pattern.rs 当前仅支持 `$Identifier` |

#### P3 — 测试覆盖倍增

| 任务 | 优先级 | 依赖 | 说明 |
|------|--------|------|------|
| trait/impl 集成测试 | 🟡 中 | 类代码生成 | 当前零覆盖 |
| class 继承集成测试 | 🟡 中 | 类代码生成 | 当前零覆盖 |
| module import/export 测试 | 🟡 中 | module 解析 | 当前零覆盖 |
| 异步全流程测试 | 🟡 中 | 异步调度器集成 | 当前仅有单元测试 |
| 修复 SetIterator.next() stub | 🟡 中 | 无 | 当前始终返回 None |
| CI 配置 (GitHub Actions) | 🟢 低 | 无 | 自动化构建和测试 |
| 基准测试 (criterion) | 🟢 低 | 无 | `crates/ruyic/benches/` |

### 1.2 运行时与代码生成集成

```
当前状态:

  ruyic (编译器)                    ruyi_runtime (运行时库)
  ┌──────────────┐                ┌──────────────────┐
  │ codegen/     │                │ gc.rs            │ ✅ 分代GC
  │   expr.rs    │──???──→        │ arc.rs           │ ✅ 引用计数
  │   decl.rs    │                │ async_runtime.rs │ ✅ 调度器
  │   stmt.rs    │                │ exception.rs     │ ✅ 异常机制
  └──────────────┘                └──────────────────┘
       ❌ 未连接 ❌

目标状态:

  ruyic (编译器)                    ruyi_runtime (运行时库)
  ┌──────────────┐                ┌──────────────────┐
  │ codegen/     │                │ gc.rs            │ ✅ 分代GC
  │   expr.rs    │──LLVM IR──→   │ arc.rs           │ ✅ 引用计数
  │   decl.rs    │   extern fn    │ async_runtime.rs │ ✅ 调度器
  │   stmt.rs    │                │ exception.rs     │ ✅ 异常机制
  │   runtime/   │                └──────────────────┘
  │   bindings.rs│ ← 声明所有运行时函数签名
  │   gc_ir.rs   │ ← GC root 注册 / 写屏障
  └──────────────┘
```

**核心任务**:

| 任务 | 说明 |
|------|------|
| 运行时函数绑定 | 新建 `codegen/runtime_bindings.rs`，声明所有 `_ruyi_*` 运行时函数的 LLVM 签名 |
| GC root 注册 | 函数入口/出口注册 GC root；赋值时插入 write barrier |
| 异步调度器集成 | `main()` 入口初始化 `Scheduler`；`async fn` 编译为 Future + 恢复点 |
| 异常机制连接 | try 块生成 landing pad；throw 生成 `ruyi_throw` 调用 |
| 链接器集成 | Driver 将 `ruyi_runtime.a` 静态链接到输出二进制 |

### 1.3 标准库扩展

按实用性排序——缺少这些，开发者写不出有用程序。

#### P0 — 无此不能写出真实程序

| 模块 | 关键 API | 实现策略 |
|------|---------|---------|
| **math** | `PI`, `E`, `sqrt`, `pow`, `sin`, `cos`, `tan`, `log`, `abs`, `ceil`, `floor`, `round` | 内建函数 `__math_*` 调用 libc `math.h` |
| **json** | `JSON.parse(str): dyn`, `JSON.stringify(val): string` | 初始版本用递归下降解析器；后续可换用 simdjson |
| **time** | `Time.now(): float`, `Time.sleep(ms: int)`, `Date` 类 | 内建函数 `__time_*` 调用 POSIX `clock_gettime` |
| **random** | `Random.nextInt(max: int): int`, `Random.nextFloat(): float` | 内建函数 `__random_*`；初始用 xorshift，后续换 ChaCha8 |

#### P1 — 让标准库更完整

| 模块 | 关键 API | 实现策略 |
|------|---------|---------|
| **fmt** | `fmt.Printf(template, args...)`, `fmt.Sprintf(template, args...): string` | 内建格式化，类似 Rust `print!` 宏 |
| **regex** | `Regex.new(pattern: string): Regex`, `.match(str)`, `.replace(str, repl)` | 内建函数绑定 RE2 或 PCRE |
| **collections 扩展** | `Array.sort()`, `Array.indexOf()`, `Array.contains()`, `Array.first/last`, `Iterator.enumerate()`, `Iterator.zip()`, `Iterator.take(n)`, `Iterator.skip(n)` | 纯 Ruyi 实现 + 少量内建函数 |
| **os** | `Os.env(name): string?`, `Os.cwd(): string`, `Os.setEnv(name, val)` | 内建函数调用 POSIX API |

#### P2 — 高级特性

| 模块 | 关键 API | 实现策略 |
|------|---------|---------|
| **channel** | `Channel\<T>.new()`, `.send(val)`, `.receive(): T` | 基于运行时调度器的异步通道 |
| **sync** | `Mutex\<T>`, `RwLock\<T>`, `Semaphore` | 内建函数调用 pthread 同步原语 |
| **base64** | `Base64.encode(data: Array\<int>): string`, `Base64.decode(str): Array\<int>` | 纯 Ruyi 实现 |
| **hash** | `Hash.sha256(data): string`, `Hash.md5(data): string` | 内建函数绑定 OpenSSL 或 Blake3 |

---

## 第二阶段：生态建设与包管理

> **目标**: 让开发者能发现、发布、依赖第三方库，构建真实项目

```
生态架构:

  ruyi.toml                 ruyi.lock                包注册中心
  ┌──────────────┐         ┌──────────────┐         ┌──────────────┐
  │ [package]    │         │ # 自动生成    │         │ ruyi-registry │
  │ name = "..." │─→解析→  │ sha256 = ... │─→下载→  │   .io         │
  │ version = .. │         │ deps = ...   │         │              │
  │ [deps]       │         └──────────────┘         └──────────────┘
  │ http = "1.2" │              ↑
  └──────────────┘              │
         ↑                      │
         │                      │
  ┌──────┴───────┐        ┌─────┴──────┐
  │ build.ry     │        │ 缓存仓库    │
  │ 自定义构建步骤 │        │ ~/.ruyi/cache│
  └──────────────┘        └────────────┘
```

### 2.1 包管理器 ruyipkg

#### 清单格式 (`ruyi.toml`)

```toml
[package]
name = "my-app"
version = "0.1.0"
authors = ["Author <email>"]
edition = "2026"
description = "A Ruyi application"
license = "MIT"

[dependencies]
http = ">=1.0.0"
json = "^2.1.0"
regex = { version = "1.5.0", optional = true }

[dev-dependencies]
test = "^0.1.0"

[features]
default = ["regex"]
full = ["regex", "http/tls"]

[build]
target = "native"           # 或 "wasm32", "aarch64-linux"
opt_level = 2               # 0, 1, 2
```

#### 核心命令

```
ruyi init [name]           # 初始化新项目
ruyi build                 # 编译项目
ruyi run                   # 编译并运行
ruyi test                  # 运行测试
ruyi check                 # 仅类型检查
ruyi fmt                   # 格式化代码
ruyi add <package>         # 添加依赖
ruyi remove <package>       # 移除依赖
ruyi update                 # 更新依赖
ruyi publish                # 发布到注册中心
ruyi install <package>     # 全局安装可执行包
ruyi run <package>          # 直接运行远程包
```

#### 依赖解析

- **最小版本选择 (MVS)** 策略（同 Go modules）
- 语义化版本 (SemVer)
- `ruyi.lock` 包含精确版本 + 内容哈希
- 支持 Git URL 作为依赖源：`http = { git = "https://github.com/user/http", rev = "abc123" }`
- 支持 path 本地依赖：`utils = { path = "../utils" }`

#### 工作空间

```toml
# 根目录 ruyi.toml
[workspace]
members = ["crates/*", "examples/*"]

# crates/http/ruyi.toml
[package]
name = "http"
version = "0.1.0"
```

### 2.2 包注册中心

| 阶段 | 功能 |
|------|------|
| MVP | Git URL 依赖 + 缓存（无注册中心） |
| V1 | `ruyi-registry.io` 中心化注册；`ruyi publish` / `ruyi add` |
| V2 | 搜索、评分、安全审计、版本弃用通知 |

#### 注册中心协议设计

```
GET  /api/v1/packages/{name}              # 包信息
GET  /api/v1/packages/{name}/{version}    # 特定版本
GET  /api/v1/packages/{name}/{version}.ry  # 下载源码
POST /api/v1/packages                     # 发布新版本
GET  /api/v1/search?q={query}             # 搜索包
```

- 内容寻址存储（类似 Zig 的哈希方案）
- OIDC 可信发布（类似 PyPI）
- SBOM (Software Bill of Materials) 支持

### 2.3 构建系统

#### 增量编译

```
构建 DAG:

  source.ry ──→ Lexer ──→ Tokens ──→ Parser ─–→ AST ──→ ...
                                                   │
                                     指纹 = hash(source + 依赖)
                                                   │
                                     未改变 → 跳过编译
                                     已改变 → 重新编译
```

- **模块级指纹**: 每个模块计算内容哈希；未改变的模块跳过
- **并行编译**: 独立模块并行编译（类似 `cargo -j`）
- **内容寻址缓存**: `~/.ruyi/cache/` 存储编译产物，哈希为键

#### 跨平台编译

```bash
ruyi build --target aarch64-linux    # ARM64 Linux
ruyi build --target x86_64-macos     # x64 macOS
ruyi build --target wasm32            # WebAssembly (未来)
```

### 2.4 第三方生态加速

> "crates.io 是 Rust 的标准库" —— 生态即标准库

**优先鼓励开发的社区库**:

| 类别 | 包名举例 | 说明 |
|------|---------|------|
| HTTP | `http`, `http-server` | HTTP 客户端/服务器 |
| 数据库 | `sql`, `postgres`, `redis` | 数据库驱动 |
| 序列化 | `serde`, `json`, `toml`, `yaml` | (json 可能在 stdlib) |
| 测试 | `test`, `mock`, `bench` | 测试框架 |
| 日志 | `log`, `tracing` | 结构化日志 |
| CLI | `cli`, `term` | 命令行参数解析、终端颜色 |
| 加密 | `crypto`, `tls` | TLS、哈希、签名 |
| Web 框架 | `rouille`, `forge` | Web 应用框架 |

**官方胶水包**（stdlib 与社区之间的桥梁）:

| 包 | 说明 |
|-----|------|
| `ruyi-test` | 测试框架（`@test` 属性、断言、覆盖率） |
| `ruyi-cli` | CLI 参数解析 |
| `ruyi-http` | HTTP 客户端/服务器基础 |

---

## 第三阶段：开发工具支持

> **目标**: 让 Ruyi 在 IDE 中获得一流的开发体验

### 3.1 LSP 语言服务器

**架构**:

```
┌─────────────┐     LSP Protocol     ┌──────────────────┐
│ VS Code      │◄───────────────────→│ ruyi-language-    │
│ Vim/Neovim  │     JSON-RPC         │ server            │
│ Emacs       │                      │                   │
│ Helix       │                      │ ┌───────────────┐ │
│ Zed         │                      │ │ Incremental   │ │
└─────────────┘                      │ │ Parser        │ │
                                     │ ├───────────────┤ │
                                     │ │ Type Checker  │ │
                                     │ │ (reused from  │ │
                                     │ │  compiler)    │ │
                                     │ ├───────────────┤ │
                                     │ │ Index DB      │ │
                                     │ │ (symbol table)│ │
                                     │ └───────────────┘ │
                                     └──────────────────┘
```

**MVP 功能 (LSP v1)**:

| 功能 | LSP 方法 | 说明 |
|------|---------|------|
| 诊断 | `textDocument/publishDiagnostics` | 编译错误/警告实时显示 |
| 悬停类型 | `textDocument/hover` | 显示变量类型、函数签名 |
| 跳转定义 | `textDocument/definition` | 跳到符号定义处 |
| 查找引用 | `textDocument/references` | 列出所有使用位置 |
| 自动补全 | `textDocument/completion` | 关键字、函数、变量补全 |
| 符号重命名 | `textDocument/rename` | 安全重命名跨文件 |

**V2 功能**:

| 功能 | 说明 |
|------|------|
| 代码操作 | 自动修复、添加 import |
| Inlay Hints | 类型注解、参数名 |
| 语义高亮 | 基于类型的语法高亮 |
| 文档符号 | `textDocument/documentSymbol` 大纲视图 |
| 工作区符号 | `workspace/symbol` 全项目搜索 |
| 签名帮助 | `textDocument/signatureHelp` 函数参数提示 |

**实现策略**:
- 复用编译器前端（Lexer + Parser + TypeChecker）
- 增量分析：只重分析改动的文件及其依赖
- 索引数据库：SQLite 或内存索引，存储符号表和引用关系
- 独立 crate: `ruyi-language-server`

### 3.2 格式化工具 ruyifmt

**设计原则**（学习 gofmt）:
- **零配置**: 不提供格式化选项，一种风格统一所有
- **基于 AST**: 解析后重新打印，不是文本替换
- **范围格式化**: LSP 调用时只格式化编辑范围

**核心规则**:

```ruyi
// 缩进：2 空格
fn example() {
  let x = 1;
}

// 二元运算符空格
let sum = a + b;

// 逗号后空格
fn add(a: int, b: int): int { ... }

// 长行断行（max_width=100）
let result = someVeryLongFunctionName(
  arg1, arg2, arg3
);
```

### 3.3 代码检查工具 ruyilint

**分为三级**:

| 级别 | 示例规则 |
|------|---------|
| **错误 (Error)** | 未使用变量、不可达代码、类型不匹配 |
| **警告 (Warn)** | 冗余空模式、不必要的 `!` 断言、`dyn` 类型可窄化 |
| **建议 (Suggest)** | 可简化的 match、可合并的 if-else、更显式的类型注解 |

**自动修复**:

```
warning: unnecessary null assertion
  --> src/main.ry:10:15
   |
10 |   let name: string = user?.name!;
   |                       ^^^^^^^^^^ help: use nullish coalescing instead
   |
10 |   let name: string = user?.name ?? "unknown";
```

### 3.4 调试器支持

**方案选择**: 封装 LLDB/LLDAP，提供 Ruyi 感知的调试体验

```
ruyi-dap (Debug Adapter Protocol)
┌──────────────┐
│ VS Code      │◄── DAP ──→│ ruyi-dap │◄── LLDB ──→│ ruyi binary │
│ IDE          │            │  adapter  │             │ + DWARF     │
└──────────────┘            └──────────┘             └─────────────┘
```

**MVP 功能**:
- 断点设置（行断点、函数断点）
- 单步执行（step over, step into, step out）
- 变量查看（局部变量、全局变量）
- 调用栈显示
- 条件断点

**依赖**: `--debug` 编译标志生成 DWARF 调试信息

### 3.5 文档生成器 ruyidoc

**从 `/** */` 注释生成 HTML 文档**:

```ruyi
/**
 * 计算两个数的最大值。
 *
 * @param a 第一个数
 * @param b 第二个数
 * @returns 较大的数
 *
 * @example
 * let m = max(3, 5);  // m === 5
 */
fn max(a: int, b: int): int {
  return if a > b { a } else { b };
}
```

**输出**: 跨引用 HTML 文档（类、函数、trait、模块索引）

### 3.6 编辑器集成

| 编辑器 | 形式 | 功能 |
|--------|------|------|
| **VS Code** | 扩展 (`ruyi-vscode`) | 语法高亮 + LSP + 调试 |
| **Neovim** | LSP 配置 | 通过 `nvim-lspconfig` |
| **Vim** | LSP 配置 | 通过 `vim-lsp` 或 `coc.nvim` |
| **Emacs** | LSP 配置 | 通过 `lsp-mode` |
| **Helix** | LSP 配置 | 内置 LSP 支持 |
| **Zed** | 扩展 | LSP + 语法高亮 |

**VS Code 扩展优先实现**:
- `.ry` 文件识别和语法高亮（TextMate 语法）
- LSP 客户端配置
- 调试适配器配置
- 代码片段（snippets）

---

## 里程碑与时间线

### Phase 1 — 基础库实现（0-6 月）

```
Month 1-2: 编译器核心补全
  ├── 运行时-代码生成集成（字符串/数组/分配器）
  ├── 类代码生成实现
  ├── try/catch/throw 代码生成
  └── CI (GitHub Actions) 配置

Month 3-4: 控制流 + 类型系统
  ├── match 语句代码生成
  ├── for-in / for-of 代码生成
  ├── trait bound 实际检查
  ├── 标准库 P0: math, json, time, random
  └── 集成测试倍增 (35 → 100+)

Month 5-6: 标准库 P1 + 测试
  ├── fmt, regex, collections 扩展
  ├── 可选链 / null 断言代码生成
  ├── 复合赋值 / 标签 break/continue
  ├── 修复 SetIterator
  └── 基准测试 suite
```

**Phase 1 交付物**: Ruyi 可以编译运行包含类、异常处理、模式匹配、完整标准库的真实程序

### Phase 2 — 生态建设与包管理（6-12 月）

```
Month 7-8: 包管理器 MVP
  ├── ruyi.toml 解析器
  ├── 依赖解析器 (MVS)
  ├── Git URL 依赖获取
  ├── ruyi.lock 生成
  └── ruyi build / ruyi run / ruyi test

Month 9-10: 包注册中心 + 构建系统
  ├── 增量编译（模块级指纹）
  ├── 并行构建
  ├── ruyi-registry.io 原型
  ├── ruyi publish / ruyi add
  └── 工作空间支持

Month 11-12: 生态启动
  ├── 官方胶水包: ruyi-test, ruyi-cli, ruyi-http
  ├── 标准库 P2: channel, sync, base64, hash
  ├── 社区包激励计划
  └── 0.1.0 正式发布
```

**Phase 2 交付物**: 完整的包管理生态，开发者可以发布和依赖第三方库

### Phase 3 — 开发工具支持（12-18 月）

```
Month 13-14: LSP v1
  ├── 增量解析器
  ├── 诊断推送
  ├── 悬停类型 + 跳转定义
  ├── 自动补全
  └── ruyifmt 格式化器

Month 15-16: 工具链深化
  ├── ruyilint 代码检查
  ├── 查找引用 + 符号重命名
  ├── VS Code 扩展 (语法高亮 + LSP)
  ├── ruyidoc 文档生成器
  └── Neovim/Vim/Emacs 配置模板

Month 17-18: 调试 + 成熟
  ├── DAP 调试器适配器
  ├── 条件断点 + 变量查看
  ├── LSP v2 (代码操作, inlay hints)
  ├── 性能分析集成
  └── 0.2.0 正式发布
```

**Phase 3 交付物**: 一流 IDE 支持，开发者体验接近 Rust/Go 水平

---

## 关键成功因素

### 1. 先解决编译器阻塞项

**当前最大瓶颈: 代码生成不完整**。类编译为空操作，数组/对象/模板字符串无法编译，try/catch 未连接。没有这些，任何上层生态都是空中楼阁。

> **优先级**: 运行时-代码生成集成 > 类代码生成 > stdlib > 包管理 > 开发工具

### 2. 解决 M×N 集成问题

学习 Zig 的教训：新语言的最大障碍不是语言本身，而是工具链生态。

> 设计包管理器时，**协议先行**——确保 Dependabot、Snyk、IDE 等第三方工具可以轻松支持 Ruyi。

### 3. 平衡极简与实用

- Rust: "crates.io 就是标准库"——极简 std + 丰富生态
- Go: "电池内置"——stdlib 包含 HTTP、JSON、测试

> **Ruyi 策略**: 最小核心 + 官方推荐库。stdlib 包含基础类型和 I/O；HTTP、数据库等留给生态，但官方维护胶水包 (`ruyi-http`, `ruyi-test`)。

### 4. 从第一天起考虑安全

- 锁文件包含内容哈希（防篡改）
- 语义化版本严格执行（破坏性变更必须升主版本号）
- 后续: 包签名、安全审计集成

### 5. 让编译器对开发者友好

> 参考 Rust: "友好的编译器、一流的文档、顶级的工具链"

- 错误信息包含代码位置、修复建议、相关文档链接
- `ruyi check` 快速反馈（不编译，仅类型检查）
- LSP 让 IDE 成为开发环境的一部分