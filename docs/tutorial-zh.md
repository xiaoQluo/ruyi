# Ruyi 语言教程

> **版本**: 0.1.0
> **日期**: 2026-05-02
> **面向读者**: 熟悉 JavaScript、TypeScript、Rust 或类似语言的程序员

---

## 目录

1. [快速开始](#1-getting-started)
2. [基础语法](#2-basic-syntax)
3. [控制流](#3-control-flow)
4. [函数](#4-functions)
5. [类与对象](#5-classes-and-objects)
6. [类型系统](#6-type-system)
7. [泛型](#7-generics)
8. [特征 (Trait)](#8-traits)
9. [模式匹配](#9-pattern-matching)
10. [错误处理](#10-error-handling)
11. [异步编程](#11-async-programming)
12. [模块](#12-modules)

---

## 附录

- [附录 C：内置函数与标准库](#appendix-c-built-in-functions-and-standard-library)
- [附录 D：完整示例——一个简单的 CLI 工具](#appendix-d-complete-example---a-simple-cli-tool)

---

## 1. 快速开始 {#1-getting-started}

### 1.1 Ruyi 是什么？

Ruyi 是一门编译型通用编程语言，其语法基础建立在 JavaScript 严格模式之上。它移除了 JavaScript 中容易引起问题的特性，同时保留了熟悉的语法风格。Ruyi 通过 LLVM 编译为原生机器码，在各平台上提供高性能支持。

核心特性：

- **熟悉的语法**：如果你了解 JavaScript，那么你已经掌握了 Ruyi 的大部分语法。
- **编译为原生代码**：使用 LLVM 生成快速、独立的二进制文件。
- **渐进式类型**：在静态类型注解和动态类型之间自由选择。
- **空值安全**：不再有 `undefined`。可空类型是显式声明的。
- **模式匹配**：一流的 `match` 表达式，支持解构。
- **特征 (Trait)**：类似接口的契约，支持静态和动态分发。
- **泛型**：参数多态，配合单态化实现。
- **异步/等待**：基于绿色线程的并发，采用工作窃取调度器。
- **异常处理**：通过 LLVM landing pads 实现零成本的 try/catch/finally。
- **宏**：声明式的、卫生宏，在编译时生成代码。

### 1.2 安装

要安装 Ruyi 编译器（`ruyic`），请克隆仓库并从源码构建：

```bash
git clone https://github.com/example/ruyi.git
cd ruyi
cargo build --release
```

编译器二进制文件将位于 `./target/release/ruyic`。将其添加到 PATH：

```bash
export PATH="$PWD/target/release:$PATH"
```

验证安装：

```bash
ruyic --version
```

预期输出：

```
ruyic 0.1.0
```

### 1.3 Hello, World!

创建一个名为 `hello.ry` 的文件：

```ruyi
print("Hello, Ruyi!");
```

编译并运行：

```bash
ruyic hello.ry -o hello
./hello
```

预期输出：

```
Hello, Ruyi!
```

### 1.4 一个稍大的示例

下面是一个计算斐波那契数列的程序：

```ruyi
fn fib(n: int): int {
  if (n <= 1) {
    return n;
  }
  return fib(n - 1) + fib(n - 2);
}

fn main() {
  for (let i = 0; i < 10; i = i + 1) {
    print("fib(" + i + ") = " + fib(i));
  }
}
```

编译并运行：

```bash
ruyic fibonacci.ry -o fib
./fib
```

预期输出：

```
fib(0) = 0
fib(1) = 1
fib(2) = 1
fib(3) = 2
fib(4) = 3
fib(5) = 5
fib(6) = 8
fib(7) = 13
fib(8) = 21
fib(9) = 34
```

### 1.5 编译器标志

Ruyi 编译器支持以下常用标志：

| 标志 | 说明 |
|------|-------------|
| `-o <output>` | 指定输出二进制文件名 |
| `--emit-llvm` | 输出 LLVM IR 而非二进制文件 |
| `-O0`, `-O1`, `-O2` | 优化级别（默认：`-O2`） |
| `--debug` | 包含调试符号 |
| `--version` | 打印编译器版本 |

示例：输出 LLVM IR 以检查编译器生成的内容：

```bash
ruyic hello.ry --emit-llvm
```

### 1.6 项目结构

一个典型的 Ruyi 项目结构如下：

```
my-project/
  main.ry           # Entry point
  utils.ry          # Utility module
  lib/
    math.ry         # Library module
    http/
      client.ry     # Nested module
  ruyi.toml        # Project configuration
```

每个 `.ry` 文件都是一个模块。模块通过相对路径或绝对路径导入。详见 [第 12 章：模块](#12-modules)。

### 常见陷阱

- **没有 `var`**：Ruyi 移除了 `var`。使用 `let` 声明可变变量，使用 `const` 声明不可变变量。
- **没有 `undefined`**：Ruyi 只有 `null`。未初始化的变量默认值为 `null`。
- **没有 `==` 或 `!=`**：只存在严格相等（`===`、`!==`）。没有隐式类型强制转换。
- **分号**：语句以分号结尾。编译器的自动分号插入（ASI）规则比 JavaScript 更清晰，但你仍应显式书写分号。

---

## 2. 基础语法 {#2-basic-syntax}

### 2.1 变量：`let` 与 `const`

Ruyi 有两个变量声明关键字：

- `let` 声明一个**可变**变量。
- `const` 声明一个**不可变**变量（不可重新赋值）。

```ruyi
let x = 42;
x = 100;          // OK: let is mutable

const PI = 3.14159;
// PI = 3;        // ERROR: const cannot be reassigned
```

`let` 和 `const` 都是**块级作用域**。在块内声明的变量在块外不可见：

```ruyi
{
  let inner = "visible inside";
  print(inner);
}
// print(inner);  // ERROR: inner is not defined here
```

### 2.2 内置类型

Ruyi 提供以下内置基本类型：

| 类型 | 说明 | 示例 |
|------|-------------|---------|
| `int` | 64-bit signed integer | `42`, `-7`, `0xFF` |
| `float` | 64-bit floating point | `3.14`, `1e10`, `0.5` |
| `bool` | Boolean | `true`, `false` |
| `string` | UTF-8 string | `"hello"`, `'world'` |
| `null` | Null type (only value: `null`) | `null` |
| `void` | No return value | (used in function return types) |
| `dyn` | Dynamic type (runtime checked) | (see Chapter 6) |
| `never` | Bottom type (unreachable) | (see Chapter 10) |

### 2.3 类型注解

你可以使用冒号为变量添加类型注解：

```ruyi
let count: int = 42;
let name: string = "Ruyi";
let ratio: float = 0.75;
let active: bool = true;
```

当没有提供注解时，Ruyi 会从初始化表达式推断类型：

```ruyi
let x = 42;           // x: int (inferred)
let y = "hello";      // y: string (inferred)
let z = true;         // z: bool (inferred)
```

如果没有初始化表达式也没有注解，类型默认为 `dyn`：

```ruyi
let unknown;          // unknown: dyn
```

### 2.4 数字字面量

Ruyi 支持多种数字字面量格式：

```ruyi
// Decimal
let decimal = 42;
let negative = -7;

// Floating point
let pi = 3.14159;
let scientific = 1e10;
let small = 0.001;

// Hexadecimal
let hex = 0xFF;       // 255

// Octal
let octal = 0o77;     // 63

// Binary
let binary = 0b1010;  // 10

// BigInt (arbitrary precision)
let big = 100n;
```

### 2.5 字符串字面量

Ruyi 支持三种字符串字面量形式：

```ruyi
// Double-quoted
let greeting = "Hello, Ruyi!";

// Single-quoted
let name = 'World';

// Template literals (with interpolation)
let message = `Hello, ${name}!`;
let result = `The sum is: ${2 + 3}`;
```

模板字面量可以跨越多行：

```ruyi
let multi = `line one
line two
line three`;
```

转义序列在所有字符串形式中都有效：

```ruyi
let escaped = "line1\nline2\ttab";
let unicode = "emoji: \u{1F600}";
```

### 2.6 运算符

#### 算术运算符

```ruyi
let sum = 10 + 3;       // 13
let diff = 10 - 3;      // 7
let product = 10 * 3;   // 30
let quotient = 10 / 3;  // 3 (integer division for int)
let remainder = 10 % 3; // 1
let power = 2 ** 8;     // 256
```

#### 比较运算符

Ruyi 只使用**严格相等**。不存在 `==` 或 `!=`：

```ruyi
let eq = 5 === 5;       // true
let neq = 5 !== 3;      // true
let lt = 3 < 5;         // true
let gt = 5 > 3;         // true
let lte = 3 <= 3;       // true
let gte = 5 >= 5;       // true
```

没有隐式类型强制转换：

```ruyi
// In JavaScript: "5" == 5 is true
// In Ruyi: this is a compile error
// "5" === 5  // ERROR: type mismatch (string vs int)
```

#### 逻辑运算符

```ruyi
let and = true && false;   // false
let or = true || false;    // true
let not = !true;           // false
```

#### 空值合并

`??` 运算符在左侧操作数为 `null` 时返回右侧操作数：

```ruyi
let name: string? = null;
let displayName = name ?? "anonymous";  // "anonymous"
```

#### 可选链

`?.` 运算符安全地访问可空值上的属性：

```ruyi
let user: User? = findUser(1);
let userName = user?.name;    // string? (null if user is null)
```

### 2.7 注释

```ruyi
// Single-line comment

/* Multi-line
   comment */

/**
 * Documentation comment.
 * Preserved for tooling.
 */
```

### 常见陷阱

- **没有隐式强制转换**：`"5" + 3` 是编译错误。使用 `"5" + toString(3)`。
- **没有 `==`**：只存在 `===` 和 `!==`。
- **没有 `undefined`**：只有 `null` 表示值的缺失。
- **块级作用域**：`let` 和 `const` 是块级作用域，而非函数级作用域。

---

## 3. 控制流 {#3-control-flow}

### 3.1 `if` / `else`

`if` 语句计算一个条件并执行相应的分支：

```ruyi
let x = 10;

if (x > 0) {
  print("positive");
} else if (x < 0) {
  print("negative");
} else {
  print("zero");
}
```

条件必须是 `bool` 类型。没有真值/假值强制转换：

```ruyi
// In JavaScript: if ("hello") { } works
// In Ruyi: this is a compile error
// if ("hello") { }  // ERROR: expected bool, got string
```

### 3.2 `if` 作为表达式

在 Ruyi 中，`if` 可以作为返回值的表达式使用：

```ruyi
let sign = if (x > 0) {
  "positive"
} else if (x < 0) {
  "negative"
} else {
  "zero"
};
```

两个分支必须返回兼容的类型。`if` 表达式的类型是各分支类型的最小上界。

### 3.3 `for` 循环

Ruyi 支持 C 风格的 `for` 循环：

```ruyi
for (let i = 0; i < 10; i = i + 1) {
  print(i);
}
```

反向迭代：

```ruyi
for (let i = items.length - 1; i >= 0; i = i - 1) {
  process(items[i]);
}
```

### 3.4 `for-in` 循环

遍历对象的键或数组的索引：

```ruyi
let obj = { name: "Ruyi", version: "0.1.0" };

for (let key in obj) {
  print(key + ": " + obj[key]);
}
```

### 3.5 `for-of` 循环

遍历可迭代对象的值：

```ruyi
let items = ["apple", "banana", "cherry"];

for (let item of items) {
  print(item);
}
```

### 3.6 `while` 循环

当条件为真时执行代码块：

```ruyi
let i = 0;
while (i < 10) {
  print(i);
  i = i + 1;
}
```

### 3.7 `break` 与 `continue`

`break` 退出最内层的循环：

```ruyi
for (let i = 0; i < 100; i = i + 1) {
  if (i === 50) {
    break;
  }
  print(i);
}
// Prints 0 through 49
```

`continue` 跳到下一次迭代：

```ruyi
for (let i = 0; i < 10; i = i + 1) {
  if (i % 2 === 0) {
    continue;
  }
  print(i);
}
// Prints odd numbers: 1, 3, 5, 7, 9
```

### 3.8 带标签的 `break` 和 `continue`

你可以为循环添加标签，并使用 `break` 或 `continue` 跳转到特定标签：

```ruyi
outer: for (let i = 0; i < 10; i = i + 1) {
  for (let j = 0; j < 10; j = j + 1) {
    if (i * j > 50) {
      break outer;
    }
    print(i + " * " + j + " = " + (i * j));
  }
}
```

### 常见陷阱

- **没有真值/假值**：条件必须是 `bool` 类型。`if (0)`、`if ("")` 和 `if (null)` 都是编译错误。
- **没有 `do-while`**：Ruyi 没有 `do-while` 循环。使用 `while`，条件在开头检查。
- **无限循环**：`while (true)` 是合法的。确保包含 `break` 或能使条件最终变为假的变更。

---

## 4. 函数 {#4-functions}

### 4.1 函数声明

函数使用 `fn` 关键字声明：

```ruyi
fn add(a: int, b: int): int {
  return a + b;
}

let result = add(3, 5);  // 8
```

`fn` 关键字取代了 JavaScript 的 `function`。它更短，并且与其他声明（`class`、`trait`、`macro`）保持一致。

### 4.2 返回类型推断

当没有提供返回类型注解时，Ruyi 会从 `return` 语句推断：

```ruyi
fn add(a: int, b: int) {
  return a + b;
}
// Inferred: fn add(a: int, b: int): int
```

如果函数没有 `return` 语句，推断的返回类型为 `void`：

```ruyi
fn greet(name: string) {
  print("Hello, " + name);
}
// Inferred: fn greet(name: string): void
```

如果存在多个返回路径，返回类型是所有返回类型的最小上界：

```ruyi
fn maybeNumber(flag: bool) {
  if (flag) {
    return 42;       // int
  } else {
    return 3.14;     // float
  }
}
// Inferred: fn maybeNumber(flag: bool): float
```

### 4.3 箭头函数

箭头函数为函数表达式提供了简洁的语法：

```ruyi
let double = (x) => x * 2;
let greet = (name) => { print("Hi, " + name); };
let add = (a, b) => a + b;
```

单个表达式的箭头函数可以省略花括号和 `return`：

```ruyi
let square = (x) => x * x;
// Equivalent to:
let square = (x) => { return x * x; };
```

### 4.4 参数

#### 默认参数

```ruyi
fn greet(name: string = "World") {
  print("Hello, " + name);
}

greet();             // "Hello, World"
greet("Ruyi");      // "Hello, Ruyi"
```

#### 剩余参数

```ruyi
fn sum(...numbers: Array<int>): int {
  let total = 0;
  for (let n of numbers) {
    total = total + n;
  }
  return total;
}

sum(1, 2, 3, 4);     // 10
```

#### 解构参数

```ruyi
fn printPoint({ x, y }: { x: float, y: float }) {
  print("(" + x + ", " + y + ")");
}

let point = { x: 3.0, y: 4.0 };
printPoint(point);   // "(3, 4)"
```

### 4.5 闭包

函数可以捕获其外层作用域中的变量：

```ruyi
fn makeCounter(): fn(): int {
  let count = 0;
  return () => {
    count = count + 1;
    return count;
  };
}

let counter = makeCounter();
print(counter());    // 1
print(counter());    // 2
print(counter());    // 3
```

### 4.6 高阶函数

函数可以接受其他函数作为参数：

```ruyi
fn map(arr: Array<int>, f: fn(int) -> int): Array<int> {
  let result = [];
  for (let item of arr) {
    result.push(f(item));
  }
  return result;
}

let doubled = map([1, 2, 3], (x) => x * 2);
// doubled: [2, 4, 6]
```

### 常见陷阱

- **没有 `function` 关键字**：使用 `fn`。
- **没有 `arguments` 对象**：使用剩余参数（`...args`）。
- **没有自动 `this` 绑定**：方法显式使用 `self`。箭头函数按词法捕获 `self`。
- **返回类型很重要**：如果你忘记 `return`，函数返回 `null`（或推断为 `void`）。

---

## 5. 类与对象 {#5-classes-and-objects}

### 5.1 类声明

类使用 `class` 关键字声明：

```ruyi
class Point {
  x: float;
  y: float;

  fn new(x: float, y: float) {
    self.x = x;
    self.y = y;
  }

  fn distance(other: Point): float {
    let dx = self.x - other.x;
    let dy = self.y - other.y;
    return (dx ** 2 + dy ** 2) ** 0.5;
  }
}
```

### 5.2 构造函数

`new` 方法作为构造函数。创建新实例时调用它：

```ruyi
let p = Point.new(3.0, 4.0);
print(p.distance(Point.new(0.0, 0.0)));  // 5.0
```

在构造函数内部，`self` 指代正在创建的实例。通过给 `self.fieldName` 赋值来初始化字段。

### 5.3 静态方法

静态方法使用 `static` 关键字声明：

```ruyi
class Point {
  x: float;
  y: float;

  fn new(x: float, y: float) {
    self.x = x;
    self.y = y;
  }

  static fn origin(): Point {
    return Point.new(0.0, 0.0);
  }
}

let origin = Point.origin();
```

### 5.4 继承

类可以使用 `extends` 继承其他类：

```ruyi
class Shape {
  color: string;

  fn new(color: string) {
    self.color = color;
  }

  fn area(): float {
    return 0.0;
  }
}

class Circle extends Shape {
  radius: float;

  fn new(radius: float, color: string) {
    super.new(color);
    self.radius = radius;
  }

  fn area(): float {
    return 3.14159 * self.radius ** 2;
  }
}

let circle = Circle.new(5.0, "red");
print(circle.area());    // 78.53975
print(circle.color);     // "red"
```

使用 `super` 调用父类的构造函数或方法。

### 5.5 Getter 和 Setter

```ruyi
class Temperature {
  _celsius: float;

  fn new(celsius: float) {
    self._celsius = celsius;
  }

  get celsius(): float {
    return self._celsius;
  }

  set celsius(value: float) {
    self._celsius = value;
  }

  get fahrenheit(): float {
    return self._celsius * 9.0 / 5.0 + 32.0;
  }
}

let temp = Temperature.new(100.0);
print(temp.fahrenheit);    // 212.0
temp.celsius = 0.0;
print(temp.fahrenheit);    // 32.0
```

### 5.6 对象字面量

对于简单的数据结构，对象字面量提供了比类更轻量的替代方案：

```ruyi
let person = {
  name: "Alice",
  age: 30,
  city: "New York"
};

print(person.name);    // "Alice"
```

对象字面量支持展开语法：

```ruyi
let defaults = { theme: "light", fontSize: 14 };
let userPrefs = { fontSize: 16 };
let config = { ...defaults, ...userPrefs };
// config: { theme: "light", fontSize: 16 }
```

### 常见陷阱

- **没有原型链**：Ruyi 移除了 JavaScript 的原型继承。使用 `class` 和 `extends`。
- **`self` 而非 `this`**：方法使用 `self` 指代当前实例。这避免了 JavaScript 中令人困惑的 `this` 绑定行为。
- **字段必须声明**：类字段必须使用类型注解声明。
- **没有 `delete`**：你不能删除对象属性。改为赋值为 `null`。

---

## 6. 类型系统 {#6-type-system}

### 6.1 渐进式类型

Ruyi 使用**渐进式类型系统**，将静态类型检查与动态类型检查相结合。你可以选择添加类型注解以获得编译时安全性，或省略注解并依赖运行时检查。

```ruyi
// Static typing (compile-time checked)
let x: int = 42;
let y: string = "hello";

// Dynamic typing (runtime checked)
let a = 42;           // a: int (inferred)
let b;                // b: dyn (no annotation, no initializer)
```

### 6.2 `dyn` 类型

`dyn` 是动态类型。它表示在运行时检查类型的值：

```ruyi
let value: dyn = 42;
value = "hello";      // OK: dyn can hold any type
value = true;         // OK
```

当 `dyn` 值在静态类型上下文中使用时，会插入运行时检查：

```ruyi
let value: dyn = 42;
let x: int = value;   // Runtime check: throws TypeError if value is not int
```

### 6.3 类型推断

Ruyi 使用双向类型推断。对于变量声明，类型从初始化表达式推断：

```ruyi
let x = 42;           // x: int
let y = "hello";      // y: string
let z = true;         // z: bool
let arr = [1, 2, 3];  // arr: Array<int>
```

**字面量类型推断**：

| 字面量 | 推断类型 |
|---------|---------------|
| `42` | `int` |
| `3.14` | `float` |
| `100n` | `bigint` |
| `"hello"` | `string` |
| `true` / `false` | `bool` |
| `null` | `null` |
| `[1, 2, 3]` | `Array<int>` |
| `{ x: 1, y: 2 }` | `{ x: int, y: int }` |

### 6.4 可空类型

Ruyi 拥有健全的可空类型系统。没有 `undefined`。可空类型必须使用 `?` 显式声明：

```ruyi
let name: string = "Ruyi";     // cannot be null
let maybe: string? = null;      // can be null
```

#### 可选链

```ruyi
let user: User? = findUser(1);
let userName = user?.name;           // string?
let city = user?.address?.city;      // string?
```

#### 空值合并

```ruyi
let name = user?.name ?? "anonymous";    // string
let count = config.count ?? 0;           // int
```

#### 非空断言

`!` 运算符断言一个可空值不为 `null`：

```ruyi
let name: string? = getUser();
let safe: string = name!;    // throws if name is null
```

#### 类型收窄

在空值检查后，编译器会收窄类型：

```ruyi
let name: string? = getUser();

if (name !== null) {
  // name is narrowed to string here
  print(name.length);
}

// name is string? again here
```

### 6.5 函数类型

函数类型写作 `fn(T1, T2, ...) -> R`：

```ruyi
let add: fn(int, int) -> int = (a, b) => a + b;
let log: fn(string) -> void = (msg) => { print(msg); };
```

### 6.6 结构子类型

对象类型使用结构子类型。对象类型 `{ a: int, b: int, c: int }` 是 `{ a: int, b: int }` 的子类型：

```ruyi
let point3d = { x: 1.0, y: 2.0, z: 3.0 };
let point2d: { x: float, y: float } = point3d;  // OK: point3d has all required fields
```

### 常见陷阱

- **没有隐式强制转换**：`"5" + 3` 是编译错误。使用显式转换。
- **`dyn` 不是万能的**：使用 `dyn` 会插入运行时检查。它不是替代正确类型的方案。
- **可空是显式的**：`string` 不能保存 `null`。如果可能为 `null`，请使用 `string?`。
- **收窄在重新赋值后重置**：在 `name = getUser()` 之后，之前的收窄会失效。

---

## 7. 泛型 {#7-generics}

### 7.1 泛型函数

泛型函数通过尖括号引入类型参数：

```ruyi
fn identity<T>(x: T): T {
  return x;
}

let a = identity(42);           // a: int
let b = identity("hello");      // b: string
let c = identity(true);         // c: bool
```

### 7.2 特征约束

类型参数可以被特征约束：

```ruyi
fn max<T: Comparable>(a: T, b: T): T {
  return if a.compare(b) > 0 { a } else { b };
}

let m = max(3, 5);              // OK: int implements Comparable
// max(true, false);            // ERROR: bool does not implement Comparable
```

多个约束使用 `+`：

```ruyi
fn process<T: Comparable + Clone>(value: T) {
  let copy = value.clone();
  let comparison = value.compare(copy);
  print(comparison);
}
```

### 7.3 泛型类

类可以是泛型的：

```ruyi
class Option<T> {
  value: T?;

  fn new(value: T?) {
    self.value = value;
  }

  fn isSome(): bool {
    return self.value !== null;
  }

  fn unwrap(): T {
    if (self.value === null) {
      throw Error("unwrap on None");
    }
    return self.value;
  }

  fn map<U>(f: fn(T) -> U): Option<U> {
    if (self.value === null) {
      return Option.new(null);
    }
    return Option.new(f(self.value!));
  }
}

let some = Option.new(42);
print(some.unwrap());           // 42

let none: Option<int> = Option.new(null);
print(none.isSome());           // false
```

### 7.4 泛型类型别名

```ruyi
type Result<T, E> = Ok<T> | Err<E>;
type Callback<T> = fn(T) -> void;
type Pair<T> = { first: T, second: T };
```

### 7.5 泛型类型推断

Ruyi 从上下文推断泛型类型参数：

```ruyi
fn wrap<T>(value: T): Option<T> {
  return Option.new(value);
}

let x = wrap(42);       // x: Option<int>
let y = wrap("hello");  // y: Option<string>
```

### 7.6 单态化

Ruyi 对泛型使用单态化。在每个调用点，编译器生成泛型函数的特化副本：

```ruyi
fn identity<T>(x: T): T { return x; }

let a = identity(42);       // generates identity_int(x: int): int
let b = identity("hello");  // generates identity_string(x: string): string
```

这产生了快速的、类型专属的代码，没有运行时开销。

### 常见陷阱

- **特征约束是操作所必需的**：如果你想比较值，你需要 `T: Comparable`。
- **单态化可能增加二进制体积**：每种唯一的类型组合都会生成函数的一个新副本。
- **`dyn` 禁用单态化**：使用 `dyn` 调用泛型函数时，会使用带运行时检查的单一版本。

---

## 8. 特征 (Trait) {#8-traits}

### 8.1 特征声明

特征定义了一个类型可以实现的契约：

```ruyi
trait Printable {
  fn format(self): string;
}

trait Comparable<T> {
  fn compare(self, other: T): int;
}

trait Iterator<T> {
  fn next(self): T?;
}
```

特征方法没有方法体。它们只是签名。

### 8.2 特征实现

类型通过 `impl` 块来实现特征：

```ruyi
impl Printable for string {
  fn format(self): string {
    return self;
  }
}

impl Printable for int {
  fn format(self): string {
    return toString(self);
  }
}

impl<T: Printable> Printable for Array<T> {
  fn format(self): string {
    let result = "[";
    for (let item of self) {
      result = result + item.format();
    }
    return result + "]";
  }
}
```

### 8.3 静态分发

当具体类型在编译时已知，特征方法调用使用静态分发：

```ruyi
fn printIt<T: Printable>(value: T) {
  print(value.format());    // static dispatch: monomorphized
}

printIt("hello");    // calls string.format() directly
printIt(42);         // calls int.format() directly
```

编译器为每种类型生成 `printIt` 的特化版本，使用直接函数调用。

### 8.4 动态分发（特征对象）

当具体类型未知时，使用 `dyn Trait` 进行动态分发：

```ruyi
let items: Array<dyn Printable> = ["hello", 42, true];
for (let item of items) {
  print(item.format());    // dynamic dispatch: vtable lookup
}
```

特征对象由一个数据指针和一个 vtable 指针组成：

```
TraitObject {
  data: *void,        // pointer to the concrete value
  vtable: *VTable,    // pointer to method implementations
}
```

### 8.5 默认方法实现

特征可以提供默认实现：

```ruyi
trait Iterator<T> {
  fn next(self): T?;

  fn collect(self): Array<T> {
    let result = [];
    while let Some(item) = self.next() {
      result.push(item);
    }
    return result;
  }
}
```

没有覆盖 `collect` 的实现会继承默认实现：

```ruyi
impl Iterator<int> for NumberRange {
  fn next(self): int? {
    // ...
  }
  // collect() is inherited from the trait
}
```

### 8.6 特征对象向下转型

特征对象可以通过模式匹配向下转型为它们的实际类型：

```ruyi
let y: dyn Printable = "hello";

match (y) {
  s as string => { print("string: " + s); }
  n as int => { print("int: " + n); }
  _ => { print("unknown type"); }
}
```

### 8.7 孤儿规则

`impl` 块必须与特征或被实现类型位于同一个模块中。这防止了冲突的实现：

```ruyi
// OK: implementing your trait for a built-in type
impl Printable for string { ... }

// OK: implementing a built-in trait for your type
impl Comparable for MyType { ... }

// ERROR: implementing someone else's trait for someone else's type
// impl SomeExternalTrait for SomeExternalType { ... }
```

### 常见陷阱

- **特征方法在声明中没有方法体**：只有签名。默认实现是一个独立的特性。
- **`dyn Trait` 擦除了具体类型**：你只能通过特征对象访问特征方法。
- **孤儿规则防止冲突**：你不能为外部类型实现外部特征。

---

## 9. 模式匹配 {#9-pattern-matching}

### 9.1 `match` 表达式

`match` 表达式将值与一系列模式进行比较：

```ruyi
let value = 3;

match (value) {
  0 => { print("zero"); }
  1 => { print("one"); }
  2 => { print("two"); }
  _ => { print("other"); }
}
```

`_` 模式是通配符，可以匹配任何值。它必须是最后一个分支。

### 9.2 字面量模式

匹配字面量值：

```ruyi
match (status) {
  200 => { print("OK"); }
  404 => { print("Not Found"); }
  500 => { print("Server Error"); }
  _ => { print("Unknown: " + status); }
}
```

### 9.3 或模式

使用 `|` 匹配多个模式：

```ruyi
match (value) {
  1 | 2 | 3 => { print("small"); }
  4 | 5 | 6 => { print("medium"); }
  _ => { print("large"); }
}
```

### 9.4 守卫子句

使用 `if` 为匹配分支添加条件：

```ruyi
match (n) {
  0 => { print("zero"); }
  n if (n > 0 && n < 10) => { print("single digit: " + n); }
  n if (n >= 10 && n < 100) => { print("double digit: " + n); }
  _ => { print("other"); }
}
```

### 9.5 解构对象

```ruyi
let result = { status: 200, body: "Hello" };

match (result) {
  { status: 200, body } => { print(body); }
  { status: 404 } => { print("not found"); }
  { status, body } => { print("error " + status + ": " + body); }
  _ => { print("unknown response"); }
}
```

### 9.6 解构数组

```ruyi
let list = [1, 2, 3, 4, 5];

match (list) {
  [] => { print("empty"); }
  [first] => { print("single: " + first); }
  [first, second, ...rest] => {
    print("first: " + first + ", second: " + second);
    print("rest: " + rest);
  }
  _ => { print("other"); }
}
```

### 9.7 `if-let` 语句

`if-let` 语句将模式匹配与条件执行相结合：

```ruyi
let point = { x: 3.0, y: 4.0 };

if let { x, y } = point {
  print("point at (" + x + ", " + y + ")");
}
```

带有 `else` 子句：

```ruyi
let result = Ok(42);

if let Ok(value) = result {
  print("success: " + value);
} else {
  print("failed");
}
```

### 9.8 `while-let` 语句

`while-let` 语句在模式匹配时循环：

```ruyi
while let Some(item) = iterator.next() {
  process(item);
}
```

### 9.9 `as` 模式

将整个匹配值绑定到一个名称：

```ruyi
match (value) {
  { x, y } as point => {
    print("point: " + point);
    print("x: " + x + ", y: " + y);
  }
  _ => { print("not a point"); }
}
```

### 常见陷阱

- **穷尽性**：必须覆盖所有可能的值。如果需要，使用 `_` 作为兜底。
- **顺序很重要**：模式从上到下尝试匹配。更具体的模式应排在前面。
- **守卫在模式匹配后求值**：守卫子句只在模式匹配成功后运行。

---

## 10. 错误处理 {#10-error-handling}

### 10.1 `try` / `catch` / `finally`

Ruyi 使用异常进行错误处理：

```ruyi
try {
  let result = riskyOperation();
  print(result);
} catch (e: Error) {
  print("Error: " + e.message);
} finally {
  cleanup();
}
```

### 10.2 多个 `catch` 子句

`catch` 子句按顺序尝试。第一个匹配的子句处理异常：

```ruyi
try {
  doSomething();
} catch (e: TypeError) {
  print("Type error: " + e.message);
} catch (e: RangeError) {
  print("Range error: " + e.message);
} catch (e: Error) {
  print("General error: " + e.message);
}
```

`catch` 子句匹配子类型。`catch (e: Error)` 可以捕获 `TypeError`、`RangeError` 以及所有其他 `Error` 子类型。

### 10.3 不带绑定的 `catch`

如果你不需要异常变量，可以省略它：

```ruyi
try {
  doSomething();
} catch {
  print("something failed");
}
```

### 10.4 `throw` 语句

使用 `throw` 抛出异常：

```ruyi
fn divide(a: int, b: int): int {
  if (b === 0) {
    throw Error("division by zero");
  }
  return a / b;
}
```

### 10.5 自定义错误类型

通过继承 `Error` 创建自定义错误类型：

```ruyi
class ValidationError extends Error {
  field: string;

  fn new(field: string, message: string) {
    super.new(message);
    self.field = field;
  }
}

fn validateAge(age: int) {
  if (age < 0) {
    throw ValidationError.new("age", "age cannot be negative");
  }
}
```

### 10.6 `never` 类型

总是抛出的函数可以使用返回类型 `never` 注解：

```ruyi
fn fail(message: string): never {
  throw Error(message);
}
```

`never` 类型是底类型（bottom type）。它是所有类型的子类型，意味着 `never` 表达式可以在任何上下文中使用：

```ruyi
let x: int = if (condition) {
  42
} else {
  fail("impossible");    // never is a subtype of int
};
```

### 10.7 `finally` 的保证

`finally` 块**总是**执行，无论 `try` 块如何退出：

| try 退出方式 | finally 行为 |
|----------|-----------------|
| 正常完成 | 在 try 之后执行 |
| 抛出异常 | 在异常传播前执行 |
| `return` 语句 | 在 return 前执行 |
| `break` / `continue` | 在控制转移前执行 |

```ruyi
fn withFile(path: string): string {
  let file = openFile(path);
  try {
    return file.readAll();
  } finally {
    file.close();    // always executes
  }
}
```

### 10.8 异常抑制

如果 `finally` 在另一个异常传播时抛出异常，`finally` 的异常将取代原始异常：

```ruyi
try {
  throw Error("original");
} finally {
  throw Error("finally");    // this replaces "original"
}
// Caught exception: "finally"
```

### 10.9 零成本异常

Ruyi 异常使用零成本异常表。当没有异常抛出时，没有运行时开销。只有在异常实际抛出时才付出代价。

### 常见陷阱

- **没有受检异常**：Ruyi 不要求函数声明它们抛出的异常。
- **`finally` 可以抑制异常**：如果 `finally` 抛出异常，它会取代原始异常。
- **`catch` 顺序很重要**：将具体的异常类型放在通用类型之前。
- **`never` 很有用**：总是抛出的函数应返回 `never`，以帮助类型检查器。

---

## 11. 异步编程 {#11-async-programming}

### 11.1 `async` 函数

使用 `async` 声明异步函数：

```ruyi
async fn fetchData(url: string): string {
  let response = await http.get(url);
  return response.body;
}
```

`async` 函数返回一个 `Future<T>`：

```ruyi
let future: Future<string> = fetchData("https://example.com");
let result: string = await future;
```

### 11.2 `await` 表达式

`await` 运算符挂起当前异步函数，直到 future 完成：

```ruyi
async fn loadAll(urls: Array<string>): Array<string> {
  let results = [];
  for (let url of urls) {
    results.push(await fetchData(url));
  }
  return results;
}
```

`await` 只能在 `async` 函数内部使用。

### 11.3 并发执行

要并发运行多个 future，生成它们并等待全部完成：

```ruyi
async fn loadConcurrent(urls: Array<string>): Array<string> {
  let futures = [];
  for (let url of urls) {
    futures.push(spawn(fetchData(url)));
  }

  let results = [];
  for (let future of futures) {
    results.push(await future);
  }
  return results;
}
```

### 11.4 异步箭头函数

```ruyi
let fetch = async (url) => await http.get(url);
let process = async (data) => {
  let result = await transform(data);
  return result;
};
```

### 11.5 异步迭代器

异步迭代器异步地产生值：

```ruyi
async fn* readLines(file: File): AsyncIterator<string> {
  while let line = await file.readLine() {
    yield line;
  }
}

for await (let line of readLines(file)) {
  print(line);
}
```

`for await` 循环脱糖为：

```ruyi
let iter = readLines(file);
while let Some(line) = await iter.next() {
  print(line);
}
```

### 11.6 绿色线程调度器

Ruyi 使用工作窃取调度器来管理绿色线程：

- **工作者（Workers）**：执行绿色线程的操作系统线程。
- **任务队列**：每个工作者拥有一个本地双端队列，存放就绪的 future。
- **工作窃取**：当一个工作者的队列为空时，它会从另一个工作者那里窃取任务。

这提供了高效的并发，开销极小。

### 11.7 阻塞操作

阻塞操作不应当在绿色线程中调用：

```ruyi
// Wrong: blocks the worker
let data = fs.readFileSync("file.txt");

// Correct: async I/O
let data = await fs.readFile("file.txt");

// Or: offload to blocking thread pool
let data = await spawn_blocking(|| fs.readFileSync("file.txt"));
```

### 11.8 异步与异常

异步函数中的异常通过 Future 传播：

```ruyi
async fn risky(): int {
  throw Error("async error");
}

try {
  let result = await risky();
} catch (e: Error) {
  // catches the async error
}
```

如果异步函数抛出异常，`Future` 会以错误状态完成。在出错的 future 上 `await` 会重新抛出异常。

### 常见陷阱

- **`await` 只能在 `async` 中使用**：你不能在非异步函数中使用 `await`。
- **Future 是惰性的**：future 在被 await 或 spawn 之前不会开始执行。
- **不要阻塞工作者**：同步 I/O 会阻塞整个工作者线程。使用异步 I/O 或 `spawn_blocking`。
- **异常跨越 await 边界**：`await` 会重新抛出失败 future 中的异常。

---

## 12. 模块 {#12-modules}

### 12.1 模块结构

每个 `.ry` 源文件都是一个模块。模块基于文件系统层次组织：

```
src/
  main.ry              -> module main
  utils.ry             -> module utils
  http/
    client.ry          -> module http::client
    server.ry          -> module http::server
```

### 12.2 导入声明

#### 命名导入

```ruyi
import { add, subtract } from "./math";

let sum = add(3, 5);
let diff = subtract(10, 3);
```

#### 重命名导入

```ruyi
import { add as plus } from "./math";

let sum = plus(3, 5);
```

#### 命名空间导入

```ruyi
import * as utils from "./utils";

utils.formatDate(now());
utils.parseNumber("42");
```

#### 默认导入

```ruyi
import HttpClient from "./http";

let client = HttpClient.new();
```

#### 组合导入

```ruyi
import HttpClient, { Request, Response } from "./http";
```

#### 副作用导入

```ruyi
import "./setup";    // runs module initialization, no imports
```

### 12.3 导出声明

默认情况下，所有顶层声明都是**私有的**。使用 `export` 使它们公开：

```ruyi
// math.ry
fn internalHelper(): int { ... }          // private

export fn add(a: int, b: int): int {      // public
  return a + b;
}

export fn subtract(a: int, b: int): int {
  return a - b;
}
```

#### 命名导出

```ruyi
export { add, subtract };
export { add as plus };
```

#### 重新导出

```ruyi
export * from "./math";
export { add, subtract } from "./math";
```

#### 默认导出

```ruyi
export default class App {
  fn run() { ... }
}

export default fn main() {
  print("Hello!");
}
```

### 12.4 导入解析

导入路径按以下方式解析：

1. **相对路径**（`./` 或 `../`）：相对于导入文件所在目录解析。
2. **绝对路径**（无前缀）：从项目的源码根目录解析。
3. **标准库路径**（`std::`）：从标准库解析。

解析会同时尝试 `<path>.ry` 和 `<path>/index.ry`：

```ruyi
import { foo } from "./math";
// Tries: ./math.ry, then ./math/index.ry
```

### 12.5 循环依赖检测

Ruyi 在编译时检测循环依赖：

```ruyi
// a.ry
import { foo } from "./b";

// b.ry
import { bar } from "./a";    // ERROR: circular dependency
```

要解决循环依赖：

- 将共享代码提取到第三个模块中。
- 仅对类型使用前置声明。
- 重构模块层次结构。

### 12.6 模块初始化

模块首次导入时，其顶层语句按顺序执行：

```ruyi
// config.ry
export let config = loadConfig();    // executes on first import
```

每个模块恰好初始化一次。初始化顺序遵循依赖图。

### 12.7 名称解析与遮蔽

名称按以下顺序解析：

1. 局部作用域（当前块）
2. 函数作用域（参数和局部变量）
3. 模块作用域（顶层声明）
4. 导入的名称
5. 内置名称（`int`、`string`、`null` 等）

内层作用域可以遮蔽外层作用域的名称：

```ruyi
let x = 1;           // module-level x

fn example() {
  let x = 2;         // shadows module-level x
  print(x);          // prints 2
}
```

遮蔽导入的名称会产生警告。

### 常见陷阱

- **默认是私有的**：声明是私有的，除非显式导出。
- **没有循环导入**：编译器检测并拒绝循环依赖。
- **相对路径相对于文件**：`src/utils/helper.ry` 中的 `./math` 解析到 `src/utils/math.ry`。
- **模块只初始化一次**：顶层代码只在首次导入时运行。

---

## 附录 A：快速参考 {#appendix-a-quick-reference}

### 关键字

```
let         const       fn          class
trait       match       if          else
for         while       return      throw
try         catch       finally     async
await       import      export      macro
type        true        false       null
self        super       this
```

### 运算符优先级（从高到低）

| 优先级 | 运算符 | 结合性 |
|------------|-----------|---------------|
| 18 | `.` `?.` `()` `[]` | 左结合 |
| 17 | `++` `--` `!` `~` `+` `-` `await` | 右结合 |
| 16 | `**` | 右结合 |
| 15 | `*` `/` `%` | 左结合 |
| 14 | `+` `-` | 左结合 |
| 13 | `<<` `>>` `>>>` | 左结合 |
| 12 | `<` `>` `<=` `>=` | 左结合 |
| 11 | `===` `!==` | 左结合 |
| 10 | `&` | 左结合 |
| 9 | `^` | 左结合 |
| 8 | `\|` | 左结合 |
| 7 | `&&` | 左结合 |
| 6 | `\|\|` | 左结合 |
| 5 | `??` | 左结合 |
| 4 | `?:` | 右结合 |
| 3 | `=>` | 右结合 |
| 2 | `=` `+=` `-=` 等 | 右结合 |
| 1 | `,` | 左结合 |

### 内置类型

| 类型 | 说明 |
|------|-------------|
| `int` | 64-bit signed integer |
| `float` | 64-bit floating point |
| `bool` | Boolean |
| `string` | UTF-8 string |
| `null` | Null type |
| `void` | No return value |
| `dyn` | Dynamic type |
| `never` | Bottom type |
| `bigint` | Arbitrary precision integer |

---

## 附录 B：从 JavaScript 迁移到 Ruyi 指南 {#appendix-b-javascript-to-ruyi-migration-guide}

| JavaScript | Ruyi | 说明 |
|------------|-------|-------|
| `var x` | `let x` | 块级作用域，而非函数级作用域 |
| `undefined` | `null` | 单一空值 |
| `==` / `!=` | `===` / `!==` | 只支持严格相等 |
| `function() {}` | `fn() {}` | 更短的关键字 |
| `this` in methods | `self` | 显式，没有绑定困惑 |
| `arguments` | `...args` | 剩余参数 |
| `prototype` | `class` / `trait` | 基于类的继承 |
| `with` | （已移除） | 没有等价物 |
| `eval()` | （已移除） | 没有等价物 |
| `delete obj.prop` | `obj.prop = null` | 不支持属性删除 |
| `function*` | `async fn*` | 只有异步生成器 |
| `typeof null` | `"null"` | 修复了 JS 的 bug |

---

## 附录 C：内置函数与标准库 {#appendix-c-built-in-functions-and-standard-library}

Ruyi 提供了多层级的内置功能。有些函数无需任何 `import` 即可使用，另一些则需要显式导入模块。

### C.1 编译器内置函数（无需导入）

这些函数**硬编码在编译器中**，在任何 Ruyi 程序中都无需 `import` 语句即可使用。它们在代码生成阶段被特殊处理。

#### `print(value)`

将值打印到 stdout，末尾带一个换行符。支持所有基本类型和数组。

```ruyi
print(42);              // "42\n"
print(3.14);            // "3.140000\n"
print("hello");         // "hello\n"
print([1, 2, 3]);       // "[1, 2, 3]\n"
```

| 类型 | 格式 |
|------|------|
| `int` | `%ld`（有符号 64 位整数） |
| `float` | `%f`（浮点数） |
| `string` | `%s`（C 字符串） |
| `Array<T>` | `[elem1, elem2, ...]` |
| 其他 | `<unknown>` |

#### `spawn(fn)`

在工作窃取调度器上生成一个绿色线程（轻量级并发任务）。返回任务句柄。

```ruyi
let task = spawn(() => {
  // 并发运行
  doHeavyWork();
});
```

---

### C.2 核心模块（自动可用）

`core.ry` 模块**自动对所有 Ruyi 程序可用**。它提供基本类型上的方法，这些方法映射到编译器内建函数（`__builtin_*`）。

#### String 方法

| 方法 | 签名 | 说明 |
|------|------|------|
| `length` | `fn length(self: string): int` | 返回字符数 |
| `slice` | `fn slice(self: string, start: int, end: int): string` | 提取子串 [start, end) |
| `find` | `fn find(self: string, substr: string): int` | 返回首次出现的索引，未找到返回 -1 |
| `replace` | `fn replace(self: string, from: string, to: string): string` | 替换首次出现的子串 |
| `toUpperCase` | `fn toUpperCase(self: string): string` | 转换为大写 |
| `toLowerCase` | `fn toLowerCase(self: string): string` | 转换为小写 |
| `trim` | `fn trim(self: string): string` | 去除首尾空白 |
| `contains` | `fn contains(self: string, substr: string): bool` | 检查是否包含子串 |
| `startsWith` | `fn startsWith(self: string, prefix: string): bool` | 检查前缀 |
| `endsWith` | `fn endsWith(self: string, suffix: string): bool` | 检查后缀 |
| `split` | `fn split(self: string, delimiter: string): Array<string>` | 按分隔符拆分 |

```ruyi
let s = "hello world";
s.length();           // 11
s.slice(0, 5);        // "hello"
s.find("world");      // 6
s.replace("world", "Ruyi");  // "hello Ruyi"
s.toUpperCase();      // "HELLO WORLD"
s.contains("hello");  // true
s.startsWith("hello"); // true
s.endsWith("world");  // true
s.split(" ");         // ["hello", "world"]
```

#### Int 方法

| 方法 | 签名 | 说明 |
|------|------|------|
| `toString` | `fn toString(self: int): string` | 转换为字符串 |
| `abs` | `fn abs(self: int): int` | 绝对值 |
| `min` | `fn min(self: int, other: int): int` | 两个整数中的最小值 |
| `max` | `fn max(self: int, other: int): int` | 两个整数中的最大值 |

```ruyi
let n = -42;
n.toString();   // "-42"
n.abs();        // 42
3.min(5);       // 3
3.max(5);       // 5
```

#### Float 方法

| 方法 | 签名 | 说明 |
|------|------|------|
| `toString` | `fn toString(self: float): string` | 转换为字符串 |
| `abs` | `fn abs(self: float): float` | 绝对值 |
| `min` | `fn min(self: float, other: float): float` | 两个浮点数中的最小值 |
| `max` | `fn max(self: float, other: float): float` | 两个浮点数中的最大值 |
| `round` | `fn round(self: float): int` | 四舍五入到最近整数 |
| `floor` | `fn floor(self: float): int` | 向下取整 |
| `ceil` | `fn ceil(self: float): int` | 向上取整 |

```ruyi
let f = 3.7;
f.toString();   // "3.7"
f.abs();        // 3.7
f.round();      // 4
f.floor();      // 3
f.ceil();       // 4
```

#### Bool 方法

| 方法 | 签名 | 说明 |
|------|------|------|
| `toString` | `fn toString(self: bool): string` | 转换为 "true" 或 "false" |

```ruyi
true.toString();    // "true"
false.toString();   // "false"
```

---

### C.3 标准库模块（需要导入）

这些模块必须通过 `import { ... } from "std::module"` 显式导入。

#### IO 模块（`std::io`）

控制台和文件 I/O 操作。

```ruyi
import { readLine } from "std::io";
```

| 函数 | 签名 | 说明 |
|------|------|------|
| `readLine` | `fn readLine(): string?` | 从 stdin 读取一行 |

**文件系统（`std::fs`）：**

文件系统操作已迁移至 `fs` 模块。从 `"std::fs"` 导入：

```ruyi
import { readFile, writeFile, exists, mkdir } from "std::fs";
```

主要函数（均提供 `*Async` 异步版本）：

| 函数 | 签名 | 说明 |
|------|------|------|
| `readFile` | `fn readFile(path: string): string` | 读取整个文件 |
| `writeFile` | `fn writeFile(path: string, content: string): void` | 写入字符串到文件 |
| `exists` | `fn exists(path: string): bool` | 检查路径是否存在 |
| `isFile` | `fn isFile(path: string): bool` | 检查是否为文件 |
| `isDir` | `fn isDir(path: string): bool` | 检查是否为目录 |
| `mkdir` | `fn mkdir(path: string, recursive: bool = false): void` | 创建目录 |
| `deleteFile` | `fn deleteFile(path: string): void` | 删除文件 |


#### Collections 模块（`std::collections`）

泛型集合类型：`Array<T>`、`Map<K, V>`、`Set<T>` 和 `Iterator<T>`。

**Array<T> 方法：**

| 方法 | 签名 | 说明 |
|------|------|------|
| `get` | `fn get(self, index: int): T` | 获取指定索引的元素 |
| `set` | `fn set(self, index: int, value: T): void` | 设置指定索引的元素 |
| `push` | `fn push(self, value: T): void` | 在末尾添加元素 |
| `pop` | `fn pop(self): T` | 移除并返回最后一个元素 |
| `map` | `fn map<U>(self, f: fn(T): U): Array<U>` | 转换元素 |
| `filter` | `fn filter(self, pred: fn(T): bool): Array<T>` | 过滤元素 |
| `reduce` | `fn reduce<U>(self, init: U, f: fn(U, T): U): U` | 归约为单个值 |
| `forEach` | `fn forEach(self, f: fn(T): void): void` | 对每个元素应用函数 |
| `iter` | `fn iter(self): ArrayIterator<T>` | 创建迭代器 |

**Map<K, V> 方法：**

| 方法 | 签名 | 说明 |
|------|------|------|
| `get` | `fn get(self, key: K): Option<V>` | 按键获取值 |
| `set` | `fn set(self, key: K, value: V): void` | 设置键值对 |
| `delete` | `fn delete(self, key: K): bool` | 移除条目 |
| `has` | `fn has(self, key: K): bool` | 检查键是否存在 |
| `keys` | `fn keys(self): Array<K>` | 获取所有键 |
| `values` | `fn values(self): Array<V>` | 获取所有值 |
| `entries` | `fn entries(self): Array<[K, V]>` | 获取所有键值对 |

**Set<T> 方法：**

| 方法 | 签名 | 说明 |
|------|------|------|
| `add` | `fn add(self, value: T): void` | 添加元素 |
| `delete` | `fn delete(self, value: T): bool` | 移除元素 |
| `has` | `fn has(self, value: T): bool` | 检查元素是否存在 |
| `union` | `fn union(self, other: Set<T>): Set<T>` | 并集 |
| `intersection` | `fn intersection(self, other: Set<T>): Set<T>` | 交集 |
| `difference` | `fn difference(self, other: Set<T>): Set<T>` | 差集 |

**Iterator<T> 特征：**

| 方法 | 签名 | 说明 |
|------|------|------|
| `next` | `fn next(self): Option<T>` | 获取下一个元素 |
| `forEach` | `fn forEach(self, f: fn(T): void): void` | 对每个元素应用函数 |
| `map` | `fn map<U>(self, f: fn(T): U): Iterator<U>` | 转换元素 |
| `filter` | `fn filter(self, pred: fn(T): bool): Iterator<T>` | 过滤元素 |
| `reduce` | `fn reduce<U>(self, init: U, f: fn(U, T): U): U` | 归约为单个值 |

#### Option 类型（`std::option`）

```ruyi
enum Option<T> {
    Some(T),
    None
}
```

| 方法 | 签名 | 说明 |
|------|------|------|
| `isSome` | `fn isSome<T>(self: Option<T>): bool` | 检查是否为 Some |
| `isNone` | `fn isNone<T>(self: Option<T>): bool` | 检查是否为 None |
| `unwrap` | `fn unwrap<T>(self: Option<T>): T` | 获取值（None 时 panic） |
| `unwrapOr` | `fn unwrapOr<T>(self: Option<T>, default: T): T` | 获取值或默认值 |
| `unwrapOrElse` | `fn unwrapOrElse<T>(self: Option<T>, f: fn(): T): T` | 获取值或计算默认值 |
| `map` | `fn map<T, U>(self: Option<T>, f: fn(T): U): Option<U>` | 转换包含的值 |
| `andThen` | `fn andThen<T, U>(self: Option<T>, f: fn(T): Option<U>): Option<U>` | 链式计算 |
| `filter` | `fn filter<T>(self: Option<T>, pred: fn(T): bool): Option<T>` | 按谓词过滤 |
| `flatten` | `fn flatten<T>(self: Option<Option<T>>): Option<T>` | 展平嵌套 Option |
| `okOr` | `fn okOr<T, E>(self: Option<T>, err: E): Result<T, E>` | 转换为 Result |
| `okOrElse` | `fn okOrElse<T, E>(self: Option<T>, f: fn(): E): Result<T, E>` | 转换为 Result（计算错误值） |
| `forEach` | `fn forEach<T>(self: Option<T>, f: fn(T): void): void` | 如果是 Some 则应用函数 |
| `toString` | `fn toString<T>(self: Option<T>): string` | 字符串表示 |

#### Result 类型（`std::result`）

```ruyi
enum Result<T, E> {
    Ok(T),
    Err(E)
}
```

| 方法 | 签名 | 说明 |
|------|------|------|
| `isOk` | `fn isOk<T, E>(self: Result<T, E>): bool` | 检查是否为 Ok |
| `isErr` | `fn isErr<T, E>(self: Result<T, E>): bool` | 检查是否为 Err |
| `unwrap` | `fn unwrap<T, E>(self: Result<T, E>): T` | 获取值（Err 时 panic） |
| `unwrapOr` | `fn unwrapOr<T, E>(self: Result<T, E>, default: T): T` | 获取值或默认值 |
| `unwrapOrElse` | `fn unwrapOrElse<T, E>(self: Result<T, E>, f: fn(E): T): T` | 获取值或计算默认值 |
| `map` | `fn map<T, U, E>(self: Result<T, E>, f: fn(T): U): Result<U, E>` | 转换 Ok 值 |
| `mapErr` | `fn mapErr<T, E, F>(self: Result<T, E>, f: fn(E): F): Result<T, F>` | 转换 Err 值 |
| `andThen` | `fn andThen<T, U, E>(self: Result<T, E>, f: fn(T): Result<U, E>): Result<U, E>` | 链式计算 |
| `filter` | `fn filter<T, E>(self: Result<T, E>, pred: fn(T): bool): Result<T, E>` | 按谓词过滤 |
| `ok` | `fn ok<T, E>(self: Result<T, E>): Option<T>` | 转换为 Option |
| `err` | `fn err<T, E>(self: Result<T, E>): Option<E>` | 将 Err 转为 Option |
| `forEach` | `fn forEach<T, E>(self: Result<T, E>, f: fn(T): void): void` | 如果是 Ok 则应用函数 |
| `toOption` | `fn toOption<T, E>(self: Result<T, E>): Option<T>` | 转换为 Option |
| `toBool` | `fn toBool<T, E>(self: Result<T, E>): bool` | 转换为 bool |
| `toString` | `fn toString<T, E>(self: Result<T, E>): string` | 字符串表示 |

#### Error 类型（`std::error`）

错误层次结构和工具函数。

**错误类：**

| 类 | 说明 |
|------|------|
| `Error` | 基础错误类，包含消息和原因链 |
| `TypeError` | 类型检查或转换失败 |
| `RuntimeError` | 运行时操作失败 |
| `RangeError` | 索引越界 |
| `AssertionError` | 断言失败 |
| `ArgumentError` | 无效的参数值 |
| `NullError` | 需要值但遇到 null |
| `ArithmeticError` | 除以零等算术错误 |
| `IteratorError` | 迭代器相关问题 |
| `ParseError` | 解析失败 |

**工具函数：**

| 函数 | 签名 | 说明 |
|------|------|------|
| `isError` | `fn isError(value: dynamic): bool` | 检查值是否为 Error |
| `assert` | `fn assert(condition: bool, message: string): void` | 断言条件 |
| `assertNotNull` | `fn assertNotNull<T>(value: T, message: string): void` | 断言非 null |
| `errorWithCause` | `fn errorWithCause(message: string, cause: Error): Error` | 创建带原因的错误 |

#### String 模块（`std::string`）

纯字符串工具函数。字符串实例方法（`split`、`contains`、`trim` 等）已在 `core` 模块中自动可用。

| 函数 | 签名 | 说明 |
|------|------|------|
| `join` | `fn join(array: Array<dyn>, separator: string = ""): string` | 连接数组元素 |
| `fromCharCode` | `fn fromCharCode(code: int): string` | 从字符码创建 |
| `fromCharCodes` | `fn fromCharCodes(codes: Array<int>): string` | 从字符码数组创建 |
| `concat` | `fn concat(args: ...string): string` | 连接字符串 |
| `template` | `fn template(template: string, values: Array<dyn>): string` | 格式化模板 |
| `processTemplate` | `fn processTemplate(parts: Array<dyn>, context: dyn): string` | 处理模板字面量 |

#### Path 模块（`std::path`）

文件系统路径操作工具。

| 方法 | 签名 | 说明 |
|------|------|------|
| `Path.join` | `static fn join(paths: ...string): string` | 连接路径段 |
| `Path.basename` | `static fn basename(path: string): string` | 获取文件名 |
| `Path.basenameNoExt` | `static fn basenameNoExt(path: string): string` | 获取不带扩展名的文件名 |
| `Path.dirname` | `static fn dirname(path: string): string` | 获取目录名 |
| `Path.extname` | `static fn extname(path: string): string` | 获取文件扩展名 |
| `Path.isAbsolute` | `static fn isAbsolute(path: string): bool` | 检查是否为绝对路径 |
| `Path.isRelative` | `static fn isRelative(path: string): bool` | 检查是否为相对路径 |
| `Path.resolve` | `static fn resolve(base: string, relative: string): string` | 解析相对路径 |
| `Path.normalize` | `static fn normalize(path: string): string` | 规范化路径 |
| `Path.withoutExt` | `static fn withoutExt(path: string): string` | 移除扩展名 |
| `Path.changeExt` | `static fn changeExt(path: string, newExt: string): string` | 更改扩展名 |
| `Path.compare` | `static fn compare(path1: string, path2: string): int` | 比较路径 |
| `Path.equals` | `static fn equals(path1: string, path2: string): bool` | 检查路径是否相等 |
| `Path.parents` | `static fn parents(path: string): Array<string>` | 获取父目录 |
| `Path.isChildOf` | `static fn isChildOf(parent: string, child: string): bool` | 检查是否为子路径 |
| `Path.relative` | `static fn relative(from: string, to: string): string` | 获取相对路径 |
| `Path.separator` | `static fn separator(): string` | 获取平台路径分隔符 |

#### Process 模块（`std::process`）

进程管理和系统命令执行。

| 函数 | 签名 | 说明 |
|------|------|------|
| `Process.create` | `static fn create(command: string, options: ProcessOptions?): Process` | 创建进程 |
| `Process.exec` | `static fn exec(command: string): ProcessResult` | 执行命令 |
| `Process.execWith` | `static fn execWith(command: string, options: ExecOptions): ProcessResult` | 带选项执行 |
| `Process.spawn` | `static fn spawn(command: string, args: Array<string>?): Process` | 生成子进程 |
| `Process.spawnWith` | `static fn spawnWith(command: string, args: Array<string>?, options: ProcessOptions): Process` | 带选项生成 |
| `getEnv` | `fn getEnv(name: string): string?` | 获取环境变量 |
| `setEnv` | `fn setEnv(name: string, value: string): void` | 设置环境变量 |
| `getAllEnv` | `fn getAllEnv(): Map<string, string>` | 获取所有环境变量 |
| `getPID` | `fn getPID(): int` | 获取当前进程 ID |
| `getPPID` | `fn getPPID(): int` | 获取父进程 ID |
| `getPlatform` | `fn getPlatform(): string` | 获取平台（"linux"、"macos"、"windows"） |
| `getCPUCount` | `fn getCPUCount(): int` | 获取 CPU 核心数 |
| `getTotalMemory` | `fn getTotalMemory(): int` | 获取总内存（字节） |
| `getFreeMemory` | `fn getFreeMemory(): int` | 获取可用内存（字节） |

---

### C.4 运行时内部函数

这些函数在编译时被声明到 LLVM 模块中，由编译器的代码生成内部使用。**用户不应直接调用这些函数**——它们由编译器自动调用。

#### 垃圾回收

| 运行时函数 | 说明 |
|------------|------|
| `ruyi_gc_alloc(size)` | 在 GC 堆上分配内存 |
| `ruyi_gc_collect()` | 触发垃圾回收 |
| `ruyi_gc_add_root(ptr)` | 添加 GC 根 |
| `ruyi_gc_remove_root(ptr)` | 移除 GC 根 |
| `ruyi_gc_write_barrier(parent, field)` | 分代 GC 的写屏障 |

#### 异常处理

| 运行时函数 | 说明 |
|------------|------|
| `ruyi_throw(exception)` | 抛出异常 |
| `ruyi_get_pending_exception()` | 获取待处理异常 |
| `ruyi_clear_pending_exception()` | 清除待处理异常 |
| `ruyi_begin_catch(exception)` | 进入 catch 块 |
| `ruyi_end_catch()` | 退出 catch 块 |

#### 字符串操作

| 运行时函数 | 说明 |
|------------|------|
| `ruyi_str_concat(a, b)` | 连接两个字符串 |

#### 异步/调度器

| 运行时函数 | 说明 |
|------------|------|
| `ruyi_async_poll(future, waker)` | 轮询异步 future |
| `ruyi_spawn(future)` | 在调度器上生成任务 |
| `ruyi_wake_task(task)` | 唤醒休眠任务 |
| `ruyi_run_scheduler()` | 运行工作窃取调度器 |

---

## 附录 D：完整示例——一个简单的 CLI 工具 {#appendix-d-complete-example---a-simple-cli-tool}

下面是一个完整的 Ruyi 程序，展示了多个特性协同工作：

```ruyi
import { readLine } from "std::io";
import { parseInt } from "std::string";

// Trait definition
trait Display {
  fn display(self): string;
}

// Generic class with trait bound
class Stack<T: Display> {
  items: Array<T>;

  fn new() {
    self.items = [];
  }

  fn push(item: T) {
    self.items.push(item);
  }

  fn pop(): T? {
    if (self.items.length === 0) {
      return null;
    }
    return self.items.pop();
  }

  fn isEmpty(): bool {
    return self.items.length === 0;
  }
}

// Implement trait for int
impl Display for int {
  fn display(self): string {
    return toString(self);
  }
}

// Implement trait for string
impl Display for string {
  fn display(self): string {
    return self;
  }
}

// Generic function with trait bound
fn printStack<T: Display>(stack: Stack<T>) {
  let temp = [];
  while !stack.isEmpty() {
    let item = stack.pop()!;
    print("  [" + item.display() + "]");
    temp.push(item);
  }
  // Restore stack
  for (let i = temp.length - 1; i >= 0; i = i - 1) {
    stack.push(temp[i]);
  }
}

// Async function example
async fn fetchConfig(): string {
  // Simulated async operation
  return "config loaded";
}

fn main() {
  // Stack usage
  let stack = Stack<int>.new();
  stack.push(1);
  stack.push(2);
  stack.push(3);

  println("Stack contents:");
  printStack(stack);

  // Pattern matching
  let result = stack.pop();
  match (result) {
    Some(value) => { println("Popped: " + value.display()); }
    None => { println("Stack was empty"); }
    _ => { println("Unexpected"); }
  }

  // Null safety
  let name: string? = null;
  let displayName = name ?? "Guest";
  println("Hello, " + displayName);

  // Error handling
  try {
    let config = fetchConfig();
    println(config);
  } catch (e: Error) {
    println("Failed to load config: " + e.message);
  }
}
```

本示例展示了：

- **特征 (Trait)**：`Display` 特征，以及 `int` 和 `string` 的实现
- **泛型**：`Stack<T>`，带有特征约束 `T: Display`
- **模式匹配**：对 `Option<T>` 结果使用 `match`
- **空值安全**：`??` 运算符用于默认值
- **异步/等待**：`async fn` 用于异步操作
- **错误处理**：`try/catch` 用于异常处理
- **控制流**：`for`、`while`、`if` 语句
- **类**：`Stack` 类，包含方法和字段

---

*Ruyi 语言教程结束*
