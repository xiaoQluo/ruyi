# Ruyi — Agent Guidelines

## What This Is

Ruyi is a **compiled programming language** (JS-like syntax) targeting native machine code via LLVM. The compiler (`ruyic`) is written in Rust.

## Project Structure

```
ruyi/
├── crates/
│   ├── ruyic/           # Compiler crate (main binary + lib)
│   │   ├── src/
│   │   │   ├── main.rs      # CLI driver (clap)
│   │   │   ├── lib.rs       # Public API
│   │   │   ├── driver.rs    # Compilation pipeline orchestrator
│   │   │   ├── lexer/       # Tokenizer
│   │   │   ├── parser/      # AST parser
│   │   │   ├── macro_expand/ # Declarative macro system
│   │   │   ├── typechecker/ # Gradual type checker + inference
│   │   │   ├── codegen/     # LLVM IR generation (inkwell)
│   │   │   ├── gc/          # Garbage collector (generational)
│   │   │   ├── runtime/     # Runtime support
│   │   │   └── diagnostics/ # Error reporting
│   │   └── tests/       # Integration + unit tests per module
│   └── ruyi_runtime/    # Runtime library (GC, async, exceptions)
├── stdlib/              # Standard library (.ry source files)
├── examples/            # Example .ry programs
├── docs/
│   ├── spec.md          # Language specification (authoritative)
│   └── tutorial.md      # User tutorial
└── Cargo.toml           # Workspace root
```

## Compilation Pipeline

`driver.rs` orchestrates: **Source → Lexer → Parser → Macro Expansion → TypeChecker → CodeGen (LLVM) → Linker**

CLI flags: `-o <output>`, `--emit-llvm`, `--emit-ast`, `--emit-typed-ast`, `--check`, `-O0/-O1/-O2`, `--debug`

## Developer Commands

```bash
# Build
make build-release          # Release build (binary at ./target/release/ruyic)
make build-debug            # Debug build (faster, no optimizations)
make build-runtime          # Runtime-only check (no LLVM needed)

# Install
make install                # Build + install to ~/.ruyi/bin/ruyic

# Check (faster, no linking)
make check                  # Full workspace check
make check-runtime          # Runtime-only check (no LLVM needed)

# Test
make test                   # Run all workspace tests
make test-single TEST=typechecker   # Run single test file

# Lint & Format
make lint                   # Run clippy
make lint-fix               # Run clippy with auto-fix
make fmt                    # Format code
make fmt-check              # Check formatting without modifying

# Examples
make run-example EXAMPLE=hello        # Compile and run an example
make compile-example EXAMPLE=hello    # Compile example to LLVM IR
make compile-file FILE=examples/hello.ry  # Compile a .ry file

# Maintenance
make clean                  # Clean all build artifacts
make clean-examples         # Clean only example outputs

# Help
make help                   # Display all available targets
```

## Setup Requirements

- **LLVM 20 is required** for the full build (inkwell binding). Without it, `cargo build` fails on `llvm-sys`.
  - macOS: `brew install llvm@20` then set `LLVM_SYS_201_PREFIX`
  - Runtime-only development: `--no-default-features` on `ruyi_runtime` skips inkwell
- Rust 2021 edition, workspace resolver = "2"

## Code Conventions

- **rustfmt**: 4-space tabs, max_width=100, Unix newlines
- **clippy**: warn-by-default enabled
- **零警告原则**: 所有编译警告必须在提交前解决，禁止引入新的警告
  - 警告视为错误处理，不得忽略或压制
  - 如确需临时保留，必须在代码中添加明确注释说明原因
- **Javadoc-style doc comments** on all public items (`/** ... */` with `@author`, `@date`)
- **Error types**: Use `thiserror` for derive, `anyhow` for application-level
- **修改原则**: 所有修改必须从方案完整性和合理性角度出发
  - **完整性**: 修改覆盖所有相关场景，不留遗漏（边界条件、错误处理、依赖影响）
  - **合理性**: 方案符合代码规范、架构设计原则和最佳实践，避免引入技术债务

## Testing

- Tests live alongside source: `crates/ruyic/tests/` (one file per module: `lexer.rs`, `parser.rs`, `typechecker.rs`, etc.)
- Runtime tests: `crates/ruyi_runtime/tests/`
- Integration test fixtures in `crates/ruyic/tests/integration/`
- Benchmarks: `crates/ruyic/benches/` (criterion)

## Language Quick Reference (.ry files)

- Keywords: `let`, `const`, `fn`, `class`, `trait`, `match`, `if`, `else`, `for`, `while`, `return`, `throw`, `try`, `catch`, `finally`, `async`, `await`, `import`, `export`, `macro`, `type`
- No `var`, no `undefined`, no `==`/`!=` (strict `===`/`!==` only), no `function` (use `fn`)
- Methods use `self` (not `this`)
- Nullable types explicit: `string?`, null assertion: `value!`
- Built-in types: `int` (i64), `float` (f64), `bool`, `string`, `null`, `void`, `dyn`, `never`, `bigint`
- Semicolons required (stricter ASI than JS)

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `inkwell` | LLVM bindings (llvm20-1 feature) |
| `clap` | CLI parsing (derive) |
| `thiserror` / `anyhow` | Error handling |
| `log` / `env_logger` | Logging |
| `criterion` | Benchmarking |

## Authoritative Sources

- Language spec: `docs/spec.md`
- Tutorial: `docs/tutorial.md`
- Compiler pipeline: `crates/ruyic/src/driver.rs`
- CLI entry: `crates/ruyic/src/main.rs`

## Workflow Rules

### 版本切换检查清单

切换版本前逐项确认:

- [ ] 运行 `make check` 确认代码可编译
- [ ] 运行 `make build-release` 确认完整编译通过
- [ ] 运行 `make test` 确认测试通过
- [ ] 更新 `Cargo.toml` 中的 workspace `version` 字段
- [ ] 更新 `crates/ruyic/src/main.rs` 中 `#[command(version = "...")]` 的版本号
- [ ] 确认版本号格式为 `v{major}.{minor}.{patch}` (如 `v0.3.0`)
- [ ] 更新 `docs/roadmap.md` 和 `docs/roadmap-zh.md` 中的版本状态
- [ ] 为新功能创建示例 `.ry` 文件并编译验证:
  ```bash
  make run-example EXAMPLE=new_feature
  ```
- [ ] 运行 `make lint` 确认无警告
- [ ] 运行 `make fmt` 确认代码格式一致
- [ ] 确认当前分支已合并到目标分支, 无未提交的更改

### 分支策略

- **main**: 只接受 merge commit, 不直接推送. 每个 merge commit 对应一个已发布的版本.
- **dev/v{major}.{minor}**: 开发分支, 从 main 创建, 永久保留. 命名示例: `dev/v0.3`.
- 新功能在 dev 分支上开发, 完成后 merge 到 main 并发布.
- 禁止 force push 到 main 或任何 dev 分支.

### Tag规范

- 格式: `vX.Y.Z` (如 `v0.3.0`, `v1.0.0`)
- 使用 annotated tag: `git tag -a vX.Y.Z -m "Release vX.Y.Z"`
- Tag 必须打在 main 分支的 merge commit 上, 不得打在开发分支
- 打 tag 前确认该 commit 已通过所有测试

### 版本发布流程

1. 在 dev 分支完成所有功能开发和测试
2. **更新 `docs/roadmap.md` 和 `docs/roadmap-zh.md` 中的路线图状态**，标记已完成的任务和当前进度
3. 创建 Pull Request 合并到 main, 确认 CI 通过
4. 在 main 的 merge commit 上创建 annotated tag
5. 推送代码和 tag: `git push origin vX.Y.Z`
6. 在路线图中更新版本标记为"已发布"

### Commit消息规范

遵循 Conventional Commits 格式:

```
<type>(<scope>): <description>

[optional body]
```

常用 type:
- `feat`: 新功能
- `fix`: 修复 bug
- `docs`: 文档变更
- `refactor`: 代码重构
- `test`: 测试相关
- `chore`: 构建/工具链变更

示例:
```
feat(parser): add pattern matching support
fix(typechecker): resolve generic inference edge case
docs: update tutorial for async/await
```

### 环境要求

**有 LLVM 环境** (完整编译验证):
```bash
cargo build --release          # 完整构建
cargo test --workspace         # 运行全部测试
ruyic examples/hello.ry -o hello && ./hello  # 编译并运行示例
```

**无 LLVM 环境** (仅运行时验证):
```bash
cargo check -p ruyi_runtime --no-default-features  # 跳过 inkwell
cargo check --workspace                              # 仅类型检查, 不链接
```

### 计划执行飞书通知（秘书铁蛋）

每次计划开始和结束时，必须通过飞书 CLI（即「秘书铁蛋」应用机器人）向指定 chat 发送 Interactive 卡片通知。
**旧邮件通道已彻底废弃**：包括 `email-smtp-send` skill、Resend API、`RESEND_API_KEY` 环境变量、`feather.lzg@foxmail.com` 收件箱等均不再使用。

#### 触发时机

- **计划开始时**：在创建 TODO 列表后，立即发送「计划开始」卡片。
- **计划结束时**：在 Final Verification Wave 通过、所有任务标记为 `completed` 后，发送「计划完成」卡片。

#### 工具与权限

- 调用入口：`lark-cli im +messages-send`（详见 `lark-im` skill），命令参考详见 `~/.agents/skills/lark-im/references/lark-im-messages-send.md`。
- 卡片构造：必须遵循 `~/.agents/skills/lark-im/references/card/lark-im-card-create.md` 工作流的 Step 1–5（设计 → 组件文档 → 构造 → P0–P7 自检 → 发送），**禁止手写或复制粘贴卡片 JSON**。
- 身份：`--as bot`（lark-cli 应用机器人「秘书铁蛋」），需具备 `im:message:send_as_bot` scope，配置/认证问题走 `lark-shared` skill。
- 目标 chat：在执行通知前通过 `lark-cli im +chat-list --as bot` 解析目标 chat_id（`oc_xxx`），固化于本节下「配置项」。

#### 卡片规范（Card 2.0）

统一采用 Card 2.0（根节点 `"schema": "2.0"`），`config.width_mode = "default"`，header 配色按状态区分：

| 状态 | header.template | 信息焦点 |
|------|----------------|----------|
| 计划开始 | `green` | 计划名 / 任务总数 / 任务清单 / 开始时间 |
| 计划完成 | `blue` | 计划名 / 完成度 / 验证结果 / 修改文件清单 / 结束时间 |

正文遵循 P0–P7 阻断项：单个最强焦点（header）、信息分组（`div.fields` + `hr`）、2–5 个视觉块、对齐配色语义一致。

#### 「计划开始」卡片模板

```json
{
  "schema": "2.0",
  "config": { "width_mode": "default" },
  "header": {
    "template": "green",
    "title": { "tag": "plain_text", "content": "[Ruyi] 计划开始: {plan-name}" }
  },
  "body": {
    "elements": [
      {
        "tag": "div",
        "fields": [
          { "is_short": true, "text": { "tag": "lark_md", "content": "**计划**\n{plan-name}" } },
          { "is_short": true, "text": { "tag": "lark_md", "content": "**开始时间**\n{start-time}" } },
          { "is_short": true, "text": { "tag": "lark_md", "content": "**任务数**\n{task-total}" } }
        ]
      },
      { "tag": "hr" },
      {
        "tag": "markdown",
        "content": "**任务清单**\n{task-todo-list}"
      },
      {
        "tag": "markdown",
        "content": "<font color='grey'>触发：{trigger-context}</font>"
      }
    ]
  }
}
```

#### 「计划完成」卡片模板

```json
{
  "schema": "2.0",
  "config": { "width_mode": "default" },
  "header": {
    "template": "blue",
    "title": { "tag": "plain_text", "content": "[Ruyi] 计划完成: {plan-name}" }
  },
  "body": {
    "elements": [
      {
        "tag": "div",
        "fields": [
          { "is_short": true, "text": { "tag": "lark_md", "content": "**计划**\n{plan-name}" } },
          { "is_short": true, "text": { "tag": "lark_md", "content": "**结束时间**\n{end-time}" } },
          { "is_short": true, "text": { "tag": "lark_md", "content": "**完成度**\n{completed}/{total}" } }
        ]
      },
      { "tag": "hr" },
      {
        "tag": "markdown",
        "content": "**验证结果**\n{verification-status}"
      },
      {
        "tag": "markdown",
        "content": "**修改文件**（共 {file-count} 项）\n{file-list}"
      },
      {
        "tag": "markdown",
        "content": "<font color='grey'>收尾：{end-time} · 触发：{trigger-context}</font>"
      }
    ]
  }
}
```

#### 调用示例

```bash
PLAN_NAME="example-plan"
CHAT_ID="oc_c680851be821a4b8d4a1bb17350b2a47"   # opencode任务消息提示群（2026-07-11 实测）
TS="$(date -u +'%Y%m%dT%H%M%SZ')"

# 计划开始
lark-cli im +messages-send \
  --as bot \
  --chat-id "$CHAT_ID" \
  --msg-type interactive \
  --content "$(cat /tmp/plan-start-card.json)" \
  --idempotency-key "plan-${PLAN_NAME}-start-${TS}"

# 计划完成
lark-cli im +messages-send \
  --as bot \
  --chat-id "$CHAT_ID" \
  --msg-type interactive \
  --content "$(cat /tmp/plan-end-card.json)" \
  --idempotency-key "plan-${PLAN_NAME}-end-${TS}"
```

#### 配置项

| 名称 | 取值 | 说明 |
|------|------|------|
| `LARK_BOT_CHAT_ID` | `oc_c680851be821a4b8d4a1bb17350b2a47` | 接收卡片的目标群（`opencode任务消息提示`）chat_id，**2026-07-11 实测确认**。其他环境下可执行 `lark-cli im +chat-list --as bot` 检索后覆写。 |
| 飞书开发者后台 | `im:message:send_as_bot` scope | bot 发送消息必需；权限不足时按 `lark-shared` 提示用户去后台开通并复核 `console_url`。 |

#### 失败与重试

- **网络/API 失败**（exit ≠ 0 或 `ok != true`）：记录 `[error.type] / message / hint` 三段日志，**不阻塞**计划执行。
- **卡片校验失败**（如 `at/person` 组件含非法 open_id）：根据 `card-create.md` Step 4 修复后重发，重试上限 **3 次**。
- **3 次均失败**：**降级为纯文本**，用 `lark-cli im +messages-send --as bot --chat-id "$CHAT_ID" --text $'Plan {plan-name} {status}\n...'` 兜底。
- **兜底亦失败**：仅写 stderr，等待下次心跳或人工介入，不抛错上抛打断流程。

#### 幂等键规范

- 格式：`plan-{plan-name}-{start|end}-{yyyyMMddTHHmmssZ}`（UTC ISO 紧凑格式）。
- 含义：相同 plan + 状态 + 时间窗内，仅投递一次，避免重复打扰。
- 兜底降级为 `--text` 时同样保留幂等键。
