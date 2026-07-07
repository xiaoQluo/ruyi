Ruyi 语言编译工具链（ruyic）采用基于 **Cargo Workspace** 的 Rust 原生构建配置，结合 **环境变量** 和 **命令行参数** 进行运行时行为控制。其配置系统主要涵盖以下三个层面：

### 1. 构建与依赖配置 (Cargo Workspace)
项目使用 `Cargo.toml` 定义多包工作区，统一管理编译器核心 (`crates/ruyic`) 和运行时库 (`crates/ruyi_runtime`)。
- **统一版本管理**：通过 `[workspace.package]` 共享版本号、作者和许可证信息。
- **依赖继承**：子包通过 `.workspace = true` 继承公共依赖（如 `inkwell`, `clap`, `log`），确保依赖版本一致性。
- **构建优化**：在根目录定义了 `dev` 和 `release` 两种 Profile，Release 模式开启 LTO (Link Time Optimization) 以优化生成的二进制文件性能。

### 2. 运行时环境配置 (Environment Variables)
编译器驱动层 (`Driver`) 和诊断渲染器通过读取环境变量来调整其行为：
- **`RUYI_HOME`**：用于定位标准库路径。`ModuleResolver` 会优先在 `$RUYI_HOME/stdlib` 目录下查找模块，若未设置则回退到本地 `stdlib/` 目录或搜索路径。
- **`TERM`**：用于控制诊断信息的颜色输出。`ColorScheme::Auto` 模式下，若 `TERM` 变量值为 `dumb` 则禁用 ANSI 颜色代码，否则根据平台特性自动启用。

### 3. 编译器驱动配置 (CLI & Options)
通过 `clap` 库解析命令行参数，生成 `CompileOptions` 结构体，控制编译流水线的各个阶段：
- **输入/输出**：指定源文件路径 (`input`) 和输出目标 (`output`)。
- **发射类型 (`EmitType`)**：支持多种中间产物输出，包括原生二进制 (`Binary`)、LLVM IR (`LlvmIr`)、抽象语法树 (`Ast`)、类型化 AST (`TypedAst`) 以及仅类型检查 (`Check`)。
- **优化等级 (`OptLevel`)**：支持 O0, O1, O2 三种优化级别，直接传递给 LLVM 后端。
- **目标架构**：支持通过 `--target` 指定交叉编译的目标三元组。

### 4. 模块解析与标准库加载
- **自动加载**：`Driver` 在编译开始时会自动尝试加载 `error`, `core`, `collections` 等基础标准库模块。
- **搜索策略**：模块解析遵循优先级顺序：绝对路径 -> `$RUYI_HOME/stdlib` -> 相对路径 -> 搜索路径 -> 本地 `stdlib/` 目录。

### 开发者规范
- **新增配置项**：若需增加编译器行为控制，应优先通过 `clap` 添加 CLI 参数，并在 `CompileOptions` 中同步更新。
- **环境依赖**：涉及文件系统路径查找时，应尊重 `RUYI_HOME` 环境变量，并提供合理的本地回退机制。
- **诊断输出**：所有终端输出应通过 `DiagnosticRenderer` 处理，以支持基于 `TERM` 变量的颜色自适应。