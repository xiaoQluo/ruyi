# Execution Contract

## Scope

修复 `examples/run_examples.sh` 剩余 2 个失败,目标 33/33 PASS。

## In Scope

1. Lexer `$` token 处理(scanner.rs 修复)
2. Codegen 顶层 let 全局化(generator.rs + decl.rs)
3. 2 个 macro_expand 测试修复(test_macro_expand_with_arg, test_macro_registry_user_macros)

## Out of Scope

- Macro hygiene 完整化
- 全局变量的 `mut` 语义
- 模板字符串 `${expr}` 展开(已基础工作)
- 14 个 WIP 预存 parser 失败
- 其他 31 个已通过 examples 的行为变化

## Acceptance

| 指标 | 基线 | 目标 |
|---|---|---|
| Examples 通过 | 31/33 | **33/33** |
| 编译警告 | 0 | 0 |
| macro_expand 测试 | 13/15 | 15/15 |
| 全 cargo test | pass | pass |

## Risks

- **R1**: Lexer 移除 `$` 从 ident 可能影响模板字符串外的代码(变量名带 `$`)
  - 缓解: 全量 cargo test + examples 验证
- **R2**: LLVM global 初始化器对复杂类型支持有限
  - 缓解: 对于不能静态初始化的,在 main 入口生成 store 指令

## Rollback

如果 lexer/codegen 修改导致大规模回归:
1. revert 相关 commit
2. 退回 31/33 baseline 状态
3. 记录具体失败场景,单独 fix