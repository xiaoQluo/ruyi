# REQ-LX-001: Lexer `$` 应作为独立 token

## 需求

lexer 必须把 `$` 作为独立的 `Token::Dollar` 输出,而不是 identifier 的一部分。

## 当前问题

`crates/ruyic/src/lexer/scanner.rs:63` 和 `line 433` 把 `$` 当作 identifier 字符:

```rust
// line 63
'a'..='z' | 'A'..='Z' | '_' | '$' => self.scan_ident_or_keyword(),

// line 433
fn is_ident_part(&self, ch: char) -> bool {
    matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '$')
}
```

这导致输入 `$x` 被 lex 为单个 `Ident("$x")` token,而不是 `Dollar` + `Ident("x")`。

## 影响

1. **Macro pattern 解析失败**: `parse_pattern` 依赖 `Token::Dollar` 来识别 metavariable
2. **Macro body 替换失效**: `apply_template` 在 `expand.rs:694` 检查 `Token::Dollar`
3. **测试失败**: `test_macro_expand_with_arg`, `test_macro_registry_user_macros`
4. **Example 失败**: `examples/macros.ry` (Exit 1)

## 修复

### REQ-LX-001.1 字符集排除

- `scanner.rs:63` 移除 `$` 从 ident 起始字符集
- `scanner.rs:433` 移除 `$` 从 ident part 字符集

### REQ-LX-001.2 独立 `$` 处理

- `scanner.rs:63` 之前新增 `$` 分支:
  ```rust
  '$' => {
      self.advance();
      Token::Dollar
  }
  ```
- 注意: `${` 已在 `line 58-62` 单独处理为 `Token::TemplateExprStart`,优先级更高(`if self.peek_char(1) == '{'`)

### REQ-LX-001.3 测试

- 单元测试: `lex("$x")` 返回 `[Dollar, Ident("x")]`
- 单元测试: `lex("${a}")` 返回 `[TemplateExprStart, Ident("a"), RBrace]`
- 现有测试 `test_macro_expand_with_arg` 应通过

## 验收

- [ ] 所有 lexer 单元测试通过
- [ ] `test_macro_expand_with_arg` 通过
- [ ] `test_macro_registry_user_macros` 通过
- [ ] `examples/macros.ry` 编译运行成功
