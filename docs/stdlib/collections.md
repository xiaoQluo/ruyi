# collections — 集合模块

## 概述

`collections` 模块提供泛型集合类型和特质，包括数组操作、Map、Set 以及迭代器适配器。

**源文件**: `stdlib/collections.ry`

**导入**: `import { ... } from "./collections"`

---

## 数值运算特质

### `Add` 特质
```ruyi
trait Add {
    fn add(self, other: Self): Self;
}
```
实现 `int` 和 `float` 类型的加法运算符重载。

### `Mul` 特质
```ruyi
trait Mul {
    fn mul(self, other: Self): Self;
}
```
实现 `int` 和 `float` 类型的乘法运算符重载。

---

## Iterator 特质

```ruyi
trait Iterator<T> {
    fn next(self): T?;
}
```
集合遍历的标准接口。返回 `null` 表示迭代结束。

---

## ArrayOps 特质 (Array 方法)

`ArrayOps<T>` 特质为所有 `Array<T>` 实例提供以下方法：

| 方法 | 签名 | 说明 |
|------|------|------|
| `length` | `fn length(self): int` | 返回数组长度 |
| `get` | `fn get(self, index: int): T` | 获取指定索引元素，越界抛出 `RangeError` |
| `set` | `fn set(self, index: int, value: T): void` | 设置指定索引元素，越界抛出 `RangeError` |
| `push` | `fn push(self, value: T): Array<T>` | 在末尾添加元素 |
| `pop` | `fn pop(self): T` | 移除并返回末尾元素，空数组抛出 `RangeError` |
| `map` | `fn map<U>(self, f: fn(T) -> U): Array<U>` | 映射变换 |
| `filter` | `fn filter(self, pred: fn(T) -> bool): Array<T>` | 过滤满足条件的元素 |
| `reduce` | `fn reduce<U>(self, init: U, f: fn(U, T) -> U): U` | 归约为单一值 |
| `forEach` | `fn forEach(self, f: fn(T) -> void): void` | 遍历执行函数 |
| `contains` | `fn contains(self, value: T): bool` | 检查是否包含指定值 |
| `iter` | `fn iter(self): ArrayIterator<T>` | 创建数组迭代器 |
| `sort` | `fn sort(self): Array<T>` | 返回排序后的新数组（插入排序） |
| `indexOf` | `fn indexOf(self, value: T): int` | 返回首次出现索引，未找到返回 -1 |
| `first` | `fn first(self): T?` | 返回第一个元素，空数组返回 null |
| `last` | `fn last(self): T?` | 返回最后一个元素，空数组返回 null |
| `slice` | `fn slice(self, begin: int, end: int): Array<T>` | 返回 `[begin, end)` 范围的新数组 |
| `concat` | `fn concat(self, other: Array<T>): Array<T>` | 拼接两个数组 |

---

## ArrayIterator

数组迭代器，实现 `Iterator<T>` 接口。

| 方法 | 签名 | 说明 |
|------|------|------|
| `new` | `fn new(arr: Array<T>)` | 创建迭代器 |
| `next` | `fn next(self): T?` | 返回下一个元素 |

**扩展方法** (Spec 4.9):

| 方法 | 签名 | 说明 |
|------|------|------|
| `takeWhile` | `fn takeWhile(self, pred: fn(T) -> bool): Iterator<T>` | 返回满足谓词的元素，遇到第一个 false 停止 |
| `skipWhile` | `fn skipWhile(self, pred: fn(T) -> bool): Iterator<T>` | 跳过满足谓词的元素，返回剩余部分 |
| `chain` | `fn chain(self, other: Iterator<T>): Iterator<T>` | 将两个迭代器串联 |
| `enumerate` | `fn enumerate(self): Iterator<T>` | 返回 `(index, value)` 对 |
| `zip` | `fn zip(self, other: Iterator<T>): Iterator<T>` | 将两个迭代器压缩为 `(a, b)` 对 |
| `sum` | `fn sum(self): T` | 元素求和（需实现 `Add`） |
| `product` | `fn product(self): T` | 元素求积（需实现 `Mul`） |
| `any` | `fn any(self, pred: fn(T) -> bool): bool` | 任一元素满足谓词返回 true |
| `all` | `fn all(self, pred: fn(T) -> bool): bool` | 全部元素满足谓词返回 true |

---

## Map

键值对映射表。

| 方法 | 签名 | 说明 |
|------|------|------|
| `new` | `fn new()` | 创建空 Map |
| `size` | `fn size(self): int` | 返回条目数 |
| `get` | `fn get(self, key: K): V?` | 获取键对应的值，不存在返回 null |
| `set` | `fn set(self, key: K, value: V): void` | 设置键值对 |
| `delete` | `fn delete(self, key: K): bool` | 删除键，返回是否删除成功 |
| `has` | `fn has(self, key: K): bool` | 检查键是否存在 |
| `keys` | `fn keys(self): Array<K>` | 返回所有键 |
| `values` | `fn values(self): Array<V>` | 返回所有值 |
| `iter` | `fn iter(self): MapIterator<K, V>` | 创建迭代器 |

### MapIterator

| 方法 | 签名 | 说明 |
|------|------|------|
| `new` | `fn new(m: Map<K, V>)` | 创建迭代器 |
| `next` | `fn next(self): { key: K, value: V }?` | 返回下一个键值对 |

---

## Set

集合实现。

| 方法 | 签名 | 说明 |
|------|------|------|
| `new` | `fn new()` | 创建空 Set |
| `size` | `fn size(self): int` | 返回元素数 |
| `add` | `fn add(self, value: T): void` | 添加元素 |
| `delete` | `fn delete(self, value: T): bool` | 删除元素，返回是否删除成功 |
| `has` | `fn has(self, value: T): bool` | 检查元素是否存在 |
| `iter` | `fn iter(self): SetIterator<T>` | 创建迭代器 |
| `toArray` | `fn toArray(self): Array<T>` | 返回所有元素的数组 |

### SetIterator

| 方法 | 签名 | 说明 |
|------|------|------|
| `new` | `fn new(s: Set<T>)` | 创建迭代器 |
| `next` | `fn next(self): T?` | 返回下一个元素 |

---

## 迭代器适配器

| 类 | 说明 |
|------|------|
| `TakeWhileIterator<T>` | 在谓词为 true 时产生元素，遇到 false 停止 |
| `SkipWhileIterator<T>` | 跳过谓词为 true 的元素，产生剩余部分 |
| `ChainedIterator<T>` | 将两个迭代器串联 |
| `EnumeratedIterator<T>` | 产生 `(index, value)` 对的迭代器 |
| `ZippedIterator<T>` | 将两个迭代器压缩为 `(a, b)` 对的迭代器 |

---

## 注意事项

- `ArrayOps` 特质自动为所有 `Array<T>` 实现
- `sort` 使用插入排序算法，返回新数组，不修改原数组
- `concat` 和 `slice` 不修改原数组，返回新数组
- 迭代器适配器（`TakeWhile`、`SkipWhile` 等）是惰性求值的
