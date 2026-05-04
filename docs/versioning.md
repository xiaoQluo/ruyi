# Ruyi Versioning

> **Version**: 0.1.0 | **Date**: 2026-05-04 | **Status**: Draft

## 概述

本文档定义 Ruyi 项目的版本管理规范，涵盖版本号规则、分支模型、Tag 规范、版本切换流程、Commit 消息格式、回滚程序和环境验证要求。所有参与 Ruyi 开发的成员必须遵循本规范。

适用范围：Ruyi 编译器（`ruyic`）、运行时库（`ruyi_runtime`）、标准库（`stdlib`）及相关工具链。

---

## 版本号规则

Ruyi 采用 [Semantic Versioning 2.0.0](https://semver.org/) 格式：

```
MAJOR.MINOR.PATCH
```

| 部分 | 说明 | 触发条件 |
|------|------|----------|
| **MAJOR** | 主版本号 | 不兼容的 API 变更、语言语法破坏性修改 |
| **MINOR** | 次版本号 | 向后兼容的功能新增、stdlib 模块扩展 |
| **PATCH** | 修订号 | 向后兼容的缺陷修复、性能优化 |

### 版本阶段标识

| 标识 | 含义 | 示例 |
|------|------|------|
| `alpha` | 内部测试，功能不完整 | `0.2.0-alpha.1` |
| `beta` | 公开测试，功能冻结 | `0.2.0-beta.1` |
| `rc` | 候选发布，仅修复阻塞性 Bug | `0.2.0-rc.1` |
| _(无后缀)_ | 正式稳定版 | `0.2.0` |

### 版本号递增规则

- 发布 MAJOR 版本时，MINOR 和 PATCH 归零
- 发布 MINOR 版本时，PATCH 归零
- PATCH 版本可随时发布，不受 MINOR 限制

---

## 分支模型

Ruyi 采用 **main + dev/vX.Y** 双轨分支结构：

```
main ────────────────────────────────────────────── (稳定，仅接受 merge commit)
  │
  ├── dev/v0.2 ──────────────────────────────────── (已发布)
  │
  ├── dev/v0.3 ──────────────────────────────────── (已发布)
  │
  └── dev/v0.4 ──────────────────────────────────── (开发中)
```

### 分支职责

| 分支 | 用途 | 推送规则 |
|------|------|----------|
| `main` | 稳定发布线 | 仅接受来自 dev 分支的 merge commit，禁止直接推送 |
| `dev/vX.Y` | 功能开发线 | 从 main 创建，永久保留，可自由推送 |

### 分支命名规则

- 开发分支格式：`dev/v{MAJOR}.{MINOR}`
- 示例：`dev/v0.4`、`dev/v1.0`
- 禁止使用 `feature/`、`bugfix/` 等临时分支命名

### 分支操作约束

- 禁止 force push 到 `main` 或任何 `dev/vX.Y` 分支
- 禁止在 `main` 上直接提交代码
- 禁止删除已发布的 dev 分支

---

## Tag规范

Tag 用于标记正式发布的版本，必须遵循以下规则：

### 格式要求

- 格式：`v{MAJOR}.{MINOR}.{PATCH}`
- 示例：`v0.2.0`、`v0.3.1`、`v1.0.0`
- 必须使用 annotated tag（含签名和说明）

### 创建命令

```bash
# 切换到 main 分支
git checkout main

# 创建 annotated tag
git tag -a v0.4.0 -m "Release v0.4.0"

# 推送 tag 到远程仓库
git push origin v0.4.0
```

### Tag 约束

- Tag 必须打在 `main` 分支的 merge commit 上
- 禁止在 dev 分支上打 Tag
- 禁止删除或移动已推送的 Tag
- 每个 Tag 对应一次正式版本发布

---

## 版本切换检查清单

版本切换分为五个阶段，每个阶段包含若干检查项。执行版本发布时，必须逐项确认并通过所有检查。

### 关闭旧版本

| # | 检查项 | 命令/操作 | 状态 |
|---|--------|-----------|------|
| 1 | 确认当前 dev 分支无未提交更改 | `git status` | - [ ] |
| 2 | 运行 workspace 类型检查 | `cargo check --workspace` | - [ ] |
| 3 | 运行全部测试 | `cargo test --workspace` | - [ ] |
| 4 | 运行 clippy 检查 | `cargo clippy --workspace` | - [ ] |
| 5 | 确认代码格式一致 | `cargo fmt` | - [ ] |
| 6 | 编译并运行示例程序 | `ruyic examples/hello.ry -o hello && ./hello` | - [ ] |
| 7 | 确认所有 P0 任务已完成 | 对照 roadmap.md 检查 | - [ ] |

### 合并到 main

| # | 检查项 | 命令/操作 | 状态 |
|---|--------|-----------|------|
| 8 | 切换到 main 分支 | `git checkout main` | - [ ] |
| 9 | 拉取最新代码 | `git pull origin main` | - [ ] |
| 10 | 合并 dev 分支到 main | `git merge --no-ff dev/vX.Y` | - [ ] |
| 11 | 确认合并后代码可编译 | `cargo build --release` | - [ ] |

### 打 Tag

| # | 检查项 | 命令/操作 | 状态 |
|---|--------|-----------|------|
| 12 | 创建 annotated tag | `git tag -a vX.Y.Z -m "Release vX.Y.Z"` | - [ ] |
| 13 | 推送 tag 到远程 | `git push origin vX.Y.Z` | - [ ] |

### 更新文档

| # | 检查项 | 命令/操作 | 状态 |
|---|--------|-----------|------|
| 14 | 更新 `docs/roadmap.md` 版本状态 | 编辑 Version Release Status 表格 | - [ ] |
| 15 | 更新 `docs/roadmap-zh.md` 版本状态 | 同步中文版 | - [ ] |

### 启动新版本

| # | 检查项 | 命令/操作 | 状态 |
|---|--------|-----------|------|
| 16 | 从 main 创建新 dev 分支 | `git checkout -b dev/vX.{Y+1}` | - [ ] |
| 17 | 更新 `Cargo.toml` workspace version | 编辑 `version = "X.Y.0"` | - [ ] |
| 18 | 更新 `main.rs` 版本号 | 编辑 `#[command(version = "vX.Y.0")]` | - [ ] |
| 19 | 推送新 dev 分支到远程 | `git push -u origin dev/vX.{Y+1}` | - [ ] |
| 20 | 创建新版本示例文件 | `examples/new_feature.ry` | - [ ] |
| 21 | 编译验证示例文件 | `ruyic examples/new_feature.ry -o examples/target/new_feature && examples/target/new_feature` | - [ ] |

---

## Commit消息规范

Ruyi 遵循 [Conventional Commits](https://www.conventionalcommits.org/) 格式：

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

### Type 类型

| Type | 说明 | 示例 |
|------|------|------|
| `feat` | 新功能 | `feat(parser): add pattern matching support` |
| `fix` | 修复 Bug | `fix(typechecker): resolve generic inference edge case` |
| `docs` | 文档变更 | `docs: update tutorial for async/await` |
| `refactor` | 代码重构 | `refactor(codegen): simplify class layout logic` |
| `test` | 测试相关 | `test: add integration tests for trait system` |
| `chore` | 构建/工具链 | `chore: update LLVM dependency to 14.0.6` |
| `perf` | 性能优化 | `perf(lexer): reduce allocation in string tokenization` |
| `ci` | CI/CD 相关 | `ci: add GitHub Actions workflow` |

### Scope 范围

Scope 标识变更影响的模块，常用值：

- `lexer`、`parser`、`typechecker`、`codegen`、`macro_expand`
- `driver`、`gc`、`runtime`、`diagnostics`
- `stdlib`、`examples`、`docs`

### 格式约束

- description 使用祈使句（"add" 而非 "added" 或 "adds"）
- description 首字母小写
- description 末尾不加句号
- body 和 footer 为可选，用于说明变更原因或关联 Issue

---

## 回滚程序

当版本发布后发现问题时，按以下三种场景执行回滚。

### 场景A：合并后发现问题（未打 Tag）

问题在 merge 到 main 后发现，但尚未创建 Tag。

```bash
# 1. 确认当前 HEAD 是需要回滚的 merge commit
git log --oneline -5

# 2. 回退 main 到 merge 前的 commit
git checkout main
git reset --hard HEAD~1

# 3. 强制推送 main（仅限此场景）
git push origin main --force-with-lease

# 4. 通知团队成员 main 已回退
```

**注意事项**：
- 使用 `--force-with-lease` 而非 `--force`，防止覆盖他人推送
- 回退后在 dev 分支上修复问题，重新走合并流程

### 场景B：已打 Tag 后发现严重 Bug

问题在 Tag 发布后发现，需要紧急修复。

```bash
# 1. 在 main 上创建 hotfix 分支
git checkout main
git checkout -b hotfix/vX.Y.Z

# 2. 修复 Bug 并提交
# ... 修复代码 ...
git add .
git commit -m "fix(scope): description of critical bug fix"

# 3. 运行验证
cargo test --workspace
cargo build --release

# 4. 合并回 main
git checkout main
git merge --no-ff hotfix/vX.Y.Z

# 5. 创建新的 PATCH 版本 Tag
git tag -a vX.Y.{Z+1} -m "Release vX.Y.{Z+1} (hotfix)"
git push origin main
git push origin vX.Y.{Z+1}

# 6. 删除 hotfix 分支
git branch -d hotfix/vX.Y.Z
```

**注意事项**：
- 仅修复阻塞性 Bug，不引入新功能
- 同步将修复 cherry-pick 到当前活跃 dev 分支

### 场景C：需要紧急回滚

发现严重安全问题或数据损坏，需要立即回退到上一稳定版本。

```bash
# 1. 确认上一稳定版本的 Tag
git tag -l | sort -V

# 2. 回退 main 到上一稳定 Tag
git checkout main
git reset --hard v{PREVIOUS_VERSION}

# 3. 删除有问题的 Tag
git tag -d v{BAD_VERSION}
git push origin :refs/tags/v{BAD_VERSION}

# 4. 强制推送 main
git push origin main --force-with-lease

# 5. 发布紧急 PATCH 版本（可选）
git tag -a vX.Y.{Z+1} -m "Release vX.Y.{Z+1} (emergency rollback)"
git push origin vX.Y.{Z+1}
```

**注意事项**：
- 紧急回滚优先于问题排查，先恢复稳定再分析原因
- 删除远程 Tag 后通知所有开发者执行 `git fetch --prune --tags`

---

## 环境验证最低要求

每个版本发布前，必须满足以下最低验证要求。

### 有 LLVM 环境（完整验证）

适用于具备 LLVM 14 开发环境的机器：

```bash
# 1. 完整构建
cargo build --release

# 2. 运行全部测试
cargo test --workspace

# 3. 编译并运行示例
ruyic examples/hello.ry -o hello && ./hello

# 4. Clippy 检查（无警告）
cargo clippy --workspace

# 5. 格式检查
cargo fmt -- --check
```

### 无 LLVM 环境（最小验证）

适用于不具备 LLVM 环境的机器，仅验证代码可编译：

```bash
# 1. 运行时检查（跳过 inkwell）
cargo check -p ruyi_runtime --no-default-features

# 2. Workspace 类型检查（不链接）
cargo check --workspace
```

### 验证通过标准

| 检查项 | 通过标准 |
|--------|----------|
| `cargo build --release` | 退出码 0，无错误 |
| `cargo test --workspace` | 全部测试通过，0 failures |
| `cargo clippy --workspace` | 0 warnings, 0 errors |
| `cargo fmt -- --check` | 无格式差异 |
| 示例程序运行 | 正常输出，退出码 0 |

---

> 本文档与 `docs/roadmap.md` 和 `AGENTS.md` 中的版本策略保持一致。如有冲突，以 `AGENTS.md` 为准。
