## 1. 构建系统概览

Ruyi 语言编译工具链采用 **Cargo Workspace** 作为核心构建编排工具，配合 **Makefile** 提供简化的开发工作流入口。项目通过 `inkwell` crate 绑定 LLVM 14 进行代码生成，并包含一个独立的运行时库 `ruyi_runtime`。

- **构建工具**: Rust Cargo (Edition 2021)
- **代码生成后端**: LLVM 14 (via `inkwell`)
- **包管理**: Cargo Workspace (`resolver = "2"`)
- **CI/CD**: GitHub Actions

## 2. 关键文件与目录结构

| 文件/路径 | 作用 |
| :--- | :--- |
| `Cargo.toml` | Workspace 根配置，定义成员 `crates/ruyic` 和 `crates/ruyi_runtime`，统一管理依赖版本（如 `inkwell`, `clap`）及发布优化配置（LTO, codegen-units=1）。 |
| `Makefile` | 提供 `build`, `test`, `install`, `fmt`, `lint` 等常用命令的快捷入口，封装了复杂的 Cargo 参数。 |
| `.github/workflows/ci.yml` | 自动化 CI 流程，负责在 Ubuntu 环境下安装 LLVM 14 依赖并执行全量测试。 |
| `examples/run_examples.sh` | 强大的示例程序验证脚本，支持编译、运行、基线比对（`.expected` 文件）以及更新基线。 |
| `docs/versioning.md` | 详细的版本管理规范，定义了语义化版本、分支模型（`main` + `dev/vX.Y`）及回滚程序。 |

## 3. 架构与约定

### 3.1 Workspace 组织
项目分为两个主要 Crate：
1.  **`ruyic`**: 编译器前端与后端，包含 Lexer, Parser, Typechecker, Codegen 等模块。依赖 `ruyi_runtime`。
2.  **`ruyi_runtime`**: 运行时核心，提供 GC、异常处理和异步调度。编译为 `staticlib` 和 `rlib`，以便链接到生成的二进制文件中。

### 3.2 构建配置
- **Release 优化**: 在 `Cargo.toml` 中开启了 `lto = true` 和 `codegen-units = 1`，以追求极致的运行时性能。
- **LLVM 依赖**: 本地构建需设置 `LLVM_SYS_140_PREFIX` 环境变量指向 LLVM 14 安装路径。CI 环境中通过 `apt-get` 安装 `llvm-14-dev`。

### 3.3 测试与验证
- **单元测试**: 使用 `cargo test --workspace` 运行。
- **集成测试**: `ruyic/tests/integration` 目录下存放了大量 `.ry` 源文件和对应的 `.expected` 输出文件。
- **示例验证**: `examples/run_examples.sh` 脚本提供了三种模式：
    - `default`: 编译并运行所有示例。
    - `--verify`: 将运行输出与 `.expected` 基线文件比对。
    - `--update`: 用当前运行结果更新基线文件。

## 4. 开发者规范

### 4.1 常用构建命令
```bash
# 快速检查代码（无需链接，速度快）
make check

# 构建 Release 版本
make build-release

# 运行全量测试
make test

# 格式化代码并运行 Clippy
make fmt && make lint
```

### 4.2 版本发布流程
遵循 `docs/versioning.md` 定义的规范：
1.  **分支策略**: 功能在 `dev/vX.Y` 分支开发，稳定后通过 `--no-ff` 合并至 `main`。
2.  **Tag 规范**: 在 `main` 分支上打 Annotated Tag，格式为 `vMAJOR.MINOR.PATCH`。
3.  **环境验证**: 发布前必须在有 LLVM 环境的机器上通过 `cargo build --release`、`cargo test --workspace` 及 `cargo clippy` 检查。

### 4.3 提交信息规范
遵循 Conventional Commits 格式：`<type>(<scope>): <description>`。
- **Type**: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `perf`, `ci`。
- **Scope**: 模块名，如 `lexer`, `parser`, `codegen`, `gc` 等。