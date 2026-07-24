# Ruyi 语言规范

## 词法与语法规范

> **版本**: 0.5.1-draft
> **日期**: 2026-05-05
> **状态**: Working Draft — 与当前实现对齐

---

## 目录

1. [简介](#1-简介)
2. [词法结构](#2-词法结构)
   - 2.1 [源文本](#21-源文本)
   - 2.2 [注释](#22-注释)
   - 2.3 [词法单元](#23-词法单元)
   - 2.4 [关键字](#24-关键字)
   - 2.5 [标识符](#25-标识符)
   - 2.6 [字面量](#26-字面量)
   - 2.7 [运算符与分隔符](#27-运算符与分隔符)
   - 2.8 [空白与行终止符](#28-空白与行终止符)
3. [语法规则](#3-语法规则)
   - 3.1 [记号说明](#31-记号说明)
   - 3.2 [源文件](#32-源文件)
   - 3.3 [声明](#33-声明)
   - 3.4 [语句](#34-语句)
   - 3.5 [表达式](#35-表达式)
   - 3.6 [模式](#36-模式)
   - 3.7 [类型注解](#37-类型注解)
   - 3.8 [模块](#38-模块)
4. [模式匹配](#4-模式匹配)
5. [泛型](#5-泛型)
6. [空值安全](#6-空值安全)
7. [移除的 JavaScript 特性](#7-移除的-javascript-特性)
8. [类型系统语义](#8-类型系统语义)
9. [可空类型语义](#9-可空类型语义)
10. [泛型语义](#10-泛型语义)
11. [特征 (Trait) 语义](#11-特征-trait-语义)
12. [内存模型](#12-内存模型)
13. [异常语义](#13-异常语义)
14. [异步/Await 语义](#14-异步await-语义)
15. [模块语义](#15-模块语义)
16. [宏语义](#16-宏语义)

---

## 1. 简介

Ruyi 是一种编译型的通用编程语言，其语法基础建立在 JavaScript 严格模式之上。它移除了 JavaScript 中问题重重的特性，同时保留了熟悉的语法风格。Ruyi 通过 LLVM 编译为原生机器码，在各平台上提供高性能运行。

本文档定义了 Ruyi 的词法结构与语法规则。它采用 ECMAScript 风格的规范格式，并使用 BNF 文法记号。

---

## 2. 词法结构

### 2.1 源文本

Ruyi 源文本是由 Unicode 码点组成的序列，采用 UTF-8 编码。源文本从左到右扫描，将码点序列转换为词法单元。

### 2.2 注释

注释在词法分析阶段被视为空白字符，不产生词法单元。

```
Comment ::
  SingleLineComment
  MultiLineComment
  DocumentationComment

SingleLineComment ::
  // SingleLineCommentCharsopt

MultiLineComment ::
  /* MultiLineCommentCharsopt */

DocumentationComment ::
  /** DocumentationCommentCharsopt */
```

单行注释在第一个行终止符处结束。多行注释可以跨越多行。文档注释会被工具保留，但不具备语法意义。

### 2.3 词法单元

输入流被转换为一个词法单元序列。每个词法单元是能够构成有效词法单元的最长码点序列。

```
Token ::
  Keyword
  Identifier
  Literal
  OperatorOrPunctuator

InputElement ::
  Token
  Comment
  WhiteSpace
  LineTerminator
```

### 2.4 关键字

关键字是具有特殊语法意义的保留标识符，不能用作普通标识符。

```
Keyword :: one of
  let         const       fn          class
  trait       impl        match       if
  else        for         while       return
  throw       try         catch       finally
  async       await       import      export
  macro       type        true        false
  null        self        super       this
  in          instanceof  typeof      void
  delete      as          extends     dyn
  static      get         set         new
  of          break       continue    yield
  _
```

**关键字说明**:

| 关键字 | 用途 |
|---------|---------|
| `let` | 可变变量声明 |
| `const` | 不可变变量声明 |
| `fn` | 函数声明 |
| `class` | 类声明 |
| `trait` | 特征 (Trait) 声明 |
| `impl` | 特征实现块 |
| `match` | 模式匹配表达式 |
| `if` | 条件语句/表达式 |
| `else` | 条件分支的替代分支 |
| `for` | 循环语句 |
| `while` | 条件循环语句 |
| `return` | 从函数返回 |
| `throw` | 抛出异常 |
| `try` | 开始异常处理块 |
| `catch` | 捕获异常 |
| `finally` | 执行清理代码 |
| `async` | 声明异步函数 |
| `await` | 等待异步结果 |
| `import` | 从模块导入 |
| `export` | 从模块导出 |
| `macro` | 声明宏 |
| `type` | 类型别名声明 |
| `true` | 布尔真字面量 |
| `false` | 布尔假字面量 |
| `null` | 空值字面量 |
| `self` | 引用当前实例（在方法中） |
| `super` | 引用父类 |
| `this` | 引用当前上下文 |
| `in` | 键成员检查 / for-in 循环 |
| `instanceof` | 类型检查运算符 |
| `typeof` | 运行时类型检查运算符 |
| `void` | void 表达式运算符 |
| `delete` | 属性删除运算符（已解析，代码生成有限） |
| `as` | 类型转换 / 模式别名 |
| `extends` | 类继承 / 特征超特征 |
| `dyn` | 动态分发 / 特征对象 |
| `static` | 静态类成员 |
| `get` | getter 方法定义 |
| `set` | setter 方法定义 |
| `new` | 对象实例化 |
| `of` | 值迭代 / for-of 循环 |
| `break` | 退出封闭循环 |
| `continue` | 跳过到下一次循环迭代 |
| `yield` | 生成器 yield（已解析；代码生成为 no-op） |
| `_` | 通配符模式（match/解构） |

### 2.5 标识符

标识符用于命名变量、函数、类型及其他程序实体。

```
Identifier ::
  IdentifierStart IdentifierPartsopt

IdentifierStart ::
  UnicodeIDStart
  _
  $

IdentifierParts ::
  IdentifierPart
  IdentifierParts IdentifierPart

IdentifierPart ::
  UnicodeIDContinue
  _
  $
  UnicodeCombiningMark
  UnicodeDigit
```

标识符区分大小写。`myVar` 和 `myvar` 是不同的标识符。标识符不得与任何关键字匹配。

**示例**:
```
x
count
_myVar
$element
firstName
camelCaseName
```

### 2.6 字面量

字面量表示源代码中的固定值。

```
Literal ::
  NullLiteral
  BooleanLiteral
  NumericLiteral
  StringLiteral
  TemplateLiteral
  ArrayLiteral
  ObjectLiteral
```

#### 2.6.1 空值字面量

```
NullLiteral ::
  null
```

`null` 字面量表示值的缺失。Ruyi 没有 `undefined`。

#### 2.6.2 布尔字面量

```
BooleanLiteral :: one of
  true  false
```

#### 2.6.3 数字字面量

```
NumericLiteral ::
  DecimalLiteral
  HexIntegerLiteral
  OctalIntegerLiteral
  BinaryIntegerLiteral
  BigIntLiteral

DecimalLiteral ::
  DecimalIntegerLiteral . DecimalDigitsopt ExponentPartopt
  . DecimalDigits ExponentPartopt
  DecimalIntegerLiteral ExponentPartopt

DecimalIntegerLiteral ::
  DecimalDigit
  NonZeroDigit DecimalDigitsopt

DecimalDigits ::
  DecimalDigit
  DecimalDigits DecimalDigit

DecimalDigit :: one of
  0 1 2 3 4 5 6 7 8 9

NonZeroDigit :: one of
  1 2 3 4 5 6 7 8 9

ExponentPart ::
  ExponentIndicator SignedInteger

ExponentIndicator :: one of
  e E

SignedInteger ::
  DecimalDigits
  + DecimalDigits
  - DecimalDigits

HexIntegerLiteral ::
  0x HexDigit+
  0X HexDigit+

HexDigit :: one of
  0 1 2 3 4 5 6 7 8 9 a b c d e f A B C D E F

OctalIntegerLiteral ::
  0o OctalDigit+
  0O OctalDigit+

OctalDigit :: one of
  0 1 2 3 4 5 6 7

BinaryIntegerLiteral ::
  0b BinaryDigit+
  0B BinaryDigit+

BinaryDigit :: one of
  0 1

BigIntLiteral ::
  DecimalLiteral n
  HexIntegerLiteral n
  OctalIntegerLiteral n
  BinaryIntegerLiteral n
```

**数字字面量示例**:
```
42          // 十进制整数
3.14        // 十进制浮点数
1e10        // 科学计数法
0xFF        // 十六进制 (255)
0o77        // 八进制 (63)
0b1010      // 二进制 (10)
100n        // 大整数
```

#### 2.6.4 字符串字面量

```
StringLiteral ::
  " DoubleStringCharsopt "
  ' SingleStringCharsopt '

DoubleStringChars ::
  DoubleStringChar
  DoubleStringChars DoubleStringChar

DoubleStringChar ::
  SourceCharacter but not " or \ or LineTerminator
  EscapeSequence

SingleStringChars ::
  SingleStringChar
  SingleStringChars SingleStringChar

SingleStringChar ::
  SourceCharacter but not ' or \ or LineTerminator
  EscapeSequence

EscapeSequence ::
  \ EscapeCharacter
  \ HexEscapeSequence
  \ UnicodeEscapeSequence

EscapeCharacter :: one of
  " ' \ b f n r t v 0

HexEscapeSequence ::
  x HexDigit HexDigit

UnicodeEscapeSequence ::
  u{ HexDigit+ }
  u HexDigit HexDigit HexDigit HexDigit
```

**字符串字面量示例**:
```
"hello"
'world'
"line1\nline2"
"tab\there"
"unicode: \u{1F600}"
```

#### 2.6.5 模板字面量

```
TemplateLiteral ::
  ` TemplateCharsopt `
  ` TemplateHead Expression TemplateSpans

TemplateSpans ::
  TemplateTail
  TemplateMiddle Expression TemplateSpans

TemplateHead ::
  ${ Expression } TemplateCharsopt

TemplateMiddle ::
  ${ Expression } TemplateCharsopt

TemplateTail ::
  } TemplateCharsopt `

TemplateChars ::
  TemplateChar
  TemplateChars TemplateChar

TemplateChar ::
  SourceCharacter but not ` or \ or $ or LineTerminator
  EscapeSequence
```

**模板字面量示例**:
```
`hello ${name}`
`result: ${x + y}`
`multi
line
string`
```

#### 2.6.6 数组字面量

```
ArrayLiteral ::
  [ ElementListopt ]

ElementList ::
  SpreadElement
  ElementList , SpreadElementopt

SpreadElement ::
  AssignmentExpression
  ... AssignmentExpression
```

#### 2.6.7 对象字面量

```
ObjectLiteral ::
  { PropertyListopt }

PropertyList ::
  PropertyDefinition
  PropertyList , PropertyDefinition

PropertyDefinition ::
  IdentifierName : AssignmentExpression
  IdentifierName
  ... AssignmentExpression
  [ Expression ] : AssignmentExpression
```

### 2.7 运算符与分隔符

运算符与分隔符是具有特殊语法意义的一个或多个码点序列。

```
OperatorOrPunctuator ::
  Operator
  Punctuator

Operator :: one of
  ===  !==  ==  !=  <  >  <=  >=
  +  -  *  /  %  **
  &  |  ^  ~  <<  >>  >>>
  &&  ||  ??
  !  ??  ?.
  =  +=  -=  *=  /=  %=  **=
  &=  |=  ^=  <<=  >>=  >>>=
  &&=  ||=  ??=
  =>
  ++  --
  in  instanceof
  typeof  void  delete
  yield

Punctuator :: one of
  {  }  (  )  [  ]
  .  ,  ;  :  ?
  @  #  ...  ::
  <  >  $
```

**运算符优先级**（从高到低）:

| 优先级 | 运算符 | 结合性 |
|------------|-----------|---------------|
| 18 | `?.` `.` `()` `[]` | 从左到右 |
| 17 | `++` `--` `!`（前缀）`~` `+`（一元）`-`（一元）`typeof` `void` `delete` `await` | 从右到左 |
| 16 | `**` | 从右到左 |
| 15 | `*` `/` `%` | 从左到右 |
| 14 | `+` `-` | 从左到右 |
| 13 | `<<` `>>` `>>>` | 从左到右 |
| 12 | `<` `>` `<=` `>=` `in` `instanceof` | 从左到右 |
| 11 | `===` `!==` `==` `!=` | 从左到右 |
| 10 | `&` | 从左到右 |
| 9 | `^` | 从左到右 |
| 8 | `\|` | 从左到右 |
| 7 | `&&` | 从左到右 |
| 6 | `\|\|` | 从左到右 |
| 5 | `??` | 从左到右 |
| 4 | `?:`（三元） | 从右到左 |
| 3 | `=>` | 从右到左 |
| 2 | `=` `+=` `-=` `*=` `/=` `%=` `**=` `&=` `\|=` `^=` `<<=` `>>=` `>>>=` `&&=` `\|\|=` `??=` | 从右到左 |
| 1 | `,` | 从左到右 |

**后缀运算符**（最高优先级，在上述所有运算符之后应用）:

| 运算符 | 说明 |
|----------|-------------|
| `!` | 非空断言：`e!` 断言 `e` 不为 null |
| `++` | 后自增（已解析；代码生成通过前缀实现） |
| `--` | 后自减（已解析；代码生成通过前缀实现） |

**Ruyi 关键运算符**:

| 运算符 | 名称 | 说明 |
|----------|------|-------------|
| `===` | 严格相等 | 值与类型均相等（无强制转换） |
| `!==` | 严格不等 | 严格相等的否定 |
| `==` | 遗留相等 | 已解析；代码生成映射到 `===` 行为 |
| `!=` | 遗留不等 | 已解析；代码生成映射到 `!==` 行为 |
| `?.` | 可选链 | 对可空值进行安全的属性访问 |
| `??` | 空值合并 | 若左操作数为 null，则返回右操作数 |
| `!`（后缀） | 非空断言 | 断言可空值不为 null；若为 null 则在运行时抛出 |
| `=>` | 箭头 | 定义箭头函数与 match 分支 |
| `...` | 展开/剩余 | 展开元素或收集剩余参数 |
| `**` | 幂运算 | 幂运算符 |

### 2.8 空白与行终止符

```
WhiteSpace ::
  <TAB>
  <VT>
  <FF>
  <SP>
  <NBSP>
  <BOM>
  <USP>

LineTerminator ::
  <LF>
  <CR>
  <LS>
  <PS>
  <CR><LF>
```

空白用于分隔词法单元，但本身不具备语义意义。行终止符影响自动分号插入。

---

## 3. 语法规则

### 3.1 记号说明

语法规则使用扩展巴科斯-瑙尔范式（EBNF）。约定如下：

- `::` 引入一条产生式规则
- `::=` 引入一条递归产生式
- `|` 分隔不同备选
- `[ ]` 标记可选元素
- `( )` 对元素进行分组
- `*` 表示零次或多次重复
- `+` 表示一次或多次重复
- `opt` 下标表示可选
- **粗体** 终结符为字面量词法单元
- *斜体* 非终结符引用其他产生式
- `one of` 列出单个词法单元的备选

### 3.2 源文件

```
SourceFile ::
  ModuleItemListopt

ModuleItemList ::
  ModuleItem
  ModuleItemList ModuleItem

ModuleItem ::
  ImportDeclaration
  ExportDeclaration
  StatementListItem

StatementListItem ::
  Declaration
  Statement
```

### 3.3 声明

#### 3.3.1 变量声明

```
Declaration ::
  LexicalDeclaration
  FunctionDeclaration
  ClassDeclaration
  TraitDeclaration
  ImplDeclaration
  TypeAliasDeclaration
  MacroDeclaration

LexicalDeclaration ::
  let BindingList ;
  const BindingList ;

BindingList ::
  Binding
  BindingList , Binding

Binding ::
  BindingPattern Initializeropt TypeAnnotationopt

BindingPattern ::
  Identifier
  ObjectBindingPattern
  ArrayBindingPattern

ObjectBindingPattern ::
  { BindingPropertyListopt }

BindingPropertyList ::
  BindingProperty
  BindingPropertyList , BindingProperty

BindingProperty ::
  IdentifierName : BindingPattern
  IdentifierName
  ... IdentifierName

ArrayBindingPattern ::
  [ BindingElementListopt ]

BindingElementList ::
  BindingElement
  BindingElementList , BindingElementopt

BindingElement ::
  BindingPattern Initializeropt

Initializer ::
  = AssignmentExpression
```

**示例**:
```ruyi
let x = 42;
const PI = 3.14159;
let name: string = "Ruyi";
let { first, last } = person;
let [head, ...tail] = list;
const { x: a, y: b } = point;
```

#### 3.3.2 函数声明

```
FunctionDeclaration ::
  fn BindingIdentifier TypeParametersopt ( FormalParameterListopt ) ReturnTypeAnnotationopt FunctionBody

FormalParameterList ::
  FormalParameter
  FormalParameterList , FormalParameter

FormalParameter ::
  BindingPattern TypeAnnotationopt
  ... IdentifierName TypeAnnotationopt
  BindingPattern TypeAnnotationopt = AssignmentExpression

ReturnTypeAnnotation ::
  : TypeAnnotation

FunctionBody ::
  { FunctionBodyStatementListopt }

FunctionBodyStatementList ::
  FunctionBodyStatement
  FunctionBodyStatementList FunctionBodyStatement

FunctionBodyStatement ::
  Statement
  Declaration
```

**示例**:
```ruyi
fn add(a: int, b: int): int {
  return a + b;
}

fn greet(name: string) {
  print("Hello, " + name);
}

fn identity<T>(x: T): T {
  return x;
}

fn max<T: Comparable>(a: T, b: T): T {
  return if a > b { a } else { b };
}
```

#### 3.3.3 箭头函数

```
ArrowFunction ::
  ArrowParameters => ConciseBody
  async ArrowParameters => ConciseBody

ArrowParameters ::
  BindingIdentifier
  ( FormalParameterListopt )

ConciseBody ::
  Expression
  { FunctionBodyStatementListopt }
```

**示例**:
```ruyi
let double = (x) => x * 2;
let greet = (name) => { print("Hi, " + name); };
let add = (a, b) => a + b;
let fetch = async (url) => await http.get(url);
```

#### 3.3.4 类声明

```
ClassDeclaration ::
  Annotations? class BindingIdentifier TypeParametersopt ClassHeritageopt { ClassBodyopt }

Annotations ::
  @ IdentifierName
  Annotations @ IdentifierName

ClassHeritage ::
  extends LeftHandSideExpression

ClassBody ::
  ClassElementListopt

ClassElementList ::
  ClassElement
  ClassElementList ClassElement

ClassElement ::
  MethodDefinition
  FieldDefinition
  static MethodDefinition
  static FieldDefinition
  ;

MethodDefinition ::
  PropertyName ( FormalParameterListopt ) ReturnTypeAnnotationopt { FunctionBodyStatementListopt }
  async PropertyName ( FormalParameterListopt ) ReturnTypeAnnotationopt { FunctionBodyStatementListopt }
  get PropertyName ( ) ReturnTypeAnnotationopt { FunctionBodyStatementListopt }
  set PropertyName ( FormalParameter ) { FunctionBodyStatementListopt }
  fn PropertyName TypeParametersopt ( FormalParameterListopt ) ReturnTypeAnnotationopt { FunctionBodyStatementListopt }

FieldDefinition ::
  PropertyName TypeAnnotationopt Initializeropt ;

PropertyName ::
  IdentifierName
  StringLiteral
  NumericLiteral
  [ Expression ]
```

**示例**:
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

  static fn origin(): Point {
    return Point.new(0.0, 0.0);
  }
}

class Circle extends Shape {
  radius: float;

  fn new(radius: float) {
    self.radius = radius;
  }
}
```

#### 3.3.5 特征 (Trait) 声明

```
TraitDeclaration ::
  trait BindingIdentifier TypeParametersopt { TraitBodyopt }

TraitBody ::
  TraitElementListopt

TraitElementList ::
  TraitElement
  TraitElementList TraitElement

TraitElement ::
  TraitMethodSignature
  TraitFieldSignature
  ;

TraitMethodSignature ::
  fn PropertyName ( FormalParameterListopt ) ReturnTypeAnnotationopt ;

TraitFieldSignature ::
  PropertyName : TypeAnnotation ;
```

**示例**:
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

#### 3.3.6 类型别名声明

```
TypeAliasDeclaration ::
  type BindingIdentifier TypeParametersopt = TypeAnnotation ;
```

**示例**:
```ruyi
type Result<T, E> = Ok<T, E> | Err<T, E>;
type Callback<T> = fn(T) -> void;
type Point2D = { x: float, y: float };
```

#### 3.3.7 宏声明

```
MacroDeclaration ::
  macro BindingIdentifier { MacroRules }

MacroRules ::
  MacroRule
  MacroRules MacroRule

MacroRule ::
  ( MacroPattern ) => { MacroBody }

MacroPattern ::
  MacroPatternToken
  MacroPattern MacroPatternToken
  $ IdentifierName
  $( MacroPattern ) Separatoropt Repeater

MacroBody ::
  MacroBodyToken
  MacroBody MacroBodyToken
  $ IdentifierName
  $( MacroBody ) Separatoropt Repeater

Repeater :: one of
  * + ?

Separator ::
  ,
  ;
```

**示例**:
```ruyi
macro debug {
  ($expr) => {
    print("DEBUG: " + stringify($expr) + " = " + $expr);
  }
}

macro vec {
  ($($elem),*) => {
    {
      let mut v = Array::new();
      $(v.push($elem);)*
      v
    }
  }
}
```

#### 3.3.8 特征实现声明

Impl 块提供特定类型的特征实现：

```
ImplDeclaration ::
  impl TypeParametersopt TraitName TypeArgs? for TypeAnnotation { ClassBodyopt }

TypeArgs ::
  < TypeList >
```

**示例**:
```ruyi
impl Printable for Point {
  fn format(self): string {
    return "(" + self.x + ", " + self.y + ")";
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

### 3.4 语句

```
Statement ::
  BlockStatement
  ExpressionStatement
  IfStatement
  IfLetStatement
  WhileStatement
  WhileLetStatement
  ForStatement
  ForInStatement
  ForOfStatement
  ReturnStatement
  ThrowStatement
  TryStatement
  MatchStatement
  BreakStatement
  ContinueStatement
  YieldStatement
  LabeledStatement
  EmptyStatement
```

#### 3.4.1 块语句

```
BlockStatement ::
  { StatementListopt }

StatementList ::
  StatementListItem
  StatementList StatementListItem
```

#### 3.4.2 表达式语句

```
ExpressionStatement ::
  Expression ;
```

表达式语句不得以 `{` 或 `fn` 开头，以避免与块语句和函数声明产生歧义。

#### 3.4.3 If 语句

```
IfStatement ::
  if ( Expression ) Statement ElseClauseopt

ElseClause ::
  else Statement
```

**示例**:
```ruyi
if (x > 0) {
  print("positive");
} else if (x < 0) {
  print("negative");
} else {
  print("zero");
}
```

#### 3.4.4 While 语句

```
WhileStatement ::
  while ( Expression ) Statement
```

**示例**:
```ruyi
while (i < 10) {
  print(i);
  i = i + 1;
}
```

#### 3.4.5 For 语句

```
ForStatement ::
  for ( ForInitializeropt ; Expressionopt ; ForUpdateopt ) Statement

ForInitializer ::
  LexicalDeclaration
  Expression

ForUpdate ::
  Expression
```

**示例**:
```ruyi
for (let i = 0; i < 10; i = i + 1) {
  print(i);
}

for (let i = items.length - 1; i >= 0; i = i - 1) {
  process(items[i]);
}
```

#### 3.4.6 For-In 语句

```
ForInStatement ::
  for ( let BindingIdentifier in Expression ) Statement
```

遍历对象的键或数组的索引。

**示例**:
```ruyi
for (let key in obj) {
  print(key + ": " + obj[key]);
}
```

#### 3.4.7 For-Of 语句

```
ForOfStatement ::
  for ( let BindingIdentifier of Expression ) Statement
  for ( let BindingIdentifier of async Expression ) Statement
```

遍历可迭代对象的值。`async` 形式用于遍历异步可迭代对象。

**示例**:
```ruyi
for (let item of items) {
  process(item);
}

for (let line of async readLines(file)) {
  print(line);
}
```

#### 3.4.8 Return 语句

```
ReturnStatement ::
  return Expressionopt ;
```

从当前函数返回。若未提供表达式，则返回 `null`。

**示例**:
```ruyi
return 42;
return;
return a + b;
```

#### 3.4.9 Throw 语句

```
ThrowStatement ::
  throw Expression ;
```

抛出异常。表达式必须求值为 `Error` 或其子类型。

**示例**:
```ruyi
throw Error("something went wrong");
throw TypeError("expected string");
```

#### 3.4.10 Try 语句

```
TryStatement ::
  try Block CatchClauseopt FinallyClauseopt

CatchClause ::
  catch ( BindingPattern TypeAnnotationopt ) Block
  catch Block

FinallyClause ::
  finally Block
```

**示例**:
```ruyi
try {
  let result = riskyOperation();
  print(result);
} catch (e: Error) {
  print("Error: " + e.message);
} finally {
  cleanup();
}

try {
  doSomething();
} catch {
  print("something failed");
}
```

#### 3.4.11 Match 语句

```
MatchStatement ::
  match ( Expression ) { MatchArmsopt }

MatchArms ::
  MatchArm
  MatchArms MatchArm

MatchArm ::
  Pattern MatchGuardopt => Block
  _ => Block

MatchGuard ::
  if ( Expression )
```

**示例**:
```ruyi
match (value) {
  1 => { print("one"); }
  2 => { print("two"); }
  n if (n > 10) => { print("big: " + n); }
  _ => { print("other"); }
}
```

#### 3.4.12 Break 与 Continue

```
BreakStatement ::
  break ;
  break IdentifierName ;

ContinueStatement ::
  continue ;
  continue IdentifierName ;
```

`break` 退出最内层封闭的循环。带标签时，退出标签语句。`continue` 跳过到最内层封闭循环的下一次迭代。

**示例**:
```ruyi
outer: for (let i = 0; i < 10; i = i + 1) {
  for (let j = 0; j < 10; j = j + 1) {
    if (j > 5) {
      break outer;
    }
  }
}
```

#### 3.4.13 Yield 语句

```
YieldStatement ::
  yield Expressionopt ;
```

`yield` 语句挂起生成器函数并产生一个值。当前作为语句解析；代码生成将其视为 no-op。

**示例**:
```ruyi
fn* countUp(limit: int) {
  for (let i = 0; i < limit; i = i + 1) {
    yield i;
  }
}
```

#### 3.4.14 标签语句

```
LabeledStatement ::
  IdentifierName : Statement
```

标记语句以供 `break` 和 `continue` 使用。

**示例**:
```ruyi
loop: while (true) {
  if (done) {
    break loop;
  }
}
```

#### 3.4.15 空语句

```
EmptyStatement ::
  ;
```

### 3.5 表达式

```
Expression ::
  AssignmentExpression
  Expression , AssignmentExpression

AssignmentExpression ::
  ConditionalExpression
  LeftHandSideExpression = AssignmentExpression
  LeftHandSideExpression AssignmentOperator AssignmentExpression
  ArrowFunction

AssignmentOperator :: one of
  *=  /=  %=  +=  -=  <<=  >>=  >>>=  &=  ^=  |=  **=  &&=  ||=  ??=

ConditionalExpression ::
  LogicalOrExpression
  LogicalOrExpression ? AssignmentExpression : AssignmentExpression

LogicalOrExpression ::
  LogicalAndExpression
  LogicalOrExpression || LogicalAndExpression

LogicalAndExpression ::
  BitwiseOrExpression
  LogicalAndExpression && BitwiseOrExpression

BitwiseOrExpression ::
  BitwiseXorExpression
  BitwiseOrExpression | BitwiseXorExpression

BitwiseXorExpression ::
  BitwiseAndExpression
  BitwiseXorExpression ^ BitwiseAndExpression

BitwiseAndExpression ::
  EqualityExpression
  BitwiseAndExpression & EqualityExpression

EqualityExpression ::
  RelationalExpression
  EqualityExpression === RelationalExpression
  EqualityExpression !== RelationalExpression
  EqualityExpression == RelationalExpression
  EqualityExpression != RelationalExpression

RelationalExpression ::
  ShiftExpression
  RelationalExpression < ShiftExpression
  RelationalExpression > ShiftExpression
  RelationalExpression <= ShiftExpression
  RelationalExpression >= ShiftExpression
  RelationalExpression instanceof ShiftExpression
  RelationalExpression in ShiftExpression

ShiftExpression ::
  AdditiveExpression
  ShiftExpression << AdditiveExpression
  ShiftExpression >> AdditiveExpression
  ShiftExpression >>> AdditiveExpression

AdditiveExpression ::
  MultiplicativeExpression
  AdditiveExpression + MultiplicativeExpression
  AdditiveExpression - MultiplicativeExpression

MultiplicativeExpression ::
  ExponentiationExpression
  MultiplicativeExpression * ExponentiationExpression
  MultiplicativeExpression / ExponentiationExpression
  MultiplicativeExpression % ExponentiationExpression

ExponentiationExpression ::
  UnaryExpression
  UnaryExpression ** ExponentiationExpression

UnaryExpression ::
  LeftHandSideExpression
  ++ UnaryExpression
  -- UnaryExpression
  + UnaryExpression
  - UnaryExpression
  ~ UnaryExpression
  ! UnaryExpression
  await UnaryExpression
  typeof UnaryExpression
  void UnaryExpression
  delete UnaryExpression

LeftHandSideExpression ::
  CallExpression
  MemberExpression

CallExpression ::
  MemberExpression Arguments
  CallExpression Arguments
  CallExpression [ Expression ]
  CallExpression . IdentifierName
  CallExpression ?. IdentifierName
  super Arguments

MemberExpression ::
  PrimaryExpression
  MemberExpression [ Expression ]
  MemberExpression . IdentifierName
  MemberExpression ?. IdentifierName
  MemberExpression TemplateLiteral
  super . IdentifierName
  super [ Expression ]
  new MemberExpression Arguments

PrimaryExpression ::
  Identifier
  Literal
  ArrayLiteral
  ObjectLiteral
  FunctionExpression
  ClassExpression
  ( Expression )
  ThisExpression
  TemplateLiteral
  IfExpression
  MatchExpression
  NewExpression
  NullAssertExpression

ThisExpression ::
  this

FunctionExpression ::
  fn BindingIdentifieropt ( FormalParameterListopt ) ReturnTypeAnnotationopt { FunctionBodyStatementListopt }

ClassExpression ::
  class BindingIdentifieropt TypeParametersopt ClassHeritageopt { ClassBodyopt }

Arguments ::
  ( ArgumentListopt )

ArgumentList ::
  AssignmentExpression
  ArgumentList , AssignmentExpression
  ... AssignmentExpression
```

#### 3.5.1 可选链

```
OptionalChain ::
  ?. IdentifierName
  ?. [ Expression ]
  ?. Arguments
  ?. TemplateLiteral
```

`?.` 运算符在左操作数为 `null` 时短路，整个链求值为 `null`。

**示例**:
```ruyi
let name = user?.profile?.name;
let first = arr?.[0];
let result = obj?.method?.();
```

#### 3.5.2 空值合并

```
NullishCoalescingExpression ::
  LogicalOrExpression
  NullishCoalescingExpression ?? LogicalOrExpression
```

`??` 运算符在左操作数为 `null` 时返回右操作数，否则返回左操作数。

**示例**:
```ruyi
let name = user?.name ?? "anonymous";
let count = config.count ?? 0;
let value = maybeNull ?? fallback ?? default;
```

#### 3.5.3 Await 表达式

```
AwaitExpression ::
  await UnaryExpression
```

`await` 运算符挂起 `async` 函数的执行，直到操作数解析完成。它只能出现在 `async` 函数内部。

**示例**:
```ruyi
async fn fetchData(url: string): string {
  let response = await http.get(url);
  return response.body;
}

async fn loadAll(urls: Array<string>): Array<string> {
  let results = [];
  for (let url of urls) {
    results.push(await fetchData(url));
  }
  return results;
}
```

### 3.5.4 If 表达式

`if` 结构可用作求值为值的表达式：

```
IfExpression ::
  if ( Expression ) Expression ElseExpressionopt

ElseExpression ::
  else Expression
```

与 `if` 语句不同，`if` 表达式的分支不使用大括号，且始终产生一个值。如果未提供 `else` 分支且条件为假，则表达式求值为 `null`。

**示例**:
```ruyi
let result = if (x > 0) { "positive" } else { "non-positive" };
let max = if (a > b) { a } else { b };
let msg = if (ready) { "go" };  // msg 是 string?（未就绪时为 null）
```

### 3.5.5 Match 表达式

`match` 结构可用作表达式：

```
MatchExpression ::
  match ( Expression ) { MatchArmsopt }
```

match 表达式求值为匹配分支体的值。所有分支必须产生兼容的类型。

**示例**:
```ruyi
let label = match (n) {
  0 => "zero",
  1 => "one",
  _ => "many",
};

let result = match (response) {
  { status: 200, body } => body,
  { status: 404 } => "not found",
  _ => "error",
};
```

### 3.5.6 New 表达式

`new` 运算符创建类的实例：

```
NewExpression ::
  new MemberExpression Arguments
```

**示例**:
```ruyi
let point = new Point(1.0, 2.0);
let config = new Config({ debug: true });
```

### 3.5.7 非空断言表达式

后缀 `!` 运算符断言可空值不为 null：

```
NullAssertExpression ::
  Expression !
```

在运行时，如果值为 `null`，则抛出运行时错误。结果类型是表达式类型的非可空形式。

**示例**:
```ruyi
let name: string? = getUser();
let safe: string = name!;  // 若 name 为 null 则抛出
let len = name!.length;    // 安全：name! 是 string
```

### 3.6 模式

```
Pattern ::
  IdentifierPattern
  LiteralPattern
  ObjectPattern
  ArrayPattern
  RestPattern
  AsPattern
  OrPattern
  WildcardPattern

IdentifierPattern ::
  Identifier

LiteralPattern ::
  NumericLiteral
  StringLiteral
  BooleanLiteral
  null

ObjectPattern ::
  { ObjectPatternFieldsopt }

ObjectPatternFields ::
  ObjectPatternField
  ObjectPatternFields , ObjectPatternField

ObjectPatternField ::
  IdentifierName : Pattern
  IdentifierName
  ... IdentifierName

ArrayPattern ::
  [ ArrayPatternElementsopt ]

ArrayPatternElements ::
  Pattern
  ArrayPatternElements , Patternopt

RestPattern ::
  ... IdentifierName

AsPattern ::
  Pattern as Identifier

OrPattern ::
  Pattern | Pattern

WildcardPattern ::
  _
```

### 3.7 类型注解

```
TypeAnnotation ::
  : Type

Type ::
  PrimaryType
  NullableType
  FunctionType
  GenericType
  DynType

PrimaryType ::
  Identifier
  { TypeFieldListopt }
  [ Type ]
  ( TypeListopt )

TypeFieldList ::
  TypeField
  TypeFieldList , TypeField

TypeField ::
  IdentifierName : Type

TypeList ::
  Type
  TypeList , Type

NullableType ::
  PrimaryType ?

FunctionType ::
  fn ( TypeListopt ) -> Type

GenericType ::
  Identifier < TypeList >

DynType ::
  dyn Identifier
  dyn Identifier < TypeList >
```

**内置类型名称**:

| 类型 | 说明 |
|------|-------------|
| `int` | 64 位有符号整数 |
| `float` | 64 位浮点数 |
| `bool` | 布尔值 (true/false) |
| `string` | UTF-8 字符串 |
| `null` | 空类型（唯一值: null） |
| `void` | 无返回值 |
| `dyn` | 动态类型（运行时检查） |
| `never` | 底类型（不可达） |
| `bigint` | 任意精度整数 |

**特殊类型**:

| 类型 | 说明 |
|------|-------------|
| `Future<T>` | 表示产生 `T` 的异步计算 |
| `dyn TraitName` | 用于动态分发的特征对象 |
| `Array<T>` | 元素类型为 `T` 的数组（从 `[T]` 脱糖） |

**示例**:
```ruyi
let x: int = 42;
let name: string? = null;
let fn: fn(int, int) -> int = add;
let items: Array<string> = [];
let point: { x: float, y: float } = { x: 0.0, y: 0.0 };
let printable: dyn Printable = getPrintable();
let future: Future<string> = fetchData(url);
```

### 3.8 模块

#### 3.8.1 导入声明

```
ImportDeclaration ::
  import ImportClause FromClause ;
  import FromClause ;

ImportClause ::
  ImportedDefaultBinding
  NameSpaceImport
  NamedImports
  ImportedDefaultBinding , NameSpaceImport
  ImportedDefaultBinding , NamedImports

ImportedDefaultBinding ::
  IdentifierName

NameSpaceImport ::
  * as IdentifierName

NamedImports ::
  { NamedImportListopt }

NamedImportList ::
  NamedImport
  NamedImportList , NamedImport

NamedImport ::
  IdentifierName
  IdentifierName as IdentifierName

FromClause ::
  from StringLiteral
```

**示例**:
```ruyi
import { add, subtract } from "./math";
import * as utils from "./utils";
import HttpClient from "./http";
import HttpClient, { Request, Response } from "./http";
import "./side-effect-module";
```

#### 3.8.2 导出声明

```
ExportDeclaration ::
  export ExportClause FromClause ;
  export NamedExports ;
  export VariableStatement
  export FunctionDeclaration
  export ClassDeclaration
  export TraitDeclaration
  export TypeAliasDeclaration
  export DefaultDeclaration

ExportClause ::
  *
  NamedExports

NamedExports ::
  { NamedExportListopt }

NamedExportList ::
  NamedExport
  NamedExportList , NamedExport

NamedExport ::
  IdentifierName
  IdentifierName as IdentifierName

DefaultDeclaration ::
  export default Expression ;
  export default FunctionDeclaration
  export default ClassDeclaration
```

**示例**:
```ruyi
export { add, subtract };
export { add as plus };
export * from "./math";
export const PI = 3.14159;
export fn double(x: int): int { return x * 2; }
export default class App { }
export default fn main() { }
```

---

## 4. 模式匹配

Ruyi 通过 `match` 表达式和 `if-let` 语句提供原生的模式匹配支持。

### 4.1 Match 表达式

`match` 表达式将表达式与一系列模式进行匹配，第一个匹配的分支会执行。

```ruyi
match (value) {
  0 => { print("zero"); }
  1 | 2 => { print("one or two"); }
  n if (n > 10 && n < 20) => { print("teen: " + n); }
  100 => { print("hundred"); }
  _ => { print("other"); }
}
```

### 4.2 Match 中的解构

模式可以解构对象和数组：

```ruyi
match (result) {
  { status: 200, body } => { print(body); }
  { status: 404 } => { print("not found"); }
  { status, body } => { print("error " + status + ": " + body); }
  _ => { print("unknown response"); }
}

match (list) {
  [] => { print("empty"); }
  [first] => { print("single: " + first); }
  [first, second, ...rest] => { print("first: " + first + ", second: " + second); }
  _ => { print("other"); }
}
```

### 4.3 If-Let 语句

`if-let` 语句将模式匹配与条件执行结合：

```
IfLetStatement ::
  if let Pattern = Expression Block ElseClauseopt
```

**示例**:
```ruyi
if let { x, y } = point {
  print("point at (" + x + ", " + y + ")");
}

if let [first, ...rest] = list {
  print("head: " + first);
}

if let Ok(value) = result {
  print("success: " + value);
} else {
  print("failed");
}
```

### 4.4 While-Let 语句

```
WhileLetStatement ::
  while let Pattern = Expression Block
```

**示例**:
```ruyi
while let Some(item) = iterator.next() {
  process(item);
}
```

---

## 5. 泛型

Ruyi 通过泛型支持参数化多态。泛型可用于函数、类、特征 (Trait) 和类型别名。

### 5.1 类型参数

```
TypeParameters ::
  < TypeParameterList >

TypeParameterList ::
  TypeParameter
  TypeParameterList , TypeParameter

TypeParameter ::
  IdentifierName
  IdentifierName : TraitBound
  IdentifierName : TraitBoundList

TraitBoundList ::
  TraitBound
  TraitBoundList + TraitBound

TraitBound ::
  Identifier
```

### 5.2 泛型函数

```ruyi
fn identity<T>(x: T): T {
  return x;
}

fn max<T: Comparable>(a: T, b: T): T {
  return if a > b { a } else { b };
}

fn map<T, U>(arr: Array<T>, f: fn(T) -> U): Array<U> {
  let result = [];
  for (let item of arr) {
    result.push(f(item));
  }
  return result;
}
```

### 5.3 泛型类

```ruyi
class Some<T> {
  value: T;

  fn new(value: T) {
    self.value = value;
  }

  fn isSome(self): bool {
    return true;
  }

  fn unwrap(self): T {
    return self.value;
  }
}

class None {
  fn new() { }

  fn isSome(self): bool {
    return false;
  }

  fn unwrap(self): never {
    throw RuntimeError.new("unwrap on None");
  }
}

type Option<T> = Some<T> | None;

class Map<K, V> {
  fn get(key: K): V?;
  fn set(key: K, value: V): void;
  fn keys(): Array<K>;
  fn values(): Array<V>;
}
```

### 5.4 泛型特征 (Trait)

```ruyi
trait From<T> {
  fn from(value: T): self;
}

trait Into<T> {
  fn into(self): T;
}

impl From<int> for string {
  fn from(value: int): string {
    return toString(value);
  }
}
```

### 5.5 泛型类型推断

Ruyi 根据上下文推断泛型类型参数：

```ruyi
fn wrap<T>(value: T): Option<T> {
  return Option.new(value);
}

let x = wrap(42);       // x: Option<int>
let y = wrap("hello");  // y: Option<string>
```

---

## 6. 空值安全

Ruyi 通过健全的可空类型系统消除了「十亿美元错误」。不存在 `undefined`，只有 `null`，且可空类型必须显式声明。

### 6.1 可空类型

```
NullableType ::
  Type ?
```

可空类型 `T?` 可以持有类型 `T` 的值或 `null`。非可空类型不能持有 `null`。

```ruyi
let name: string = "Ruyi";    // 不能为 null
let maybe: string? = null;     // 可以为 null
let count: int = 42;           // 不能为 null
let maybeCount: int? = null;   // 可以为 null
```

### 6.2 可选链

```
OptionalChainExpression ::
  MemberExpression ?. IdentifierName
  MemberExpression ?. [ Expression ]
  MemberExpression ?. Arguments
```

`?.` 运算符在可空值上安全地访问属性。若接收者为 `null`，整个表达式求值为 `null`，不会抛出异常。

```ruyi
let user: User? = findUser(id);
let name = user?.name;           // string?
let city = user?.address?.city;  // string?
let len = user?.name?.length;    // int?
```

### 6.3 空值合并

```
NullishCoalescingExpression ::
  Expression ?? Expression
```

`??` 运算符为可空表达式提供默认值：

```ruyi
let name = user?.name ?? "anonymous";    // string
let count = config.count ?? 0;           // int
let value = maybe ?? fallback ?? default; // T
```

### 6.4 非空断言

```
NullAssertion ::
  Expression !
```

`!` 运算符断言可空值不为 null。若运行时值为 null，则抛出运行时错误。

```ruyi
let name: string? = getUser();
let safe: string = name!;  // 若 name 为 null 则抛出异常
```

### 6.5 类型收窄

在进行 null 检查后，编译器会在受保护的作用域内收窄变量类型：

```ruyi
let name: string? = getUser();

if (name !== null) {
  // 此处 name 被收窄为 string
  print(name.length);
}

// 此处 name 恢复为 string?
```

---

## 7. 移除的 JavaScript 特性

Ruyi 移除了以下 JavaScript 特性。每一项移除都包含理由及 Ruyi 的替代方案。

### 7.1 已移除的特性

| JS 特性 | 状态 | Ruyi 替代方案 | 理由 |
|------------|--------|-------------------|-----------|
| `undefined` | **已移除** | `null` | 两个类似 null 的值会造成混淆。Ruyi 使用单一的 `null` 值。 |
| `var` | **已移除** | `let`, `const` | `var` 的函数作用域提升会导致 bug。块级作用域的 `let`/`const` 更安全。 |
| `==` 和 `!=` | **已映射** | `===` 和 `!==` | 为兼容性而解析；代码生成映射到 `===`/`!==` 行为。无隐式强制转换。 |
| 隐式类型强制转换 | **已移除** | 显式转换 | `"5" + 3` 得到 `"53"`，而 `"5" - 3` 得到 `2`，这是不一致的。Ruyi 要求显式类型转换。 |
| 原型链 | **已移除** | `class`, `trait` | 基于原型的继承令人困惑。基于类的继承更清晰、更熟悉。 |
| `with` 语句 | **已移除** | 无 | `with` 使静态分析变得不可能，并引入作用域歧义。 |
| `arguments` 对象 | **已移除** | 剩余参数 `...args` | `arguments` 对象是类数组但不是真正的数组。剩余参数是真正的数组。 |
| 自动分号插入的边界情况 | **已减少** | 更清晰的 ASI 规则 | Ruyi 简化了 ASI 以避免最令人惊讶的情况。 |
| 以 `0` 开头的八进制字面量 | **已移除** | `0o` 前缀 | `0777` 是八进制而 `0999` 是十进制，这很混乱。显式的 `0o` 前缀更清晰。 |
| `function` 关键字 | **已移除** | `fn` | 更短，与其他声明一致。 |
| `function*` / 生成器 | **部分** | `yield` 关键字已解析 | `yield` 已解析为关键字和语句；代码生成将其视为 no-op。完整的生成器支持计划中。 |
| `this` 绑定复杂性 | **已简化** | 词法 `self` | 箭头函数词法捕获 `self`。方法显式使用 `self`。 |
| 任意字符串的动态属性访问 | **已限制** | 索引签名 | Ruyi 将动态属性访问限制为带类型的索引签名。 |
| `eval()` | **已移除** | 无 | `eval` 是安全风险并阻止优化。 |
| 对象属性上的 `delete` | **有限** | 赋值为 `null` | `delete` 已解析为一元运算符；完整的代码生成支持有限。推荐使用 `obj.prop = null`。 |
| `typeof` 对 `null` 返回 `"object"` | **已修复** | `typeof null` 返回 `"null"` | 修正了 JS 中 `typeof null === "object"` 的 bug。 |
| 稀疏数组 | **已移除** | 以 `null` 填充的密集数组 | 稀疏数组有不可见的空洞。Ruyi 数组始终是密集的。 |
| `Number`, `String`, `Boolean` 包装对象 | **已移除** | 仅原始类型 | 包装对象会产生令人困惑的同一性行为（`new String("a") !== "a"`）。 |

### 7.2 详细理由

#### 7.2.1 `undefined` → `null`

JavaScript 有两个类似 null 的值：`null`（有意缺失）和 `undefined`（无意缺失）。这种区分很少有用，且导致需要不断检查两个值。

Ruyi 使用单一的 `null` 值。未初始化变量在动态上下文中默认值为 `null`。缺失的函数参数为 `null`。不存在的对象属性返回 `null`。

```ruyi
// JS: 两种表示「无」的方式
let a = null;
let b;  // undefined

// Ruyi: 一种方式
let a = null;
let b;  // null
```

#### 7.2.2 `var` → `let` / `const`

`var` 声明是函数作用域且会被提升，导致令人困惑的行为：

```javascript
// JS: var 泄漏出块级作用域
for (var i = 0; i < 10; i++) { }
console.log(i); // 10 - i 仍然可访问！
```

Ruyi 只有 `let`（可变，块级作用域）和 `const`（不可变，块级作用域）：

```ruyi
// Ruyi: 块级作用域
for (let i = 0; i < 10; i = i + 1) { }
// 此处无法访问 i
```

#### 7.2.3 `==` → `===`

JavaScript 的 `==` 在比较前会进行类型强制转换，导致意外结果：

```javascript
// JS: 令人困惑的相等性
0 == false       // true
"" == false      // true
[] == false      // true
null == undefined // true
"5" == 5         // true
```

Ruyi 完全移除了 `==` 和 `!=`。只存在 `===`（严格相等）和 `!==`（严格不等）：

```ruyi
// Ruyi: 仅严格相等
0 === false      // false（类型不同）
"5" === 5        // false（类型不同）
null === null    // true
```

注意：`==` 和 `!=` 为兼容性目的仍被解析器接受，但代码生成会将其映射到 `===`/`!==` 行为，不会产生隐式类型强制转换。

#### 7.2.4 隐式强制转换 → 显式转换

JavaScript 在许多上下文中静默转换类型：

```javascript
// JS: 隐式强制转换
"5" + 3    // "53"（字符串拼接）
"5" - 3    // 2（数值减法）
5 + null   // 5（null 被强制转换为 0）
5 + true   // 6（true 被强制转换为 1）
```

Ruyi 要求显式类型转换：

```ruyi
// Ruyi: 显式转换
"5" + toString(3)    // "53"
parseInt("5") - 3    // 2
5 + 0                // 5（无 null 强制转换）
5 + 1                // 6（无 bool 强制转换）
```

#### 7.2.5 原型链 → 类/特征 (Trait)

JavaScript 基于原型的继承功能强大但令人困惑：

```javascript
// JS: 原型继承
function Animal(name) { this.name = name; }
Animal.prototype.speak = function() { };

function Dog(name) { Animal.call(this, name); }
Dog.prototype = Object.create(Animal.prototype);
Dog.prototype.bark = function() { };
```

Ruyi 使用熟悉的类语法：

```ruyi
// Ruyi: 类继承
class Animal {
  name: string;
  fn new(name: string) { self.name = name; }
  fn speak() { }
}

class Dog extends Animal {
  fn new(name: string) { super.new(name); }
  fn bark() { }
}
```

---

## 附录 A: 完整词法单元参考

### A.1 关键字词法单元

```
let, const, fn, class, trait, impl, dyn, match, if, else, for, while,
return, throw, try, catch, finally, async, await, import,
export, macro, type, true, false, null, self, super, this,
in, instanceof, typeof, void, delete, as, extends, static,
get, set, new, of, break, continue, yield, _
```

### A.2 运算符词法单元

```
===, !==, ==, !=, <, >, <=, >=,
+, -, *, /, %, **,
&, |, ^, ~, <<, >>, >>>,
&&, ||, ??,
!, ?.,
=, +=, -=, *=, /=, %=, **=,
&, |=, ^=, <<=, >>=, >>>=,
&&=, ||=, ??=,
=>, ++, --,
in, instanceof, typeof, void, delete, yield
```
===, !==, ==, !=, <, >, <=, >=,
+, -, *, /, %, **,
&, |, ^, ~, <<, >>, >>>,
&&, ||, ??,
!, ?.,
=, +=, -=, *=, /=, %=, **=,
&=, |=, ^=, <<=, >>=, >>>=,
&&=, ||=, ??=,
=>, ++, --,
in, instanceof, typeof, void, delete
```

### A.3 分隔符词法单元

```
{, }, (, ), [, ],
., ,, ;, :, ?,
@, #, ..., ::, $,
<, >
```

### A.4 字面量形式

```
null, true, false
42, 3.14, 1e10, 0xFF, 0o77, 0b1010, 100n
"hello", 'world', `template ${expr}`
```

---

## 附录 B: 文法汇总

### B.1 声明文法

```
Declaration     → LexicalDeclaration | FunctionDeclaration | ClassDeclaration
                | TraitDeclaration | ImplDeclaration | TypeAliasDeclaration | MacroDeclaration
LexicalDecl     → let BindingList ; | const BindingList ;
FunctionDecl    → fn Identifier TypeParams? ( Params? ) ReturnType? { Body }
ClassDecl       → @Annot* class Identifier TypeParams? extends Expr? { ClassBody }
TraitDecl       → trait Identifier TypeParams? { TraitBody }
ImplDecl        → impl TypeParams? TraitName TypeArgs? for Type { ClassBody }
TypeAlias       → type Identifier TypeParams? = Type ;
MacroDecl       → macro Identifier { MacroRules }
```

### B.2 语句文法

```
Statement       → Block | IfStmt | IfLetStmt | WhileStmt | WhileLetStmt
                | ForStmt | ForInStmt | ForOfStmt | ReturnStmt | ThrowStmt
                | TryStmt | MatchStmt | BreakStmt | ContinueStmt | YieldStmt
                | LabeledStmt | ExprStmt | EmptyStmt
IfStmt          → if ( Expr ) Stmt else Stmt?
IfLetStmt       → if let Pattern = Expr Block else Stmt?
WhileStmt       → while ( Expr ) Stmt
WhileLetStmt    → while let Pattern = Expr Block
ForStmt         → for ( Init? ; Expr? ; Update? ) Stmt
ForInStmt       → for ( let Identifier in Expr ) Stmt
ForOfStmt       → for ( let Identifier of [async] Expr ) Stmt
ReturnStmt      → return Expr? ;
ThrowStmt       → throw Expr ;
TryStmt         → try Block catch ( Pattern ) Block? finally Block?
MatchStmt       → match ( Expr ) { MatchArms }
BreakStmt       → break Identifier? ;
ContinueStmt    → continue Identifier? ;
YieldStmt       → yield Expr? ;
LabeledStmt     → Identifier : Stmt
```

### B.3 表达式文法

```
Expression      → AssignmentExpr ( , AssignmentExpr )*
AssignmentExpr  → ConditionalExpr | LeftHandSide = AssignmentExpr
                | LeftHandSide AssignOp AssignmentExpr | ArrowFunction
ConditionalExpr → LogicalOrExpr ? AssignmentExpr : AssignmentExpr
LogicalOrExpr   → LogicalAndExpr ( || LogicalAndExpr )*
LogicalAndExpr  → BitwiseOrExpr ( && BitwiseOrExpr )*
EqualityExpr    → RelationalExpr ( === RelationalExpr | !== RelationalExpr )*
RelationalExpr  → ShiftExpr ( < ShiftExpr | > ShiftExpr | <= ShiftExpr | >= ShiftExpr )*
AdditiveExpr    → MultiplicativeExpr ( + MultiplicativeExpr | - MultiplicativeExpr )*
Multiplicative  → ExponentiationExpr ( * ExponentiationExpr | / ExponentiationExpr | % ExponentiationExpr )*
Exponentiation  → UnaryExpr ( ** ExponentiationExpr )?
UnaryExpr       → LeftHandSideExpr | ++ UnaryExpr | -- UnaryExpr | + UnaryExpr
                | - UnaryExpr | ~ UnaryExpr | ! UnaryExpr | await UnaryExpr
                | typeof UnaryExpr | void UnaryExpr | delete UnaryExpr
PostfixExpr     → LeftHandSideExpr !          (非空断言)
LeftHandSide    → CallExpr | MemberExpr
CallExpr        → MemberExpr Arguments | CallExpr Arguments | CallExpr [ Expr ]
                | CallExpr . Identifier | CallExpr ?. Identifier
MemberExpr      → PrimaryExpr | MemberExpr [ Expr ] | MemberExpr . Identifier
                | MemberExpr ?. Identifier | MemberExpr TemplateLiteral
PrimaryExpr     → Identifier | Literal | ArrayLiteral | ObjectLiteral
                | FunctionExpr | ClassExpr | ( Expr ) | this | TemplateLiteral
                | if ( Expr ) Expr else Expr  (if 表达式)
                | match ( Expr ) { Arms }     (match 表达式)
                | new MemberExpr Arguments    (new 表达式)
```

### B.4 模式文法

```
Pattern         → Identifier | Literal | { ObjectPatternFields }
                | [ ArrayPatternElements ] | ... Identifier | Pattern as Identifier
                | Pattern | Pattern | _
```

### B.5 类型文法

```
Type            → Identifier | Type? | fn ( Types ) -> Type | Identifier < Types >
                | { TypeFields } | [ Type ] | dyn Identifier | dyn Identifier < Types >
```

### B.6 模块文法

```
ImportDecl      → import ImportClause from StringLiteral ;
                | import StringLiteral ;
ImportClause    → Identifier | * as Identifier | { NamedImports }
                | Identifier , { NamedImports }
ExportDecl      → export { NamedExports } ;
                → export * from StringLiteral ;
                → export Declaration
                → export default Expr ;
```

---

## 附录 C: 运算符优先级表

| 优先级 | 运算符 | 说明 | 结合性 |
|------------|----------|-------------|---------------|
| 18 | `.` `?.` `()` `[]` | 成员访问、调用、索引 | 左 |
| 17 | `++` `--` `!`（前缀）`~` `+` `-` `await` `typeof` `void` `delete` | 一元 | 右 |
| 16 | `**` | 幂运算 | 右 |
| 15 | `*` `/` `%` | 乘法类 | 左 |
| 14 | `+` `-` | 加法类 | 左 |
| 13 | `<<` `>>` `>>>` | 位移 | 左 |
| 12 | `<` `>` `<=` `>=` `in` `instanceof` | 关系 | 左 |
| 11 | `===` `!==` `==` `!=` | 相等 | 左 |
| 10 | `&` | 按位与 | 左 |
| 9 | `^` | 按位异或 | 左 |
| 8 | `\|` | 按位或 | 左 |
| 7 | `&&` | 逻辑与 | 左 |
| 6 | `\|\|` | 逻辑或 | 左 |
| 5 | `??` | 空值合并 | 左 |
| 4 | `?:` | 三元条件 | 右 |
| 3 | `=>` | 箭头函数 | 右 |
| 2 | `=` `+=` `-=` `*=` `/=` `%=` `**=` `&=` `\|=` `^=` `<<=` `>>=` `>>>=` `&&=` `\|\|=` `??=` | 赋值 | 右 |
| 1 | `,` | 序列 | 左 |

**后缀运算符**（在上述所有运算符之后应用）:

| 运算符 | 说明 |
|----------|-------------|
| `!` | 非空断言：`e!` 断言 `e` 不为 null |

---

---

## 8. 类型系统语义

Ruyi 采用**渐进式类型系统**，将静态类型检查与动态类型检查相结合。程序员可以选择添加类型注解以获得编译时安全性，或省略注解并依赖运行时检查。该系统的设计使得静态类型与动态类型能够无矛盾地共存。

### 8.1 渐进式类型模型

#### 8.1.1 类型注解语义

Ruyi 中的每个绑定都有一个关联的类型。该类型通过以下两种机制之一确定：

1. **显式注解**: 当提供类型注解时，编译器使用该类型进行静态检查。
2. **隐式推断**: 当未提供注解时，编译器尝试根据初始化表达式推断类型。若推断失败，则类型默认设为 `dyn`。

```
TypeAssignment(binding, annotation, initializer) =
  if annotation is present:
    verify initializer <: annotation
    return annotation
  else if initializer type can be inferred:
    return inferred type
  else:
    return dyn
```

**示例**:
```ruyi
let x = 42;           // x: int（从字面量推断）
let y: int = 42;      // y: int（显式注解）
let z;                // z: dyn（无注解，无初始化表达式）
let f = (a) => a + 1; // f: fn(dyn) -> dyn（参数未注解）
```

#### 8.1.2 `dyn` 类型

`dyn` 是动态类型。它表示在运行时而非编译时检查类型的值。`dyn` 与所有类型一致，意味着：

- 任何值都可以赋值给 `dyn`。
- `dyn` 类型的值可以在期望特定类型的任何上下文中使用，并会插入运行时检查。

**形式化一致性关系**（写作 `~`）：

```
T ~ dyn    for all types T
dyn ~ T    for all types T
```

这意味着 `dyn` 在渐进式类型的意义上既是所有类型的子类型也是超类型。然而，这并不意味着放弃了类型安全性。当 `dyn` 值流入静态类型上下文时，会插入**运行时类型检查**（类型转换）。

#### 8.1.3 运行时类型检查（类型转换插入）

当 `dyn` 类型的值被用在期望静态类型 `T` 的上下文中时，编译器会插入运行时检查：

```
cast<T>(v: dyn): T
```

类型转换操作：
1. 检查 `v` 的运行时类型标签。
2. 若运行时类型匹配 `T`（或是 `T` 的子类型），则将 `v` 作为类型 `T` 返回。
3. 若运行时类型不匹配，则在运行时抛出 `TypeError`。

**类型转换插入规则**：

| 上下文 | 规则 |
|---------|------|
| 函数调用 `f(arg)`，其中 `f: fn(T) -> R` 且 `arg: dyn` | 插入 `cast<T>(arg)` |
| 方法调用 `obj.method()`，其中 `obj: dyn` | 插入运行时方法查找 |
| 属性访问 `obj.prop`，其中 `obj: dyn` | 插入运行时属性查找 |
| 二元运算 `a + b`，其中任一操作数为 `dyn` | 对两个操作数插入运行时类型检查 |
| 从函数返回 `return v`，其中返回类型为 `T` 且 `v: dyn` | 插入 `cast<T>(v)` |
| 赋值 `let x: T = v`，其中 `v: dyn` | 插入 `cast<T>(v)` |

#### 8.1.4 渐进式类型一致性

渐进式类型系统满足**渐进式保证**：

- **静态保证**: 若一个程序在不使用 `dyn` 的情况下通过类型检查，则它具有与完全静态类型程序相同的安全属性。
- **动态保证**: 若一个包含 `dyn` 的程序通过了所有运行时检查，则其行为与将所有 `dyn` 替换为推断出的运行时类型的同一程序完全一致。
- **迁移保证**: 为正常工作的动态程序添加类型注解永远不会改变其运行时行为（除非注解本身不正确，此时会报编译时错误）。

### 8.2 类型推断算法

Ruyi 采用**双向类型推断**算法，将局部类型推断与基于约束求解的泛型函数推断相结合。

#### 8.2.1 双向类型检查

类型检查器在两种模式下运行：

- **检查模式** (`Gamma |- e <= T`)：在上下文 `Gamma` 中验证表达式 `e` 具有类型 `T`。
- **合成模式** (`Gamma |- e => T`)：在上下文 `Gamma` 中确定表达式 `e` 的类型 `T`。

**关键规则**：

```
[Syn-Var]    Gamma(x) = T
             -----------------
             Gamma |- x => T

[Syn-Let]    Gamma |- e1 => T1    Gamma, x:T1 |- e2 => T2
             -------------------------------------------
             Gamma |- let x = e1; e2 => T2

[Check-Lam]  Gamma, x:T1 |- body <= T2
             ---------------------------------
             Gamma |- (x) => body <= fn(T1) -> T2

[Syn-App]    Gamma |- f => fn(T1) -> T2    Gamma |- arg <= T1
             -------------------------------------------------
             Gamma |- f(arg) => T2

[Syn-If]     Gamma |- cond => bool    Gamma |- then => T    Gamma |- else => T
             ----------------------------------------------------------------
             Gamma |- if (cond) { then } else { else } => T
```

#### 8.2.2 局部类型推断

对于没有注解的变量声明，Ruyi 根据初始化表达式推断类型：

```ruyi
let x = 42;           // 42 是 int 字面量，因此 x: int
let y = "hello";      // 字符串字面量，因此 y: string
let z = true;         // bool 字面量，因此 z: bool
let arr = [1, 2, 3];  // int 字面量数组，因此 arr: Array<int>
```

**字面量推断规则**：

| 字面量 | 推断类型 |
|---------|---------------|
| 整数字面量（无后缀） | `int` |
| 浮点数字面量（含 `.` 或 `e`） | `float` |
| BigInt 字面量（后缀 `n`） | `bigint` |
| 字符串字面量 | `string` |
| `true` / `false` | `bool` |
| `null` | `null` |
| 数组字面量 `[e1, e2, ...]` | `Array<lub(T1, T2, ...)>` |
| 对象字面量 `{ k1: v1, ... }` | `{ k1: T1, ... }` |

一组类型的**最小上界**（lub）是集合中所有类型都可以赋值到的最具体类型：

```
lub(int, int) = int
lub(int, float) = float
lub(T, T) = T
lub(T, dyn) = dyn
lub(dyn, dyn) = dyn
lub(T, U) = dyn    (when T and U are unrelated)
```

#### 8.2.3 函数返回类型推断

当函数没有返回类型注解时，Ruyi 根据所有 `return` 语句推断返回类型：

```ruyi
fn add(a: int, b: int) {    // 无返回注解
  return a + b;              // a + b: int
}
// 推断为: fn add(a: int, b: int): int
```

若存在多个返回路径，返回类型是所有返回类型的 lub：

```ruyi
fn maybeNumber(flag: bool) {
  if (flag) {
    return 42;               // int
  } else {
    return 3.14;             // float
  }
}
// 推断为: fn maybeNumber(flag: bool): float
```

若函数没有 `return` 语句，推断的返回类型为 `void`。

#### 8.2.4 泛型的基于约束推断

对于泛型函数，Ruyi 在类型检查期间收集类型约束，并通过统一化（unification）求解：

```ruyi
fn map<T, U>(arr: Array<T>, f: fn(T) -> U): Array<U> {
  // ...
}

let result = map([1, 2, 3], (x) => x * 2);
// 约束: T = int（来自数组）, U = int（来自 x * 2）
// 结果: Array<int>
```

约束求解器：
1. 为每个未绑定的类型参数创建类型变量。
2. 遍历函数体，生成相等约束。
3. 通过统一化求解约束。
4. 若约束不可满足，则报告类型错误。
5. 若存在多个解，选择最具体的那个。

### 8.3 类型层次与自类型

Ruyi 对对象类型采用结构子类型系统，对命名类型采用名义子类型系统。

#### 8.3.1 子类型规则

```
[Sub-Refl]   T <: T

[Sub-Trans]  T <: U    U <: V
             ---------------
             T <: V

[Sub-Null]   T <: T?

[Sub-Object] { f1: T1, ..., fn: Tn, ... } <: { f1: U1, ..., fm: Um }
             if m <= n and for each fi in the supertype: Ti <: Ui

[Sub-Function] fn(U1, ..., Un) -> R <: fn(T1, ..., Tn) -> S
               if Ti <: Ui (contravariant in parameters)
               and R <: S (covariant in return)

[Sub-Array]    Array<T> <: Array<U>    if T <: U
```

#### 8.3.2 类型兼容性

若一个类型是另一个类型的子类型，或两者均为 `dyn`，则这两个类型**兼容**：

```
compatible(T, U) = T <: U || U <: T || T = dyn || U = dyn
```

### 8.4 动态类型运行时表示

在运行时，`dyn` 值携带一个**类型标签**，用于标识其具体类型：

```
DynValue {
  tag: TypeTag,      // 标识具体类型的枚举
  value: RawValue,   // 实际值位模式
}
```

**TypeTag 枚举**：

```
TypeTag ::
  IntTag          // 64 位有符号整数
  FloatTag        // 64 位 IEEE 754 浮点数
  BoolTag         // 布尔值
  StringTag       // 指向字符串对象的指针
  ArrayTag        // 指向数组对象的指针
  ObjectTag       // 指向对象的指针
  FunctionTag     // 指向函数闭包的指针
  NullTag         // null 值
  BigIntTag       // 任意精度整数
  ErrorTag        // 异常对象
  TraitObjectTag  // 特征对象（虚表 + 数据）
```

运行时类型检查将 `tag` 字段与期望类型进行比较。对于自类型检查（例如将 `dyn` 值赋给特征类型），使用标签查找特征实现。

---

## 9. 可空类型语义

### 9.1 可空类型的构成

对于任意类型 `T`，可空类型 `T?` 构成如下：

```
T? = T | null
```

`T?` 是与 `T` 不同的类型。以下规则管辖可空类型：

```
[Null-Intro]   null : T?                    (for any T)
[Null-Elim]    v : T?    v !== null
                -----------------
                v : T                         (after null check)

[Null-Sub]     T <: T?                      (T is a subtype of T?)
[Null-Double]  (T?)? = T?                   (nullable of nullable is nullable)
```

### 9.2 可选链语义

`?.` 运算符在可空接收者上提供安全的属性访问。其语义通过短路求值定义：

```
e?.prop  =  if (e === null) { null } else { e.prop }
```

`e?.prop` 的结果类型始终为可空：

```
If   e : T?    and    T.prop : U
Then e?.prop : U?
```

**链式可选访问**：

```
user?.profile?.name
```

展开为：

```
if (user === null) {
  null
} else {
  if (user.profile === null) {
    null
  } else {
    user.profile.name
  }
}
```

若链中任何中间值为 `null`，整个链求值为 `null`。结果类型是最终属性类型的可空形式。

**可选方法调用**：

```
obj?.method(args)
```

展开为：

```
if (obj === null) {
  null
} else {
  obj.method(args)
}
```

仅在 `obj` 不为 `null` 时才调用方法。结果类型是方法返回类型的可空形式。

**可选索引**：

```
arr?.[index]
```

展开为：

```
if (arr === null) {
  null
} else {
  arr[index]
}
```

### 9.3 空值合并语义

`??` 运算符为可空表达式提供默认值：

```
e1 ?? e2  =  if (e1 !== null) { e1 } else { e2 }
```

**类型推导**：

```
If   e1 : T?    and    e2 : U    and    T <: U
Then e1 ?? e2 : U
```

结果类型是 `e1` 类型的非空形式与 `e2` 类型的最小上界：

```
type(e1 ?? e2) = lub(nonNull(type(e1)), type(e2))
```

其中 `nonNull(T?) = T` 且 `nonNull(T) = T`。

**链式合并**：

```
a ?? b ?? c
```

左结合：

```
(a ?? b) ?? c
```

结果类型是链中所有非空类型的最小上界。

### 9.4 非空断言

`!` 运算符断言可空值不为 null：

```
e!  =  if (e === null) { throw NullAssertionError() } else { e }
```

**类型规则**：

```
If   e : T?
Then e! : T
```

`!` 运算符从类型中移除可空包装。在运行时，它执行 null 检查，若值为 null 则抛出异常。

### 9.5 基于控制流的类型收窄

在进行 null 检查后，编译器会在受保护的作用域内收窄被检查变量的类型：

```ruyi
let name: string? = getUser();

if (name !== null) {
  // 此处 name 被收窄为 string
  print(name.length);    // OK: 此处 name 为 string
}

// 此处 name 恢复为 string?
```

**收窄规则**：

| 检查 | 真分支 | 假分支 |
|-------|-------------|--------------|
| `x !== null` | `x` 收窄为 `T` | `x` 收窄为 `null` |
| `x === null` | `x` 收窄为 `null` | `x` 收窄为 `T` |
| `x != null` | `x` 收窄为 `T` | `x` 收窄为 `null` |
| `x == null` | `x` 收窄为 `null` | `x` 收窄为 `T` |

收窄适用于：
- `if` / `else` 分支
- 三元 `?:` 分支
- 带 null 守卫的 `match` 分支
- 条件包含 null 检查的循环体

收窄**不会**在函数调用或可变重新赋值之间持久化：

```ruyi
let name: string? = getUser();

if (name !== null) {
  someFunction();     // 函数调用可能有副作用
  print(name.length); // 仍然 OK: 收窄在纯调用间保持不变
}

name = getUser();     // 重新赋值会重置收窄
```

### 9.6 可空类型与泛型

可空类型与泛型的交互方式如下：

```ruyi
class Some<T> {
  value: T;

  fn new(value: T) {
    self.value = value;
  }

  fn unwrap(self): T {
    return self.value;    // 直接返回 T
  }
}

class None {
  fn unwrap(self): never {
    throw RuntimeError.new("unwrap on None");
  }
}

type Option<T> = Some<T> | None;
```

`Option<T>` 与 `T?` 是不同的。`Option<T>` 是可以携带额外方法的包装类型，而 `T?` 是内置的可空类型。

---

## 10. 泛型语义

### 10.1 类型参数化

泛型声明引入作为具体类型占位符的类型参数：

```ruyi
fn identity<T>(x: T): T { return x; }
class Box<T> { value: T; }
trait Iterator<T> { fn next(self): T?; }
```

类型参数的作用域限定在其声明内，并在实例化点替换为具体类型。

### 10.2 特征 (Trait) 约束

类型参数可以通过特征约束进行限制：

```ruyi
fn max<T: Comparable>(a: T, b: T): T { ... }
fn sort<T: Comparable + Clone>(arr: Array<T>): Array<T> { ... }
```

**多重约束**使用 `+` 语法，要求类型实现所有列出的特征：

```
T: A + B    means    T implements A AND T implements B
```

**约束语义**：

当类型参数 `T` 具有特征约束 `Trait` 时，编译器：
1. 验证任何替代 `T` 的具体类型都实现了 `Trait`。
2. 允许在类型 `T` 的值上调用 `Trait` 定义的方法。
3. 为动态分派生成特征字典（虚表指针），或为静态分派进行单态化。

### 10.3 单态化

Ruyi 使用**单态化**作为泛型的主要代码生成策略。在每个调用点，编译器生成一个泛型函数的专用副本，将类型参数替换为具体类型。

**单态化过程**：

1. **收集**: 在类型检查期间，收集泛型函数的所有调用点及其使用的具体类型。
2. **替换**: 对于每种唯一组合的具体类型，创建泛型函数的专用版本。
3. **代码生成**: 为每个专用版本生成 LLVM IR。
4. **去重**: 若同一专用版本在多个调用点使用，则只生成一次。

**示例**：

```ruyi
fn identity<T>(x: T): T { return x; }

let a = identity(42);       // 生成 identity_int(x: int): int
let b = identity("hello");  // 生成 identity_string(x: string): string
```

生成：

```ruyi
fn identity_int(x: int): int { return x; }
fn identity_string(x: string): string { return x; }
```

**单态化与特征约束**：

当泛型函数具有特征约束时，单态化版本包含特征实现：

```ruyi
fn max<T: Comparable>(a: T, b: T): T {
  return if a.compare(b) > 0 { a } else { b };
}

let m = max(3, 5);  // 使用 int 的 Comparable 实现生成 max_int
```

### 10.4 泛型与动态类型

泛型函数可以使用 `dyn` 参数调用：

```ruyi
fn identity<T>(x: T): T { return x; }

let x: dyn = 42;
let y = identity(x);    // T = dyn, 返回 dyn
```

当 `dyn` 用作类型参数时：
1. 泛型函数不进行单态化。而是使用单一的 `dyn` 版本。
2. 函数内的所有操作都使用运行时类型检查。
3. `dyn` 上的特征约束通过特征对象查找在运行时检查。

**交互规则**：

| 场景 | 行为 |
|----------|----------|
| 以所有静态类型调用泛型 | 单态化 |
| 以 `dyn` 类型参数调用泛型 | 使用 dyn 版本（不进行单态化） |
| 以 `dyn` 调用带特征约束的泛型 | 运行时特征查找 |
| 以静态类型调用带特征约束的泛型 | 单态化并静态分派 |

### 10.5 泛型类型别名

类型别名可以是泛型的：

```ruyi
type Result<T, E> = Ok<T, E> | Err<T, E>;
type Callback<T> = fn(T) -> void;
```

泛型类型别名在使用点展开：

```
Result<int, Error>  expands to  Ok<int, Error> | Err<int, Error>
```

### 10.6 型变

Ruyi 的泛型类型具有以下型变：

| 类型构造器 | 型变 | 规则 |
|-----------------|----------|------|
| `Array<T>` | 协变 | `Array<S> <: Array<T>` if `S <: T` |
| `fn(T) -> R` | 参数逆变，返回协变 | `fn(U) -> S <: fn(T) -> R` if `T <: U` and `S <: R` |
| `Option<T>` | 协变 | `Option<S> <: Option<T>` if `S <: T` |
| `Result<T, E>` | 两者皆协变 | `Result<S, F> <: Result<T, E>` if `S <: T` and `F <: E` |

---

## 11. 特征 (Trait) 语义

### 11.1 特征声明

特征定义了类型必须实现的一组方法：

```ruyi
trait Printable {
  fn format(self): string;
}

trait Comparable<T> {
  fn compare(self, other: T): int;
}
```

**特征语义**：

1. 特征声明定义了**契约**，而非实现。
2. 特征方法没有方法体（仅签名）。
3. 特征可以是泛型的（特征自身带有类型参数）。
4. 特征可以具有**默认方法实现**（见 11.4 节）。

### 11.2 特征实现

类型通过 `impl` 块实现特征：

```ruyi
impl Printable for string {
  fn format(self): string {
    return self;
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

**实现规则**：

1. **孤儿规则**: `impl` 块必须与特征或被实现类型位于同一模块中。这可以防止冲突实现。
2. **一致性**: 对于任意给定类型和特征，在任何作用域内最多只能有一个实现可见。
3. **泛型 impl**: Impl 块可以是泛型的，拥有自己的类型参数和约束。

### 11.3 静态分派与动态分派

Ruyi 支持对特征方法的静态分派和动态分派。

#### 11.3.1 静态分派（单态化）

当编译时已知具体类型时，特征方法调用使用静态分派：

```ruyi
fn printIt<T: Printable>(value: T) {
  print(value.format());    // 静态分派: T 已知
}

printIt("hello");    // 直接调用 string.format()
printIt(42);         // 直接调用 int.format()
```

编译器为每种具体类型单态化 `printIt`，生成直接的函数调用，无需虚表查找。

#### 11.3.2 动态分派（特征对象）

当编译时未知具体类型时，特征方法调用通过特征对象使用动态分派：

```ruyi
let items: Array<dyn Printable> = ["hello", 42, true];
for (let item of items) {
  print(item.format());    // 动态分派: 虚表查找
}
```

**特征对象表示**：

```
TraitObject {
  data: *void,        // 指向具体值的指针
  vtable: *VTable,    // 指向该类型对应特征虚表的指针
}

VTable {
  format: fn(*void) -> string,    // 每个特征方法对应的函数指针
  // ... 其他方法
}
```

在运行时，通过索引虚表来查找具体类型的正确方法实现。

#### 11.3.3 分派选择规则

| 上下文 | 分派方式 |
|---------|----------|
| 带特征约束的泛型函数 | 静态（单态化） |
| 特征对象 (`dyn Trait`) | 动态（虚表） |
| 具体类型上的直接方法调用 | 静态（直接调用） |
| 通过 `dyn` 变量的方法调用 | 动态（虚表） |

### 11.4 默认方法实现

特征可以为方法提供默认实现：

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

默认方法：
1. 被所有未覆盖它们的实现继承。
2. 可以调用其他特征方法（包括抽象方法）。
3. 通过静态分派使用时进行单态化。
4. 通过动态分派时使用虚表查找。

### 11.5 特征对象与类型擦除

当值被转换为特征对象 (`dyn Trait`) 时，具体类型被擦除：

```ruyi
let x: string = "hello";
let y: dyn Printable = x;    // 类型被擦除为 dyn Printable
```

**类型擦除规则**：

1. 特征对象保留具体值，但隐藏其类型。
2. 只有通过特征定义的方法才可以通过特征对象访问。
3. 原始类型可以通过模式匹配恢复（见 11.6 节）。

### 11.6 特征对象向下转换

特征对象可以向下转换为它们的具体类型：

```ruyi
let y: dyn Printable = "hello";

match (y) {
  s as string => { print("string: " + s); }
  n as int => { print("int: " + n); }
  _ => { print("unknown type"); }
}
```

向下转换使用运行时类型标签比较。若标签匹配目标类型，则进行值转换。否则，跳过该 match 分支。

---

## 12. 内存模型

### 12.1 内存管理策略

Ruyi 支持两种内存管理策略：

| 策略 | 默认 | 使用场景 |
|----------|---------|----------|
| **GC** (垃圾回收) | 是 | 通用代码，快速开发 |
| **ARC** (自动引用计数) | 否 | 性能关键路径，确定性释放 |

### 12.2 GC 内存区域

默认情况下，所有堆分配对象都由垃圾回收器管理。

#### 12.2.1 GC 对象布局

```
GC 对象头部:
+----------------+----------------+----------------+
| TypeTag (8b)   | Flags (8b)     | Size (16b)     |
+----------------+----------------+----------------+
| Forwarding Ptr (32b) | Reserved (32b)            |
+----------------+----------------+----------------+
| Payload (variable size)                         |
+-------------------------------------------------+
```

**头部字段**：

| 字段 | 大小 | 用途 |
|-------|------|---------|
| TypeTag | 8 位 | 标识对象类型（见 8.4 节） |
| Flags | 8 位 | 第 0 位: marked, 第 1 位: pinned, 第 2 位: in old gen, 第 3-7 位: reserved |
| Size | 16 位 | 对象总大小（含头部），单位为字节 |
| Forwarding Ptr | 32 位 | 复制 GC 期间用于指向新位置 |
| Reserved | 32 位 | 保留供将来使用 |

#### 12.2.2 GC 分代

GC 使用**分代**策略：

1. **新生代 (nursery)**: 新对象在此分配。较小（通常 1-4 MB）。回收频繁。
2. **老年代**: 在新生代中存活多次收集的对象会晋升至此。较大（通常 16-64 MB）。回收频率较低。

**晋升规则**: 对象在新生代中存活 `N` 次收集后晋升至老年代（默认 `N = 3`）。

#### 12.2.3 GC 收集算法

**新生代（复制收集器）**：
1. 识别根集（栈变量、全局变量、寄存器）。
2. 将存活对象从 nursery 复制到 survivor 空间。
3. 更新所有引用指向新位置。
4. 交换 nursery 和 survivor 空间。

**老年代（标记-压缩收集器）**：
1. 标记阶段：从根出发遍历，标记所有可达对象。
2. 压缩阶段：移动存活对象以消除碎片。
3. 更新所有引用。

**写屏障**: 当老年代对象中的指针被更新为指向新生代对象时，写屏障会记录这个跨代引用。这确保老年代对象在新生代收集期间被扫描。

### 12.3 ARC 内存区域

对象可以通过 ARC 管理显式分配：

```ruyi
let ptr: Arc<T> = Arc::new(value);
```

#### 12.3.1 ARC 对象布局

```
ARC 对象头部:
+----------------+----------------+----------------+
| TypeTag (8b)   | Flags (8b)     | Size (16b)     |
+----------------+----------------+----------------+
| RefCount (32b) | WeakCount (32b)                  |
+----------------+----------------+----------------+
| Payload (variable size)                         |
+-------------------------------------------------+
```

**引用计数规则**：

1. `Arc::new(value)` 创建一个 `RefCount = 1` 的对象。
2. `Arc::clone(&ptr)` 将 `RefCount` 加 1。
3. 当 `RefCount` 达到 0 时，对象被释放。
4. `Weak<T>` 引用增加 `WeakCount` 但不增加 `RefCount`。
5. 当 `RefCount` 和 `WeakCount` 都达到 0 时，内存被释放。

### 12.4 GC/ARC 边界规则

GC 管理的对象和 ARC 管理的对象可以相互引用，但有限制：

#### 12.4.1 GC 引用 ARC

GC 对象**可以**持有对 ARC 对象的引用：

```ruyi
let arcObj: Arc<int> = Arc::new(42);
let gcObj = { value: arcObj };    // OK: GC 持有 Arc 引用
```

GC 将 `Arc<T>` 视为不透明值。Arc 的引用计数独立于 GC 收集。

#### 12.4.2 ARC 引用 GC

ARC 对象**不能直接**持有对 GC 对象的引用：

```ruyi
let gcObj = { x: 1 };
let arcObj: Arc<SomeType> = Arc::new(gcObj);    // ERROR: ARC 不能持有 GC 引用
```

**理由**: GC 可能在收集期间移动对象，导致 ARC 对象持有的裸指针失效。若 ARC 对象需要引用 GC 对象，必须使用 `GcRef<T>` 句柄：

```ruyi
let gcObj: Gc<MyType> = Gc::new(MyType::new());
let arcObj: Arc<SomeType> = Arc::new({ handle: gcObj.clone() });
```

`GcRef<T>` 是一个 GC 跟踪的句柄，在收集期间保持有效。

#### 12.4.3 边界总结

| 方向 | 允许 | 机制 |
|-----------|---------|-----------|
| GC -> ARC | 是 | 直接引用（Arc 对 GC 不透明） |
| ARC -> GC | 否（直接） | 必须使用 `GcRef<T>` 句柄 |
| GC -> GC | 是 | 标准 GC 引用 |
| ARC -> ARC | 是 | 标准引用计数 |

### 12.5 对象布局与对齐

所有堆对象按 8 字节边界对齐以获得性能。最小对象大小为 16 字节（仅头部）。

**原始值布局**（在栈上或对象负载中）：

| 类型 | 大小 | 对齐 |
|------|------|-----------|
| `int` | 8 字节 | 8 |
| `float` | 8 字节 | 8 |
| `bool` | 1 字节 | 1 |
| `null` | 0 字节 | 1 |
| `bigint` | 可变 | 8 |
| 指针 | 8 字节 | 8 |

**字符串对象布局**：

```
字符串对象:
+----------------+----------------+----------------+
| GC 头部 (8 bytes)                              |
+----------------+----------------+----------------+
| Length (32b)   | Capacity (32b)                  |
+----------------+----------------+----------------+
| UTF-8 字节 (variable length, null-terminated)   |
+-------------------------------------------------+
```

**数组对象布局**：

```
数组对象:
+----------------+----------------+----------------+
| GC 头部 (8 bytes)                              |
+----------------+----------------+----------------+
| Length (32b)   | Capacity (32b)                  |
+----------------+----------------+----------------+
| 元素指针 (8 bytes) -> [T, T, T, ...]            |
+-------------------------------------------------+
```

数组的元素连续存储。对于 `dyn` 数组，每个元素包含一个类型标签。

### 12.6 内存安全保证

Ruyi 提供以下内存安全保证：

1. **无悬垂指针**: GC 确保存活对象永远不会被收集。ARC 确保对象仅在所有引用都被丢弃后才释放。
2. **无重复释放**: 每个对象恰好释放一次。
3. **无释放后使用**: 被收集/释放的对象永远不会被访问。
4. **无缓冲区溢出**: 数组边界在运行时检查（在编译器可证明安全时可以优化掉）。
5. **无未初始化内存**: 所有变量在使用前都已初始化（编译时检查）。

---

## 13. 异常语义

### 13.1 异常类型系统

Ruyi 中的所有异常都是内置 `Error` 类型的子类型：

```
Error
  |- TypeError
  |- RangeError
  |- NullAssertionError
  |- RuntimeError
  |- IOError
  |- CustomError (user-defined)
```

**异常对象布局**：

```
异常对象:
+----------------+----------------+----------------+
| GC 头部 (8 bytes)                              |
+----------------+----------------+----------------+
| TypeTag（标识具体的错误子类型）                  |
+----------------+----------------+----------------+
| message: string                                  |
+----------------+----------------+----------------+
| stackTrace: Array<Frame>                         |
+----------------+----------------+----------------+
```

### 13.2 try/catch/finally 求值顺序

#### 13.2.1 try 块求值

`try` 块首先被求值。若未抛出异常：
1. `try` 块正常完成。
2. `finally` 块（若存在）执行。
3. 控制流继续到整个 try 语句之后。

若在 `try` 块求值期间抛出异常：
1. `try` 块的求值被中断。
2. 异常传播到 `catch` 子句。

#### 13.2.2 catch 子句求值

当异常到达 `catch` 子句时：

1. 将异常类型与 catch 模式进行比较。
2. 若模式匹配，则将异常绑定到 catch 变量并执行 catch 块。
3. 若模式不匹配，则异常传播到下一个封闭的 `catch` 或 `finally`。

```ruyi
try {
  riskyOperation();
} catch (e: TypeError) {
  // 处理 TypeError 及其子类型
} catch (e: Error) {
  // 处理所有其他错误
}
```

**Catch 模式匹配**：

```
catch (e: T) matches exception E  if  E <: T
```

多个 catch 子句按顺序尝试。第一个匹配的子句处理异常。

#### 13.2.3 finally 块求值

`finally` 块**总是**执行，无论 `try` 块如何退出：

| try 退出方式 | finally 行为 |
|----------|-----------------|
| 正常完成 | 在 try 之后执行 |
| 抛出异常 | 在异常传播之前执行 |
| `return` 语句 | 在 return 之前执行 |
| `break` / `continue` | 在控制转移之前执行 |
| catch 中抛出异常 | 在新异常传播之前执行 |

**try/catch/finally 求值顺序**：

```
try { A } catch (e) { B } finally { C }
```

1. 求值 `A`。
2. 若 `A` 抛出异常 `E`：
   a. 将 `E` 与 catch 模式匹配。
   b. 若匹配，求值 `B`。
   c. 若不匹配，跳过 `B`，传播 `E`。
3. 求值 `C`（总是）。
4. 若 `B` 抛出了新异常，传播它。
5. 若 `A` 抛出异常且没有 catch 匹配，传播原始异常。

#### 13.2.4 finally 与异常抑制

若 `finally` 块在另一个异常正在传播时抛出了异常，则 `finally` 异常**取代**原始异常：

```ruyi
try {
  throw Error("original");
} finally {
  throw Error("finally");    // 此异常取代 "original"
}
// 捕获的异常: "finally"
```

### 13.3 异常传播

异常沿调用栈向上传播，直到找到匹配的 `catch` 子句：

```
fn a() { throw Error("oops"); }
fn b() { a(); }
fn c() {
  try { b(); }
  catch (e: Error) { /* handles it */ }
}
```

**传播步骤**：

1. 异常在 `a()` 中抛出。
2. `a()` 没有 catch，因此异常传播到 `b()`。
3. `b()` 没有 catch，因此异常传播到 `c()`。
4. `c()` 有匹配的 catch，因此异常被处理。
5. 传播路径上的所有 `finally` 块按顺序（从内到外）执行。

若在任何层级都找不到匹配的 `catch`，程序将以未处理异常错误终止。

### 13.4 异常与类型系统

异常与类型系统的交互方式如下：

1. **无受检异常**: Ruyi 不要求函数声明它们抛出的异常。所有异常都是非受检的。
2. **`never` 返回类型**: 总是抛出的函数可以用返回类型 `never` 注解：

```ruyi
fn fail(message: string): never {
  throw Error(message);
}
```

`never` 类型是底类型。它是所有类型的子类型，意味着 `never` 表达式可以在任何上下文中使用。

3. **析构函数中的异常安全**: 当异常传播通过作用域时，所有局部变量都会被销毁。若析构函数（drop 处理程序）在展开期间抛出异常，程序将中止（防止双重 panic）。

### 13.5 零开销异常实现

Ruyi 异常使用**零开销异常表**（基于 Itanium ABI / DWARF EH）：

1. **正常路径**: 不抛出异常时没有开销。编译器生成异常表（而非内联检查）。
2. **抛出路径**: `throw` 查找当前指令指针的异常表，找到最近的着陆垫（landing pad），并跳转到它。
3. **着陆垫**: 着陆垫将异常类型与 catch 子句匹配，并相应分派。

这种方法确保正常执行路径不受异常处理的开销影响。

---

## 14. 异步/Await 语义

### 14.1 Future/Promise 模型

Ruyi 的异步模型基于 **Future** 模式。`async` 函数返回 `Future<T>`：

```ruyi
async fn fetchData(url: string): string {
  let response = await http.get(url);
  return response.body;
}

// 调用点:
let future: Future<string> = fetchData("https://example.com");
let result: string = await future;
```

**Future 语义**：

1. `Future<T>` 表示一个最终会产生类型 `T` 值的计算。
2. `Future` 是**惰性**的：它在被 await 或显式生成之前不会开始执行。
3. `await` 挂起当前异步函数直到 future 完成。
4. 当 future 完成时，`await` 以结果值恢复执行。

### 14.2 异步函数转换

`async` 函数被编译器转换为**状态机**：

```ruyi
// 源码:
async fn example(x: int): int {
  let a = await fetchA(x);     // 挂起点 1
  let b = await fetchB(a);     // 挂起点 2
  return a + b;
}
```

转换为：

```
enum ExampleState {
  Start(x: int),
  AfterFetchA(a: int),
  AfterFetchB(b: int),
}

fn example_step(state: &mut ExampleState) -> Poll<int> {
  match state {
    Start(x) => {
      match fetchA(x).poll() {
        Ready(a) => { *state = AfterFetchA(a); }
        Pending => return Pending;
      }
    }
    AfterFetchA(a) => {
      match fetchB(a).poll() {
        Ready(b) => { *state = AfterFetchB(b); }
        Pending => return Pending;
      }
    }
    AfterFetchB(b) => {
      return Ready(b + a);    // 注意: a 被捕获在状态中
    }
  }
}
```

**状态机属性**：

1. 每个 `await` 点成为状态机中的一个状态。
2. 跨越 `await` 点的局部变量存储在状态枚举中。
3. 状态机实现 `poll()` 方法，返回 `Poll<T>`：
   - `Ready(T)`: 计算完成。
   - `Pending`: 计算尚未完成，调用者应再次 poll。

### 14.3 绿色线程调度

Ruyi 使用**工作窃取调度器**管理绿色线程：

#### 14.3.1 调度器架构

```
+---------------------------------------------------+
|                    调度器                           |
|  +-----------+  +-----------+  +-----------+      |
|  | Worker 0  |  | Worker 1  |  | Worker N  |      |
|  | Task Queue|  | Task Queue|  | Task Queue|      |
|  +-----------+  +-----------+  +-----------+      |
+---------------------------------------------------+
```

1. **工作者**: 执行绿色线程的 OS 线程。每个工作者有一个本地任务队列。
2. **任务队列**: 就绪 future 的双端队列（deque）。工作者从底部 push，从底部 pop。
3. **工作窃取**: 当工作者队列为空时，它从其他工作者队列的顶部窃取任务。

#### 14.3.2 任务生命周期

1. **生成**: `async fn` 调用创建一个 future。`spawn(future)` 将其 push 到当前工作者的队列。
2. **Poll**: 工作者 pop 一个 future 并调用 `poll()`。
3. **就绪**: 若 `poll()` 返回 `Ready`，future 完成。结果已存储。
4. **挂起**: 若 `poll()` 返回 `Pending`，future 被重新入队。工作者执行下一个 future。
5. **唤醒**: 当 I/O 操作完成时，它调用关联 future 的 `wake()`，将其重新入队。

#### 14.3.3 阻塞操作

阻塞操作（如同步 I/O）不得从绿色线程调用，因为它们会阻塞工作者线程。应改用异步 I/O 或 `spawn_blocking`：

```ruyi
// 错误: 阻塞工作者
let data = fs.readFileSync("file.txt");

// 正确: 异步 I/O
let data = await fs.readFile("file.txt");

// 或: 卸载到阻塞线程池
let data = await spawn_blocking(|| fs.readFileSync("file.txt"));
```

### 14.4 异步与异常的交互

异步函数中的异常通过 Future 传播：

```ruyi
async fn risky(): int {
  throw Error("async error");
}

try {
  let result = await risky();
} catch (e: Error) {
  // 捕获异步错误
}
```

**规则**：

1. 若异步函数抛出异常，`Future` 以错误状态完成。
2. 对已出错的 future 进行 `await` 会在等待上下文中重新抛出异常。
3. 异常不会跨越任务边界传播，除非通过 `await` 显式传播。

### 14.5 异步迭代器

异步迭代器异步产生值：

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

**AsyncIterator 特征**：

```ruyi
trait AsyncIterator<T> {
  fn next(self): Future<T?>;
}
```

`for await` 脱糖为：

```ruyi
let iter = readLines(file);
while let Some(line) = await iter.next() {
  print(line);
}
```

---

## 15. 模块语义

### 15.1 模块结构

每个源文件是一个模块。模块基于文件系统组织成层次化命名空间：

```
src/
  main.ry          -> module main
  utils.ry         -> module utils
  http/
    client.ry      -> module http::client
    server.ry      -> module http::server
```

### 15.2 导入解析

导入语句将模块路径解析为源文件：

```ruyi
import { add, subtract } from "./math";
import * as utils from "./utils";
import HttpClient from "./http/client";
```

**解析算法**：

1. **相对路径** (`./` 或 `../`): 相对于导入文件所在目录解析。
2. **绝对路径** (无前缀): 从项目的源根目录解析。
3. **标准库路径** (`std::`): 从标准库解析。

**解析步骤**：

1. 将模块路径转换为文件路径：
   - `./math` -> `./math.ry` 或 `./math/index.ry`
   - `http/client` -> `http/client.ry` 或 `http/client/index.ry`
2. 检查文件是否存在。
3. 若未找到，检查该名称目录下的 `index.ry`。
4. 若仍未找到，报告模块解析错误。

### 15.2.1 标准库 (stdlib)

Ruyi 附带标准库（`stdlib/`），提供核心类型、数据结构和系统工具。stdlib 位于 `$RUYI_HOME/stdlib`。

**RUYI_HOME**:

Ruyi 使用 `RUYI_HOME` 环境变量定位安装目录：

| 路径 | 说明 |
|------|------|
| `$RUYI_HOME/bin` | 编译器二进制文件（`ruyic` 等） |
| `$RUYI_HOME/stdlib` | 标准库模块 |

如果未设置 `RUYI_HOME`，编译器会回退到在当前工作目录的相对路径下查找 `stdlib/` 目录（适用于开发环境）。

**stdlib 模块布局**：

| 模块 | 说明 |
|------|------|
| `core` | 基础类型方法（`string`、`int`、`float`、`bool` 的 trait impl，自动加载） |
| `option` | `Option<T>`（`Some`/`None`）和 `Result<T, E>`（`Ok`/`Err`）用于可空值处理和错误处理（自动加载） |
| `error` | 错误层次结构（`Error`、`TypeError`、`RuntimeError`、`RangeError`、`AssertionError`、`ArgumentError`、`NullError`、`ArithmeticError`、`IteratorError`、`ParseError`）以及 `assert()` 和 `assertNotNull()` |
| `collections` | 泛型集合（`Array<T>`、`Map<K, V>`、`Set<T>`）和 `Iterator<T>` 特征 |
| `string` | 纯字符串工具函数（`join`、`fromCharCode`、`fromCharCodes`、`concat`、`template`、`processTemplate`） |
| `io` | 控制台 I/O（`readLine`）和文件操作（`File.readText`、`File.writeText`、`File.readLines`、`File.exists`、`File.mkdir` 等），含异步变体 |
| `path` | 路径操作（`Path.join`、`Path.basename`、`Path.dirname`、`Path.extname`、`Path.isAbsolute`、`Path.normalize`、`Path.resolve` 等） |
| `process` | 进程管理（`Process.exec`、`Process.spawn`、`Process.create`）、环境变量（`getEnv`、`setEnv`）和系统信息（`getPID`、`getPlatform`、`getCPUCount` 等） |

**导入 stdlib 模块**：

```ruyi
// 通过文件名导入 stdlib 模块
import { readLine } from "./io";
import { readFile, writeFile } from "./fs";
import { Path } from "./path";
import { Process, getEnv } from "./process";
import { assert, assertNotNull } from "./error";
import { Option, Some, None, Result, Ok, Err } from "./option";
import { Array, Map, Set, Iterator } from "./collections";
```

**预声明的内置符号**：

以下符号无需导入即可使用（由类型检查器预声明）：

| 符号 | 类型 | 说明 |
|------|------|------|
| `print` | `fn(dyn): void` | 输出到 stdout |
| `spawn` | `fn(dyn): dyn` | 生成异步任务 |
| `toString` | `fn(dyn): string` | 将任意值转换为字符串 |
| `Error` | `fn(string): Error` | Error 构造函数 |

### 15.3 循环依赖检测

Ruyi 在编译时检测循环依赖：

```
// a.ry
import { foo } from "./b";

// b.ry
import { bar } from "./a";    // ERROR: 循环依赖
```

**检测算法**：

1. 构建**模块依赖图**，其中节点是模块，边是导入关系。
2. 在图上执行**深度优先搜索**（DFS）。
3. 若发现回边（访问到已在当前 DFS 栈中的节点），则存在循环依赖。
4. 报告涉及的模块的完整路径。

**解决方案**: 循环依赖必须通过以下方式打破：
- 将共享代码提取到第三个模块中。
- 使用前向声明（仅用于类型）。
- 重新组织模块层次结构。

### 15.4 导出可见性

默认情况下，模块中的所有顶层声明都是**私有**的（仅在模块内可见）。`export` 关键字使其变为公共：

```ruyi
// math.ry
fn add(a: int, b: int): int { ... }       // 私有
export fn subtract(a: int, b: int): int { ... }  // 公共
```

**可见性级别**：

| 级别 | 关键字 | 可见范围 |
|-------|---------|------------|
| 私有 | (默认) | 仅当前模块 |
| 公共 | `export` | 任何导入此模块的模块 |

**重新导出**：

```ruyi
export { add, subtract } from "./math";
```

重新导出使得导入的名称对导入当前模块的模块可用。

### 15.5 模块初始化

当模块首次被导入时，其顶层语句按顺序执行：

```ruyi
// config.ry
export let config = loadConfig();    // 在首次导入时执行
```

**初始化规则**：

1. 每个模块恰好初始化一次（单例初始化）。
2. 初始化顺序遵循依赖图（依赖项在依赖者之前初始化）。
3. 循环依赖在初始化开始前被检测。
4. 若初始化抛出异常，程序终止。

### 15.6 名称解析与遮蔽

名称按以下顺序解析：

1. 局部作用域（当前块）。
2. 函数作用域（参数和局部变量）。
3. 模块作用域（当前模块的顶层声明）。
4. 导入名称（来自 `import` 语句）。
5. 内置名称（`int`, `string`, `null` 等）。

**遮蔽**: 内层作用域可以遮蔽外层作用域的名称：

```ruyi
let x = 1;           // 模块级 x

fn example() {
  let x = 2;         // 遮蔽模块级 x
  print(x);          // 输出 2
}
```

允许遮蔽，但若被遮蔽的名称来自导入模块，则会产生警告。

---

## 16. 宏语义

### 16.1 声明式宏展开

Ruyi 宏是**声明式**的（基于模式），类似于 Rust 的 `macro_rules!`。宏在编译时、类型检查之前展开。

```ruyi
macro debug {
  ($expr) => {
    print("DEBUG: " + stringify($expr) + " = " + $expr);
  }
}

debug(x + 1);    // 展开为: print("DEBUG: " + "x + 1" + " = " + (x + 1));
```

### 16.2 宏展开规则

#### 16.2.1 模式匹配

宏规则按顺序尝试。第一个模式匹配输入的规则会被使用：

```ruyi
macro vec {
  () => {
    Array::new()
  }
  ($elem) => {
    { let v = Array::new(); v.push($elem); v }
  }
  ($($elem),*) => {
    {
      let v = Array::new();
      $(v.push($elem);)*
      v
    }
  }
}
```

**模式元素**：

| 模式 | 匹配 |
|---------|---------|
| `$name` | 单个表达式、语句或词法单元 |
| `$(...)` | 重复模式 |
| `$(...),*` | 零个或多个，逗号分隔 |
| `$(...),+` | 一个或多个，逗号分隔 |
| `$(...)?` | 零个或一个 |

#### 16.2.2 展开过程

1. **解析**: 宏调用被解析为词法单元序列（而非表达式）。
2. **匹配**: 将词法单元序列按顺序与每条规则的模式进行匹配。
3. **替换**: 元变量 (`$name`) 被替换为匹配到的词法单元。
4. **重复**: 重复模式 (`$(...)*`) 对每个匹配组进行展开。
5. **发出**: 生成的词法单元替换源代码中的宏调用。
6. **重新解析**: 发出的词法单元被重新解析为 Ruyi 代码。

### 16.3 宏卫生

Ruyi 宏是**卫生的**: 宏引入的标识符不会与调用作用域中的标识符冲突。

```ruyi
macro swap {
  ($a, $b) => {
    let temp = $a;    // 'temp' 是卫生的
    $a = $b;
    $b = temp;
  }
}

let temp = 100;
let x = 1;
let y = 2;
swap(x, y);
// 宏内部的 'temp' 不会遮蔽外部的 'temp'
// 外部 'temp' 仍然是 100
```

**卫生实现**：

1. 每次宏展开被分配一个唯一的**语法上下文**（整数 ID）。
2. 宏引入的标识符被标记上此上下文。
3. 在名称解析期间，标识符按名称和上下文匹配。
4. 来自调用作用域的标识符（作为元变量传递）保留其原始上下文。

**例外**: `stringify` 和 `quote` 内置宏函数作用于其参数的表面语法，忽略卫生。

### 16.4 内置宏函数

| 函数 | 说明 |
|----------|-------------|
| `stringify($x)` | 将匹配到的词法单元转换为字符串字面量 |
| `file!()` | 展开为当前文件路径（字符串） |
| `line!()` | 展开为当前行号（int） |
| `column!()` | 展开为当前列号（int） |

### 16.5 宏展开顺序

宏以**不动点**过程展开：

1. 扫描 AST 中的宏调用。
2. 展开所有找到的宏。
3. 若展开后的输出包含新的宏调用，从第 1 步重复。
4. 当不再发现宏调用，或达到最大深度时停止（默认：64）。

**最大深度**: 为防止无限展开，编译器将宏展开深度限制为 64 层。若超过此限制，则报告编译时错误。

### 16.6 宏与模块交互

一个模块中定义的宏可以在另一个模块中使用：

```ruyi
// macros.ry
export macro debug {
  ($expr) => { print("DEBUG: " + stringify($expr)); }
}

// main.ry
import { debug } from "./macros";
debug(x);    // 正常工作
```

**导出规则**：

1. 宏必须使用 `export macro` 显式导出。
2. 导入的宏在导入模块的上下文中展开。
3. 宏卫生确保导出的宏不会意外捕获导入模块中的名称。

---

*语义与类型系统规范结束*

*词法与语法规范结束*
