# Fix Pattern Matching Example - Learnings

## Summary
修复了 `examples/pattern_matching.ry` 的编译错误。

## Root Cause
Codegen 不支持以下特性:
1. **模块导入** (`import { Array } from "collections"`) - codegen 未实现模块系统
2. **对象解构模式** (`{ status: 200, body } =>`) - codegen 报 "Member access only supported on identifiers"
3. **数组解构** (`[first, second, ...rest]`) - codegen 不支持数组模式匹配
4. **字面量对象解构** (`if let { x, y } = point`) - 同上，codegen 不支持对象解构

## 修复策略
- 移除 `import` 语句
- 将对象解构 match 改为 `match (response.status)` + 直接字段访问
- 移除整个 `describeList` 函数（数组解构 + 依赖已移除的 Array 类型）
- 简化 `ifLetDemo` 只保留 nullable 类型的 if-let（codegen 支持）
- 简化 `asPatternDemo` 使用 guard 子句代替对象解构

## 已验证的特性
文件最终保留并验证了以下特性:
- 字面量模式 (200, 404, 500)
- Or 模式 (1|2|3, 4|5|6)
- Guard 子句 (if n > 0 && n < 10)
- 对象字段访问 match (match response.status)
- if-let 与 nullable 类型
- while-let 与 nullable 类型
- Match as 表达式
