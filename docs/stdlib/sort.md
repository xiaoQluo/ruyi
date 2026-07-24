# sort — 排序与搜索模块

## 概述

`sort` 模块提供通用的排序和搜索函数。所有函数为纯函数——输入数组永不修改，结果始终返回新数组。

**源文件**: `stdlib/sort.ry`

**导入**: `import { ... } from "./sort"`

---

## 核心排序

| 函数 | 签名 | 说明 |
|------|------|------|
| `sortBy` | `fn sortBy<T>(arr: Array<T>, comparator: fn(T, T) -> int): Array<T>` | 使用自定义比较器升序排序 |
| `sortByDesc` | `fn sortByDesc<T>(arr: Array<T>, comparator: fn(T, T) -> int): Array<T>` | 使用自定义比较器降序排序 |

## 按键排序

| 函数 | 签名 | 说明 |
|------|------|------|
| `sortByKey` | `fn sortByKey<T>(arr: Array<T>, keyFn: fn(T) -> float): Array<T>` | 按数值键升序排序 |
| `sortByKeyDesc` | `fn sortByKeyDesc<T>(arr: Array<T>, keyFn: fn(T) -> float): Array<T>` | 按数值键降序排序 |
| `sortByStringKey` | `fn sortByStringKey<T>(arr: Array<T>, keyFn: fn(T) -> string): Array<T>` | 按字符串键升序排序 |
| `sortByStringKeyDesc` | `fn sortByStringKeyDesc<T>(arr: Array<T>, keyFn: fn(T) -> string): Array<T>` | 按字符串键降序排序 |

---

## 搜索

| 函数 | 签名 | 说明 |
|------|------|------|
| `binarySearch` | `fn binarySearch<T>(arr: Array<T>, target: T, comparator: fn(T, T) -> int): int` | 二分查找，返回索引或 -1 |
| `binarySearchInsert` | `fn binarySearchInsert<T>(arr: Array<T>, target: T, comparator: fn(T, T) -> int): int` | 二分查找插入位置 |
| `findIndex` | `fn findIndex<T>(arr: Array<T>, predicate: fn(T) -> bool): int` | 线性搜索第一个满足条件的索引 |
| `find` | `fn find<T>(arr: Array<T>, predicate: fn(T) -> bool): T?` | 线性搜索第一个满足条件的元素 |

---

## 排序验证

| 函数 | 签名 | 说明 |
|------|------|------|
| `isSorted` | `fn isSorted<T>(arr: Array<T>, comparator: fn(T, T) -> int): bool` | 检查数组是否已排序 |

---

## 极值查找

### 按数值键

| 函数 | 签名 | 说明 |
|------|------|------|
| `minBy` | `fn minBy<T>(arr: Array<T>, keyFn: fn(T) -> float): T?` | 返回数值键最小的元素 |
| `maxBy` | `fn maxBy<T>(arr: Array<T>, keyFn: fn(T) -> float): T?` | 返回数值键最大的元素 |

### 按字符串键

| 函数 | 签名 | 说明 |
|------|------|------|
| `minByString` | `fn minByString<T>(arr: Array<T>, keyFn: fn(T) -> string): T?` | 返回字符串键最小的元素 |
| `maxByString` | `fn maxByString<T>(arr: Array<T>, keyFn: fn(T) -> string): T?` | 返回字符串键最大的元素 |

### 按自定义比较器

| 函数 | 签名 | 说明 |
|------|------|------|
| `minWith` | `fn minWith<T>(arr: Array<T>, comparator: fn(T, T) -> int): T?` | 返回比较器结果最小的元素 |
| `maxWith` | `fn maxWith<T>(arr: Array<T>, comparator: fn(T, T) -> int): T?` | 返回比较器结果最大的元素 |

---

## 数组工具

| 函数 | 签名 | 说明 |
|------|------|------|
| `reverse` | `fn reverse<T>(arr: Array<T>): Array<T>` | 返回逆序新数组 |
| `copy` | `fn copy<T>(arr: Array<T>): Array<T>` | 返回数组的浅拷贝 |

---

## 注意事项

- 所有函数不修改输入数组，返回新数组
- 排序使用插入排序算法，适用于中小规模数据
- 比较器需返回 -1、0 或 1
- 极值查找函数在空数组上返回 null
