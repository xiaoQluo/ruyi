# Proposal: v0.5.5 残留 P0 缺陷修复

## Why

Ruyi v0.5.5 编译器与运行时存在 **7 项 P0 缺陷**，阻塞 v0.5.5 发布。这些缺陷致 `cargo test` 中 91 个 `#[ignore]` 测试无法启用、21 个 codegen 测试 FAIL、async 流程完全无法运行、GC 用占位分配器、运行时库未链入二进制、trait 约束检查始终返回 `true`。

具体缺陷（roadmap-zh.md 已锁定）：

| # | 缺陷 | 阻塞面 |
|---|------|--------|
| 1 | `ruyi_await` 是空操作 | async/await 完全无功能 |
| 2 | T9 修复不完整 | 21/27 codegen 测试 FAIL（RangeError/ArrayIterator 不可作构造器调用） |
| 3 | try/catch landing pad 未真正落地 | 跨函数异常无法被 catch 捕获 |
| 4 | GC 用占位分配器（裸 malloc） | 真实 generational GC 未启用 |
| 5 | 运行时库链接为裸 `cc` | `ruyi_runtime` 未链入二进制 |
| 6 | `spawn` 内建未实现 | green thread 启动入口缺失 |
| 7 | trait 约束检查为空 | `generics.rs::check_bounds()` 始终返回 `true` |

## What Changes

### 阶段 1：基础设施 + 测试解锁（4 项并行）

#### 1.1 GC 双模式（#4）

- 新增 `--gc=stub`（默认，开发用）和 `--gc=real`（生产用）编译标志
- codegen 中所有堆分配点根据 flag 切换调用 `cc_alloc` 或 `ruyi_gc_alloc`
- `ruyi_runtime/gc/` 已有的 generational GC 框架代码连入

#### 1.2 运行时库静态链接（#5）

- `driver.rs` 不再裸 `cc`，改为链入 `libruyi_runtime.a`
- 静态库由 `cargo build -p ruyi_runtime --release` 预编译产物
- 构建产物为单文件可执行

#### 1.3 T9 收尾 + stdlib 审查（#2）

- `stdlib/collections.ry` 中 `RangeError` / `ArrayIterator` 改为可构造器调用
- 同步审查 stdlib 其余 7 模块的正确性（合并自 `stdlib合理性检查` 探索）
- 启用 21 个原本 FAIL 的 codegen 测试

#### 1.4 trait 约束检查（#7）

- `generics.rs::check_bounds()` 实际验证 impl 存在
- 启用 32 个 `#[ignore]` typechecker 测试中至少 5 个

### 阶段 2：异常与异步（2 项并行）

#### 2.1 `ruyi_await` 真实异步（#1）

- 引入工作窃取调度器
- `ruyi_await` 真正 poll future 并让出协程
- 单元测试覆盖 poll/resume 路径

#### 2.2 try/catch landing pad（#3）

- 代码生成 try 体时使用 LLVM `invoke` 指令
- catch block 起始处生成 `landingpad`
- 启用 13 个 `#[ignore]` try_catch_invoke 测试

### 阶段 3：spawn 内建（#6）

- `spawn(fn)` 启动 green thread
- 内部调用调度器 + GC 分配栈帧
- 单元测试 + integration 测试覆盖

### 阶段 4：整体回归与归档

- `cargo test --workspace` 全绿（除合理保留的 `#[ignore]`）
- `cargo clippy --workspace` 零 warning
- release-archivist 流程

## Scope

### In Scope

- 7 项 P0 缺陷修复
- stdlib/collections.ry 现状审查 + 8 模块正确性扫描
- 新增 `--gc=<mode>` 编译标志
- 静态链接 `ruyi_runtime`
- `spawn` 内建
- `ruyi_await` 工作窃取调度
- try/catch landing pad
- trait 约束实际验证

### Out of Scope（Scope Fence）

- ❌ P1/P2/P3 缺陷（12+ 项）—— 后续 change
- ❌ stdlib/math.ry, stdlib/time.ry, stdlib/json.ry —— 后续 change
- ❌ 性能优化、二进制压缩
- ❌ 文档/tutorial 大幅重写
- ❌ 失败测试 3 项历史遗留（`test_from_annotation_generic` 等）
- ❌ 重新设计 finally 复杂语义（defer、stack unwind）
- ❌ catch 类型匹配的多分支优化
- ❌ ruyi_runtime 异常表的 GC 集成优化
- ❌ v0.2-codegen-gaps 历史 tasks.md 中的 30.6K 任务清单（归档至 `docs/archive/`）

## Impact

| 影响面 | 评估 |
|--------|------|
| 编译产物 | 静态链接后二进制 +200KB–1MB；try/catch 函数 IR +2–5% |
| 运行行为 | async 真正运行；GC 真实启用；try/catch 正确传播；spawn 启动 green thread |
| 测试 | 91 个 `#[ignore]` 中至少 21 + 5 + 13 + 3 + spawn = **42 个** 可启用 |
| 性能 | 阶段 1 启用 `--gc=stub` 时与原行为等价；`--gc=real` 时 GC 开销约 +30% 编译时间 |
| ABI | 不变（仅内部 API 扩展） |
| 架构 | `driver.rs` 引入 `--gc=stub/real` 标志处理；linker 配置更新 |
| 兼容性 | 所有现有 examples 33/33 通过；CI 流程不变 |

## Capabilities

### 修改能力 (MODIFIED)

- `compiler-cli`: 新增 `--gc=<mode>` 编译标志
- `compiler-driver`: 链入 `ruyi_runtime` 静态库而非裸 `cc`
- `compiler-codegen-alloc`: GC 调用根据 `--gc` 切换
- `compiler-codegen-async`: `ruyi_await` 真实化（阶段 2）
- `compiler-codegen-stmt`: `compile_try` 使用 `invoke + landing pad`（阶段 2）
- `compiler-typechecker-generics`: `check_bounds` 实际验证（阶段 1）
- `runtime-gc`: 连入 codegen（阶段 1）
- `stdlib-collections`: `RangeError` / `ArrayIterator` 可构造器化（阶段 1）

### 新增能力 (ADDED)

- `compiler-builtin-spawn`: `spawn(fn)` 启动 green thread（阶段 3）
- `compiler-gc-flag`: `--gc=stub` / `--gc=real` 切换（阶段 1）

### 删除能力 (REMOVED)

- 无

## Acceptance

```bash
# 1. 编译验证（无 LLVM 也可）
cargo check --workspace                              → 零警告
cargo check -p ruyi_runtime --no-default-features   → 零警告
cargo clippy --workspace                              → 零警告

# 2. 测试（无需 LLVM）
cargo test --workspace                               → 全部通过
cargo test -p ruyi_runtime --lib --no-default-features → 全部通过

# 3. 阶段 1 端到端 codegen 测试（需要 LLVM 14）
LLVM_SYS_140_PREFIX=... ruyic examples/hello.ry -o hello
./hello                                              → exit 0
cargo test -p ruyic --test codegen -- --ignored --test-threads=1
                                                     → 21 个原 FAIL 测试全部 PASS

# 4. 阶段 2/3 端到端 codegen 测试
cargo test -p ruyic --test try_catch_invoke -- --ignored --test-threads=1
                                                     → 13 个测试全部 PASS
cargo test -p ruyic --test compilation_throw_unreachable -- --ignored --test-threads=1
                                                     → 3 个测试全部 PASS

# 5. examples 套件
bash examples/run_examples.sh                         → Total: 33 | Passed: 33 | Failed: 0

# 6. 二进制产物验证
file target/release/ruyic                            → ELF/Mach-O executable
ldd target/release/ruyic 2>/dev/null | grep ruyi_runtime
                                                     → 无输出（已静态链接）
otool -L target/release/ruyic | grep ruyi_runtime    → 无输出（macOS 已静态链接）

# 7. docs 一致性
docs/roadmap-zh.md P0 缺陷表                         → 所有项目 ✅ 关闭
```