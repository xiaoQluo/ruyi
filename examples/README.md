# examples/ — Ruyi 示例代码目录

本目录下的 `.ry` 文件都是**面向用户的可编译运行示例**,会被
`run_examples.sh` 自动扫描并执行。

## 目录结构

| 子目录 | 用途 | 导入方式 |
|---|---|---|
| `basics/` | 语言基础语法 + 自动加载的 stdlib 模块 | 无需 import |
| `types/` | 类型系统(泛型、trait、模式匹配、类型别名) | 按需 import |
| `oop/` | 面向对象(class、object literal、accessor) | 按需 import |
| `concurrency/` | 并发与异步(async、thread、channel、fiber、sync) | 按需 import |
| `stdlib/` | 标准库模块演示 | 每个示例头部 import |
| `comprehensive/` | 综合演示(覆盖多个 stdlib 模块) | import 各模块 |

## "演示跳过"标注规范

当前 Ruyi 运行时存在若干**已知限制**(参见 `changes/v0.5.5-residual-fixes/FINAL_REPORT.md`
和 `docs/pending-issues.md`),某些示例的演示因此只能"局部执行"。本目录约定
如下统一标注,以便读者一眼识别:

### 代码注释中的标注

```ruyi
// ⚠ skipped (原因): 这部分演示被跳过
```

**已使用的具体原因**(在 `examples/` 中出现过的):
- `Thread.spawn 为 stub`: 跨线程代码段跳过(recv/sleep 会挂起)
- `Fiber.spawn 为 stub`: 协程 join/recv 跳过
- `RWLock.readLock() 与 tryReadLock() 共用 self._guard 字段`: 写锁演示跳过
- `当前 codegen 下连续多行 print 后跟 printOk 会崩溃`: pool/json 详细演示跳过
- `当前平台 (LLVM 14 macOS) 不支持运行时栈展开`: catch 块不会真正触发

### 运行输出中的标注

```ruyi
print("  ⚠ <api>.xxx() skipped (<原因>)\n");
```

控制台中看到 `⚠` 字符即表示此处演示被局部跳过,只验证了 API 的可用性
而非完整功能。

### 添加新的"跳过"标注

在源码中写 `// skipped (原因)` 时,请遵循:
1. **必须给出原因** — 不写"skipped"无原因的标注
2. **使用 `⚠` 字符** — 便于在控制台输出中识别
3. **简短** — 一行内能写完就一行;长说明用注释块单独写
4. **关联已知 issue** — 重要跳过项应在 `docs/pending-issues.md` 有对应条目

## 添加新示例

```bash
# 1. 在对应分类目录下创建文件
echo 'fn main() { print("hello\n"); }' > examples/basics/my_demo.ry

# 2. 不需要修改 run_examples.sh — 它用 find 自动发现 .ry 文件

# 3. (可选) 编写 .expected 基线:
#    examples/target/my_demo.expected 包含期望输出

# 4. 本地运行验证:
make build-release
./target/release/ruyic examples/basics/my_demo.ry -o /tmp/my_demo
/tmp/my_demo
```

## 相关文档

- [`docs/spec.md`](../../docs/spec.md) — 语言规范(权威)
- [`docs/tutorial.md`](../../docs/tutorial.md) — 教程
- [`docs/roadmap.md`](../../docs/roadmap.md) — 路线图与 P0/P1 待办项
- [`docs/pending-issues.md`](../../docs/pending-issues.md) — 当前未修复缺陷
- [`run_examples.sh`](./run_examples.sh) — 示例运行器
