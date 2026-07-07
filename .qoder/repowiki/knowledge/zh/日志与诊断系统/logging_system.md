Ruyi 编译工具链采用**混合式输出策略**，将底层的运行时/调试日志（Logging）与编译器前端的用户级诊断（Diagnostics）进行了明确分离。

### 1. 核心系统与框架
- **底层日志框架**：使用 Rust 生态标准的 `log` crate 作为门面，配合 `env_logger` 进行初始化。这主要用于编译器内部开发调试、性能追踪或运行时库（Runtime）的状态记录。
- **用户诊断系统**：实现了自定义的 `diagnostics` 模块，不依赖第三方日志库，而是直接通过 `std::io` 向终端输出结构化、带颜色的高亮错误信息。该系统模仿了 Rust 编译器（rustc）的诊断风格，提供源码上下文、错误代码（Error Codes）及修复建议。

### 2. 关键文件与包
- **依赖配置**：`Cargo.toml` (Workspace) 统一声明了 `log = "0.4"` 和 `env_logger = "0.11"`。
- **诊断渲染引擎**：`crates/ruyic/src/diagnostics/render.rs` 包含了 `DiagnosticRenderer` 和 `ConsoleFormatter`，负责处理 ANSI 颜色代码、源码行高亮以及多行错误信息的排版。
- **错误代码体系**：`crates/ruyic/src/diagnostics/codes.rs` 定义了严格的错误分类（E1xxx-E4xxx 为错误，W1xxx 为警告），确保每个诊断信息都有唯一的身份标识。
- **驱动入口**：`crates/ruyic/src/main.rs` 中通过 `eprintln!` 处理最顶层的致命错误，并调用 `Driver` 执行编译流程。

### 3. 架构约定
- **分层输出原则**：
    - **Level 1 (User Facing)**：编译错误、类型不匹配、语法错误等必须通过 `diagnostics` 模块渲染，提供精准的行列号和源码片段。
    - **Level 2 (System/Debug)**：编译器内部的逻辑流转、GC 状态、LLVM IR 生成细节等，应使用 `log::debug!` 或 `log::info!`，并通过环境变量 `RUST_LOG` 控制开启。
    - **Level 3 (CLI Status)**：简单的成功提示或版本信息直接使用 `println!`。
- **颜色自适应**：诊断系统内置了 `ColorScheme`，能根据 `TERM` 环境变量自动判断是否启用彩色输出，避免在 CI 或非终端环境中产生乱码。

### 4. 开发者规范
- **禁止在生产路径使用 `println!` 报告错误**：所有面向用户的错误必须封装为 `RenderDiagnostic` 并通过 `DiagnosticRenderer` 输出。
- **错误代码规范化**：新增诊断类型时，必须在 `codes.rs` 中注册唯一的 `ErrorCode`，并在 `ERROR_INDEX` 中补充描述。
- **日志初始化**：若需要在 `ruyic` 中启用详细日志，需在 `main` 函数早期调用 `env_logger::init()`，并遵循 `log` crate 的级别规范（`error`, `warn`, `info`, `debug`, `trace`）。