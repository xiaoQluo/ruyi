# changes/ 状态索引

spec-superflow 规划产物目录。本索引由 2026-10-18 逐项对照当前代码核实生成（项目 skill 依赖的 `ssf` CLI 与 `scripts/spec-superflow.mjs` 在本机不存在，状态字段为手工修正，判定证据为实测代码位置）。

约定：**本项目不维护中心 spec 基线**，各 change 的 `specs/` 是 delta 快照，只留档不回写（与 v0.5.5–v0.5.9 收尾一致）。

## 已归档（实现已在 main）

| change | 原状态 | 核实依据（摘要） |
|--------|--------|------------------|
| fix-codegen-gaps | closing | `codegen/decl.rs:86-87` 宏/类型别名跳过；`codegen/patterns.rs:36` `compile_bigint_match`；`ruyi_runtime/src/builtins.rs:313` `ruyi_bigint_eq` |
| fix-remaining-examples | closing | lexer `$` 方案被 `7a04081` 反向否决（详见其 tasks.md 补记）；顶层 let 全局化见 `codegen/generator.rs:83,1254,1570` |
| fix-try-catch-invoke | closing | landing pad IR 生成在用；**跨函数传播仍未通**，剩余项转 fix-exception-unwinder |
| v0.5.5-residual-fixes | closing | T-1.3.1 起 21 个 codegen 测试解除忽略 |
| v0.5.6-codegen-doc-drift | closing | 文档漂移修复 |
| v0.5.7-p1-defects | closing | 9 项 P1（supertraits/narrowing/exhaustiveness/self-referential/async GC roots/random/fmt/test/collections） |
| v0.5.8-stdlib-core | closing | math/time/json stdlib |
| v0.5.9-stdlib-cleanup | closing | archive 异常、表驱动 codegen、parser dyn 可选、fmt 8 参 FFI、clippy 核验 |
| stdlib-io-path-process-ffi | **executing** | `ruyi_runtime/src/{io,path,process}_ffi.rs` 均在；交付提交 `920eccf` 是 main 祖先 |
| v0.5.9-class-codegen | **state 空 / dp_2_approved** | `codegen/expr.rs:2399-2404`、`:2494-2505`、`:2570`；相关测试已进默认套件（codegen 94 passed / 0 ignored） |
| fix-exception-unwinder | **specifying** | 原 unwinder 设计被 pending-exception 协议取代；实际修复落在 runtime 哨兵/传播出口/外层 try 查找/检查点/用户类映射五处。`tests/try_catch_invoke.rs` 14/14（含 6 个原 `#[ignore]`）。详见 yaml 的 Execution addendum |

## 无状态文件，核实为已完成（视为已归档，未补造状态文件）

| change | 核实依据（摘要） |
|--------|------------------|
| fix-codegen-string-dispatch | `codegen/expr.rs:7004-7036` 类型分派表覆盖 String/Int/Float/Bool/Array/Byte/Generic/Nullable/Dynamic；`:7234,7294` `__string_<snake>` 约定；`basename(..).length` 复现路径已通 |
| fix-parser-typechecker-syntax | 11 项构造根因均可定位：`parser.rs:2130`（Array 后缀）、`:515/:283`（ExternFn）、`:2311,2348`（Shr/ShrAssign）、`ast.rs:249`、`ast.rs:264`、`inference.rs:815-819,239,293,2686-2690` |
| fix-rustic-static-method | `typechecker/inference.rs:152,229,665,2119,2186` 静态/实例分派；跨模块用例 `tests/integration/cases/stdlib/path_ffi.ry` 直接 `Path.basename(...)` |
| v0.2-codegen-gaps | 其缺口由 v0.5.5-residual-fixes 关闭，见 docs/roadmap.md「T9 缺口收尾」段 |

## 未进入执行 — 仍为有效待办（状态保持 specifying，勿当已收尾）

| change | 状态 | 剩余问题（实测） |
|--------|------|------------------|
| fix-batch-low-risk-defects | specifying | 12 条 codegen 路径中 6 条仍返回 "not yet supported"：`codegen/expr.rs:6906,7330,9113,10716-10727`、`codegen/decl.rs:490`（嵌套成员访问、间接调用、复杂赋值、复杂 new、复杂模式） |
| add-test-runner-cli | 仅 proposal.md | `main.rs:46-47,75` 有 `--test`，但 `driver.rs:591-625` 只打印发现数即返回；`runtime/test_registry.rs` 无 `run`；PASS/FAIL 报告与 exit code、harness 全缺。roadmap 10.1（`docs/roadmap.md:327`）仍为未完成 |
| native-enum-type | 仅 proposal.md | 只有 `lexer/scanner.rs:499` + `token.rs:102` 认出关键字；`parser/ast.rs` 无 `Declaration::Enum`、`Pattern` 无 `Constructor`；`stdlib/option.ry:25` 仍是 `class Some<T>`；spec `docs/spec.md:3967` 标「计划中」 |
| stdlib合理性检查 | abandoned | 保持 abandoned，delta specs 仅留档，不得回写 |

## 悬空待办（无对应 change 目录）

`fix-codegen-string-dispatch`、`fix-parser-typechecker-syntax`、`fix-rustic-static-method` 三份 proposal 均点名后续变更 `remove-stdlib-workarounds`，但该目录不存在；`stdlib/fs.ry:59-130` 仍直连 `__path_*` FFI。需另立 change 认领。
