## 1. 系统与方法
项目采用 **Cargo Workspace** 作为统一的依赖管理系统。通过根目录的 `Cargo.toml` 定义工作区成员（`crates/ruyic` 和 `crates/ruyi_runtime`），并使用 `resolver = "2"` 以支持更精确的特性解析。

- **集中化版本控制**：利用 `[workspace.dependencies]` 统一管理第三方库（如 `inkwell`, `clap`, `thiserror` 等）的版本，确保各子包使用一致的依赖版本。
- **锁定文件**：使用 `Cargo.lock` 记录所有直接和间接依赖的精确版本及校验和，保证构建的可复现性。
- **外部依赖源**：所有第三方依赖均从官方 crates.io 索引获取，未配置私有仓库或 vendoring 策略。

## 2. 关键文件与包
- **`Cargo.toml` (Root)**: 定义工作区结构、共享元数据（版本、作者、License）以及全局依赖版本约束。
- **`Cargo.lock`**: 自动生成的依赖快照，包含 900+ 个包的详细信息，是 CI/CD 构建一致性的基石。
- **`crates/ruyic/Cargo.toml`**: 编译器核心引擎的依赖清单，引用了 `ruyi_runtime` 作为路径依赖，并继承了工作区的 `inkwell` (LLVM 绑定) 和 `clap` (CLI 解析) 等依赖。
- **`crates/ruyi_runtime/Cargo.toml`**: 运行时库的依赖清单，编译为 `staticlib` 和 `rlib`，通过可选特性（features）管理对 `inkwell` 的依赖。

## 3. 架构与约定
- **路径依赖 (Path Dependencies)**: 内部模块间通过 `path = "../..."` 进行链接，实现了编译器前端与运行时的解耦与协同开发。
- **特性门控 (Feature Gating)**: `ruyi_runtime` 使用 `[features]` 机制，允许在不依赖 LLVM 的情况下仅编译运行时核心逻辑（尽管默认开启 `inkwell`）。
- **环境依赖注入**: 针对 `inkwell` 这种强依赖系统环境的库，项目约定在构建环境中设置 `LLVM_SYS_140_PREFIX`（如在 `.github/workflows/ci.yml` 中所示），以指向本地安装的 LLVM 14 库。

## 4. 开发者规则
- **版本同步**: 新增或更新第三方库时，应优先在根 `Cargo.toml` 的 `[workspace.dependencies]` 中声明，子包通过 `.workspace = true` 引用。
- **LLVM 环境准备**: 由于依赖 `inkwell`，开发者必须在本地安装 LLVM 14 开发库，并正确配置环境变量，否则 `cargo build` 将失败。
- **锁文件提交**: `Cargo.lock` 必须提交至版本控制系统，以确保团队成员和 CI 环境获得完全相同的依赖树。