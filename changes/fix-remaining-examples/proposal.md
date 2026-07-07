# Fix Remaining Examples

## Status

- **Spec-superflow state**: approved-for-build
- **Change name**: `fix-remaining-examples`
- **Workflow**: hotfix
- **Batches planned**: 4

## Intent

解决 `examples/run_examples.sh` 剩余 2 个失败:

1. `macros.ry` (Exit 1): `macro error: no matching rule for macro 'debug' at unknown`
2. `type_aliases.ry` (Exit 139): 顶层 `let` 在非 main 函数访问时 segfault

目标: 33/33 examples pass + 零警告。

## Root Causes

### Issue 1: macros.ry — lexer 把 `$x` 合并为单个 `Ident`

**症状**: `macro debug { ($x) => { print($x); } } debug(42);` 展开失败。

**根因**: `crates/ruyic/src/lexer/scanner.rs:63, 433` 把 `$` 当作 identifier 字符
(`'a'..='z' | 'A'..='Z' | '_' | '$'`),导致 `$x` 被合并为单个 `Ident("$x")` token。

**后果**:
- `parse_pattern` 看不到 `Token::Dollar`,无法识别 metavariable
- pattern parser 把整个 `Ident("$x")` 当作字面量,匹配 `debug(42)` 时永远失败
- macro body 内的 `$x` 替换也失效(`apply_template` 在 line 694 检查 `Token::Dollar`)

**修复**: 从 ident 字符集移除 `$`,新增 `$` 单独处理产生 `Token::Dollar`(已存在于 `Token` enum,line 258)。
字符串插值 `${...}` 已在 line 58-62 单独处理为 `Token::TemplateExprStart`,不受影响。

### Issue 2: type_aliases.ry — 顶层 let 分配在 main 函数的 stack frame

**症状**: 顶层 `let user_name: string = "Alice";` 在 `simple_alias_demo()` 中访问时 segfault。

**根因**: 顶层 `let` 声明在 codegen 时作为 `main` 函数的 stack 变量分配。
非 main 函数通过 `lookup_variable` 查找到同一个 `PointerValue`,但该指针指向 main 函数的 stack frame,
main 返回后 stack 被回收,访问悬空指针导致 segfault。

**修复方案**: 顶层 `let` 应作为 LLVM module-level `global` 变量分配,带 `internal` linkage 和初始值 initializer。
非 main 函数通过 `load global` 访问。

## Approach

**Batch 1**: 修复 lexer — 移除 `$` 从 ident 字符集,新增 `$` → `Token::Dollar` 分支。
验证: 4 个 macro_expand 测试通过 + macros.ry 编译运行成功。

**Batch 2**: 修复 codegen — 顶层 `let` 改为 LLVM global 变量。
- 收集所有顶层 `let`/`const` 声明
- 每个声明创建 module-level `global` with initializer
- `lookup_variable` 对全局名返回指向 global 的指针
- `main` 函数不再为顶层 let 分配 stack

**Batch 3**: 添加单元测试 + integration test。
- lexer: `$x` 应产生 `[Dollar, Ident("x")]`
- codegen: 全局 let 在非 main 函数中可访问

**Batch 4**: 全量验证。
- 33/33 examples pass
- 零警告
- `cargo test --workspace` 全通过
- 已修复 1 个 macro_expand 测试 + 1 个 macro_registry 测试

## Acceptance Criteria

1. `bash examples/run_examples.sh` → 33/33 PASS
2. `cargo build --release` → exit 0, 零警告
3. `cargo test --workspace` → 全部通过(包括 macro_expand 2 个修复的测试)
4. `make lint` → 零警告

## Out of Scope

- Macro hygiene(宏卫生)实现完整化(已基础工作)
- 复杂类型如 `Dyn`/`Trait` 对象的全局变量
- 全局变量的 `mut` 语义
- 模板字符串的 `${expr}` 展开(已基础工作)
