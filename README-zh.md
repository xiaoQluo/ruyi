# Ruyi（如意）

> 一门基于 JavaScript 严格模式语法的编译型通用编程语言，通过 LLVM 编译为原生机器码。

[English README](README.md)

## 概述

Ruyi 移除了 JavaScript 中有问题的特性，同时保留了熟悉的语法。它通过 LLVM 编译为原生机器码，在各平台上提供高性能表现。

## 核心特性

- **熟悉的语法** — 如果你懂 JavaScript，你就已经掌握了大部分 Ruyi 语法。
- **编译为原生代码** — 使用 LLVM 生成快速、独立的可执行文件。
- **渐进式类型系统** — 在静态类型注解和动态类型（`dyn`）之间自由选择。
- **空值安全** — 没有 `undefined`。可空类型必须显式声明（`string?`）。
- **模式匹配** — 一等公民的 `match` 表达式，支持解构和守卫条件。
- **Trait（特质）** — 类似接口的契约，支持静态和动态分发（`dyn Trait`）。
- **泛型** — 参数化多态，通过单态化实现零运行时开销。
- **Async/await** — 基于工作窃取调度器的绿色线程并发模型。
- **异常处理** — 通过 LLVM landing pad 实现零开销的 try/catch/finally。
- **宏系统** — 声明式、卫生的编译时代码生成。

## 快速开始

### 前置要求

- **Rust**（2021 edition）
- **LLVM 20**（完整构建必需）
  - macOS: `brew install llvm@20` 然后设置 `LLVM_SYS_201_PREFIX`
  - Linux: 通过包管理器安装（如 `apt install llvm-14-dev`）

### 从源码构建

```bash
git clone https://github.com/xiaoQluo/ruyi.git
cd ruyi
cargo build --release
```

编译器二进制文件位于 `./target/release/ruyic`。

### Hello, World!

创建 `hello.ry`：

```ruyi
print("Hello, Ruyi!");
```

编译并运行：

```bash
./target/release/ruyic hello.ry -o hello
./hello
```

## 编译器用法

```bash
ruyic <输入文件> [选项]
```

| 选项 | 说明 |
|------|------|
| `-o <输出>` | 指定输出二进制文件名 |
| `--emit-llvm` | 输出 LLVM IR 而非二进制文件 |
| `--emit-ast` | 输出 AST（调试用） |
| `--emit-typed-ast` | 输出带类型的 AST（调试用） |
| `--check` | 仅解析和类型检查（不生成代码） |
| `-O0`, `-O1`, `-O2` | 优化级别（默认 `-O0`） |
| `--debug` | 包含调试符号 |
| `--version` | 打印编译器版本 |

## 项目结构

```
ruyi/
├── crates/
│   ├── ruyic/              # 编译器（二进制 + 库）
│   │   ├── src/
│   │   │   ├── main.rs     # CLI 驱动（clap）
│   │   │   ├── driver.rs   # 编译流程编排器
│   │   │   ├── lexer/      # 词法分析器
│   │   │   ├── parser/     # AST 解析器
│   │   │   ├── macro_expand/  # 声明式宏系统
│   │   │   ├── typechecker/   # 渐进式类型检查 + 类型推断
│   │   │   ├── codegen/    # LLVM IR 代码生成（inkwell）
│   │   │   ├── gc/         # 垃圾回收器
│   │   │   ├── runtime/    # 运行时支持
│   │   │   └── diagnostics/# 错误报告
│   │   └── tests/          # 各模块测试 + 集成测试用例
│   └── ruyi_runtime/       # 运行时库（GC、异步、异常）
├── stdlib/                  # 标准库（.ry 源文件）
├── examples/                # 示例 .ry 程序
└── docs/
    ├── spec.md              # 语言规范
    └── tutorial.md          # 用户教程
```

## 开发者命令

```bash
# 构建
cargo build --release              # Release 版本
cargo build -p ruyic               # Debug 版本

# 检查（更快，不链接）
cargo check --workspace
cargo check -p ruyi_runtime --no-default-features  # 仅运行时（无需 LLVM）

# 测试
cargo test --workspace
cargo test -p ruyic --test typechecker   # 单个测试文件

# 代码检查与格式化
cargo clippy --workspace
cargo fmt
```

## 语言亮点

### 与 JavaScript 的区别

| JavaScript | Ruyi | 原因 |
|------------|------|------|
| `var x` | `let x` | 块级作用域，非函数作用域 |
| `undefined` | `null` | 单一空值 |
| `==` / `!=` | `===` / `!==` | 仅严格相等，无隐式类型转换 |
| `function() {}` | `fn() {}` | 更短的关键字 |
| 方法中的 `this` | `self` | 显式引用，无绑定混淆 |
| `arguments` | `...args` | 真正的数组（Rest 参数） |
| `prototype` | `class` / `trait` | 基于类的继承 |
| `with` | _(已移除)_ | 无替代 |
| `eval()` | _(已移除)_ | 无替代 |
| `delete obj.prop` | `obj.prop = null` | 不可删除属性 |

### 内置类型

| 类型 | 说明 |
|------|------|
| `int` | 64 位有符号整数 |
| `float` | 64 位浮点数 |
| `bool` | 布尔值（`true` / `false`） |
| `string` | UTF-8 字符串 |
| `null` | 空类型（唯一值：`null`） |
| `void` | 无返回值 |
| `dyn` | 动态类型（运行时检查） |
| `never` | 底部类型（不可达） |
| `bigint` | 任意精度整数 |

### 示例：斐波那契数列

```ruyi
fn fib(n: int): int {
  if (n <= 1) {
    return n;
  }
  return fib(n - 1) + fib(n - 2);
}

fn main() {
  for (let i = 0; i < 10; i = i + 1) {
    print("fib(" + i + ") = " + fib(i));
  }
}
```

### 示例：类与 Trait

```ruyi
trait Printable {
  fn format(self): string;
}

class Point {
  x: float;
  y: float;

  fn new(x: float, y: float) {
    self.x = x;
    self.y = y;
  }
}

impl Printable for Point {
  fn format(self): string {
    return "(" + self.x + ", " + self.y + ")";
  }
}
```

## 文档

- [语言规范](docs/spec.md) — 权威参考
- [教程](docs/tutorial.md) — 入门指南
- [中文规范](docs/spec-zh.md) — 语言规范（中文版）
- [中文教程](docs/tutorial-zh.md) — 教程（中文版）

## 许可证

本项目采用以下两种许可证之一（由你选择）：

- Apache License, Version 2.0（[LICENSE-APACHE](LICENSE-APACHE) 或 <http://www.apache.org/licenses/LICENSE-2.0>）
- MIT License（[LICENSE-MIT](LICENSE-MIT) 或 <http://opensource.org/licenses/MIT>）
