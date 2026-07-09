# Ruyi 发展路线图

> **版本**: 0.5.4 | **日期**: 2026-07-07 | **状态**: 已发布
>
> [English](roadmap.md)

## 概要

Ruyi 是一门通过 LLVM 编译为原生机器码的编程语言。本路线图定义三个阶段：

1. **基础库实现（v0.2–v0.5）** — 修复 stdlib 缺陷，补全核心模块（math/time/json/regex/random），完善代码生成（类/对象/try-catch/for循环），对接运行时（GC/async/异常）
2. **生态建设与包管理（v0.6–v0.9）** — 包管理器、注册中心、锁文件、工作空间、构建系统
3. **开发工具支持（v1.0+）** — LSP、格式化器、代码检查器、测试运行器、文档生成器、调试器、IDE 插件

---

## 版本发布状态

| 版本 | 分支 | 状态 | 发布日期 | 标签 |
|------|------|------|---------|------|
| v0.2 | dev/v0.2 | ✅ 已发布 | 2026-05 | v0.2.0 (待补打) |
| v0.3 | dev/v0.3 | ✅ 已发布 | 2026-05 | v0.3.0 (待补打) |
| v0.4 | dev/v0.4 | ✅ 已发布 | 2026-05 | v0.4.0 |
| v0.5 | dev/v0.5 | ✅ 已发布 | 2026-05 | v0.5.1 |
| v0.5.2 | dev/v0.5.2 | ✅ 已发布 | 2026-05 | v0.5.2 |
| v0.5.3 | dev/v0.5.3 | ✅ 已发布 | 2026-05 | v0.5.3 |
| v0.5.4 | dev/v0.5.4 | ✅ 已发布 | 2026-07 | v0.5.4 |

---

## 现状评估

### 编译器模块

| 模块 | 完成度 | 主要缺口 |
|------|--------|----------|
| **词法分析器** | ~95% | 文档注释未特殊处理 |
| **解析器** | ~65% | match 守卫、计算属性名、泛型语法需验证 |
| **类型检查器** | ~90% | Trait 约束未执行；超特质未检查；`impl Trait for Type` 不完整 |
| **代码生成** | ~60% | **不支持成员访问（`obj.prop`）、数组/对象字面量、BigInt/模板字符串、for/for-in/for-of、try/catch、break/continue、类布局** |
| **宏展开** | ~60% | 复杂重复模式、卫生性边界情况 |
| **驱动器** | ~80% | 运行时已链接；模块系统内联而非正式导入 |
| **GC** | 编译器端 ~70% / 运行时 85% | 编译器端 GC 为存根；运行时 GC 未对接代码生成 |
| **运行时** | 编译器端 ~60% / 运行时库 70% | **`ruyi_await` 为空操作；async 同步运行；运行时无 stdlib 类型** |

### 标准库

| 模块 | 行数 | 状态 | 缺口 |
|------|------|------|------|
| `core.ry` | 219 | ✅ 完整 | String/Int/Float/Bool 内建函数正常工作 |
| `string.ry` | 312 | ✅ 完整 | 与 core.ry String 模块重叠 |
| `io.ry` | 192 | ✅ 完整 | 控制台 I/O + File 类（含异步变体） |
| `error.ry` | 230 | ✅ 完整 | 9 种错误类型 + assert/assertNotNull |
| `option.ry` | 175 | ✅ 完整 | Option\<T\> 枚举含全部组合器 |
| `result.ry` | 190 | ✅ 完整 | Result\<T,E\> 枚举含全部组合器 |
| `process.ry` | 509 | ✅ 完整 | 进程管理 + 环境变量 |
| `path.ry` | 262 | ✅ 完整 | 路径操作（含异步变体） |
| `collections.ry` | 529 | ⚠️ 部分 | **SetIterator.next() 为损坏的存根**；缺 sort/contains/indexOf/first/last |

**关键缺失模块**: `math`、`time/datetime`、`json`、`regex`、`random`、`fmt`、`net`、`buffer`、`test`

### 测试基础设施

| 领域 | 数量 | 状态 |
|------|------|------|
| 单元测试（词法/解析/类型等） | ~2400+ | ✅ 稳固 |
| 集成测试（.ry 文件） | 58 个用例 | ⚠️ 仅覆盖 ~30% 语言规范 |
| 运行时测试 | 3 个文件 | ⚠️ 基础覆盖 |
| 性能基准 | criterion 套件 | ✅ 存在 |
| CI/CD | ❌ 无 | 无 GitHub Actions |
| 属性测试 | ❌ 无 | 无 proptest |
| 模糊测试 | ❌ 无 | 无 cargo-fuzz |

**零集成测试的语言特性**: class/OOP、trait 系统、宏、import/export、类型别名、深层模式匹配、解构、`for-of`/`for-in`、bigint、`never` 类型、ARC 类

---

## 阶段一：基础库实现（v0.2–v0.5）

### 目标

让 Ruyi 能够端到端编写真实程序：类可用、异常可用、async 真正异步运行、stdlib 覆盖 80% 使用场景。

### v0.2 — 代码生成补全（优先级：关键）

> 没有成员访问和类布局，任何非平凡程序都无法编译。

| # | 任务 | 描述 | 优先级 |
|---|------|------|--------|
| 1.1 | **类布局与成员访问** | 实现 `compile_class`（当前为空操作）：字段布局、`self.field` 访问、`new` 构造器、方法分派 | P0 ✅ |
| 1.2 | **对象字面量代码生成** | 将 `{ key: value }` 编译为运行时结构 | P0 ✅ |
| 1.3 | **数组字面量代码生成** | 将 `[1, 2, 3]` 编译为运行时数组，支持 `push`/`pop`/索引访问 | P0 ✅ |
| 1.4 | **字符串拼接** | `+` 运算符用于字符串（当前仅数字 `+` 可用） | P0 ✅ |
| 1.5 | **for 循环代码生成** | C 风格 `for`、`for-in`、`for-of`（当前均不支持） | P0 ✅ |
| 1.6 | **break/continue** | 已有 `loop_stack`，只需代码生成 | P1 ✅ |
| 1.7 | **try/catch/finally** | `ruyi_runtime` 中已有 landing pad 支持；对接到代码生成 | P0 |
| 1.8 | **throw 表达式** | 映射为运行时 `throw_exception` 调用 | P1 |
| 1.9 | **match 语句** | 将 match 编译为链式 if-else 或 switch | P1 |
| 1.10 | **模板字面量** | 将 `` `Hello ${name}` `` 编译为字符串拼接 | P1 |
| 1.11 | **BigInt 字面量** | 将 `100n` 编译为运行时 bigint 类型 | P2 |
| 1.12 | **成员表达式** | `obj.prop` 和 `obj?.prop` 代码生成（当前不支持） | P0 ✅ |
| 1.13 | **方法调用** | `obj.method(args)` 代码生成，含 `self` 绑定 | P0 ✅ |

**进度（2026-07-09，v0.2-codegen-gaps 变更，T7/T8/T9）**：

第一、二批代码生成工作已在分支 `dev/v0.2-codegen-gaps` 上落地：
- **T2** (`65f514c`) 修正类分配尺寸（1.1 部分）。
- **T3** (`bed00d7`) 解析类字段与自身方法用于成员访问（1.12、1.13）。
- **T4** (`6618b11`) 通过 `loop_stack` 接通带标签的 `break`/`continue`（1.6）。
- **T6** (`fc01bcb`) 新增 `ruyi_obj_get` / `ruyi_obj_keys` FFI（1.2）。
- **T8**（本次变更）新增 5 个示例 + 8 个集成测试夹具，覆盖各项能力。

剩余缺口在自动加载的 `stdlib/collections.ry` 无法完成类型检查：T9 (`809e6c9`) 将 `RangeError` / `ArrayIterator` 识别为 Named 类型，但未让它们可作为构造器调用，因此 `throw RangeError("...")` 仍会在用户代码运行前中止编译。`tests/codegen.rs` 中 27 个 `#[ignore]` 代码生成测试（含 T8 新增的 8 个夹具）均带有引用此 stdlib 缺口的 `// TODO:` 阻塞说明，将在后续变更使 `RangeError` / `ArrayIterator` 可构造后通过。

### v0.3 — 运行时对接（优先级：关键）

> 运行时 GC、async 调度器和异常处理已存在但未对接代码生成。

| # | 任务 | 描述 | 优先级 |
|---|------|------|--------|
| 2.1 | **链接运行时库** | 驱动器必须将 `ruyi_runtime` 链接到生成的二进制（当前使用裸 `cc`） | P0 |
| 2.2 | **GC 分配对接** | 用 `ruyi_gc_alloc`/`ruyi_gc_collect` 替换占位分配器 | P0 |
| 2.3 | **async 真正异步** | 用真正的 future 轮询替换空操作 `ruyi_await`，通过工作窃取调度器 | P0 |
| 2.4 | **`spawn` 内建函数** | 实现 `spawn(fn)` 在调度器上启动绿色线程 | P0 |
| 2.5 | **异常 landing pad** | 从 try/catch 代码生成调用 `ruyi_exception_try`/`ruyi_exception_catch` | P0 |
| 2.6 | **async GC 根** | `register_async_roots` 当前为空操作；注册挂起任务 | P1 |
| 2.7 | **线程本地 GC 堆** | 将多线程 GC 对接到 async 运行时 | P2 |

### v0.4 — 类型检查加固（优先级：高）

| # | 任务 | 描述 | 优先级 |
|---|------|------|--------|
| 3.1 | **执行 trait 约束** | `check_bounds()` 在 generics.rs 中当前始终返回 true；实际验证 impl 存在 | P0 |
| 3.2 | **超特质检查** | 填充并验证 `supertraits` 字段 | P1 |
| 3.3 | **完整 `impl Trait for Type`** | 支持独立 `impl Printable for string { ... }`（当前不完整） | P0 |
| 3.4 | **null 以外的类型缩窄** | `instanceof`、`typeof`、match 模式后的类型缩窄 | P1 |
| 3.5 | **穷尽性检查** | 验证 match 分支覆盖所有情况；不完整模式发出警告 | P1 |
| 3.6 | **自引用类型检查** | 类在字段类型中引用 `self` | P1 |

### v0.5 — 标准库扩展（优先级：高）

| # | 任务 | 描述 | 优先级 |
|---|------|------|--------|
| 4.1 | **修复 SetIterator** | `SetIterator.next()` 当前始终返回 `None`——实现正确的集合迭代 | P0 |
| 4.2 | **`math.ry`** | Pi、E、sqrt、pow、sin、cos、tan、asin、acos、atan、log、log10、exp、abs、min、max | P0 |
| 4.3 | **`time.ry`** | Duration、Timestamp、sleep（同步+异步）、日期格式化 | P0 |
| 4.4 | **`json.ry`** | JSON.parse、JSON.stringify 含类型安全反序列化 | P0 |
| 4.5 | **`random.ry`** | Random.nextInt、nextFloat、nextBool、nextBytes、seed | P1 |
| 4.6 | **`fmt.ry`** | 格式化字符串：`fmt.format("{} 今年 {} 岁", name, age)` | P1 |
| 4.7 | **`regex.ry`** | Regex 类：match、replace、split | P2 |
| 4.8 | **`test.ry`** | 内建测试框架：`@test` 属性、assert、assertEq、assertThrows | P1 |
| 4.9 | **扩展 `collections.ry`** | Array.sort、.contains、.indexOf、.first、.last、.slice、.concat；Iterator.takeWhile、.skipWhile、.chain、.enumerate、.zip、.sum、.product、.any、.all | P1 |
| 4.10 | **合并 `core.ry` + `string.ry`** | 重复的 String 方法；合并为一个模块 | P2 |
| 4.11 | **`buffer.ry`** | Buffer/ByteArray 类型用于二进制数据 | P2 |
| 4.12 | **`net.ry`** | TCPClient、TCPServer（基本套接字 I/O） | P2 |

---

## 阶段二：生态建设与包管理（v0.6–v0.9）

### 目标

让开发者能够分享代码、管理依赖、构建多包项目。

### v0.6 — 包管理器基础（优先级：高）

| # | 任务 | 描述 |
|---|------|------|
| 5.1 | **清单格式** | 定义 `ruyi.pkg`（TOML）：`[package]` name/version/edition、`[dependencies]` 含 semver、`[dev-dependencies]` |
| 5.2 | **锁文件生成** | `ruyi.lock` 含完整解析树（名称、版本、源、校验和） |
| 5.3 | **依赖解析** | SemVer 约束求解、冲突检测、最小版本选择 |
| 5.4 | **基于 Git 的依赖** | `dep = { git = "url", rev = "abc123" }` 支持（在注册中心之前） |
| 5.5 | **`ruyi build` 命令** | 编译项目含依赖解析，输出到 `target/` |
| 5.6 | **`ruyi run` 命令** | 构建 + 执行一步完成 |
| 5.7 | **`ruyi add/remove`** | 添加或删除依赖，自动更新锁文件 |
| 5.8 | **模块解析** | 映射 `import { foo } from "./bar"` 到依赖包；解析 `std::io` 到标准库 |

### v0.7 — 包注册中心（优先级：高）

| # | 任务 | 描述 |
|---|------|------|
| 6.1 | **注册中心 API** | 基于 HTTP 的稀疏索引：`GET /index/{name}`、`GET /api/v1/crates/{name}/{version}` |
| 6.2 | **`ruyi publish`** | 包验证（semver、文档、测试通过）+ 上传到注册中心 |
| 6.3 | **`ruyi install`** | 从注册中心下载并缓存包 |
| 6.4 | **Yank 支持** | 标记版本不可用（不删除） |
| 6.5 | **搜索** | `ruyi search <关键词>` 搜索包 |
| 6.6 | **文档托管** | 自动生成并托管文档到 `docs.ruyi-lang.org` |

### v0.8 — 工作空间与构建系统

| # | 任务 | 描述 |
|---|------|------|
| 7.1 | **工作空间支持** | `[workspace] members = ["crates/*"]` 用于 monorepo |
| 7.2 | **构建配置** | `[profile.debug]` / `[profile.release]` 含 optimization/debug/lto 设置 |
| 7.3 | **`--locked` / `--frozen` 标志** | 用于 CI：锁文件过期时报错 |
| 7.4 | **交叉编译** | `--target x86_64-unknown-linux-gnu` 通过 LLVM target triple |
| 7.5 | **构建脚本** | 可选 `build.ry` 用于代码生成、自定义步骤（类似 build.rs） |
| 7.6 | **增量编译** | 基于指纹的缓存：跳过未更改模块的重编译 |
| 7.7 | **远程构建缓存** | 内容寻址缓存位于 `~/.cache/ruyi/`，跨项目共享 |

### v0.9 — 生态种子

| # | 任务 | 描述 |
|---|------|------|
| 8.1 | **推荐包** | `ruyi-http`（HTTP 客户端/服务器）、`ruyi-serialize`（JSON/TOML）、`ruyi-cli`（参数解析） |
| 8.2 | **包模板** | `ruyi init --lib` / `ruyi init --bin` 脚手架 |
| 8.3 | **CI 模板** | `ruyi ci init` 生成 GitHub Actions 工作流 |
| 8.4 | **安全审计** | `ruyi audit` 检查依赖中的已知漏洞 |
| 8.5 | **依赖树** | `ruyi tree` 显示依赖图 |
| 8.6 | **过期检查** | `ruyi outdated` 报告可用的新版本 |

---

## 阶段三：开发工具支持（v1.0+）

### 目标

提供世界级开发体验：快速反馈、智能编辑、便捷调试。

### v1.0 — LSP 与格式化器（优先级：P0）

| # | 任务 | 描述 |
|---|------|------|
| 9.1 | **tree-sitter-ruyi** | 语法高亮、折叠、缩进的文法，用于任何编辑器 |
| 9.2 | **LSP 服务（v1）** | 诊断（解析+类型错误）、跳转定义、悬停、补全、文档符号 |
| 9.3 | **`ruyi fmt`** | 固执的格式化器：4空格缩进、max_width=100、Unix 换行。最小配置：`ruyifmt.toml` |
| 9.4 | **VS Code 扩展** | 通过 tree-sitter 语法高亮、LSP 集成、保存时格式化 |
| 9.5 | **JetBrains 插件** | Grammar Kit + LSP 集成，用于 IntelliJ/WebStorm |

### v1.1 — 测试运行器与代码检查（优先级：P1）

| # | 任务 | 描述 |
|---|------|------|
| 10.1 | **`ruyi test` 运行器** | 发现 `@test fn` 函数，并行运行，按名称过滤，捕获输出 |
| 10.2 | **`@test` 属性** | 标记函数为测试；`@test fn test_add() { assert_eq(1+1, 2); }` |
| 10.3 | **`@bench` 属性** | 含统计分析的基准测试函数 |
| 10.4 | **测试报告器** | TAP、JUnit XML、JSON 输出格式 |
| 10.5 | **`ruyi lint`（类似 clippy）** | 风格问题、常见错误、性能反模式 |
| 10.6 | **检查类别** | `correctness`（bug）、`style`（约定）、`complexity`（简化）、`performance`（速度） |

### v1.2 — 文档生成器（优先级：P1）

| # | 任务 | 描述 |
|---|------|------|
| 11.1 | **`ruyi doc`** | 从 `/** */` 文档注释生成 HTML 文档 |
| 11.2 | **Doctest** | 从文档注释提取并运行代码示例作为测试 |
| 11.3 | **交叉引用** | 跨模块链接类型、函数、trait |
| 11.4 | **搜索索引** | 全文搜索所有文档项 |
| 11.5 | **`ruyi doc --open`** | 构建并在浏览器中打开 |

### v1.3 — 调试器与高级工具（优先级：P2）

| # | 任务 | 描述 |
|---|------|------|
| 12.1 | **DWARF 调试信息** | 在编译二进制中发出调试符号，用于 LLDB/GDB |
| 12.2 | **DAP 集成** | Debug Adapter Protocol，用于 VS Code 调试 |
| 12.3 | **`ruyi repl`** | 交互式 REPL 含增量编译 |
| 12.4 | **LSP（v2）** | 查找引用、重命名、工作区符号、代码操作（快速修复） |
| 12.5 | **Inlay hints** | 内联显示推断类型、参数名 |
| 12.6 | **性能分析器** | `ruyi perf record` / `ruyi perf report`，使用 LLVM perf 集成 |
| 12.7 | **模糊测试** | `ruyi fuzz` 用于词法/解析器模糊测试 |

### v1.4 — IDE 完善

| # | 任务 | 描述 |
|---|------|------|
| 13.1 | **代码补全** | 上下文感知：关键字、标识符、导入、trait 方法 |
| 13.2 | **重构** | 提取函数、重命名符号、组织导入 |
| 13.3 | **代码片段** | 常用模式：`fn`、`class`、`match`、`for-of`、`try-catch` |
| 13.4 | **类型提示** | 显示推断类型：`let x = 42` → `let x: int = 42` |
| 13.5 | **错误透镜** | 编辑器内联显示错误信息 |
| 13.6 | **测试浏览器** | @test 函数树视图；运行/调试单个测试 |

---

## 时间线

```
2026 Q2-Q3  v0.2  代码生成补全（类、对象、数组、for循环、try/catch）
2026 Q3      v0.3  运行时对接（GC 连接、真正的 async、异常）
2026 Q3-Q4   v0.4  类型检查加固（trait 约束、impl for、穷尽性）
2026 Q4      v0.5  标准库扩展（math/time/json/random/fmt/test）

2027 Q1      v0.6  包管理器基础（清单、锁文件、依赖、构建、运行）
2027 Q1-Q2   v0.7  包注册中心（发布、安装、搜索、文档托管）
2027 Q2      v0.8  工作空间与构建系统（配置、交叉编译、增量编译）
2027 Q3      v0.9  生态种子（推荐包、模板、CI、审计）

2027 Q3-Q4   v1.0  LSP 与格式化器（tree-sitter、LSP v1、ruyi fmt、VS Code）
2027 Q4      v1.1  测试运行器与代码检查器（@test、@bench、ruyi lint）
2028 Q1      v1.2  文档生成器（ruyi doc、doctest、搜索）
2028 Q1-Q2   v1.3  调试器与高级工具（DWARF、DAP、REPL、perf）
2028 Q2      v1.4  IDE 完善（重构、代码片段、类型提示、测试浏览器）
```

---

## 成功标准

### 阶段一完成标准
- [ ] 能编译并运行使用类、对象、数组和字符串拼接的程序
- [ ] `try/catch/finally` 端到端工作，含真正的异常传播
- [ ] async `fn` 实际在工作窃取调度器上运行（非同步）
- [ ] GC 能在循环中正确回收无引用对象
- [ ] 当前 9 个 stdlib 模块全部通过集成测试
- [ ] 3+ 个新 stdlib 模块（math、time、json）含测试
- [ ] `cargo test` 在 stdlib 代码路径上线覆盖率 >90%
- [ ] CI 流水线在每次推送时运行（GitHub Actions）

### 阶段二完成标准
- [ ] `ruyi build` 能编译含 5+ 依赖（来自注册中心）的项目
- [ ] `ruyi test` 能发现并运行 `@test` 函数
- [ ] `ruyi publish` 能上传包到注册中心
- [ ] 锁文件确保跨机器可重现构建
- [ ] 含 3+ 成员的工作空间正确构建
- [ ] 交叉编译到至少 2 个目标（linux-x64、macos-arm64）

### 阶段三完成标准
- [ ] VS Code 扩展发布，含语法高亮 + LSP
- [ ] `ruyi fmt` 是幂等的，能处理所有语言规范语法
- [ ] `ruyi test` 在 <5 秒内运行 100+ 集成测试
- [ ] `ruyi doc` 为任意包生成可浏览的 HTML
- [ ] LSP 在 <10K 行文件上的补全/悬停响应时间 <50ms
- [ ] 调试器能设断点、单步执行、检查变量

---

## 风险缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| LLVM API 变动 | 高 | 固定 inkwell 到 LLVM 14；用 trait 抽象 LLVM 调用 |
| async 运行时 bug | 高 | v0.3 发布前进行大量 async 集成测试 |
| 包注册中心扩展 | 中 | 从基于 Git 的依赖开始（v0.6）；逐步增加注册中心 |
| LSP 性能 | 中 | 使用 tree-sitter 解析；增量类型检查 |
| 社区采用 | 中 | 聚焦开发体验：快速编译、清晰的错误信息、简单安装 |
| Stdlib 范围蔓延 | 中 | 保持核心最小；小众需求交由社区包 |

---

## 核心差异化优势（为什么选择 Ruyi？）

1. **熟悉的语法，没有陷阱** — JS 开发者能立刻读懂 Ruyi，但用 `===`、没有 `undefined`、没有 `var`、显式可空类型
2. **通过 LLVM 的原生性能** — 零开销抽象、单态化泛型、零开销异常
3. **正确的渐进式类型** — `dyn` 存在但不是魔法；显式 `?` 表示可空类型
4. **开箱即用的 async** — 工作窃取调度器在标准库中，而非第三方 crate
5. **内建测试框架** — `@test` 是语言特性，不是库