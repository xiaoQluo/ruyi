# Examples Complete Compilation Test

## TL;DR

> **Quick Summary**: 对 examples/ 目录下全部 25 个 `.ry` 文件进行编译、运行、输出比对（Golden baseline），建立回归测试基线，并编写可复用的测试脚本。
>
> **Deliverables**:
> - 25 个 `.ry` 文件的编译产物（`examples/target/{name}`）
> - 25 个 Golden baseline 文件（`examples/target/{name}.expected`）
> - 编译失败日志（`examples/target/failures.log`）
> - 测试脚本（`examples/run_examples.sh`）支持 `--verify`、`--update`、`--only` 标志
> - 测试报告（`examples/target/report.md`）
>
> **Estimated Effort**: Medium
> **Parallel Execution**: YES - 3 Waves (25-way parallel in Wave 2)
> **Critical Path**: T1 → T2 → Wave 2 (any) → T29 → T30 → F1-F4

---

## Context

### Original Request
对 examples 目录下的所有代码进行完整编译测试。

### Interview Summary
**Key Discussions**:
- 编译器 `ruyic` v0.5.0 已构建就绪（`target/release/ruyic`）
- 25 个 `.ry` 文件，11 个历史编译、14 个未编译 — **全部从头重新编译**
- 验证标准：编译 + 运行 + Golden baseline 输出比对
- WIP 文件：全部尝试编译，失败记录原因不阻塞流程
- 输出检查方式：首次运行生成 `.expected` baseline 文件，后续运行比对
- 用户确认无 stdin 交互需求

**Research Findings**:
- `examples/target/` 已存在旧编译产物（v0.3/v0.4 版本），需要清理后重新编译
- `v05_demo.ry` 使用 `Timestamp.now()` 和 `Random`（种子固定，确定性），需特殊处理
- `v05_tests.ry` 使用 `@test` 属性和 `--test` 编译标志
- `v04_features.ry` 是编译时检查文件，运行时无输出
- 所有 25 个示例文件均不读取 stdin

### Metis Review
**Identified Gaps** (addressed):
- 超时处理：编译 60s 超时，运行 10s 超时（Metis 推荐值）
- 确定性审计：每个示例运行 2 次，比对输出一致性后再创建 baseline
- 非确定性输出：`v05_demo.ry` 含时间戳输出，标记为 `FLAKY` 单独处理
- Golden 更新：脚本支持 `--update` 标志更新 baseline
- 退出码策略：每个文件记录编译/运行退出码到元数据
- 对比语义：精确匹配，自动 trim 尾部空行
- 测试脚本接口：bash 脚本，支持 `--verify`、`--update`、`--only <pattern>`、`--help`

---

## Work Objectives

### Core Objective
建立 examples/ 目录的完整回归测试基础设施：25 个示例全部编译，运行输出建立 Golden baseline，编写可复用的验证脚本。

### Concrete Deliverables
- `examples/target/` 下 25 个编译二进制 + 25 个 `.expected` 文件
- `examples/target/failures.log` 编译/运行失败详细日志
- `examples/target/report.md` 测试报告（pass/fail/skip/flaky 统计）
- `examples/run_examples.sh` 测试脚本（可复用）

### Definition of Done
- [ ] `bash examples/run_examples.sh --verify` 返回结果（pass + expected_fail = 25）
- [ ] 所有编译通过的示例的二进制文件存在于 `examples/target/`
- [ ] 所有编译通过的示例的 `.expected` 文件存在且非空（或明确标记为 SKIP_SILENT）
- [ ] `examples/target/failures.log` 记录每个失败原因
- [ ] `examples/target/report.md` 包含完整统计

### Must Have
- 全部 25 个 `.ry` 文件被尝试编译
- 编译失败记录文件名 + stderr + 退出码
- 编译成功 → 必须运行 → 必须创建 baseline
- 运行超时/崩溃记录到 failures.log
- 测试脚本支持 `--verify` 模式重新编译+运行+比对

### Must NOT Have (Guardrails)
- **禁止修改 .ry 源文件**（即使编译失败也不修改，只记录）
- **禁止添加新的 .ry 示例文件**
- **禁止修复编译器 bug**（日志记录即可）
- **禁止删除 examples/target/ 下旧的无关文件**（只清理测试产物）
- **不包含 CI/CD pipeline 配置**（只生成本地可执行脚本）
- **不包含性能基准测试**（不测量编译时间或运行时间）

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** - ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: NO（无现有 examples 测试框架）
- **Automated tests**: None（本任务即为建立测试基础设施）
- **Framework**: bash 脚本 + golden file 比对

### QA Policy
- **前端/UI**: N/A（无前端）
- **TUI/CLI**: 使用 `interactive_bash` 执行编译命令和运行二进制，捕获 stdout/stderr/exit code
- **API/Backend**: N/A
- **Library/Module**: 使用 `bash` 直接调用 ruyic 编译器

---

## Execution Strategy

### Parallel Execution Waves

> Maximize throughput by grouping independent tasks into parallel waves.
> Each wave completes before the next begins.

```
Wave 1 (Start Immediately - 环境准备, MAX PARALLEL):
├── T1: Verify LLVM environment + compiler version [quick]
├── T2: Clean examples/target/ directory [quick]
└── T3: Create test script skeleton (run_examples.sh) [quick]

Wave 2 (After Wave 1 - 编译所有 25 个文件, 25-WAY PARALLEL):
├── T4-T8: Simple examples batch [quick] x5
├── T9-T14: Control flow + functions batch [quick] x6
├── T15-T21: Type system + generics + classes + traits [unspecified-high] x7
├── T22-T23: Async examples [unspecified-high] x2
└── T24-T28: v04/v05 WIP features [unspecified-high] x5

Wave 3 (After Wave 2 - 审计 + Baseline + 脚本):
├── T29: Determinism audit + golden baseline creation [quick]
├── T30: Complete test script + verification run [quick]
└── T31: Generate test report (report.md) [quick]

Wave FINAL (After ALL tasks - 4 parallel QA):
├── F1: Plan compliance audit [oracle]
├── F2: Code quality + script review [unspecified-high]
├── F3: Real QA - run script end-to-end [unspecified-high]
└── F4: Scope fidelity check [deep]

Critical Path: T1 → T2 → Wave 2 (slowest task) → T29 → T30 → F1-F4
Parallel Speedup: ~80% faster than sequential (25 parallel compilations)
Max Concurrent: 25 (Wave 2) + 3 (Wave 1) + 3 (Wave 3) + 4 (Final)
```

### Agent Dispatch Summary

- **Wave 1**: **3** - T1 → `quick`, T2 → `quick`, T3 → `quick`
- **Wave 2**: **25** - T4-T8 → `quick`, T9-T14 → `quick`, T15-T21 → `unspecified-high`, T22-T23 → `unspecified-high`, T24-T28 → `unspecified-high`
- **Wave 3**: **3** - T29 → `quick`, T30 → `quick`, T31 → `quick`
- **Final**: **4** - F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

- [x] 1. Verify LLVM environment + compiler version

  **What to do**:
  - 检查 `LLVM_SYS_140_PREFIX` 环境变量
  - 运行 `target/release/ruyic --version` 确认输出版本号 `v0.5.x`
  - 运行 `target/release/ruyic --help` 确认 `--test`、`-o`、`--check` 等标志存在
  - 如果不满足，记录错误且 Mark task FAILED（阻塞后续所有任务）

  **Must NOT do**:
  - 不重新构建编译器

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 单文件环境检查，无复杂逻辑
  - **Skills**: `[]`
    - 纯 bash 命令即可完成

  **Parallelization**:
  - **Can Run In Parallel**: YES（与 T2 并行，但不能与 T2 逻辑冲突）
  - **Blocks**: Wave 2 全部任务
  - **Blocked By**: None

  **Acceptance Criteria**:
  - [ ] `echo $LLVM_SYS_140_PREFIX` 显示有效路径
  - [ ] `ruyic --version` 输出匹配 `v0\.5\.\d+`
  - [ ] `ruyic --help` 包含 `--test` 标志

  **QA Scenarios**:

  ```
  Scenario: Compiler version check
    Tool: Bash
    Steps:
      1. Run: target/release/ruyic --version
      2. Assert stdout matches regex: ruyic 0\.5\.\d+
    Expected Result: Exit code 0, version string present
    Evidence: .sisyphus/evidence/task-1-version.txt

  Scenario: LLVM environment check
    Tool: Bash
    Steps:
      1. Run: echo $LLVM_SYS_140_PREFIX
      2. Assert non-empty output
    Expected Result: Path to LLVM 14 installation
    Evidence: .sisyphus/evidence/task-1-llvm.txt
  ```

  **Commit**: NO（环境验证任务，无代码变更）

---

- [x] 2. Clean examples/target/ directory

  **What to do**:
  - 列出 `examples/target/` 下所有文件
  - 删除旧的编译二进制（无扩展名的可执行文件）和 `.ll` 文件
  - 保留目录结构、`.gitkeep`（如有）、`test_bigint.ry`（测试辅助文件）
  - 确认清理后目录干净（无旧编译产物干扰）

  **Must NOT do**:
  - 不删除 `examples/target/target/` 子目录
  - 不删除非编译产物的 `.ry` 文件

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 目录清理，简单文件操作
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO（与 T1 无冲突但 T1 更优先）
  - **Blocks**: Wave 2 全部任务
  - **Blocked By**: T1

  **Acceptance Criteria**:
  - [ ] 旧的编译二进制已删除
  - [ ] `.ll` 文件已删除
  - [ ] 目录仍存在且可写

  **QA Scenarios**:

  ```
  Scenario: Clean target directory
    Tool: Bash
    Steps:
      1. Run: ls examples/target/
      2. Assert: no files matching old binaries (array, async, control_flow, etc.)
      3. Run: mkdir -p examples/target/  # ensure writable
    Expected Result: Directory clean, writable
    Evidence: .sisyphus/evidence/task-2-cleaned.txt
  ```

  **Commit**: NO（清理操作，无代码变更）

---

- [x] 3. Create test script skeleton (run_examples.sh)

  **What to do**:
  - 创建 `examples/run_examples.sh` bash 脚本
  - 骨架功能：
    - 解析命令行参数：`--verify`（重新编译+运行+比对）、`--update`（更新 baseline）、`--only <pattern>`（按文件名过滤）
    - 定义常量：`COMPILER=target/release/ruyic`、`EXAMPLES_DIR=examples`、`TARGET_DIR=examples/target`、`COMPILE_TIMEOUT=60`、`RUN_TIMEOUT=10`
    - 定义辅助函数：`compile_file()`、`run_binary()`、`compare_output()`、`log_failure()`、`log_success()`、`print_report()`
  - 初始版本只需骨架结构和帮助信息输出

  **Must NOT do**:
  - 不在此阶段填充编译逻辑（后续任务填充）
  - 不硬编码文件列表（使用 `for f in examples/*.ry` 动态发现）

  **Recommended Agent Profile**:
  - **Category**: `writing`
    - Reason: 创建 bash 脚本，纯文本编写
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES（与 T1、T2 并行）
  - **Blocks**: T30
  - **Blocked By**: None

  **Acceptance Criteria**:
  - [ ] `examples/run_examples.sh --help` 显示用法信息
  - [ ] 脚本 `chmod +x` 可执行
  - [ ] 脚本语法通过 `bash -n examples/run_examples.sh` 检查

  **QA Scenarios**:

  ```
  Scenario: Script skeleton help output
    Tool: Bash
    Steps:
      1. Run: bash examples/run_examples.sh --help
      2. Assert stdout contains "--verify", "--update", "--only"
    Expected Result: Exit code 0, help text with flags
    Evidence: .sisyphus/evidence/task-3-help.txt
  ```

  **Commit**: NO（骨架脚本，后续任务填充）

---

### Wave 2 — Compile All 25 Examples (ALL INDEPENDENT — MAX PARALLEL)

> **通用编译模板**（以下所有编译任务遵循此模板）：
> - 编译 `examples/{name}.ry` → `examples/target/{name}`
> - 命令: `timeout 60 target/release/ruyic examples/{name}.ry -o examples/target/{name} 2>&1`
> - 特殊: `v05_tests.ry` 使用附加 `--test` 标志
> - 成功: exit 0 + 二进制存在可执行 → 记录 0 到 `{name}.meta`
> - 失败: 记录文件名 + exit code + stderr 到 `failures.log`，不阻塞其他任务
> - **Must NOT**: 修改 .ry 源文件、跳过 WIP 文件
> - **Commit**: NO（批量编译，统一处理）
> - **Skills**: `[]` for all

#### Group A: Simple Examples (Batch 1) — `quick` x5

- [x] 4. Compile `hello.ry`
  **Agent**: `quick` | **Parallel**: Wave 2, with T5-T28 | **Refs**: `examples/hello.ry`
  **QA**: `ls examples/target/hello` exists + `test -x examples/target/hello`

- [x] 5. Compile `fibonacci.ry`
  **Agent**: `quick` | **Parallel**: Wave 2 | **Refs**: `examples/fibonacci.ry`
  **QA**: `ls examples/target/fibonacci` exists + executable

- [x] 6. Compile `float_math.ry`
  **Agent**: `quick` | **Parallel**: Wave 2 | **Refs**: `examples/float_math.ry`
  **QA**: `ls examples/target/float_math` exists + executable

- [x] 7. Compile `compare_test.ry`
  **Agent**: `quick` | **Parallel**: Wave 2 | **Refs**: `examples/compare_test.ry`
  **QA**: `ls examples/target/compare_test` exists + executable

- [x] 8. Compile `ternary.ry`
  **Agent**: `quick` | **Parallel**: Wave 2 | **Refs**: `examples/ternary.ry`
  **QA**: `ls examples/target/ternary` exists + executable

#### Group B: Control Flow + Functions (Batch 2) — `quick` x6

- [x] 9. Compile `control_flow.ry`
  **Agent**: `quick` | **Parallel**: Wave 2 | **Refs**: `examples/control_flow.ry`
  **QA**: `ls examples/target/control_flow` exists + executable

- [x] 10. Compile `functions.ry`
  **Agent**: `quick` | **Parallel**: Wave 2 | **Refs**: `examples/functions.ry`
  **QA**: `ls examples/target/functions` exists + executable

- [x] 11. Compile `variables_and_types.ry`
  **Agent**: `quick` | **Parallel**: Wave 2 | **Refs**: `examples/variables_and_types.ry`
  **QA**: `ls examples/target/variables_and_types` exists + executable

- [x] 12. Compile `array.ry`
  **Agent**: `quick` | **Parallel**: Wave 2 | **Refs**: `examples/array.ry`
  **QA**: `ls examples/target/array` exists + executable

- [x] 13. Compile `try_catch.ry`
  **Agent**: `quick` | **Parallel**: Wave 2 | **Refs**: `examples/try_catch.ry`
  **QA**: `ls examples/target/try_catch` exists + executable

- [x] 14. Compile `error_handling.ry`
  **Agent**: `quick` | **Parallel**: Wave 2 | **Refs**: `examples/error_handling.ry`
  **QA**: `ls examples/target/error_handling` exists + executable

#### Group C: Type System + Generics + Classes + Traits (Batch 3) — `unspecified-high` x7

- [x] 15. Compile `type_system.ry`
  **Agent**: `unspecified-high` | **Parallel**: Wave 2 | **Refs**: `examples/type_system.ry`
  **QA**: `ls examples/target/type_system` exists + executable

- [x] 16. Compile `generics.ry`
  **Agent**: `unspecified-high` | **Parallel**: Wave 2 | **Refs**: `examples/generics.ry`
  **QA**: `ls examples/target/generics` exists + executable

- [x] 17. Compile `generics_simple.ry`
  **Agent**: `unspecified-high` | **Parallel**: Wave 2 | **Refs**: `examples/generics_simple.ry`
  **QA**: `ls examples/target/generics_simple` exists + executable

- [x] 18. Compile `generics_comprehensive.ry`
  **Agent**: `unspecified-high` | **Parallel**: Wave 2 | **Refs**: `examples/generics_comprehensive.ry`
  **QA**: `ls examples/target/generics_comprehensive` exists + executable

- [x] 19. Compile `classes_and_objects.ry`
  **Agent**: `unspecified-high` | **Parallel**: Wave 2 | **Refs**: `examples/classes_and_objects.ry`
  **QA**: `ls examples/target/classes_and_objects` exists + executable

- [x] 20. Compile `traits.ry`
  **Agent**: `unspecified-high` | **Parallel**: Wave 2 | **Refs**: `examples/traits.ry`
  **QA**: `ls examples/target/traits` exists + executable

- [x] 21. Compile `pattern_matching.ry`
  **Agent**: `unspecified-high` | **Parallel**: Wave 2 | **Refs**: `examples/pattern_matching.ry`
  **QA**: `ls examples/target/pattern_matching` exists + executable

#### Group D: Async Examples (Batch 4) — `unspecified-high` x2

- [x] 22. Compile `async.ry`
  **Agent**: `unspecified-high` | **Parallel**: Wave 2 | **Refs**: `examples/async.ry`
  **QA**: `ls examples/target/async` exists + executable

- [x] 23. Compile `async_comprehensive.ry`
  **Agent**: `unspecified-high` | **Parallel**: Wave 2 | **Refs**: `examples/async_comprehensive.ry`
  **QA**: `ls examples/target/async_comprehensive` exists + executable

#### Group E: v04/v05 WIP Features (Batch 5) — `unspecified-high` x5

- [x] 24. Compile `v04_minimal.ry`
  **Agent**: `unspecified-high` | **Parallel**: Wave 2 | **Refs**: `examples/v04_minimal.ry`
  **QA**: `ls examples/target/v04_minimal` exists + executable

- [x] 25. Compile `v04_simple.ry`
  **Agent**: `unspecified-high` | **Parallel**: Wave 2 | **Refs**: `examples/v04_simple.ry`
  **QA**: `ls examples/target/v04_simple` exists + executable

- [x] 26. Compile `v04_features.ry`
  **Agent**: `unspecified-high` | **Parallel**: Wave 2 | **Refs**: `examples/v04_features.ry`
  **Note**: 编译时检查文件，运行无输出也正常
  **QA**: `ls examples/target/v04_features` exists + executable

- [x] 27. Compile `v05_demo.ry`
  **Agent**: `unspecified-high` | **Parallel**: Wave 2 | **Refs**: `examples/v05_demo.ry`
  **Note**: 使用 `Timestamp.now()`，输出可能非确定性
  **QA**: `ls examples/target/v05_demo` exists + executable

- [x] 28. Compile `v05_tests.ry`
  **Agent**: `unspecified-high` | **Parallel**: Wave 2 | **Refs**: `examples/v05_tests.ry`
  **Note**: 使用 `--test` 标志: `ruyic examples/v05_tests.ry --test -o examples/target/v05_tests`
  **QA**: `ls examples/target/v05_tests` exists + executable

---

### Wave 3 — Audit + Baseline + Script + Report

- [x] 29. Determinism audit + golden baseline creation

  **What to do**:
  - 读取 `examples/target/failures.log` 获取编译失败的列表
  - 对每个编译成功且有 `{name}` 二进制的文件：
    1. 运行 `timeout 10 examples/target/{name} > examples/target/{name}.run1 2>&1`，记录退出码
    2. 运行 `timeout 10 examples/target/{name} > examples/target/{name}.run2 2>&1`，记录退出码
    3. `diff examples/target/{name}.run1 examples/target/{name}.run2`
       - 相同 → 复制 `.run1` 为 `{name}.expected`，记录退出码到 `{name}.meta`
       - 不同 → 标记为 `FLAKY`，日志记录差异，仍创建 `.expected`（取第一次运行）但标记 `FLAKY`
    4. 对空输出文件：检查是否正常（某些文件如 `v04_features.ry` 可能编译时检查、运行时无输出）→ 标记 `SKIP_SILENT`
    5. 运行超时（10s）：标记 `TIMEOUT`，不创建 baseline
  - 创建 `examples/target/baselines.json` 元数据文件（JSON 格式）：
    ```json
    {
      "hello": {"status": "PASS", "exit_code": 0, "baseline": "hello.expected"},
      "v05_demo": {"status": "FLAKY", "exit_code": 0, "baseline": "v05_demo.expected", "note": "Timestamp.now() produces variable output"},
      "pattern_matching": {"status": "FAIL", "exit_code": 1, "error": "compilation error at line X"}
    }
    ```
  - 清理临时 `.run1` / `.run2` 文件

  **Must NOT do**:
  - 不修改 baseline 文件内容（只创建，不编辑）
  - 不对编译失败的文件创建 baseline

  **Recommended Agent Profile**:
  - **Category**: `unspecified-low`
    - Reason: 批量运行+比对，脚本化操作
  - **Skills**: `[]`

  **Parallelization**:
  - **Blocks**: T30
  - **Blocked By**: Wave 2 (所有编译任务)

  **Acceptance Criteria**:
  - [ ] 每个编译成功的示例有对应的 `.expected` 文件
  - [ ] `baselines.json` 存在且格式正确（`jq . baselines.json` 成功）
  - [ ] 无 `.run1` / `.run2` 残留文件

  **QA Scenarios**:

  ```
  Scenario: Known-good example has matching outputs
    Tool: Bash
    Steps:
      1. Run: examples/target/hello > /tmp/h1 && examples/target/hello > /tmp/h2
      2. diff /tmp/h1 /tmp/h2
      3. Assert diff empty (outputs match)
    Expected Result: Exit code 0, no diff
    Evidence: .sisyphus/evidence/task-29-hello-match.txt

  Scenario: baselines.json is valid JSON
    Tool: Bash
    Steps:
      1. Run: cat examples/target/baselines.json | jq 'keys | length'
      2. Assert: returns number > 0
    Expected Result: Valid JSON with entries
    Evidence: .sisyphus/evidence/task-29-baselines-json.txt
  ```

  **Commit**: NO

---

- [x] 30. Complete test script + verification run

  **What to do**:
  - 在 `examples/run_examples.sh` 中填充完整逻辑：
    - **默认模式**（无 flag）：编译 + 运行所有示例，输出报告
    - **`--verify` 模式**：重新编译 + 运行 + 比对 `.expected` 文件（`diff -q` 或 `cmp -s`）
    - **`--update` 模式**：重新编译 + 运行 + 覆盖 `.expected` 文件
    - **`--only <pattern>` 模式**：仅处理匹配的示例（如 `--only "v05"` 只测 v05 开头的文件）
    - 每个文件超时处理：编译 60s、运行 10s（使用 `timeout` 命令）
    - 退出码统计：编译失败（exit ≠ 0）、运行失败（exit ≠ 0）、比对失败（diff 不匹配）、超时（TIMEOUT）
  - 运行脚本并验证：`bash examples/run_examples.sh`（首次 baseline 创建）
  - 验证脚本自身：`bash examples/run_examples.sh --verify`（二次运行应全部 PASS，因为刚创建 baseline）
  - 处理 `--verify` 失败的文件 → 如果是 FLAKY 标记的文件，log 不 fail

  **Must NOT do**:
  - 不生成 CI YAML 文件
  - 不在脚本中使用硬编码文件列表

  **Recommended Agent Profile**:
  - **Category**: `writing`
    - Reason: bash 脚本编写，纯文本
  - **Skills**: `[]`

  **Parallelization**:
  - **Blocks**: T31
  - **Blocked By**: T3, T29

  **Acceptance Criteria**:
  - [ ] `bash -n examples/run_examples.sh` 通过语法检查
  - [ ] `bash examples/run_examples.sh --verify` 返回合理的统计
  - [ ] `bash examples/run_examples.sh --only "hello"` 只测试 hello.ry
  - [ ] `bash examples/run_examples.sh --help` 显示所有标志

  **QA Scenarios**:

  ```
  Scenario: Verify mode on hello.ry
    Tool: Bash
    Steps:
      1. Run: bash examples/run_examples.sh --only "hello" --verify
      2. Assert stdout contains "PASS" and "hello"
    Expected Result: Exit code 0, hello.ry passes
    Evidence: .sisyphus/evidence/task-30-verify-hello.txt

  Scenario: Update mode overwrites baseline
    Tool: Bash
    Steps:
      1. cp examples/target/hello.expected examples/target/hello.expected.bak
      2. Run: bash examples/run_examples.sh --only "hello" --update
      3. diff examples/target/hello.expected examples/target/hello.expected.bak
      4. Assert diff empty (reproducible output)
    Expected Result: Baseline updated successfully, content unchanged for deterministic hello
    Evidence: .sisyphus/evidence/task-30-update.txt
  ```

  **Commit**: NO（最终统一处理）

---

- [x] 31. Generate test report (report.md)

  **What to do**:
  - 从 `baselines.json` 和 `failures.log` 生成 `examples/target/report.md`
  - 报告内容：
    - 总览：总文件数、PASS、FAIL、FLAKY、TIMEOUT、SKIP_SILENT 计数
    - 每个文件一行：状态 emoji（✅ ❌ ⚠️ ⏰ 🔇）+ 文件名 + 备注
    - 失败文件单独展开：文件名 + 失败类型（编译/运行/比对）+ 错误摘要
    - FLAKY 文件单独展开：文件名 + 非确定性原因
    - 运行环境信息：编译器版本、LLVM 版本、OS、日期时间
  - 格式：Markdown 表格 + 详细列表

  **Must NOT do**:
  - 不过滤失败信息（所有失败都要展示）

  **Recommended Agent Profile**:
  - **Category**: `writing`
    - Reason: Markdown 报告生成
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Blocks**: None (final output)
  - **Blocked By**: T30

  **Acceptance Criteria**:
  - [ ] `examples/target/report.md` 存在
  - [ ] 报告中总计数 = 25
  - [ ] 报告包含编译器版本信息
  - [ ] 报告包含每个文件的状态

  **QA Scenarios**:

  ```
  Scenario: Report completeness
    Tool: Bash
    Steps:
      1. Run: cat examples/target/report.md
      2. Assert contains "v0.5"
      3. Assert contains at least 25 file entries
    Expected Result: Complete report with all 25 files
    Evidence: .sisyphus/evidence/task-31-report.txt
  ```

  **Commit**: NO（最终统一处理）

---

## Final Verification Wave

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.

- [x] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists.
  For each "Must NOT Have": search for forbidden patterns.
  Check evidence files exist in `.sisyphus/evidence/`.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [x] F2. **Code Quality Review** — `unspecified-high`
  Verify: (1) `bash -n examples/run_examples.sh` passes syntax check; (2) `baselines.json` is valid JSON;
  (3) `report.md` is valid markdown; (4) No `.run1/.run2` temp files left; (5) No hardcoded file lists in script;
  (6) Script uses `set -euo pipefail`.
  Output: `Script [PASS/FAIL] | JSON [PASS/FAIL] | Report [PASS/FAIL] | Temp Cleanup [CLEAN/N issues] | VERDICT`

- [x] F3. **Real Manual QA** — `unspecified-high`
  Start from clean state: delete all `.expected` files (if any), then run:
  1. `bash examples/run_examples.sh` → creates baselines
  2. `bash examples/run_examples.sh --verify` → should pass
  3. `bash examples/run_examples.sh --only "hello" --verify` → isolated test
  4. `bash examples/run_examples.sh --update` → update baselines (should still pass verify)
  5. Check `examples/target/failures.log` for expected WIP failures
  Save evidence to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | End-to-End [PASS/FAIL] | VERDICT`

- [x] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual artifacts. Verify 1:1 mapping.
  Check no unauthorized file modifications outside examples/target/ and examples/run_examples.sh.
  Check no .ry source files modified.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

- **All**: `test(examples): complete compilation test with golden baselines`
  - Files: `examples/run_examples.sh`, `examples/target/*.expected`, `examples/target/baselines.json`, `examples/target/report.md`, `examples/target/failures.log`
  - Pre-commit: `bash examples/run_examples.sh --verify`

---

## Success Criteria

### Verification Commands
```bash
# 1. 脚本语法检查
bash -n examples/run_examples.sh                         # Expected: exit 0

# 2. 帮助输出
bash examples/run_examples.sh --help                     # Expected: shows flags

# 3. Baseline 创建
bash examples/run_examples.sh                            # Expected: generates .expected files

# 4. 验证运行
bash examples/run_examples.sh --verify                   # Expected: reports pass/fail/skip stats

# 5. 单文件隔离测试
bash examples/run_examples.sh --only "hello" --verify    # Expected: PASS hello

# 6. 报告完整性
cat examples/target/report.md                            # Expected: 25 file entries
```

### Final Checklist
- [ ] 所有 "Must Have" 满足（25 文件全部尝试编译、report.md 存在、baselines.json 有效）
- [ ] 所有 "Must NOT Have" 满足（.ry 源文件未修改、无 CI 文件生成）
- [ ] `bash examples/run_examples.sh --verify` 运行成功
- [ ] 失败文件在 `failures.log` 中有详细记录
- [ ] FLAKY 文件在 `baselines.json` 中明确标记

