# Proposal: stdlib-io-path-process-ffi

## Why

Ruyi 标准库 `stdlib/` 目录下已有 14 个 `.ry` 模块的完整 Ruyi 层实现（API 设计、Javadoc、业务逻辑），但其中 `io.ry`、`path.ry`、`process.ry` 三个系统级模块的所有函数体均为 C FFI thin-wrapper——它们直接调用 `__io_*`、`__path_*`、`__process_*` 系列符号。问题在于，这些 FFI 符号在两个关键层面完全缺失：

1. **codegen 声明层**（`crates/ruyic/src/codegen/builtins_table.rs`）：无 `BuiltinDecl` 条目 → LLVM 编译时不会生成 `declare` 指令
2. **runtime 实现层**（`crates/ruyi_runtime/src/`）：无 `extern "C" fn` 实现 → 链接时 `undefined symbol` 错误

用户只要 `import { File } from "./io"` 或 `import { Process } from "./process"`，就能在链接阶段看到 45 个 `undefined reference` 错误。这三个模块覆盖了文件系统操作、路径处理和进程管理——是任何实际项目的基础设施。

v0.5.9 的 stdlib cleanup 已将此明确标记为 Out of Scope（`.spec-superflow.yaml` dp_1: `__io_*/__process_*/__path_* hygiene`），现在是时候合龙这个缺口。

## What Changes

### 新增文件（3 个 runtime 源文件）

| 文件 | 内容 | FFI 函数数 |
|------|------|-----------|
| `crates/ruyi_runtime/src/io_ffi.rs` | `__io_read_line`、`__io_file_read_text`、`__io_file_write_text`、`__io_file_read_lines`、`__io_file_exists`、`__io_is_directory`、`__io_is_file`、`__io_file_delete`、`__io_mkdir` 及对应 async 变体 | 17 |
| `crates/ruyi_runtime/src/path_ffi.rs` | `__path_join`、`__path_basename`、`__path_dirname`、`__path_extname`、`__path_is_absolute`、`__path_normalize`、`__path_separator`、`__path_relative` | 8 |
| `crates/ruyi_runtime/src/process_ffi.rs` | `__process_create`、`__process_exec`、`__process_exec_with`、`__process_wait`、`__process_kill`、`__process_write_input`、`__process_read_output`、`__process_get_env`、`__process_set_env`、`__process_get_all_env`、`__process_get_pid`、`__process_get_ppid`、`__process_get_platform`、`__process_get_cpu_count`、`__process_get_total_memory`、`__process_get_free_memory`、`__process_signal_available` 及 async 变体 | 20 |

### 修改文件（2 个已有文件）

| 文件 | 变更 |
|------|------|
| `crates/ruyic/src/codegen/builtins_table.rs` | 新增 45 条 `BuiltinDecl` 条目（在三组 `// __io_*` / `// __path_*` / `// __process_*` 注释块中），更新末尾测试计数为 `56 + 45 = 101` |
| `crates/ruyi_runtime/src/lib.rs` | 新增 `pub mod io_ffi;`、`pub mod path_ffi;`、`pub mod process_ffi;` |

### 新增测试文件

- 每个 runtime FFI 源文件末尾包含 `#[cfg(test)] mod tests` 单元测试模块
- `crates/ruyic/tests/integration/` 下新增 `.ry` 集成测试文件（io、path、process 各一）

## Scope

### In Scope

- 实现全部 45 个 `extern "C"` FFI 函数，覆盖 IO（17）、Path（8）、Process（20）
- 在 `builtins_table.rs` 中添加 45 条 `BuiltinDecl` 声明
- 在 `lib.rs` 注册三个新模块
- Rust 层 `#[cfg(test)]` 单元测试（每个 FFI 函数至少一个 happy-path 测试）
- Ruyi 层 `.ry` 集成测试（验证端到端可编译、可运行）
- 错误处理：沿用 `ruyi_throw` 异常模式，Ruyi 层 stdlib 捕获并转换为 `IOError`/`ProcessException`
- 跨平台适配：macOS 和 Linux 均可编译运行（使用 Rust stdlib 跨平台抽象）
- 遵循 `#[no_mangle]` + `/// Safety` doc + `@author`/`@date` Javadoc 块规范

### Out of Scope

- **不修改** `stdlib/*.ry` 源文件（Ruyi 层 API 保持冻结）
- **不新增** IO/Path/Process 之外的 stdlib 模块
- **不重构** codegen 管线或 driver.rs
- **不支持** Windows（`#[cfg(not(target_os = "windows"))]` 守卫，与现有项目平台范围一致）
- **不实现** async runtime 之外的异步基础能力（async 变体委托给现有 scheduler）
- **不改造** 已有的错误类型层级（`IOError`/`ProcessException` 已在 error.ry/process.ry 中定义）
- **不修复** `json.ry` 的 `!=` vs `!==` 问题或其他预存问题

## Impact

| 影响范围 | 程度 | 说明 |
|----------|------|------|
| `crates/ruyi_runtime/` | 中 | 新增 3 个源文件，`lib.rs` 加 3 行 `pub mod` |
| `crates/ruyic/src/codegen/builtins_table.rs` | 中 | 新增 45 条 `BuiltinDecl` 条目，更新计数测试 |
| `crates/ruyic/tests/integration/` | 低 | 新增 3 个 `.ry` 集成测试文件 |
| `stdlib/*.ry` | 无 | 不修改 |
| 编译时间 | 低 | 新增 ~1500 行 Rust，无外部依赖 |
| 向后兼容 | 无影响 | 纯新增，不改变任何已有 API 或行为 |
| 测试套件 | 无回归 | 新增测试，已有测试应全部通过 |

## Capabilities

完成后，用户可获得的能力：

1. **文件 I/O**：`import { File, readLine } from "./io"` → 读写文件、检查存在性、创建目录
2. **路径操作**：`import { Path } from "./path"` → 拼接、拆分、规范化路径
3. **进程管理**：`import { Process, getEnv, getPID } from "./process"` → 执行命令、管理子进程、读写环境变量、获取系统信息
4. **异步 I/O**：所有 async 变体可与现有 async/await 调度器无缝配合

## Acceptance Criteria

1. `make build-release` — 零错误通过
2. `make test` — 全部已有测试通过 + 新增测试通过
3. `make lint` — 零新增 clippy 警告
4. `make fmt-check` — 格式化一致
5. `make check` — workspace 类型检查通过
6. IO、Path、Process 三个集成测试 `.ry` 文件通过 `ruyic --check` 且编译为二进制后运行正确
7. 9 个已有 stdlib 模块的 `--check` 不受影响（回归验证）
