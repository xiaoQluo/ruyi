# option — Option 和 Result 类型

## 概述

`option` 模块提供 `Option<T>` 和 `Result<T, E>` 两种代数数据类型，用于处理可能缺失的值和可能失败的计算。

- `Option<T> = Some<T> | None` — 可选值
- `Result<T, E> = Ok<T, E> | Err<T, E>` — 可失败结果

**源文件**: `stdlib/option.ry`

**导入**: `import { ... } from "./option"`

---

## Option 类型

```ruyi
type Option<T> = Some<T> | None;
```

### Some<T> — 包含值的变体

| 方法 | 签名 | 说明 |
|------|------|------|
| `new` | `fn new(value: T)` | 创建 Some 实例 |
| `isSome` | `fn isSome(self): bool` | 始终返回 true |
| `isNone` | `fn isNone(self): bool` | 始终返回 false |
| `unwrap` | `fn unwrap(self): T` | 返回包含的值 |
| `unwrapOr` | `fn unwrapOr(self, default: T): T` | 返回包含的值（忽略默认值） |
| `unwrapOrElse` | `fn unwrapOrElse(self, f: fn() -> T): T` | 返回包含的值（忽略闭包） |
| `map` | `fn map<U>(self, f: fn(T) -> U): Option<U>` | 对值应用变换，返回新的 `Some` |
| `andThen` | `fn andThen<U>(self, f: fn(T) -> Option<U>): Option<U>` | 链式计算 |
| `filter` | `fn filter(self, pred: fn(T) -> bool): Option<T>` | 谓词为真返回 self，否则返回 None |
| `flatten` | `fn flatten<U>(self): Option<U>` | 展平嵌套 Option |
| `okOr` | `fn okOr<E>(self, err: E): Result<T, E>` | 转换为 Ok |
| `okOrElse` | `fn okOrElse<E>(self, f: fn() -> E): Result<T, E>` | 转换为 Ok |
| `forEach` | `fn forEach(self, f: fn(T) -> void): void` | 对值应用函数 |
| `toString` | `fn toString(self): string` | 返回 `"Some(value)"` |

### None — 无值变体

| 方法 | 签名 | 说明 |
|------|------|------|
| `new` | `fn new()` | 创建 None 实例 |
| `isSome` | `fn isSome(self): bool` | 始终返回 false |
| `isNone` | `fn isNone(self): bool` | 始终返回 true |
| `unwrap` | `fn unwrap(self): never` | 抛出 `RuntimeError` |
| `unwrapOr` | `fn unwrapOr<T>(self, default: T): T` | 返回默认值 |
| `unwrapOrElse` | `fn unwrapOrElse<T>(self, f: fn() -> T): T` | 计算并返回默认值 |
| `map` | `fn map<T, U>(self, f: fn(T) -> U): Option<U>` | 返回 None |
| `andThen` | `fn andThen<T, U>(self, f: fn(T) -> Option<U>): Option<U>` | 返回 None |
| `filter` | `fn filter<T>(self, pred: fn(T) -> bool): Option<T>` | 返回 None |
| `flatten` | `fn flatten<U>(self): Option<U>` | 返回 None |
| `okOr` | `fn okOr<T, E>(self, err: E): Result<T, E>` | 返回 Err |
| `okOrElse` | `fn okOrElse<T, E>(self, f: fn() -> E): Result<T, E>` | 返回 Err |
| `forEach` | `fn forEach<T>(self, f: fn(T) -> void): void` | 无操作 |
| `toString` | `fn toString(self): string` | 返回 `"None"` |

---

## Result 类型

```ruyi
type Result<T, E> = Ok<T, E> | Err<T, E>;
```

### Ok<T, E> — 成功变体

| 方法 | 签名 | 说明 |
|------|------|------|
| `new` | `fn new(value: T)` | 创建 Ok 实例 |
| `isOk` | `fn isOk(self): bool` | 始终返回 true |
| `isErr` | `fn isErr(self): bool` | 始终返回 false |
| `unwrap` | `fn unwrap(self): T` | 返回包含的值 |
| `unwrapOr` | `fn unwrapOr(self, default: T): T` | 返回包含的值 |
| `unwrapOrElse` | `fn unwrapOrElse(self, f: fn(E) -> T): T` | 返回包含的值 |
| `map` | `fn map<U>(self, f: fn(T) -> U): Result<U, E>` | 对值应用变换，返回新的 Ok |
| `mapErr` | `fn mapErr<F>(self, f: fn(E) -> F): Result<T, F>` | 对错误应用变换（Ok 时无操作） |
| `andThen` | `fn andThen<U>(self, f: fn(T) -> Result<U, E>): Result<U, E>` | 链式计算 |
| `filter` | `fn filter(self, pred: fn(T) -> bool, error: E): Result<T, E>` | 谓词失败返回 Err |
| `ok` | `fn ok(self): Option<T>` | 转换为 `Some(value)` |
| `err` | `fn err(self): Option<E>` | 返回 None |
| `forEach` | `fn forEach(self, f: fn(T) -> void): void` | 对值应用函数 |
| `toOption` | `fn toOption(self): Option<T>` | 转换为 `Some(value)` |
| `toBool` | `fn toBool(self): bool` | 始终返回 true |
| `toString` | `fn toString(self): string` | 返回 `"Ok(value)"` |

### Err<T, E> — 错误变体

| 方法 | 签名 | 说明 |
|------|------|------|
| `new` | `fn new(error: E)` | 创建 Err 实例 |
| `isOk` | `fn isOk(self): bool` | 始终返回 false |
| `isErr` | `fn isErr(self): bool` | 始终返回 true |
| `unwrap` | `fn unwrap(self): T` | 抛出 `RuntimeError` |
| `unwrapOr` | `fn unwrapOr(self, default: T): T` | 返回默认值 |
| `unwrapOrElse` | `fn unwrapOrElse(self, f: fn(E) -> T): T` | 从错误计算默认值 |
| `map` | `fn map<U>(self, f: fn(T) -> U): Result<U, E>` | 返回 Err 不变 |
| `mapErr` | `fn mapErr<F>(self, f: fn(E) -> F): Result<T, F>` | 对错误应用变换 |
| `andThen` | `fn andThen<U>(self, f: fn(T) -> Result<U, E>): Result<U, E>` | 返回 Err 不变 |
| `filter` | `fn filter(self, pred: fn(T) -> bool, error: E): Result<T, E>` | 返回 self 不变 |
| `ok` | `fn ok(self): Option<T>` | 返回 None |
| `err` | `fn err(self): Option<E>` | 返回 `Some(error)` |
| `forEach` | `fn forEach(self, f: fn(T) -> void): void` | 无操作 |
| `toOption` | `fn toOption(self): Option<T>` | 返回 None |
| `toBool` | `fn toBool(self): bool` | 始终返回 false |
| `toString` | `fn toString(self): string` | 返回 `"Err(error)"` |

---

## 注意事项

- `Option<T>` 和 `Result<T, E>` 是联合类型别名，而非独立类
- `T` 和 `E` 类型参数在 `None` 和 `Err` 中可以是任意类型
- 在 `None` 上调用 `unwrap()` 会抛出 `RuntimeError`
- 在 `Err` 上调用 `unwrap()` 会抛出包含错误信息的 `RuntimeError`
