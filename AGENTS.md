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
# Full workspace build (requires LLVM 14-18)
cargo build --release          # Binary at ./target/release/ruyic
cargo build -p ruyic           # Debug build of compiler only

# Check without linking (faster)
cargo check --workspace

# Runtime-only check (no LLVM needed)
cargo check -p ruyi_runtime --no-default-features

# Run tests
cargo test --workspace

# Run a single test file
cargo test -p ruyic --test typechecker

# Lint
cargo clippy --workspace

# Format
cargo fmt

# Compile a .ry file
ruyic examples/hello.ry -o hello && ./hello
ruyic examples/hello.ry --emit-llvm   # Output LLVM IR
ruyic examples/hello.ry --check       # Type-check only
```

## Setup Requirements

- **LLVM 14 is required** for the full build (inkwell binding). Without it, `cargo build` fails on `llvm-sys`.
  - macOS: `brew install llvm@14` then set `LLVM_SYS_140_PREFIX`
  - Runtime-only development: `--no-default-features` on `ruyi_runtime` skips inkwell
- Rust 2021 edition, workspace resolver = "2"

## Code Conventions

- **rustfmt**: 4-space tabs, max_width=100, Unix newlines
- **clippy**: warn-by-default enabled
- **Javadoc-style doc comments** on all public items (`/** ... */` with `@author`, `@date`)
- **Error types**: Use `thiserror` for derive, `anyhow` for application-level

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
| `inkwell` | LLVM bindings (llvm14-0 feature) |
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

- [ ] 确认当前分支已合并到目标分支, 无未提交的更改
- [ ] 运行 `cargo check --workspace` 确认代码可编译
- [ ] 运行 `cargo test --workspace` 确认测试通过
- [ ] 更新 `Cargo.toml` 中的 workspace `version` 字段
- [ ] 更新 `crates/ruyic/src/main.rs` 中 `#[command(version = "...")]` 的版本号
- [ ] 确认版本号格式为 `v{major}.{minor}.{patch}` (如 `v0.3.0`)
- [ ] 更新 `docs/roadmap.md` 和 `docs/roadmap-zh.md` 中的版本状态
- [ ] 为新功能创建示例 `.ry` 文件并编译验证:
  ```bash
  ruyic examples/new_feature.ry -o examples/target/new_feature && examples/target/new_feature
  ```
- [ ] 运行 `cargo clippy --workspace` 确认无警告
- [ ] 运行 `cargo fmt` 确认代码格式一致

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
2. 创建 Pull Request 合并到 main, 确认 CI 通过
3. 在 main 的 merge commit 上创建 annotated tag
4. 推送 tag: `git push origin vX.Y.Z`
5. 更新 `docs/roadmap.md` 和 `docs/roadmap-zh.md` 标记版本已发布

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

### 计划执行邮件通知

**每次计划开始和结束时, 必须向 `feather.lzg@foxmail.com` 发送邮件通知.**

#### 计划开始时
- 在创建 TODO 列表后, 立即发送 "计划开始" 邮件
- 邮件主题格式: `[Ruyi] 计划开始: {计划名称}`
- 邮件内容包含: 计划名称、任务列表概览、开始时间

#### 计划结束时
- 在所有任务完成且 Final Verification Wave 通过后, 发送 "计划完成" 邮件
- 邮件主题格式: `[Ruyi] 计划完成: {计划名称}`
- 邮件内容包含: 计划名称、完成的任务数、修改的文件列表、结束时间、验证结果

#### 邮件发送方式
- 使用 Resend API 发送邮件 (加载 `resend` skill)
- API Key 从环境变量 `RESEND_API_KEY` 获取
- 发件人使用: `Ruyi Agent <onboarding@resend.dev>` (或已验证的域名)
- 必须包含幂等键防止重复发送: `plan-{plan-name}-{start|end}-{timestamp}`
- 邮件发送失败时记录日志, 不阻塞计划执行流程

#### 示例代码 (Node.js)
```typescript
import { Resend } from 'resend';

const resend = new Resend(process.env.RESEND_API_KEY);

// 计划开始通知
const { data: startData, error: startError } = await resend.emails.send({
  from: 'Ruyi Agent <onboarding@resend.dev>',
  to: ['feather.lzg@foxmail.com'],
  subject: '[Ruyi] 计划开始: {plan-name}',
  html: `<h2>计划开始通知</h2><p>计划: {plan-name}</p><p>开始时间: {timestamp}</p><p>任务列表: {tasks}</p>`,
}, { idempotencyKey: `plan-{plan-name}-start-{date}` });

// 计划完成通知
const { data: endData, error: endError } = await resend.emails.send({
  from: 'Ruyi Agent <onboarding@resend.dev>',
  to: ['feather.lzg@foxmail.com'],
  subject: '[Ruyi] 计划完成: {plan-name}',
  html: `<h2>计划完成通知</h2><p>计划: {plan-name}</p><p>完成时间: {timestamp}</p><p>完成任务: {completed}/{total}</p><p>验证结果: {verification}</p>`,
}, { idempotencyKey: `plan-{plan-name}-end-{date}` });
```
