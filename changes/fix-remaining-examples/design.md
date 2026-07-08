# Design

## Lexer 修复

### Before

```rust
// scanner.rs:63
'a'..='z' | 'A'..='Z' | '_' | '$' => self.scan_ident_or_keyword(),

// scanner.rs:433
fn is_ident_part(&self, ch: char) -> bool {
    matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '$')
}
```

### After

```rust
// scanner.rs:63 - 新增 $ 分支
'$' if self.peek_char(1) == '{' => {
    self.advance();
    self.advance();
    Token::TemplateExprStart
}
'$' => {
    self.advance();
    Token::Dollar
}
'a'..='z' | 'A'..='Z' | '_' => self.scan_ident_or_keyword(),

// scanner.rs:433
fn is_ident_part(&self, ch: char) -> bool {
    matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_')
}
```

## Codegen 全局变量修复

### Before

```rust
// 处理顶层 let 时,在 main 函数 entry block 分配 stack slot
// 非 main 函数访问时,lookup_variable 查找到 main 的 stack pointer
```

### After

```rust
// 1. 收集所有顶层 let/const
let top_level_lets: Vec<&Declaration> = program.items.iter()
    .filter_map(|item| match item {
        ModuleItem::Declaration(Declaration::Let { ... }) => Some(...),
        _ => None,
    })
    .collect();

// 2. 为每个创建 LLVM global
for let_decl in &top_level_lets {
    let global = self.module.add_global(llvm_type, None, &name);
    global.set_linkage(Linkage::Internal);
    global.set_initializer(&init_value);
    self.globals.insert(name.clone(), global);
}

// 3. lookup_variable 优先查 globals
fn lookup_variable(&self, name: &str) -> Option<...> {
    if let Some(global) = self.globals.get(name) {
        // 返回指向 global 的指针,非 main 函数 load 它
        return Some((global.as_pointer_value(), ty.clone()));
    }
    self.variables.get(name).map(...)
}
```

### 注意

- LLVM `global` 变量的地址本身就是 `PointerValue`,可以直接用作 `alloca` 的替代
- `set_initializer` 需要一个 `AnyValueEnum`,从字面量直接构造
- 对于需要运行时的初始化(如函数调用结果),在 main 入口生成 `store` 指令
