Ruyi 语言采用分层错误处理架构，将**编译期静态诊断**（Compiler Diagnostics）与**运行时动态异常**（Runtime Exceptions）严格分离，并辅以标准库中的函数式错误包装类型（Result/Option）。

### 1. 编译期诊断系统 (Compile-time Diagnostics)
编译器通过结构化的诊断系统报告词法、语法、类型及解析错误，旨在提供清晰的开发者反馈。

*   **错误码体系 (`crates/ruyic/src/diagnostics/codes.rs`)**：
    *   采用分类编号格式：`E1xxx` (词法), `E2xxx` (语法), `E3xxx` (类型), `E4xxx` (解析), `W1xxx` (警告)。
    *   定义了 `ErrorCode` 结构体及各类枚举（如 `TypeErrorCode::TypeMismatch`），确保错误信息的标准化和可检索性。
*   **渲染引擎 (`crates/ruyic/src/diagnostics/render.rs`)**：
    *   `DiagnosticRenderer` 支持彩色终端输出，能够展示源代码上下文（Source Context）、高亮错误位置并提供修复建议（Suggestions）。
    *   支持多行错误展示和子诊断（Children diagnostics），用于解释复杂的类型不匹配或解析冲突。
*   **模块级错误类型**：
    *   各编译阶段使用 `thiserror` 定义专用错误枚举，如 `LexerError`、`ParseError` 和 `TypeError`。
    *   `Driver` 层通过 `CompileError` 统一封装所有阶段的错误，并利用 `From` trait 实现自动转换。

### 2. 运行时异常机制 (Runtime Exception Handling)
运行时基于 LLVM 的异常处理模型（Landing Pad）和 Itanium C++ ABI 实现结构化异常处理（try-catch-finally）。

*   **异常对象模型 (`crates/ruyi_runtime/src/exception/`)**：
    *   `RuyiException` 包含 `type_id`、消息和堆栈跟踪信息。
    *   `ExceptionObject` 采用 `#[repr(C)]` 布局，确保与底层 unwinder 的兼容性。
    *   内置异常类型包括 `Error`, `TypeError`, `RangeError`, `RuntimeError`，并通过 `TypeId` 进行快速匹配。
*   **控制流与代码生成**：
    *   编译器在 `codegen/stmt.rs` 中处理 `try-catch` 语句。目前采用手动维护的 `try_stack` 配合分支指令来处理显式的 `throw` 语句。
    *   **注意**：根据审计文档 (`TRY_CATCH_AUDIT.md`)，当前的函数调用仍使用 `build_call` 而非 `build_invoke`，这意味着从被调函数中抛出的异常可能无法被当前层的 `catch` 正确拦截，这是后续开发需要完善的重点。
*   **Landing Pad 生成器**：
    *   `LandingPadGenerator` 负责生成 LLVM `landingpad` 指令，处理 catch 分派和 cleanup（finally）逻辑。

### 3. 标准库错误抽象 (Standard Library Abstractions)
为了支持函数式编程风格，标准库提供了非异常路径的错误处理工具。

*   **Result 类型 (`stdlib/result.ry`)**：
    *   实现了 `Ok<T, E>` 和 `Err<T, E>` 类，并通过类型别名 `Result<T, E>` 统一。
    *   提供 `map`, `andThen`, `unwrapOr` 等组合子，鼓励通过返回值而非异常来处理可预期的失败。
*   **Error 层次结构 (`stdlib/error.ry`)**：
    *   定义了以 `Error` 为基类的异常继承树，包括 `IOError`, `ArgumentError`, `NullError` 等，供用户在使用 `throw` 时实例化。

### 4. 开发者规范
*   **编译期**：遇到不可恢复的逻辑错误或语法违规时，应使用 `DiagnosticBag` 收集错误并最终由 `Driver` 抛出 `CompileError`。
*   **运行期**：
    *   对于严重的、不可预期的运行时故障，使用 `throw` 抛出继承自 `Error` 的异常。
    *   对于可能失败的业务逻辑（如 IO、解析），优先使用 `Result<T, E>` 类型进行返回。
    *   在编写底层 Runtime 绑定或 Codegen 逻辑时，需注意 `invoke` 指令的正确使用以确保异常传播链的完整性。