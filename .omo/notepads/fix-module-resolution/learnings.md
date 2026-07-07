# Module Resolution Fix - Learnings

## Problem
`ModuleResolver::resolve()` 方法没有处理 `./` 和 `../` 前缀的相对路径，而是直接在当前工作目录下查找模块。当运行 `ruyic examples/modules/main.ry --check` 时，`import { add } from "./math"` 报错 "module not found: \"./math\""。

## Root Cause
- `resolve` 方法签名没有 `base_path` 参数，无法知道导入文件所在目录
- `resolve_imports` 方法虽然接收 `_input_path` 参数，但用了下划线前缀（未使用），也没有传递给 `resolve`

## Changes Made
1. **`ModuleResolver::resolve`**（第 175 行）：添加 `base_path: Option<&Path>` 参数，在方法开头增加对 `./` 和 `../` 前缀的处理：
   - 如果 module_name 以 `./` 或 `../` 开头，使用 `base_path.parent()` 作为基准目录
   - 拼接 `{base_dir}/{module_name}.ry` 并检查文件是否存在
   - 如果不存在，直接返回 `ModuleNotFound` 错误（不继续尝试 stdlib 等）

2. **`resolve_imports`**（第 357 行）：将 `_input_path` 改为 `input_path`（去掉下划线前缀），并传递给 `resolve(&import_decl.source, Some(input_path))`

## Result
- "module not found" 错误已消除
- `cargo check --workspace` 通过
- `make check` 通过
- 剩余 "Unknown variable" 错误是类型检查器的独立问题，不在此修复范围内

## Edge Cases Covered
- `./` 前缀：相对于导入文件所在目录
- `../` 前缀：相对于导入文件所在目录的父目录
- `base_path` 为 `None` 时：回退到原有的标准解析逻辑（向后兼容）
- 文件不存在时：返回明确的 `ModuleNotFound` 错误

---

## Second Fix: Export Unwrapping in resolve_imports (2026-05-07)

### Problem
`resolve_imports` 合并导入模块时，直接复制了所有 `ModuleItem`（包括 `Export` 包装的项）。但类型检查器的 `infer_module_item` 跳过 `ModuleItem::Export` 项，导致导出的函数/常量不被注册到类型环境中，报 `Unknown variable: add` 等错误。

### Root Cause
- `infer_module_item` 对 `ModuleItem::Import(_)` 和 `ModuleItem::Export(_)` 都直接 return（不处理）
- 从被导入模块直接复制 Export 项到主程序后，类型检查器看不到这些声明

### Changes Made (all in `driver.rs`)

1. **新增 `push_unwrapped` 静态方法**（在 `impl Driver` 块内）：
   - 接收 `&mut Program` 和 `&ModuleItem`
   - 如果是 `ModuleItem::Export(ExportDecl::Declaration(decl))` → 推入 `ModuleItem::Declaration(decl)`
   - 如果是 `ExportDecl::DefaultFunction{...}` → 构造 `Declaration::Function{...}` 并推入
   - 如果是 `ExportDecl::DefaultClass{...}` → 构造 `Declaration::Class{...}` 并推入
   - 其他 Export 类型（Named, ReExportAll, ReExportNamed, DefaultExpr）→ 跳过
   - 非 Export 的 ModuleItem → 直接 clone 推入

2. **新增 `collect_export_names` 静态方法**：
   - 从 ModuleItem 中提取导出名称列表（用于 namespace import 构建对象字面量）
   - 处理 Declaration::Function/Class/Const/Let 以及 DefaultFunction/DefaultClass

3. **重写 `resolve_imports` 中的合并逻辑**（三阶段）：
   - **Phase 1**: 在处理模块合并之前，先收集 `ReExportAll`/`ReExportNamed` 的 sources，
     然后独立加载并合并这些重导出模块的项（解决 borrow checker 冲突）
   - **Phase 2**: 主合并——使用 `push_unwrapped` 展开所有 Export 项
   - **Phase 3**: 创建局部绑定：
     - 别名导入 `import { x as y }` → 创建 `const y = x` 声明
     - 命名空间导入 `import * as ns` → 创建 `const ns = { name1, name2, ... }` 对象字面量

### Key Design Decisions

1. **borrow checker 规避**：`resolve_imports` 需要同时访问 `self.resolver.loaded_modules`（借入）
   和 `self.resolve_imports()`（借出自引用），所以必须先将重导出的 sources 收集到 `Vec<(String, PathBuf)>` 中，
   再循环处理。不能直接在遍历 `module.items` 时调用 `self.resolve_imports`。

2. **路径解析**：re-export 的 source 是相对于所在模块的目录解析的。传入模块文件的完整路径（`canonical` 本身）
   而非 `canonical.parent()`，因为 `ModuleResolver::resolve` 内部会做 `base.parent()`。

3. **Default import 无需额外处理**：`import compute from "./math"` 中 `compute` 是 `export default fn compute`，
   通过 `DefaultFunction` 的 unwrapping 已经添加了 `Declaration::Function{name:"compute"}`，
   所以不需要单独再创建 binding（名字相同的情况）。

### Verification
- `./target/release/ruyic examples/modules/main.ry --check` → `Type checking passed.`
- `cargo check --workspace` → 通过（无新增警告）
- 全部 25 个 examples 的 `--check` → 全部通过
- 预先失败的测试（parser、typechecker、generics 中的部分测试）→ 失败计数与修改前一致
