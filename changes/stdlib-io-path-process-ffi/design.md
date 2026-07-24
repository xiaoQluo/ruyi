# Design: stdlib-io-path-process-ffi

## Context

### Current State

Ruyi 编译器管线：`Source → Lexer → Parser → Macro Expand → TypeChecker → CodeGen (LLVM) → Linker`

FFI 集成方式：`stdlib/*.ry` 调用 `__xxx_*` C 符号 → `builtins_table.rs` 声明 LLVM `declare` → `ruyi_runtime/src/` 提供 `extern "C"` 实现 → `libruyi_runtime.a` 静态链接

当前 runtime 已实现 56 个 FFI 符号（array/map/set/string/math/time/json/random/fmt），覆盖 11 个 stdlib 模块中的 8 个。IO（17 符号）、Path（8 符号）、Process（20 符号）的 FFI 层完全缺失。

### Constraints

- **语言**: Rust 2021 edition, workspace resolver = "2"
- **FFI ABI**: C ABI (`extern "C"`), 无 unwinding 跨 FFI 边界（异常通过 `ruyi_throw` 内部机制）
- **LLVM**: LLVM 14 via inkwell, `builtins_table.rs` 使用 `BuiltinSig` 枚举定义签名
- **平台**: macOS（开发主力）+ Linux（CI），不支持 Windows
- **代码规范**: `#[no_mangle]`, `/// Safety` doc, `@author`/`@date` Javadoc, `#[cfg(test)]` 测试
- **零警告原则**: clippy 零新增, rustfmt 一致
- **内存**: 返回字符串通过 `ruyi_alloc` 分配，由 GC 管理；opaque handles 通过 `Box::into_raw`/`Box::from_raw`

### Stakeholders

- Ruyi 语言用户（需要 IO/Path/Process 功能）
- stdlib 维护者（Ruyi 层 API 已冻结，只等 FFI 后端）
- Compiler 团队（builtins_table.rs 扩展需保持表驱动一致性）

## Goals

1. 消除 45 个 undefined symbol 链接错误
2. 为 IO/Path/Process 提供生产级系统调用能力
3. 不引入新的外部依赖（仅使用 Rust stdlib）
4. 遵循项目现有的 FFI 实现模式（文件结构、命名、测试风格）
5. 跨 macOS/Linux 可移植

## Decisions

### D1: Three Separate FFI Source Files

**Choice**: 新增三个独立的 FFI 源文件——`io_ffi.rs`、`path_ffi.rs`、`process_ffi.rs`——每个文件对应一个 stdlib 模块。

**Rationale**: 遵循项目既定模式。现有 `math_ffi.rs`、`time_ffi.rs`、`json_ffi.rs`、`random_ffi.rs` 均为独立文件。一一映射 stdlib 模块（io.ry → io_ffi.rs），降低认知负担，支持独立单元测试模块。

**Alternatives considered**:
- 合并为单个 `sys_ffi.rs`：虽然减少文件数，但 45 个函数塞在一个文件里不符项目惯例，测试难以隔离
- 按操作类型拆分（如 `file_ffi.rs` + `proc_ffi.rs`）：粒度与现有不一致

### D2: Async via Existing Scheduler

**Choice**: IO 和 Process 的 7 个 async 变体（`*_async`）通过 `std::thread::spawn` 在独立线程中执行同步操作，将结果通过闭包注入现有 `Scheduler`/`Future` 机制。

**Rationale**: Ruyi async 运行时（`crates/ruyi_runtime/src/async_runtime.rs`）已提供 `Scheduler`、`Task`、`Future`、`Waker` 基础设施。复用现有机制而非引入 tokio/async-std，保持依赖零增长。IO/Process 异步不需要真正的非阻塞 I/O（epoll/kqueue）——`std::thread::spawn` 的开销对于文件 I/O 和进程等待完全可接受。

**Alternatives considered**:
- 引入 tokio：引入重量级依赖，与项目哲学冲突，且 tokio 需要 async runtime 接管整个 main
- 使用 epoll/kqueue 非阻塞 I/O：实现复杂度高，对 stdlib IO 场景收益很低（文件 I/O 在 Linux 上 epoll 也不能真正非阻塞）
- 不同步实现 async 变体（标记为 stub）：用户期望 async API 能真正工作

### D3: Opaque Handle Pattern for Process

**Choice**: `Process` 句柄使用 opaque pointer 模式——Rust 侧 `Box::new(ChildProcess { child: std::process::Child, ... })` → `Box::into_raw()` → `*mut c_void`，返回给 Ruyi 侧作为 `int` 存储。`__process_wait`/`__process_kill` 等函数通过 `Box::from_raw` 重新获得所有权或引用。

**Rationale**: 与 `__builtin_map_create`/`__builtin_set_create` 使用的 opaque handle 模式完全一致。Ruyi 类型系统不支持原生指针类型，`int` 作为 opaque token 是已证明可行的方案。生命周期由 `__process_wait` 或 `__process_kill` 负责释放。

**Alternatives considered**:
- 全局进程表 + 整数 ID：增加全局状态管理复杂度，对 stdlib 简单场景过度设计
- 直接暴露 `std::process::Child` 字段：FFI 边界不能安全传递 Rust 结构体

### D4: Error via ruyi_throw, Not Return Codes

**Choice**: IO/Path/Process FFI 函数通过 `ruyi_throw(exception_object)` 报告错误，而不是返回错误码。Ruyi 层 stdlib 已经用 try/catch 包裹 FFI 调用并转换为 `IOError`/`ProcessException`。

**Rationale**: 与现有 `__string_*`、`__builtin_array_*` 等函数一致。这些函数在遇到无效输入时都通过异常报告（如 `__builtin_array_get` 索引越界抛 `RangeError`）。错误码方案需要修改 stdlib .ry 源码（scope out），而异常方案无需改动。

**Alternatives considered**:
- 返回 `int` 错误码 + out 参数：需要改动 stdlib .ry 文件，违反 scope 约束
- 返回 `Result` 结构体：Ruyi 类型系统无法直接映射 Rust enum，需要额外的 FFI 适配层

### D5: Path Varargs as Single Array Argument

**Choice**: `__path_join` 接受单个 `*mut i8`（指向 Array 句柄）而非 C 风格 `...` varargs。

**Rationale**: Ruyi stdlib `Path.join(paths: ...string)` 已将 rest 参数打包为 Array。传递单个数组指针比 C varargs 更安全（类型已知、参数计数已知），且与 `__string_join` 的签名模式一致。现有 `builtins_table.rs` 的 `BuiltinSig` 不支持 varargs。

**Alternatives considered**:
- 使用 C varargs + `va_list`：`BuiltinSig` 不支持，需扩展类型系统，增加不必要复杂度
- 多次调用拼接：增加 FFI 调用次数，违背 thin-wrapper 设计

### D6: Memory Ownership

**Choice**: 所有返回的字符串通过 `ruyi_alloc`（即 `libc::malloc`）分配，调用者（Ruyi GC）负责释放。Process/Future opaque handles 的释放由对应的生命周期函数负责（`__process_wait`/`__process_kill` 释放 Process handle）。

**Rationale**: 与 `__string_join`、`__builtin_array_push` 等所有现有 FFI 函数一致。统一的内存管理协议避免 use-after-free 和双重释放。

**Alternatives considered**:
- 调用者提供缓冲区 + 长度参数：Ruyi 侧需预分配，API 复杂
- 静态缓冲区：非线程安全，不适合多线程 async

### D7: Integration Test via .ry Files

**Choice**: 在 `crates/ruyic/tests/integration/` 下新增三个 `.ry` 测试文件（`io_test.ry`、`path_test.ry`、`process_test.ry`），通过 Rust 测试框架编译并运行。

**Rationale**: 与 `crates/ruyic/tests/typechecker.rs` 等现有集成测试模式一致。端到端验证 FFI → Ruyi 完整链路（Ruyi source → typecheck → codegen → link libruyi_runtime → execute）。

**Alternatives considered**:
- 仅 Rust 单元测试：不验证 codegen 声明和链接正确性
- 手写 C 测试程序调用 FFI：不验证 Ruyi 层包装的正确性

### D8: Platform Guard for Unix-Only Features

**Choice**: Process 模块的 `__process_kill`（信号）、`__process_signal_available` 等使用 `#[cfg(not(target_os = "windows"))]` 守卫，在非 Unix 平台上编译为 stub（返回错误/false）。

**Rationale**: 项目当前不支持 Windows，但守卫条件使代码在将来移植时不会编译失败。`std::process::Command` 在 Windows 上可用（exec/spawn），但信号机制不可用。

**Alternatives considered**:
- 完全移除 Windows 条件编译：将来移植需大量改动
- 使用 `nix` crate：引入外部依赖，项目哲学偏好 stdlib

## Risks And Trade-Offs

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| `__process_create` spawn 的子进程成为孤儿（未 wait/kill 即丢弃 handle） | 中 | 低（资源泄漏，重启恢复） | 文档说明 handle 生命周期，`Drop` 实现中 warn 日志（不可用异常，因为 Drop 不能 throw） |
| async 变体使用 `thread::spawn` 在大量并发时线程开销大 | 低 | 低（stdlib 场景不涉及大量并发 I/O） | 文档说明此为简单实现，未来可迁移到线程池 |
| `__path_relative` 在跨文件系统边界时行为不一致 | 低 | 中 | 检测不同 mount point 时抛出异常 |
| `__path_normalize` 对 Unicode/非 ASCII 路径的处理 | 低 | 低 | Rust `std::path::Path` 原生支持 UTF-8，与 `__string_*` 系列一致 |
| 进程 stdin/stdout 管道在 `__process_kill` 后未清理导致 fd 泄漏 | 中 | 中 | `__process_kill` 中显式 `drop` stdin，`__process_wait` 中 `wait()` 确保子进程回收 |

### Trade-off: Simplicity vs. Performance

IO 和 Process async 变体使用 `thread::spawn` + blocking I/O，而非 epoll/kqueue 非阻塞 I/O。对 stdlib 场景（读取配置文件、执行构建命令），这完全够用。若未来需要高并发 I/O（如 HTTP 服务器），应由上层库（如未来的 `net` 模块）提供，而非 stdlib 层。
