# Ruyi 版本管理规则制定

## TL;DR

> **Quick Summary**: 为 Ruyi 项目创建完整的版本管理规范体系，覆盖版本切换检查清单、Git 工作流、Tag 规范、Commit 消息标准，分布于 AGENTS.md（工作流规则）、roadmap.md（状态追踪）和新建 docs/versioning.md（完整规范）。

> **Deliverables**:
> - 更新 `AGENTS.md` — 扩展 Workflow Rules 章节
> - 更新 `docs/roadmap.md` + `docs/roadmap-zh.md` — 添加版本状态表 + 更新 Current State Assessment
> - 新建 `docs/versioning.md` — 完整版本管理规范

> **Estimated Effort**: Quick
> **Parallel Execution**: YES — 3 tasks 完全独立，可全部并行
> **Critical Path**: 无依赖链，全部并行

---

## Context

### Original Request
用户确认开始 v0.5 前，需要将前置问题整理成为正式规则，统一版本管理流程。

### Interview Summary
**Key Discussions**:
- 规则同时写入 3 个位置：AGENTS.md（Agent 工作流）+ roadmap.md（版本状态追踪）+ 新建 docs/versioning.md（完整规范）
- 粒度：同时产出简明检查清单和完整 Git 工作流规范
- Tag 格式：`vX.Y.Z` (SemVer)
- Tag 打在 main 的 merge commit 上
- 支持补丁版本 (vX.Y.Z)

**Key Decisions**:
- v0.2.0 和 v0.3.0 补打 Tag
- CI 作为 v0.5 第一个任务，不作为 v0.5 前置条件
- dev/v0.4 未提交的 codegen 改动提交为 v0.4 的一部分
- 单一维护者，无需多人协作规则
- 暂不包括 CHANGELOG、Feature 分支、发布脚本

### Metis Review
**Identified Gaps** (addressed):
- Tag 格式 vs 分支命名粒度：SemVer `v0.4.0`，分支名保持 `dev/v0.4`
- Patch 版本分支策略：直接在主分支修复，无需新分支
- Cargo.lock 处理：Rust 项目标准 — 提交并随版本更新
- 合并策略：merge commit（非 squash），保留完整历史
- 回滚程序：已在 versioning.md 中覆盖 3 种场景

---

## Work Objectives

### Core Objective
将版本管理流程从"口头约定"固化为"书面规则"，确保每次版本切换有章可循，消除遗漏和歧义。

### Concrete Deliverables
- `AGENTS.md`（更新）：Workflow Rules 章节扩展，含检查清单、分支策略、Tag 规范、Commit 规范
- `docs/roadmap.md`（更新）：添加 Version Release Status 表，更新 Current State Assessment
- `docs/roadmap-zh.md`（更新）：同步中文版更新
- `docs/versioning.md`（新建）：完整版本管理规范

### Definition of Done
- [ ] AGENTS.md Workflow Rules 包含版本切换检查清单、分支策略、Tag 规范、发布流程、Commit 规范
- [ ] roadmap.md 顶部有 Version Release Status 表，Current State Assessment 反映 v0.4 完成状态
- [ ] roadmap-zh.md 与 roadmap.md 内容同步
- [ ] docs/versioning.md 存在且包含：SemVer 规则、分支模型、Tag 规范、21 项检查清单、Commit 规范、回滚程序
- [ ] 三份文件之间无矛盾（AGENTS.md 的每条规则在 versioning.md 中有对应说明）

### Must Have
- 版本切换时逐项可执行的检查清单（checkboxes，非散文）
- 明确的 Tag 格式和创建命令
- 回滚操作步骤（3 种场景）
- 中英文 roadmap 同步更新

### Must NOT Have (Guardrails)
- 不执行任何 Git 操作（不创建 tag、不创建分支、不修改代码）
- 不创建 CI/CD 配置文件
- 不创建 CHANGELOG.md
- 不定义 Feature 分支命名规则
- 不创建发布脚本/自动化
- 不修改 Cargo.toml 或任何源代码

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed.

### Test Decision
- **Infrastructure exists**: N/A（纯文档任务）
- **Automated tests**: None
- **Framework**: N/A

### QA Policy
每项任务通过 grep/file 存在性验证 + 与草案内容对比确认一致性。

---

## Execution Strategy

### Parallel Execution Waves

> 三个文件完全独立，无依赖关系，全部并行。

```
Wave 1 (同时开始，全部并行):
├── Task 1: 更新 AGENTS.md [writing]
├── Task 2: 更新 roadmap.md + roadmap-zh.md [writing]
└── Task 3: 新建 docs/versioning.md [writing]

Wave FINAL (After ALL tasks):
├── Task F1: 跨文件一致性验证
└── Task F2: 内容完整性检查
```

**Critical Path**: 无 — 全部并行，3 tasks x writing agent
**Parallel Speedup**: ~100% (全部同时执行)
**Max Concurrent**: 3

---

## TODOs

- [x] 1. 更新 AGENTS.md — 扩展 Workflow Rules

  **What to do**:
  - 读取当前 `AGENTS.md` 完整内容
  - 定位 `## Workflow Rules` 章节（第 119-127 行）
  - 将该章节替换为完整的版本管理规则，包含以下子章节：
    - **版本切换检查清单**：10 项 checklist，从 `git status` 到 `tag` 验证
    - **分支策略**：main 只接受 merge，`dev/v{major}.{minor}` 命名，分支永久保留
    - **Tag 规范**：格式 `vX.Y.Z`，annotated tag，在 main merge commit 上创建
    - **版本发布流程**：5 步（确认完成 → 合并 main → 打 Tag → 更新 Roadmap → 创建下版本分支）
    - **Commit 消息规范**：`type(scope): description`，Conventional Commits
    - **环境要求**：有/无 LLVM 环境的验证命令
  - 保留原有的 3 条 Workflow Rules 内容（版本号更新、Roadmap 更新、示例编译），整合到新结构中
  - 保持 AGENTS.md 其他章节不变

  **Must NOT do**:
  - 不修改 AGENTS.md 头部结构、Project Structure、Compilation Pipeline 等章节
  - 不添加 CI/CD 相关规则
  - 不添加 Feature 分支规则
  - 不添加多人协作/Code Review 规则

  **Recommended Agent Profile**:
  - **Category**: `writing`
    - Reason: 纯文档编写，需要精确排版和专业表述
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 2, 3)
  - **Blocks**: None
  - **Blocked By**: None

  **References**:
  - `AGENTS.md:119-127` — 现有 Workflow Rules 章节，需原地替换
  - `.sisyphus/drafts/versioning-rules.md` — 草案内容，"一、AGENTS.md 新增内容" 章节为直接素材

  **Acceptance Criteria**:
  - [ ] `grep -q "版本切换检查清单" AGENTS.md` → PASS
  - [ ] `grep -q "分支策略" AGENTS.md` → PASS
  - [ ] `grep -q "Tag 规范" AGENTS.md` → PASS
  - [ ] `grep -q "版本发布流程" AGENTS.md` → PASS
  - [ ] `grep -q "Commit 消息规范" AGENTS.md` → PASS
  - [ ] `grep -q "环境要求" AGENTS.md` → PASS

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: AGENTS.md 包含所有必需章节
    Tool: Bash
    Preconditions: 文件已更新
    Steps:
      1. grep -c "版本切换检查清单" AGENTS.md → ≥ 1
      2. grep -c "### 分支策略" AGENTS.md → ≥ 1
      3. grep -c "### Tag 规范" AGENTS.md → ≥ 1
      4. grep -c "### 版本发布流程" AGENTS.md → ≥ 1
      5. grep -c "### Commit 消息规范" AGENTS.md → ≥ 1
    Expected Result: 所有 grep 返回计数 ≥ 1
    Failure Indicators: 任一 grep 返回 0
    Evidence: .sisyphus/evidence/task-1-sections.txt

  Scenario: 原有章节未被破坏
    Tool: Bash
    Preconditions: 文件已更新
    Steps:
      1. grep -c "## What This Is" AGENTS.md → = 1
      2. grep -c "## Project Structure" AGENTS.md → = 1
      3. grep -c "## Compilation Pipeline" AGENTS.md → = 1
      4. grep -c "## Developer Commands" AGENTS.md → = 1
      5. grep -c "## Testing" AGENTS.md → = 1
    Expected Result: 所有章节仍存在且唯一
    Failure Indicators: 任一章节消失或重复
    Evidence: .sisyphus/evidence/task-1-integrity.txt
  ```

  **Evidence to Capture**:
  - [ ] `.sisyphus/evidence/task-1-sections.txt`
  - [ ] `.sisyphus/evidence/task-1-integrity.txt`

  **Commit**: YES (groups with Task 2)
  - Message: `docs: add version management rules to AGENTS.md and roadmap`
  - Files: `AGENTS.md`, `docs/roadmap.md`, `docs/roadmap-zh.md`

- [x] 2. 更新 roadmap.md + roadmap-zh.md — 版本状态追踪

  **What to do**:
  - 读取 `docs/roadmap.md` 和 `docs/roadmap-zh.md`
  - 在 `## Current State Assessment` 之前插入 `## Version Release Status` 表格：
    ```
    | Version | Branch | Status | Release Date | Tag |
    |---------|--------|--------|-------------|-----|
    | v0.2    | dev/v0.2 | ✅ Released | 2026-05 | v0.2.0 (待补打) |
    | v0.3    | dev/v0.3 | ✅ Released | 2026-05 | v0.3.0 (待补打) |
    | v0.4    | dev/v0.4 | 🔄 In Progress | TBD | — |
    | v0.5    | — | ⏳ Planned | 2026 Q4 | — |
    ```
  - 更新 `Current State Assessment` 表格中已变更的字段：
    - Typechecker: ~75% → ~90%，gaps 更新为已完成的项
    - Codegen: ~45% → ~60%（v0.2 完成 + 未提交成员访问补充）
    - GC: "Stub (compiler) / 85% (runtime)" → "~70% (compiler) / 85% (runtime)"（v0.3 对接）
    - Runtime: "30% (compiler) / 70% (library)" → "~60% (compiler) / 70% (library)"
    - Driver: "Runtime not linked" → "Runtime linked"（v0.3 完成）
  - 对 roadmap-zh.md 做完全相同的修改（中文表头：版本发布状态）
  - 保持 `## Phase 1` 之后的所有内容不变（v0.5 任务描述保留）

  **Must NOT do**:
  - 不修改 v0.5 任务内容
  - 不添加新的 Phase 或计划
  - 不删除任何现有内容
  - 不创建 CI 相关条目（保持 CI/CD: ❌ None）

  **Recommended Agent Profile**:
  - **Category**: `writing`
    - Reason: 结构化文档更新，需要精确的表格格式和状态字段映射
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 3)
  - **Blocks**: None
  - **Blocked By**: None

  **References**:
  - `docs/roadmap.md:17-61` — Current State Assessment 表格，需更新完成度百分比和 Gaps
  - `docs/roadmap-zh.md:17-61` — 中文版对应位置
  - `.sisyphus/drafts/versioning-rules.md` — "二、Roadmap 新增内容" 章节为直接素材

  **Acceptance Criteria**:
  - [ ] `grep -q "Version Release Status" docs/roadmap.md` → PASS
  - [ ] `grep -q "版本发布状态" docs/roadmap-zh.md` → PASS
  - [ ] `grep -q "v0.4.*In Progress" docs/roadmap.md` → PASS
  - [ ] `grep -q "v0.5.*Planned" docs/roadmap.md` → PASS
  - [ ] roadmap.md 与 roadmap-zh.md 状态表内容一致

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: 版本状态表正确插入
    Tool: Bash
    Preconditions: 文件已更新
    Steps:
      1. grep -c "Version Release Status" docs/roadmap.md → ≥ 1
      2. grep -c "v0.2.*Released" docs/roadmap.md → ≥ 1
      3. grep -c "v0.3.*Released" docs/roadmap.md → ≥ 1
      4. grep -c "v0.4.*In Progress" docs/roadmap.md → ≥ 1
      5. grep -c "v0.5.*Planned" docs/roadmap.md → ≥ 1
    Expected Result: 所有版本有正确状态
    Failure Indicators: 任一版本缺失或状态错误
    Evidence: .sisyphus/evidence/task-2-status-table.txt

  Scenario: 中英文同步
    Tool: Bash
    Preconditions: 文件已更新
    Steps:
      1. grep -c "版本发布状态" docs/roadmap-zh.md → ≥ 1
      2. grep -c "v0.2.*已发布\|v0.2.*Released" docs/roadmap-zh.md → ≥ 1
      3. grep -c "v0.4.*进行中\|v0.4.*In Progress" docs/roadmap-zh.md → ≥ 1
    Expected Result: 中文版包含与英文版对应的状态信息
    Failure Indicators: 中文版缺失状态表或版本信息不一致
    Evidence: .sisyphus/evidence/task-2-zh-sync.txt

  Scenario: 原有内容未被删除
    Tool: Bash
    Preconditions: 文件已更新
    Steps:
      1. grep -c "## Phase 1: Foundation Library" docs/roadmap.md → ≥ 1
      2. grep -c "### v0.5.*Standard Library" docs/roadmap.md → ≥ 1
      3. grep -c "## Phase 2" docs/roadmap.md → ≥ 1
    Expected Result: Phase 1-3 章节完整保留
    Failure Indicators: 任何章节被删除
    Evidence: .sisyphus/evidence/task-2-integrity.txt
  ```

  **Evidence to Capture**:
  - [ ] `.sisyphus/evidence/task-2-status-table.txt`
  - [ ] `.sisyphus/evidence/task-2-zh-sync.txt`
  - [ ] `.sisyphus/evidence/task-2-integrity.txt`

  **Commit**: YES (groups with Task 1)
  - Message: `docs: add version management rules to AGENTS.md and roadmap`
  - Files: `docs/roadmap.md`, `docs/roadmap-zh.md`

- [x] 3. 新建 docs/versioning.md — 完整版本管理规范

  **What to do**:
  - 创建 `docs/versioning.md`，内容包含以下章节（参照草案 `.sisyphus/drafts/versioning-rules.md` "三、docs/versioning.md"）：
    - **概述**：文档目的和范围
    - **版本号规则**：SemVer 2.0，MAJOR.MINOR.PATCH
    - **分支模型**：main + dev/vX.Y 结构，生命周期图，规则说明
    - **Tag 规范**：格式 `vX.Y.Z`，annotated tag，创建和推送命令
    - **版本切换检查清单**：21 项，分 5 个阶段（关闭旧版 → 合并 main → 打 Tag → 更新文档 → 启动新版）
    - **Commit 消息规范**：Conventional Commits，类型和范围表
    - **回滚程序**：3 种场景（合并后未打Tag / 已打Tag发现Bug / 紧急回滚）
    - **环境验证最低要求**：有 LLVM 完整验证 vs 无 LLVM 有限验证
  - 确保命令示例可直接复制执行（非占位符）
  - 确保 checklist 使用 Markdown checkbox 格式

  **Must NOT do**:
  - 不创建 CI/CD 配置文件
  - 不添加自动化脚本
  - 不定义 Feature 分支规则
  - 不添加 CHANGELOG 相关规则
  - 不涉及多人协作/Code Review 流程

  **Recommended Agent Profile**:
  - **Category**: `writing`
    - Reason: 技术规范文档，需要专业规范的排版和准确的命令示例
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2)
  - **Blocks**: None
  - **Blocked By**: None

  **References**:
  - `.sisyphus/drafts/versioning-rules.md` — 草案，"三、docs/versioning.md" 章节为完整内容素材
  - 现有 `docs/roadmap.md` — 参考文档风格（标题格式、表格风格、分隔线用法）

  **Acceptance Criteria**:
  - [ ] `test -f docs/versioning.md` → PASS（文件存在）
  - [ ] `grep -q "版本号规则" docs/versioning.md` → PASS
  - [ ] `grep -q "分支模型" docs/versioning.md` → PASS
  - [ ] `grep -q "Tag 规范" docs/versioning.md` → PASS
  - [ ] `grep -q "检查清单" docs/versioning.md` → PASS
  - [ ] `grep -q "回滚" docs/versioning.md` → PASS
  - [ ] checkbox 项 ≥ 16（覆盖 21 项清单全部子项）

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: 文件创建且包含所有必需章节
    Tool: Bash
    Preconditions: 文件已创建
    Steps:
      1. test -f docs/versioning.md → exit 0
      2. grep -c "## " docs/versioning.md → ≥ 6 (概述/版本号/分支/Tag/检查清单/Commit/回滚/环境)
      3. grep -c "\- \[ \]" docs/versioning.md → ≥ 16 (checklist 项)
      4. grep -c '```bash' docs/versioning.md → ≥ 3 (命令示例代码块)
    Expected Result: 文件结构完整，checklist 和命令示例充足
    Failure Indicators: 章节数 < 6，checkbox < 16，或无命令示例
    Evidence: .sisyphus/evidence/task-3-structure.txt

  Scenario: 检查清单覆盖所有 5 个阶段
    Tool: Bash
    Preconditions: 文件已创建
    Steps:
      1. grep -c "关闭旧版本\|关闭.*版本" docs/versioning.md → ≥ 1
      2. grep -c "合并到 main\|合并.*main" docs/versioning.md → ≥ 1
      3. grep -c "打 Tag\|创建.*Tag\|git tag" docs/versioning.md → ≥ 1
      4. grep -c "更新文档\|更新.*roadmap" docs/versioning.md → ≥ 1
      5. grep -c "启动新版本\|创建.*分支\|git checkout -b" docs/versioning.md → ≥ 1
    Expected Result: 5 个阶段各有至少 1 处提及
    Failure Indicators: 任一阶段缺失
    Evidence: .sisyphus/evidence/task-3-phases.txt

  Scenario: 回滚程序覆盖 3 种场景
    Tool: Bash
    Preconditions: 文件已创建
    Steps:
      1. grep -c "未打 Tag\|未.*tag" docs/versioning.md → ≥ 1
      2. grep -c "revert\|回滚" docs/versioning.md → ≥ 1
      3. grep -c "hotfix\|PATCH\|补丁" docs/versioning.md → ≥ 1
    Expected Result: 3 种回滚场景均有覆盖
    Failure Indicators: 回滚场景 < 2
    Evidence: .sisyphus/evidence/task-3-rollback.txt
  ```

  **Evidence to Capture**:
  - [ ] `.sisyphus/evidence/task-3-structure.txt`
  - [ ] `.sisyphus/evidence/task-3-phases.txt`
  - [ ] `.sisyphus/evidence/task-3-rollback.txt`

  **Commit**: YES (单独提交)
  - Message: `docs: add comprehensive versioning specification`
  - Files: `docs/versioning.md`

---

## Final Verification Wave

- [x] F1. **跨文件一致性检查**
  逐条对比 AGENTS.md 规则与 versioning.md 对应章节：
  - versioning.md 中是否包含 AGENTS.md 所有规则的详细说明
  - roadmap.md 状态表是否与 project 实际 git 状态一致
  - 中英文 roadmap 版本状态表是否一致
  Output: `Consistency [AGENTS↔versioning N/N] | [roadmap↔zh N/N] | [roadmap↔git N/N] | VERDICT`

- [x] F2. **内容完整性检查**
  逐项验证 deliverables 要求：
  - AGENTS.md: grep 确认 Workflow Rules 含 checklist + branch strategy + tag + commit 规范
  - roadmap.md: grep 确认 Version Release Status 表 + Updated Current State Assessment
  - versioning.md: grep 确认 SemVer + Branch Model + Tag + 21-item Checklist + Commit + Rollback
  Output: `AGENTS [N/N] | roadmap [N/N] | roadmap-zh [N/N] | versioning [N/N] | VERDICT`

---

## Commit Strategy

- **1**: `docs: add version management rules and release workflow` — AGENTS.md, docs/roadmap.md, docs/roadmap-zh.md
- **2**: `docs: add comprehensive versioning specification` — docs/versioning.md

---

## Success Criteria

### Verification Commands
```bash
# AGENTS.md 包含版本管理规则
grep -q "版本切换检查清单" AGENTS.md && echo "PASS" || echo "FAIL"

# roadmap.md 包含版本状态表
grep -q "Version Release Status" docs/roadmap.md && echo "PASS" || echo "FAIL"

# roadmap-zh.md 同步更新
grep -q "版本发布状态" docs/roadmap-zh.md && echo "PASS" || echo "FAIL"

# versioning.md 存在且包含所有必需章节
grep -q "Release Checklist" docs/versioning.md && \
grep -q "Rollback" docs/versioning.md && \
grep -q "Branch Strategy" docs/versioning.md && \
grep -q "Commit Message" docs/versioning.md && echo "PASS" || echo "FAIL"
```

### Final Checklist
- [ ] 三份文件内容无矛盾
- [ ] 中英文 roadmap 同步
- [ ] 所有规则可被人类逐项执行（非抽象描述）
