# REQ-CG-002: 顶层 let 改为 LLVM global 变量

## 需求

顶层 `let` 声明在 codegen 时必须作为 LLVM module-level `global` 变量分配,带 initializer,
保证非 main 函数可以安全访问。

## 当前问题

顶层 `let user_name: string = "Alice";` 在 codegen 时被处理为 `main` 函数的 stack 变量。
非 main 函数(如 `simple_alias_demo`)通过 `lookup_variable` 查找到同一个 `PointerValue`,
但该指针指向 main 的 stack frame。main 函数返回后,stack 被回收,访问悬空指针导致 segfault。

### 复现

```bash
./target/release/ruyic examples/type_aliases.ry -o /tmp/ta
/tmp/ta
# Exit 139 (SIGSEGV)
```

## 修复方案

### REQ-CG-002.1 全局变量收集

在 codegen pipeline 开始时(在 main 函数生成之前),扫描所有顶层 `let`/`const` 声明:

- 类型注解
- 初始值
- 名字
- mutability

为每个声明创建一个 LLVM module-level `global` variable:

```rust
let global = self.module.add_global(llvm_type, None, name);
global.set_linkage(inkwell::module::Linkage::Internal);
global.set_initializer(&initializer);
```

### REQ-CG-002.2 变量查找

`lookup_variable(name)` 优先检查全局变量表,返回指向 `global` 的指针。
非 main 函数通过 `load global` 读取。

### REQ-CG-002.3 main 函数处理

main 函数不再为顶层 let 分配 stack slot,但需要执行初始化:
- 对于需要运行时代码的初始化(如函数调用),在 main 入口生成 `store` 到 global
- 对于编译时常量(如 int/float/string literal),直接作为 `initializer`

## 验收

- [ ] `examples/type_aliases.ry` 编译运行成功,Exit 0
- [ ] 全局 `let` 在非 main 函数中可读
- [ ] 已有 examples 不退化
- [ ] 零警告
