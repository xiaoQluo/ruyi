# Ruyi Language Tutorial

> **Version**: 0.1.0
> **Date**: 2026-05-02
> **Audience**: Programmers familiar with JavaScript, TypeScript, Rust, or similar languages

---

## Table of Contents

1. [Getting Started](#1-getting-started)
2. [Basic Syntax](#2-basic-syntax)
3. [Control Flow](#3-control-flow)
4. [Functions](#4-functions)
5. [Classes and Objects](#5-classes-and-objects)
6. [Type System](#6-type-system)
7. [Generics](#7-generics)
8. [Traits](#8-traits)
9. [Pattern Matching](#9-pattern-matching)
10. [Error Handling](#10-error-handling)
11. [Async Programming](#11-async-programming)
12. [Modules](#12-modules)

---

## Appendices

- [Appendix C: Built-in Functions and Standard Library](#appendix-c-built-in-functions-and-standard-library)
- [Appendix D: Complete Example - A Simple CLI Tool](#appendix-d-complete-example---a-simple-cli-tool)

---

## 1. Getting Started

### 1.1 What is Ruyi?

Ruyi is a compiled, general-purpose programming language built on the syntactic foundation of JavaScript strict mode. It removes problematic JavaScript features while retaining familiar syntax. Ruyi targets native machine code via LLVM, providing high performance across platforms.

Key features:

- **Familiar syntax**: If you know JavaScript, you already know most of Ruyi's syntax.
- **Compiled to native code**: Uses LLVM to produce fast, standalone binaries.
- **Gradual typing**: Choose between static type annotations and dynamic typing.
- **Null safety**: No more `undefined`. Nullable types are explicit.
- **Pattern matching**: First-class `match` expressions with destructuring.
- **Traits**: Interface-like contracts with static and dynamic dispatch.
- **Generics**: Parametric polymorphism with monomorphization.
- **Async/await**: Green-thread-based concurrency with a work-stealing scheduler.
- **Exception handling**: Zero-cost try/catch/finally via LLVM landing pads.
- **Macros**: Declarative, hygienic compile-time code generation.

### 1.2 Installation

To install the Ruyi compiler (`ruyic`), clone the repository and build from source:

```bash
git clone https://github.com/example/ruyi.git
cd ruyi
cargo build --release
```

The compiler binary will be available at `./target/release/ruyic`. Add it to your PATH:

```bash
export PATH="$PWD/target/release:$PATH"
```

Verify the installation:

```bash
ruyic --version
```

Expected output:

```
ruyic 0.1.0
```

### 1.3 Hello, World!

Create a file named `hello.ry`:

```ruyi
print("Hello, Ruyi!");
```

Compile and run:

```bash
ruyic hello.ry -o hello
./hello
```

Expected output:

```
Hello, Ruyi!
```

### 1.4 A Slightly Larger Example

Here is a program that computes the Fibonacci sequence:

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

Compile and run:

```bash
ruyic fibonacci.ry -o fib
./fib
```

Expected output:

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

### 1.5 Compiler Flags

The Ruyi compiler supports several useful flags:

| Flag | Description |
|------|-------------|
| `-o <output>` | Specify output binary name |
| `--emit-llvm` | Output LLVM IR instead of a binary |
| `-O0`, `-O1`, `-O2` | Optimization level (default: `-O2`) |
| `--debug` | Include debug symbols |
| `--version` | Print compiler version |

Example: emit LLVM IR to inspect what the compiler generates:

```bash
ruyic hello.ry --emit-llvm
```

### 1.6 Project Structure

A typical Ruyi project looks like this:

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

Each `.ry` file is a module. Modules are imported using relative or absolute paths. See [Chapter 12: Modules](#12-modules) for details.

### Common Pitfalls

- **No `var`**: Ruyi removed `var`. Use `let` for mutable variables and `const` for immutable ones.
- **No `undefined`**: Ruyi has only `null`. Uninitialized variables default to `null`.
- **No `==` or `!=`**: Only strict equality (`===`, `!==`) exists. No implicit type coercion.
- **Semicolons**: Statements end with semicolons. The compiler has clearer ASI rules than JavaScript, but you should still write semicolons explicitly.

---

## 2. Basic Syntax

### 2.1 Variables: `let` and `const`

Ruyi has two variable declaration keywords:

- `let` declares a **mutable** variable.
- `const` declares an **immutable** variable (cannot be reassigned).

```ruyi
let x = 42;
x = 100;          // OK: let is mutable

const PI = 3.14159;
// PI = 3;        // ERROR: const cannot be reassigned
```

Both `let` and `const` are **block-scoped**. Variables declared inside a block are not visible outside:

```ruyi
{
  let inner = "visible inside";
  print(inner);
}
// print(inner);  // ERROR: inner is not defined here
```

### 2.2 Built-in Types

Ruyi provides these built-in primitive types:

| Type | Description | Example |
|------|-------------|---------|
| `int` | 64-bit signed integer | `42`, `-7`, `0xFF` |
| `float` | 64-bit floating point | `3.14`, `1e10`, `0.5` |
| `bool` | Boolean | `true`, `false` |
| `string` | UTF-8 string | `"hello"`, `'world'` |
| `null` | Null type (only value: `null`) | `null` |
| `void` | No return value | (used in function return types) |
| `dyn` | Dynamic type (runtime checked) | (see Chapter 6) |
| `never` | Bottom type (unreachable) | (see Chapter 10) |

### 2.3 Type Annotations

You can annotate variables with types using a colon:

```ruyi
let count: int = 42;
let name: string = "Ruyi";
let ratio: float = 0.75;
let active: bool = true;
```

When no annotation is provided, Ruyi infers the type from the initializer:

```ruyi
let x = 42;           // x: int (inferred)
let y = "hello";      // y: string (inferred)
let z = true;         // z: bool (inferred)
```

If there is no initializer and no annotation, the type defaults to `dyn`:

```ruyi
let unknown;          // unknown: dyn
```

### 2.4 Numeric Literals

Ruyi supports multiple numeric literal formats:

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

### 2.5 String Literals

Ruyi supports three string literal forms:

```ruyi
// Double-quoted
let greeting = "Hello, Ruyi!";

// Single-quoted
let name = 'World';

// Template literals (with interpolation)
let message = `Hello, ${name}!`;
let result = `The sum is: ${2 + 3}`;
```

Template literals can span multiple lines:

```ruyi
let multi = `line one
line two
line three`;
```

Escape sequences work in all string forms:

```ruyi
let escaped = "line1\nline2\ttab";
let unicode = "emoji: \u{1F600}";
```

### 2.6 Operators

#### Arithmetic Operators

```ruyi
let sum = 10 + 3;       // 13
let diff = 10 - 3;      // 7
let product = 10 * 3;   // 30
let quotient = 10 / 3;  // 3 (integer division for int)
let remainder = 10 % 3; // 1
let power = 2 ** 8;     // 256
```

#### Comparison Operators

Ruyi uses **strict equality only**. There is no `==` or `!=`:

```ruyi
let eq = 5 === 5;       // true
let neq = 5 !== 3;      // true
let lt = 3 < 5;         // true
let gt = 5 > 3;         // true
let lte = 3 <= 3;       // true
let gte = 5 >= 5;       // true
```

No implicit type coercion:

```ruyi
// In JavaScript: "5" == 5 is true
// In Ruyi: this is a compile error
// "5" === 5  // ERROR: type mismatch (string vs int)
```

#### Logical Operators

```ruyi
let and = true && false;   // false
let or = true || false;    // true
let not = !true;           // false
```

#### Nullish Coalescing

The `??` operator returns the right operand if the left is `null`:

```ruyi
let name: string? = null;
let displayName = name ?? "anonymous";  // "anonymous"
```

#### Optional Chaining

The `?.` operator safely accesses properties on nullable values:

```ruyi
let user: User? = findUser(1);
let userName = user?.name;    // string? (null if user is null)
```

### 2.7 Comments

```ruyi
// Single-line comment

/* Multi-line
   comment */

/**
 * Documentation comment.
 * Preserved for tooling.
 */
```

### Common Pitfalls

- **No implicit coercion**: `"5" + 3` is a compile error. Use `"5" + toString(3)`.
- **No `==`**: Only `===` and `!==` exist.
- **No `undefined`**: Only `null` represents absence of a value.
- **Block scoping**: `let` and `const` are block-scoped, not function-scoped.

---

## 3. Control Flow

### 3.1 `if` / `else`

The `if` statement evaluates a condition and executes the corresponding branch:

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

Conditions must be of type `bool`. There is no truthy/falsy coercion:

```ruyi
// In JavaScript: if ("hello") { } works
// In Ruyi: this is a compile error
// if ("hello") { }  // ERROR: expected bool, got string
```

### 3.2 `if` as an Expression

In Ruyi, `if` can be used as an expression that returns a value:

```ruyi
let sign = if (x > 0) {
  "positive"
} else if (x < 0) {
  "negative"
} else {
  "zero"
};
```

Both branches must return compatible types. The type of the `if` expression is the least upper bound of the branch types.

### 3.3 `for` Loop

Ruyi supports the C-style `for` loop:

```ruyi
for (let i = 0; i < 10; i = i + 1) {
  print(i);
}
```

Reverse iteration:

```ruyi
for (let i = items.length - 1; i >= 0; i = i - 1) {
  process(items[i]);
}
```

### 3.4 `for-in` Loop

Iterates over the keys of an object or indices of an array:

```ruyi
let obj = { name: "Ruyi", version: "0.1.0" };

for (let key in obj) {
  print(key + ": " + obj[key]);
}
```

### 3.5 `for-of` Loop

Iterates over the values of an iterable:

```ruyi
let items = ["apple", "banana", "cherry"];

for (let item of items) {
  print(item);
}
```

### 3.6 `while` Loop

Executes a block while a condition is true:

```ruyi
let i = 0;
while (i < 10) {
  print(i);
  i = i + 1;
}
```

### 3.7 `break` and `continue`

`break` exits the innermost enclosing loop:

```ruyi
for (let i = 0; i < 100; i = i + 1) {
  if (i === 50) {
    break;
  }
  print(i);
}
// Prints 0 through 49
```

`continue` skips to the next iteration:

```ruyi
for (let i = 0; i < 10; i = i + 1) {
  if (i % 2 === 0) {
    continue;
  }
  print(i);
}
// Prints odd numbers: 1, 3, 5, 7, 9
```

### 3.8 Labeled Break and Continue

You can label loops and break or continue to a specific label:

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

### Common Pitfalls

- **No truthy/falsy**: Conditions must be `bool`. `if (0)`, `if ("")`, and `if (null)` are all compile errors.
- **No `do-while`**: Ruyi does not have a `do-while` loop. Use `while` with the condition checked at the start.
- **Infinite loops**: `while (true)` is valid. Make sure to include a `break` or mutation that eventually makes the condition false.

---

## 4. Functions

### 4.1 Function Declarations

Functions are declared with the `fn` keyword:

```ruyi
fn add(a: int, b: int): int {
  return a + b;
}

let result = add(3, 5);  // 8
```

The `fn` keyword replaces JavaScript's `function`. It is shorter and consistent with other declarations (`class`, `trait`, `macro`).

### 4.2 Return Type Inference

When no return type annotation is provided, Ruyi infers it from `return` statements:

```ruyi
fn add(a: int, b: int) {
  return a + b;
}
// Inferred: fn add(a: int, b: int): int
```

If a function has no `return` statement, the inferred return type is `void`:

```ruyi
fn greet(name: string) {
  print("Hello, " + name);
}
// Inferred: fn greet(name: string): void
```

If multiple return paths exist, the return type is the least upper bound of all returned types:

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

### 4.3 Arrow Functions

Arrow functions provide a concise syntax for function expressions:

```ruyi
let double = (x) => x * 2;
let greet = (name) => { print("Hi, " + name); };
let add = (a, b) => a + b;
```

Arrow functions with a single expression omit the braces and `return`:

```ruyi
let square = (x) => x * x;
// Equivalent to:
let square = (x) => { return x * x; };
```

### 4.4 Parameters

#### Default Parameters

```ruyi
fn greet(name: string = "World") {
  print("Hello, " + name);
}

greet();             // "Hello, World"
greet("Ruyi");      // "Hello, Ruyi"
```

#### Rest Parameters

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

#### Destructuring Parameters

```ruyi
fn printPoint({ x, y }: { x: float, y: float }) {
  print("(" + x + ", " + y + ")");
}

let point = { x: 3.0, y: 4.0 };
printPoint(point);   // "(3, 4)"
```

### 4.5 Closures

Functions capture variables from their enclosing scope:

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

### 4.6 Higher-Order Functions

Functions can accept other functions as parameters:

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

### Common Pitfalls

- **No `function` keyword**: Use `fn` instead.
- **No `arguments` object**: Use rest parameters (`...args`) instead.
- **No automatic `this` binding**: Methods use `self` explicitly. Arrow functions capture `self` lexically.
- **Return type matters**: If you forget `return`, the function returns `null` (or `void` if inferred).

---

## 5. Classes and Objects

### 5.1 Class Declarations

Classes are declared with the `class` keyword:

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

### 5.2 Constructors

The `new` method serves as the constructor. It is called when creating a new instance:

```ruyi
let p = Point.new(3.0, 4.0);
print(p.distance(Point.new(0.0, 0.0)));  // 5.0
```

Inside the constructor, `self` refers to the instance being created. Fields are initialized by assigning to `self.fieldName`.

### 5.3 Static Methods

Static methods are declared with the `static` keyword:

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

### 5.4 Inheritance

Classes can extend other classes using `extends`:

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

Use `super` to call the parent class constructor or methods.

### 5.5 Getters and Setters

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

### 5.6 Object Literals

For simple data structures, object literals provide a lightweight alternative to classes:

```ruyi
let person = {
  name: "Alice",
  age: 30,
  city: "New York"
};

print(person.name);    // "Alice"
```

Object literals support spread syntax:

```ruyi
let defaults = { theme: "light", fontSize: 14 };
let userPrefs = { fontSize: 16 };
let config = { ...defaults, ...userPrefs };
// config: { theme: "light", fontSize: 16 }
```

### Common Pitfalls

- **No prototype chain**: Ruyi removed JavaScript's prototype-based inheritance. Use `class` and `extends`.
- **`self` not `this`**: Methods use `self` to refer to the current instance. This avoids the confusing `this` binding behavior of JavaScript.
- **Fields must be declared**: Class fields must be declared with type annotations.
- **No `delete`**: You cannot delete object properties. Assign `null` instead.

---

## 6. Type System

### 6.1 Gradual Typing

Ruyi uses a **gradual type system** that combines static type checking with dynamic type checking. You can choose to annotate types for compile-time safety, or omit annotations and rely on runtime checks.

```ruyi
// Static typing (compile-time checked)
let x: int = 42;
let y: string = "hello";

// Dynamic typing (runtime checked)
let a = 42;           // a: int (inferred)
let b;                // b: dyn (no annotation, no initializer)
```

### 6.2 The `dyn` Type

`dyn` is the dynamic type. It represents a value whose type is checked at runtime:

```ruyi
let value: dyn = 42;
value = "hello";      // OK: dyn can hold any type
value = true;         // OK
```

When a `dyn` value is used in a statically-typed context, a runtime check is inserted:

```ruyi
let value: dyn = 42;
let x: int = value;   // Runtime check: throws TypeError if value is not int
```

### 6.3 Type Inference

Ruyi uses bidirectional type inference. For variable declarations, the type is inferred from the initializer:

```ruyi
let x = 42;           // x: int
let y = "hello";      // y: string
let z = true;         // z: bool
let arr = [1, 2, 3];  // arr: Array<int>
```

**Literal type inference**:

| Literal | Inferred Type |
|---------|---------------|
| `42` | `int` |
| `3.14` | `float` |
| `100n` | `bigint` |
| `"hello"` | `string` |
| `true` / `false` | `bool` |
| `null` | `null` |
| `[1, 2, 3]` | `Array<int>` |
| `{ x: 1, y: 2 }` | `{ x: int, y: int }` |

### 6.4 Nullable Types

Ruyi has a sound nullable type system. There is no `undefined`. Nullable types must be explicitly declared with `?`:

```ruyi
let name: string = "Ruyi";     // cannot be null
let maybe: string? = null;      // can be null
```

#### Optional Chaining

```ruyi
let user: User? = findUser(1);
let userName = user?.name;           // string?
let city = user?.address?.city;      // string?
```

#### Nullish Coalescing

```ruyi
let name = user?.name ?? "anonymous";    // string
let count = config.count ?? 0;           // int
```

#### Null Assertion

The `!` operator asserts that a nullable value is not null:

```ruyi
let name: string? = getUser();
let safe: string = name!;    // throws if name is null
```

#### Type Narrowing

After a null check, the compiler narrows the type:

```ruyi
let name: string? = getUser();

if (name !== null) {
  // name is narrowed to string here
  print(name.length);
}

// name is string? again here
```

### 6.5 Function Types

Function types are written as `fn(T1, T2, ...) -> R`:

```ruyi
let add: fn(int, int) -> int = (a, b) => a + b;
let log: fn(string) -> void = (msg) => { print(msg); };
```

### 6.6 Structural Subtyping

Object types use structural subtyping. An object type `{ a: int, b: int, c: int }` is a subtype of `{ a: int, b: int }`:

```ruyi
let point3d = { x: 1.0, y: 2.0, z: 3.0 };
let point2d: { x: float, y: float } = point3d;  // OK: point3d has all required fields
```

### Common Pitfalls

- **No implicit coercion**: `"5" + 3` is a compile error. Use explicit conversion.
- **`dyn` is not magic**: Using `dyn` inserts runtime checks. It is not a substitute for proper typing.
- **Nullable is explicit**: `string` cannot hold `null`. Use `string?` if null is possible.
- **Narrowing resets on reassignment**: After `name = getUser()`, previous narrowing is lost.

---

## 7. Generics

### 7.1 Generic Functions

Generic functions introduce type parameters with angle brackets:

```ruyi
fn identity<T>(x: T): T {
  return x;
}

let a = identity(42);           // a: int
let b = identity("hello");      // b: string
let c = identity(true);         // c: bool
```

### 7.2 Trait Bounds

Type parameters can be constrained by traits:

```ruyi
fn max<T: Comparable>(a: T, b: T): T {
  return if a.compare(b) > 0 { a } else { b };
}

let m = max(3, 5);              // OK: int implements Comparable
// max(true, false);            // ERROR: bool does not implement Comparable
```

Multiple bounds use `+`:

```ruyi
fn process<T: Comparable + Clone>(value: T) {
  let copy = value.clone();
  let comparison = value.compare(copy);
  print(comparison);
}
```

### 7.3 Generic Classes

Classes can be generic:

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

### 7.4 Generic Type Aliases

```ruyi
type Result<T, E> = Ok<T> | Err<E>;
type Callback<T> = fn(T) -> void;
type Pair<T> = { first: T, second: T };
```

### 7.5 Type Inference with Generics

Ruyi infers generic type parameters from context:

```ruyi
fn wrap<T>(value: T): Option<T> {
  return Option.new(value);
}

let x = wrap(42);       // x: Option<int>
let y = wrap("hello");  // y: Option<string>
```

### 7.6 Monomorphization

Ruyi uses monomorphization for generics. At each call site, the compiler generates a specialized copy of the generic function:

```ruyi
fn identity<T>(x: T): T { return x; }

let a = identity(42);       // generates identity_int(x: int): int
let b = identity("hello");  // generates identity_string(x: string): string
```

This produces fast, type-specific code with no runtime overhead.

### Common Pitfalls

- **Trait bounds are required for operations**: If you want to compare values, you need `T: Comparable`.
- **Monomorphization can increase binary size**: Each unique type combination generates a new copy of the function.
- **`dyn` disables monomorphization**: Calling a generic function with `dyn` uses a single version with runtime checks.

---

## 8. Traits

### 8.1 Trait Declarations

A trait defines a contract that types can implement:

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

Trait methods have no bodies. They are signatures only.

### 8.2 Trait Implementations

Types implement traits via `impl` blocks:

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

### 8.3 Static Dispatch

When the concrete type is known at compile time, trait method calls use static dispatch:

```ruyi
fn printIt<T: Printable>(value: T) {
  print(value.format());    // static dispatch: monomorphized
}

printIt("hello");    // calls string.format() directly
printIt(42);         // calls int.format() directly
```

The compiler generates specialized versions of `printIt` for each type, with direct function calls.

### 8.4 Dynamic Dispatch (Trait Objects)

When the concrete type is not known, use `dyn Trait` for dynamic dispatch:

```ruyi
let items: Array<dyn Printable> = ["hello", 42, true];
for (let item of items) {
  print(item.format());    // dynamic dispatch: vtable lookup
}
```

Trait objects consist of a data pointer and a vtable pointer:

```
TraitObject {
  data: *void,        // pointer to the concrete value
  vtable: *VTable,    // pointer to method implementations
}
```

### 8.5 Default Method Implementations

Traits can provide default implementations:

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

Implementations that do not override `collect` inherit the default:

```ruyi
impl Iterator<int> for NumberRange {
  fn next(self): int? {
    // ...
  }
  // collect() is inherited from the trait
}
```

### 8.6 Trait Object Downcasting

Trait objects can be downcast to their concrete types using pattern matching:

```ruyi
let y: dyn Printable = "hello";

match (y) {
  s as string => { print("string: " + s); }
  n as int => { print("int: " + n); }
  _ => { print("unknown type"); }
}
```

### 8.7 Orphan Rule

An `impl` block must be in the same module as either the trait or the type being implemented. This prevents conflicting implementations:

```ruyi
// OK: implementing your trait for a built-in type
impl Printable for string { ... }

// OK: implementing a built-in trait for your type
impl Comparable for MyType { ... }

// ERROR: implementing someone else's trait for someone else's type
// impl SomeExternalTrait for SomeExternalType { ... }
```

### Common Pitfalls

- **Trait methods have no bodies in declarations**: Only signatures. Default implementations are a separate feature.
- **`dyn Trait` erases the concrete type**: You can only access trait methods through a trait object.
- **Orphan rule prevents conflicts**: You cannot implement an external trait on an external type.

---

## 9. Pattern Matching

### 9.1 `match` Expressions

The `match` expression evaluates a value against a series of patterns:

```ruyi
let value = 3;

match (value) {
  0 => { print("zero"); }
  1 => { print("one"); }
  2 => { print("two"); }
  _ => { print("other"); }
}
```

The `_` pattern is a wildcard that matches anything. It must be the last arm.

### 9.2 Literal Patterns

Match against literal values:

```ruyi
match (status) {
  200 => { print("OK"); }
  404 => { print("Not Found"); }
  500 => { print("Server Error"); }
  _ => { print("Unknown: " + status); }
}
```

### 9.3 Or Patterns

Use `|` to match multiple patterns:

```ruyi
match (value) {
  1 | 2 | 3 => { print("small"); }
  4 | 5 | 6 => { print("medium"); }
  _ => { print("large"); }
}
```

### 9.4 Guard Clauses

Add conditions to match arms with `if`:

```ruyi
match (n) {
  0 => { print("zero"); }
  n if (n > 0 && n < 10) => { print("single digit: " + n); }
  n if (n >= 10 && n < 100) => { print("double digit: " + n); }
  _ => { print("other"); }
}
```

### 9.5 Destructuring Objects

```ruyi
let result = { status: 200, body: "Hello" };

match (result) {
  { status: 200, body } => { print(body); }
  { status: 404 } => { print("not found"); }
  { status, body } => { print("error " + status + ": " + body); }
  _ => { print("unknown response"); }
}
```

### 9.6 Destructuring Arrays

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

### 9.7 `if-let` Statement

The `if-let` statement combines pattern matching with conditional execution:

```ruyi
let point = { x: 3.0, y: 4.0 };

if let { x, y } = point {
  print("point at (" + x + ", " + y + ")");
}
```

With an `else` clause:

```ruyi
let result = Ok(42);

if let Ok(value) = result {
  print("success: " + value);
} else {
  print("failed");
}
```

### 9.8 `while-let` Statement

The `while-let` statement loops while a pattern matches:

```ruyi
while let Some(item) = iterator.next() {
  process(item);
}
```

### 9.9 `as` Patterns

Bind the entire matched value to a name:

```ruyi
match (value) {
  { x, y } as point => {
    print("point: " + point);
    print("x: " + x + ", y: " + y);
  }
  _ => { print("not a point"); }
}
```

### Common Pitfalls

- **Exhaustiveness**: All possible values must be covered. Use `_` as a catch-all if needed.
- **Order matters**: Patterns are tried top to bottom. More specific patterns should come first.
- **Guards are evaluated after pattern matching**: A guard clause only runs if the pattern matches.

---

## 10. Error Handling

### 10.1 `try` / `catch` / `finally`

Ruyi uses exceptions for error handling:

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

### 10.2 Multiple Catch Clauses

Catch clauses are tried in order. The first matching clause handles the exception:

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

Catch clauses match subtypes. `catch (e: Error)` catches `TypeError`, `RangeError`, and all other `Error` subtypes.

### 10.3 Catch Without Binding

You can omit the exception variable if you do not need it:

```ruyi
try {
  doSomething();
} catch {
  print("something failed");
}
```

### 10.4 `throw` Statement

Raise an exception with `throw`:

```ruyi
fn divide(a: int, b: int): int {
  if (b === 0) {
    throw Error("division by zero");
  }
  return a / b;
}
```

### 10.5 Custom Error Types

Create custom error types by extending `Error`:

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

### 10.6 The `never` Type

Functions that always throw can be annotated with return type `never`:

```ruyi
fn fail(message: string): never {
  throw Error(message);
}
```

The `never` type is the bottom type. It is a subtype of every type, meaning a `never` expression can be used in any context:

```ruyi
let x: int = if (condition) {
  42
} else {
  fail("impossible");    // never is a subtype of int
};
```

### 10.7 `finally` Guarantees

The `finally` block **always** executes, regardless of how the `try` block exits:

| try exit | finally behavior |
|----------|-----------------|
| Normal completion | Executes after try |
| Exception thrown | Executes before exception propagates |
| `return` statement | Executes before return |
| `break` / `continue` | Executes before control transfer |

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

### 10.8 Exception Suppression

If `finally` throws while another exception is propagating, the `finally` exception replaces the original:

```ruyi
try {
  throw Error("original");
} finally {
  throw Error("finally");    // this replaces "original"
}
// Caught exception: "finally"
```

### 10.9 Zero-Cost Exceptions

Ruyi exceptions use zero-cost exception tables. When no exception is thrown, there is no runtime overhead. The cost is only paid when an exception is actually thrown.

### Common Pitfalls

- **No checked exceptions**: Ruyi does not require functions to declare which exceptions they throw.
- **Finally can suppress exceptions**: If `finally` throws, it replaces the original exception.
- **Catch order matters**: Put specific exception types before general ones.
- **`never` is useful**: Functions that always throw should return `never` to help the type checker.

---

## 11. Async Programming

### 11.1 `async` Functions

Declare asynchronous functions with `async`:

```ruyi
async fn fetchData(url: string): string {
  let response = await http.get(url);
  return response.body;
}
```

An `async` function returns a `Future<T>`:

```ruyi
let future: Future<string> = fetchData("https://example.com");
let result: string = await future;
```

### 11.2 `await` Expression

The `await` operator suspends the current async function until the future completes:

```ruyi
async fn loadAll(urls: Array<string>): Array<string> {
  let results = [];
  for (let url of urls) {
    results.push(await fetchData(url));
  }
  return results;
}
```

`await` can only be used inside `async` functions.

### 11.3 Concurrent Execution

To run multiple futures concurrently, spawn them and await all:

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

### 11.4 Async Arrow Functions

```ruyi
let fetch = async (url) => await http.get(url);
let process = async (data) => {
  let result = await transform(data);
  return result;
};
```

### 11.5 Async Iterators

Async iterators produce values asynchronously:

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

The `for await` loop desugars to:

```ruyi
let iter = readLines(file);
while let Some(line) = await iter.next() {
  print(line);
}
```

### 11.6 Green Thread Scheduler

Ruyi uses a work-stealing scheduler for green threads:

- **Workers**: OS threads that execute green threads.
- **Task queue**: Each worker has a local deque of ready futures.
- **Work stealing**: When a worker's queue is empty, it steals tasks from another worker.

This provides efficient concurrency with minimal overhead.

### 11.7 Blocking Operations

Blocking operations must not be called from green threads:

```ruyi
// Wrong: blocks the worker
let data = fs.readFileSync("file.txt");

// Correct: async I/O
let data = await fs.readFile("file.txt");

// Or: offload to blocking thread pool
let data = await spawn_blocking(|| fs.readFileSync("file.txt"));
```

### 11.8 Async and Exceptions

Exceptions in async functions propagate through the Future:

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

If an async function throws, the `Future` completes with an error state. `await` on an errored future re-throws the exception.

### Common Pitfalls

- **`await` only in `async`**: You cannot use `await` in a non-async function.
- **Futures are lazy**: A future does not start executing until awaited or spawned.
- **Do not block workers**: Synchronous I/O blocks the entire worker thread. Use async I/O or `spawn_blocking`.
- **Exceptions cross await boundaries**: `await` re-throws exceptions from failed futures.

---

## 12. Modules

### 12.1 Module Structure

Each `.ry` source file is a module. Modules are organized hierarchically based on the file system:

```
src/
  main.ry              -> module main
  utils.ry             -> module utils
  http/
    client.ry          -> module http::client
    server.ry          -> module http::server
```

### 12.2 Import Declarations

#### Named Imports

```ruyi
import { add, subtract } from "./math";

let sum = add(3, 5);
let diff = subtract(10, 3);
```

#### Renamed Imports

```ruyi
import { add as plus } from "./math";

let sum = plus(3, 5);
```

#### Namespace Imports

```ruyi
import * as utils from "./utils";

utils.formatDate(now());
utils.parseNumber("42");
```

#### Default Imports

```ruyi
import HttpClient from "./http";

let client = HttpClient.new();
```

#### Combined Imports

```ruyi
import HttpClient, { Request, Response } from "./http";
```

#### Side-Effect Imports

```ruyi
import "./setup";    // runs module initialization, no imports
```

### 12.3 Export Declarations

By default, all top-level declarations are **private**. Use `export` to make them public:

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

#### Named Exports

```ruyi
export { add, subtract };
export { add as plus };
```

#### Re-exports

```ruyi
export * from "./math";
export { add, subtract } from "./math";
```

#### Default Exports

```ruyi
export default class App {
  fn run() { ... }
}

export default fn main() {
  print("Hello!");
}
```

### 12.4 Import Resolution

Import paths are resolved as follows:

1. **Relative paths** (`./` or `../`): Resolved relative to the importing file's directory.
2. **Absolute paths** (no prefix): Resolved from the project's source root.
3. **Standard library paths** (`std::`): Resolved from the standard library.

Resolution tries both `<path>.ry` and `<path>/index.ry`:

```ruyi
import { foo } from "./math";
// Tries: ./math.ry, then ./math/index.ry
```

### 12.5 Circular Dependency Detection

Ruyi detects circular dependencies at compile time:

```ruyi
// a.ry
import { foo } from "./b";

// b.ry
import { bar } from "./a";    // ERROR: circular dependency
```

To resolve circular dependencies:

- Extract shared code into a third module.
- Use forward declarations for types only.
- Restructure the module hierarchy.

### 12.6 Module Initialization

When a module is first imported, its top-level statements execute in order:

```ruyi
// config.ry
export let config = loadConfig();    // executes on first import
```

Each module is initialized exactly once. Initialization order follows the dependency graph.

### 12.7 Name Resolution and Shadowing

Names are resolved in this order:

1. Local scope (current block)
2. Function scope (parameters and local variables)
3. Module scope (top-level declarations)
4. Imported names
5. Built-in names (`int`, `string`, `null`, etc.)

Inner scopes can shadow outer scope names:

```ruyi
let x = 1;           // module-level x

fn example() {
  let x = 2;         // shadows module-level x
  print(x);          // prints 2
}
```

Shadowing imported names generates a warning.

### Common Pitfalls

- **Default is private**: Declarations are private unless explicitly exported.
- **No circular imports**: The compiler detects and rejects circular dependencies.
- **Relative paths are relative to the file**: `./math` in `src/utils/helper.ry` resolves to `src/utils/math.ry`.
- **Modules initialize once**: Top-level code runs only on first import.

---

## Appendix A: Quick Reference

### Keywords

```
let         const       fn          class
trait       match       if          else
for         while       return      throw
try         catch       finally     async
await       import      export      macro
type        true        false       null
self        super       this
```

### Operator Precedence (highest to lowest)

| Precedence | Operators | Associativity |
|------------|-----------|---------------|
| 18 | `.` `?.` `()` `[]` | Left |
| 17 | `++` `--` `!` `~` `+` `-` `await` | Right |
| 16 | `**` | Right |
| 15 | `*` `/` `%` | Left |
| 14 | `+` `-` | Left |
| 13 | `<<` `>>` `>>>` | Left |
| 12 | `<` `>` `<=` `>=` | Left |
| 11 | `===` `!==` | Left |
| 10 | `&` | Left |
| 9 | `^` | Left |
| 8 | `\|` | Left |
| 7 | `&&` | Left |
| 6 | `\|\|` | Left |
| 5 | `??` | Left |
| 4 | `?:` | Right |
| 3 | `=>` | Right |
| 2 | `=` `+=` `-=` etc. | Right |
| 1 | `,` | Left |

### Built-in Types

| Type | Description |
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

## Appendix B: JavaScript to Ruyi Migration Guide

| JavaScript | Ruyi | Notes |
|------------|-------|-------|
| `var x` | `let x` | Block-scoped, not function-scoped |
| `undefined` | `null` | Single null-like value |
| `==` / `!=` | `===` / `!==` | Strict equality only |
| `function() {}` | `fn() {}` | Shorter keyword |
| `this` in methods | `self` | Explicit, no binding confusion |
| `arguments` | `...args` | Rest parameters |
| `prototype` | `class` / `trait` | Class-based inheritance |
| `with` | (removed) | No equivalent |
| `eval()` | (removed) | No equivalent |
| `delete obj.prop` | `obj.prop = null` | No property deletion |
| `function*` | `async fn*` | Async generators only |
| `typeof null` | `"null"` | Fixed the JS bug |

---

## Appendix C: Built-in Functions and Standard Library

Ruyi provides several layers of built-in functionality. Some functions are available without any import, while others require explicit module imports.

### C.1 Compiler Built-in Functions (No Import Required)

These functions are **hard-coded into the compiler** and work in every Ruyi program without any `import` statement. They are handled specially during code generation.

#### `print(value)`

Prints a value to stdout followed by a newline. Supports all primitive types and arrays.

```ruyi
print(42);              // "42\n"
print(3.14);            // "3.140000\n"
print("hello");         // "hello\n"
print([1, 2, 3]);       // "[1, 2, 3]\n"
```

| Type | Format |
|------|--------|
| `int` | `%ld` (signed 64-bit) |
| `float` | `%f` (floating point) |
| `string` | `%s` (C string) |
| `Array<T>` | `[elem1, elem2, ...]` |
| Other | `<unknown>` |

#### `spawn(fn)`

Spawns a green thread (lightweight concurrent task) on the work-stealing scheduler. Returns a task handle.

```ruyi
let task = spawn(() => {
  // runs concurrently
  doHeavyWork();
});
```

---

### C.2 Core Module (Auto-Available)

The `core.ry` module is **automatically available** to all Ruyi programs. It provides methods on primitive types that map to compiler intrinsics (`__builtin_*`).

#### String Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `length` | `fn length(self: string): int` | Returns character count |
| `slice` | `fn slice(self: string, start: int, end: int): string` | Extracts substring [start, end) |
| `find` | `fn find(self: string, substr: string): int` | Returns index of first occurrence, or -1 |
| `replace` | `fn replace(self: string, from: string, to: string): string` | Replaces first occurrence |
| `toUpperCase` | `fn toUpperCase(self: string): string` | Converts to uppercase |
| `toLowerCase` | `fn toLowerCase(self: string): string` | Converts to lowercase |
| `trim` | `fn trim(self: string): string` | Removes leading/trailing whitespace |
| `contains` | `fn contains(self: string, substr: string): bool` | Checks if substring exists |
| `startsWith` | `fn startsWith(self: string, prefix: string): bool` | Checks prefix |
| `endsWith` | `fn endsWith(self: string, suffix: string): bool` | Checks suffix |
| `split` | `fn split(self: string, delimiter: string): Array<string>` | Splits by delimiter |

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

#### Int Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `toString` | `fn toString(self: int): string` | Converts to string |
| `abs` | `fn abs(self: int): int` | Absolute value |
| `min` | `fn min(self: int, other: int): int` | Minimum of two integers |
| `max` | `fn max(self: int, other: int): int` | Maximum of two integers |

```ruyi
let n = -42;
n.toString();   // "-42"
n.abs();        // 42
3.min(5);       // 3
3.max(5);       // 5
```

#### Float Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `toString` | `fn toString(self: float): string` | Converts to string |
| `abs` | `fn abs(self: float): float` | Absolute value |
| `min` | `fn min(self: float, other: float): float` | Minimum of two floats |
| `max` | `fn max(self: float, other: float): float` | Maximum of two floats |
| `round` | `fn round(self: float): int` | Rounds to nearest integer |
| `floor` | `fn floor(self: float): int` | Rounds down |
| `ceil` | `fn ceil(self: float): int` | Rounds up |

```ruyi
let f = 3.7;
f.toString();   // "3.7"
f.abs();        // 3.7
f.round();      // 4
f.floor();      // 3
f.ceil();       // 4
```

#### Bool Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `toString` | `fn toString(self: bool): string` | Converts to "true" or "false" |

```ruyi
true.toString();    // "true"
false.toString();   // "false"
```

---

### C.3 Standard Library Modules (Require Import)

These modules must be explicitly imported using `import { ... } from "std::module"`.

#### IO Module (`std::io`)

Console and file I/O operations.

```ruyi
import { readLine } from "std::io";
```

| Function | Signature | Description |
|----------|-----------|-------------|
| `readLine` | `fn readLine(): string?` | Read line from stdin |

**File System (`std::fs`):**

File system operations have moved to the `fs` module. Import from `"std::fs"`:

```ruyi
import { readFile, writeFile, exists, mkdir } from "std::fs";
```

Key functions (all with `*Async` variants):

| Function | Signature | Description |
|----------|-----------|-------------|
| `readFile` | `fn readFile(path: string): string` | Read entire file |
| `writeFile` | `fn writeFile(path: string, content: string): void` | Write string to file |
| `exists` | `fn exists(path: string): bool` | Check if path exists |
| `isFile` | `fn isFile(path: string): bool` | Check if regular file |
| `isDir` | `fn isDir(path: string): bool` | Check if directory |
| `mkdir` | `fn mkdir(path: string, recursive: bool = false): void` | Create directory |
| `deleteFile` | `fn deleteFile(path: string): void` | Delete file |


#### Collections Module (`std::collections`)

Generic collection types: `Array<T>`, `Map<K, V>`, `Set<T>`, and `Iterator<T>`.

**Array<T> Methods:**

| Method | Signature | Description |
|--------|-----------|-------------|
| `get` | `fn get(self, index: int): T` | Get element at index |
| `set` | `fn set(self, index: int, value: T): void` | Set element at index |
| `push` | `fn push(self, value: T): void` | Add element to end |
| `pop` | `fn pop(self): T` | Remove and return last element |
| `map` | `fn map<U>(self, f: fn(T): U): Array<U>` | Transform elements |
| `filter` | `fn filter(self, pred: fn(T): bool): Array<T>` | Filter elements |
| `reduce` | `fn reduce<U>(self, init: U, f: fn(U, T): U): U` | Reduce to single value |
| `forEach` | `fn forEach(self, f: fn(T): void): void` | Apply function to each |
| `iter` | `fn iter(self): ArrayIterator<T>` | Create iterator |

**Map<K, V> Methods:**

| Method | Signature | Description |
|--------|-----------|-------------|
| `get` | `fn get(self, key: K): Option<V>` | Get value by key |
| `set` | `fn set(self, key: K, value: V): void` | Set key-value pair |
| `delete` | `fn delete(self, key: K): bool` | Remove entry |
| `has` | `fn has(self, key: K): bool` | Check if key exists |
| `keys` | `fn keys(self): Array<K>` | Get all keys |
| `values` | `fn values(self): Array<V>` | Get all values |
| `entries` | `fn entries(self): Array<[K, V]>` | Get all key-value pairs |

**Set<T> Methods:**

| Method | Signature | Description |
|--------|-----------|-------------|
| `add` | `fn add(self, value: T): void` | Add element |
| `delete` | `fn delete(self, value: T): bool` | Remove element |
| `has` | `fn has(self, value: T): bool` | Check if element exists |
| `union` | `fn union(self, other: Set<T>): Set<T>` | Set union |
| `intersection` | `fn intersection(self, other: Set<T>): Set<T>` | Set intersection |
| `difference` | `fn difference(self, other: Set<T>): Set<T>` | Set difference |

**Iterator<T> Trait:**

| Method | Signature | Description |
|--------|-----------|-------------|
| `next` | `fn next(self): Option<T>` | Get next element |
| `forEach` | `fn forEach(self, f: fn(T): void): void` | Apply function to each |
| `map` | `fn map<U>(self, f: fn(T): U): Iterator<U>` | Transform elements |
| `filter` | `fn filter(self, pred: fn(T): bool): Iterator<T>` | Filter elements |
| `reduce` | `fn reduce<U>(self, init: U, f: fn(U, T): U): U` | Reduce to single value |

#### Option Type (`std::option`)

```ruyi
enum Option<T> {
    Some(T),
    None
}
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `isSome` | `fn isSome<T>(self: Option<T>): bool` | Check if Some |
| `isNone` | `fn isNone<T>(self: Option<T>): bool` | Check if None |
| `unwrap` | `fn unwrap<T>(self: Option<T>): T` | Get value (panics if None) |
| `unwrapOr` | `fn unwrapOr<T>(self: Option<T>, default: T): T` | Get value or default |
| `unwrapOrElse` | `fn unwrapOrElse<T>(self: Option<T>, f: fn(): T): T` | Get value or compute default |
| `map` | `fn map<T, U>(self: Option<T>, f: fn(T): U): Option<U>` | Transform contained value |
| `andThen` | `fn andThen<T, U>(self: Option<T>, f: fn(T): Option<U>): Option<U>` | Chain computations |
| `filter` | `fn filter<T>(self: Option<T>, pred: fn(T): bool): Option<T>` | Filter by predicate |
| `flatten` | `fn flatten<T>(self: Option<Option<T>>): Option<T>` | Flatten nested Options |
| `okOr` | `fn okOr<T, E>(self: Option<T>, err: E): Result<T, E>` | Convert to Result |
| `okOrElse` | `fn okOrElse<T, E>(self: Option<T>, f: fn(): E): Result<T, E>` | Convert to Result (computed error) |
| `forEach` | `fn forEach<T>(self: Option<T>, f: fn(T): void): void` | Apply function if Some |
| `toString` | `fn toString<T>(self: Option<T>): string` | String representation |

#### Result Type (`std::result`)

```ruyi
enum Result<T, E> {
    Ok(T),
    Err(E)
}
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `isOk` | `fn isOk<T, E>(self: Result<T, E>): bool` | Check if Ok |
| `isErr` | `fn isErr<T, E>(self: Result<T, E>): bool` | Check if Err |
| `unwrap` | `fn unwrap<T, E>(self: Result<T, E>): T` | Get value (panics if Err) |
| `unwrapOr` | `fn unwrapOr<T, E>(self: Result<T, E>, default: T): T` | Get value or default |
| `unwrapOrElse` | `fn unwrapOrElse<T, E>(self: Result<T, E>, f: fn(E): T): T` | Get value or compute default |
| `map` | `fn map<T, U, E>(self: Result<T, E>, f: fn(T): U): Result<U, E>` | Transform Ok value |
| `mapErr` | `fn mapErr<T, E, F>(self: Result<T, E>, f: fn(E): F): Result<T, F>` | Transform Err value |
| `andThen` | `fn andThen<T, U, E>(self: Result<T, E>, f: fn(T): Result<U, E>): Result<U, E>` | Chain computations |
| `filter` | `fn filter<T, E>(self: Result<T, E>, pred: fn(T): bool): Result<T, E>` | Filter by predicate |
| `ok` | `fn ok<T, E>(self: Result<T, E>): Option<T>` | Convert to Option |
| `err` | `fn err<T, E>(self: Result<T, E>): Option<E>` | Get Err as Option |
| `forEach` | `fn forEach<T, E>(self: Result<T, E>, f: fn(T): void): void` | Apply function if Ok |
| `toOption` | `fn toOption<T, E>(self: Result<T, E>): Option<T>` | Convert to Option |
| `toBool` | `fn toBool<T, E>(self: Result<T, E>): bool` | Convert to bool |
| `toString` | `fn toString<T, E>(self: Result<T, E>): string` | String representation |

#### Error Types (`std::error`)

Error hierarchy and utility functions.

**Error Classes:**

| Class | Description |
|-------|-------------|
| `Error` | Base error class with message and cause chain |
| `TypeError` | Type check or conversion failures |
| `RuntimeError` | Runtime operation failures |
| `RangeError` | Index out of bounds |
| `AssertionError` | Assertion failures |
| `ArgumentError` | Invalid argument values |
| `NullError` | Null value where value required |
| `ArithmeticError` | Division by zero, etc. |
| `IteratorError` | Iterator-related issues |
| `ParseError` | Parsing failures |

**Utility Functions:**

| Function | Signature | Description |
|----------|-----------|-------------|
| `isError` | `fn isError(value: dynamic): bool` | Check if value is Error |
| `assert` | `fn assert(condition: bool, message: string): void` | Assert condition |
| `assertNotNull` | `fn assertNotNull<T>(value: T, message: string): void` | Assert non-null |
| `errorWithCause` | `fn errorWithCause(message: string, cause: Error): Error` | Create error with cause |

#### String Module (`std::string`)

Standalone string utility functions. String instance methods (`split`, `contains`, `trim`, etc.) are already available via the `core` module (auto-loaded).

| Function | Signature | Description |
|----------|-----------|-------------|
| `join` | `fn join(array: Array<dyn>, separator: string = ""): string` | Join array elements |
| `fromCharCode` | `fn fromCharCode(code: int): string` | Create from char code |
| `fromCharCodes` | `fn fromCharCodes(codes: Array<int>): string` | Create from char codes |
| `concat` | `fn concat(args: ...string): string` | Concatenate strings |
| `template` | `fn template(template: string, values: Array<dyn>): string` | Format template |
| `processTemplate` | `fn processTemplate(parts: Array<dyn>, context: dyn): string` | Process template literals |

#### Path Module (`std::path`)

File system path manipulation utilities.

| Method | Signature | Description |
|--------|-----------|-------------|
| `Path.join` | `static fn join(paths: ...string): string` | Join path segments |
| `Path.basename` | `static fn basename(path: string): string` | Get file name |
| `Path.basenameNoExt` | `static fn basenameNoExt(path: string): string` | Get file name without extension |
| `Path.dirname` | `static fn dirname(path: string): string` | Get directory name |
| `Path.extname` | `static fn extname(path: string): string` | Get file extension |
| `Path.isAbsolute` | `static fn isAbsolute(path: string): bool` | Check if absolute |
| `Path.isRelative` | `static fn isRelative(path: string): bool` | Check if relative |
| `Path.resolve` | `static fn resolve(base: string, relative: string): string` | Resolve relative path |
| `Path.normalize` | `static fn normalize(path: string): string` | Normalize path |
| `Path.withoutExt` | `static fn withoutExt(path: string): string` | Remove extension |
| `Path.changeExt` | `static fn changeExt(path: string, newExt: string): string` | Change extension |
| `Path.compare` | `static fn compare(path1: string, path2: string): int` | Compare paths |
| `Path.equals` | `static fn equals(path1: string, path2: string): bool` | Check path equality |
| `Path.parents` | `static fn parents(path: string): Array<string>` | Get parent directories |
| `Path.isChildOf` | `static fn isChildOf(parent: string, child: string): bool` | Check if child path |
| `Path.relative` | `static fn relative(from: string, to: string): string` | Get relative path |
| `Path.separator` | `static fn separator(): string` | Get platform separator |

#### Process Module (`std::process`)

Process management and system command execution.

| Function | Signature | Description |
|----------|-----------|-------------|
| `Process.create` | `static fn create(command: string, options: ProcessOptions?): Process` | Create process |
| `Process.exec` | `static fn exec(command: string): ProcessResult` | Execute command |
| `Process.execWith` | `static fn execWith(command: string, options: ExecOptions): ProcessResult` | Execute with options |
| `Process.spawn` | `static fn spawn(command: string, args: Array<string>?): Process` | Spawn child process |
| `Process.spawnWith` | `static fn spawnWith(command: string, args: Array<string>?, options: ProcessOptions): Process` | Spawn with options |
| `getEnv` | `fn getEnv(name: string): string?` | Get environment variable |
| `setEnv` | `fn setEnv(name: string, value: string): void` | Set environment variable |
| `getAllEnv` | `fn getAllEnv(): Map<string, string>` | Get all environment variables |
| `getPID` | `fn getPID(): int` | Get current process ID |
| `getPPID` | `fn getPPID(): int` | Get parent process ID |
| `getPlatform` | `fn getPlatform(): string` | Get platform ("linux", "macos", "windows") |
| `getCPUCount` | `fn getCPUCount(): int` | Get CPU core count |
| `getTotalMemory` | `fn getTotalMemory(): int` | Get total memory in bytes |
| `getFreeMemory` | `fn getFreeMemory(): int` | Get free memory in bytes |

---

### C.4 Runtime Internal Functions

These functions are declared in the LLVM module at compile time and used internally by the compiler's code generation. **Users should not call these directly** — they are invoked automatically by the compiler.

#### Garbage Collection

| Runtime Function | Description |
|------------------|-------------|
| `ruyi_gc_alloc(size)` | Allocate memory on the GC heap |
| `ruyi_gc_collect()` | Trigger garbage collection |
| `ruyi_gc_add_root(ptr)` | Add a GC root |
| `ruyi_gc_remove_root(ptr)` | Remove a GC root |
| `ruyi_gc_write_barrier(parent, field)` | Write barrier for generational GC |

#### Exception Handling

| Runtime Function | Description |
|------------------|-------------|
| `ruyi_throw(exception)` | Throw an exception |
| `ruyi_get_pending_exception()` | Get the pending exception |
| `ruyi_clear_pending_exception()` | Clear the pending exception |
| `ruyi_begin_catch(exception)` | Enter a catch block |
| `ruyi_end_catch()` | Exit a catch block |

#### String Operations

| Runtime Function | Description |
|------------------|-------------|
| `ruyi_str_concat(a, b)` | Concatenate two strings |

#### Async/Scheduler

| Runtime Function | Description |
|------------------|-------------|
| `ruyi_async_poll(future, waker)` | Poll an async future |
| `ruyi_spawn(future)` | Spawn a task on the scheduler |
| `ruyi_wake_task(task)` | Wake a sleeping task |
| `ruyi_run_scheduler()` | Run the work-stealing scheduler |

---

## Appendix D: Complete Example - A Simple CLI Tool

Here is a complete Ruyi program that demonstrates multiple features working together:

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

This example demonstrates:

- **Traits**: `Display` trait with implementations for `int` and `string`
- **Generics**: `Stack<T>` with trait bound `T: Display`
- **Pattern matching**: `match` on `Option<T>` result
- **Null safety**: `??` operator for default values
- **Async/await**: `async fn` for asynchronous operations
- **Error handling**: `try/catch` for exception handling
- **Control flow**: `for`, `while`, `if` statements
- **Classes**: `Stack` class with methods and fields

---

*End of Ruyi Language Tutorial*
