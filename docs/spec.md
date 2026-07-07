# Ruyi Language Specification

## Lexical and Syntax Specification

> **Version**: 0.5.1-draft
> **Date**: 2026-05-05
> **Status**: Working Draft — aligned with current implementation

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Lexical Structure](#2-lexical-structure)
   - 2.1 [Source Text](#21-source-text)
   - 2.2 [Comments](#22-comments)
   - 2.3 [Tokens](#23-tokens)
   - 2.4 [Keywords](#24-keywords)
   - 2.5 [Identifiers](#25-identifiers)
   - 2.6 [Literals](#26-literals)
   - 2.7 [Operators and Punctuators](#27-operators-and-punctuators)
   - 2.8 [Whitespace and Line Terminators](#28-whitespace-and-line-terminators)
3. [Syntax Grammar](#3-syntax-grammar)
   - 3.1 [Notation](#31-notation)
   - 3.2 [Source File](#32-source-file)
   - 3.3 [Declarations](#33-declarations)
   - 3.4 [Statements](#34-statements)
   - 3.5 [Expressions](#35-expressions)
   - 3.6 [Patterns](#36-patterns)
   - 3.7 [Type Annotations](#37-type-annotations)
   - 3.8 [Modules](#38-modules)
4. [Pattern Matching](#4-pattern-matching)
5. [Generics](#5-generics)
6. [Null Safety](#6-null-safety)
7. [JavaScript Feature Removal](#7-javascript-feature-removal)
8. [Type System Semantics](#8-type-system-semantics)
9. [Nullable Type Semantics](#9-nullable-type-semantics)
10. [Generics Semantics](#10-generics-semantics)
11. [Trait Semantics](#11-trait-semantics)
12. [Memory Model](#12-memory-model)
13. [Exception Semantics](#13-exception-semantics)
14. [Async/Await Semantics](#14-asyncawait-semantics)
15. [Module Semantics](#15-module-semantics)
16. [Macro Semantics](#16-macro-semantics)

---

## 1. Introduction

Ruyi is a compiled, general-purpose programming language built on the syntactic foundation of JavaScript strict mode. It removes problematic JavaScript features while retaining familiar syntax. Ruyi targets native machine code via LLVM, providing high performance across platforms.

This document defines the lexical structure and syntax grammar of Ruyi. It uses an ECMAScript-style specification format with BNF grammar notation.

---

## 2. Lexical Structure

### 2.1 Source Text

Ruyi source text is a sequence of Unicode code points encoded in UTF-8. The source text is scanned from left to right, converting sequences of code points into tokens.

### 2.2 Comments

Comments are treated as whitespace by the lexer. They do not produce tokens.

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

Single-line comments terminate at the first line terminator. Multi-line comments may span multiple lines. Documentation comments are preserved for tooling but carry no syntactic meaning.

### 2.3 Tokens

The input stream is converted into a sequence of tokens. Each token is the longest possible sequence of code points that forms a valid token.

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

### 2.4 Keywords

Keywords are reserved identifiers that carry special syntactic meaning. They cannot be used as identifiers.

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

**Keyword descriptions**:

| Keyword | Purpose |
|---------|---------|
| `let` | Mutable variable declaration |
| `const` | Immutable variable declaration |
| `fn` | Function declaration |
| `class` | Class declaration |
| `trait` | Trait (interface) declaration |
| `impl` | Trait implementation block |
| `match` | Pattern matching expression |
| `if` | Conditional statement/expression |
| `else` | Alternative branch of conditional |
| `for` | Loop statement |
| `while` | Conditional loop statement |
| `return` | Return from function |
| `throw` | Raise an exception |
| `try` | Begin exception handling block |
| `catch` | Handle an exception |
| `finally` | Execute cleanup code |
| `async` | Declare asynchronous function |
| `await` | Wait for async result |
| `import` | Import from module |
| `export` | Export from module |
| `macro` | Declare a macro |
| `type` | Type alias declaration |
| `true` | Boolean true literal |
| `false` | Boolean false literal |
| `null` | Null literal |
| `self` | Reference to current instance (in methods) |
| `super` | Reference to parent class |
| `this` | Reference to current context |
| `in` | Key membership / for-in loop |
| `instanceof` | Type check operator |
| `typeof` | Runtime type inspection operator |
| `void` | Void expression operator |
| `delete` | Property deletion operator (parsed, limited codegen) |
| `as` | Type cast / pattern alias |
| `extends` | Class inheritance / trait supertraits |
| `dyn` | Dynamic dispatch / trait object |
| `static` | Static class member |
| `get` | Getter method definition |
| `set` | Setter method definition |
| `new` | Object instantiation |
| `of` | Value iteration / for-of loop |
| `break` | Exit enclosing loop |
| `continue` | Skip to next loop iteration |
| `yield` | Generator yield (parsed; codegen is no-op) |
| `_` | Wildcard pattern (match/destructuring) |

### 2.5 Identifiers

Identifiers name variables, functions, types, and other program entities.

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

Identifiers are case-sensitive. `myVar` and `myvar` are distinct identifiers. Identifiers must not match any keyword.

**Examples**:
```
x
count
_myVar
$element
firstName
camelCaseName
```

### 2.6 Literals

Literals represent fixed values in source code.

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

#### 2.6.1 Null Literal

```
NullLiteral ::
  null
```

The `null` literal represents the absence of a value. Ruyi has no `undefined`.

#### 2.6.2 Boolean Literals

```
BooleanLiteral :: one of
  true  false
```

#### 2.6.3 Numeric Literals

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

**Numeric literal examples**:
```
42          // decimal integer
3.14        // decimal float
1e10        // scientific notation
0xFF        // hexadecimal (255)
0o77        // octal (63)
0b1010      // binary (10)
100n        // big integer
```

#### 2.6.4 String Literals

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

**String literal examples**:
```
"hello"
'world'
"line1\nline2"
"tab\there"
"unicode: \u{1F600}"
```

#### 2.6.5 Template Literals

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

**Template literal examples**:
```
`hello ${name}`
`result: ${x + y}`
`multi
line
string`
```

#### 2.6.6 Array Literals

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

#### 2.6.7 Object Literals

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

### 2.7 Operators and Punctuators

Operators and punctuators are sequences of one or more code points that carry special syntactic meaning.

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

**Operator precedence** (highest to lowest):

| Precedence | Operators | Associativity |
|------------|-----------|---------------|
| 18 | `?.` `.` `()` `[]` | left-to-right |
| 17 | `++` `--` `!` (prefix) `~` `+` (unary) `-` (unary) `typeof` `void` `delete` `await` | right-to-left |
| 16 | `**` | right-to-left |
| 15 | `*` `/` `%` | left-to-right |
| 14 | `+` `-` | left-to-right |
| 13 | `<<` `>>` `>>>` | left-to-right |
| 12 | `<` `>` `<=` `>=` `in` `instanceof` | left-to-right |
| 11 | `===` `!==` `==` `!=` | left-to-right |
| 10 | `&` | left-to-right |
| 9 | `^` | left-to-right |
| 8 | `\|` | left-to-right |
| 7 | `&&` | left-to-right |
| 6 | `\|\|` | left-to-right |
| 5 | `??` | left-to-right |
| 4 | `?:` (ternary) | right-to-left |
| 3 | `=>` | right-to-left |
| 2 | `=` `+=` `-=` `*=` `/=` `%=` `**=` `&=` `\|=` `^=` `<<=` `>>=` `>>>=` `&&=` `\|\|=` `??=` | right-to-left |
| 1 | `,` | left-to-right |

**Postfix operators** (highest precedence, applied after all above):

| Operator | Description |
|----------|-------------|
| `!` | Null assertion: `e!` asserts `e` is not null |
| `++` | Post-increment (parsed; codegen via prefix) |
| `--` | Post-decrement (parsed; codegen via prefix) |

**Key Ruyi operators**:

| Operator | Name | Description |
|----------|------|-------------|
| `===` | Strict equality | Value and type equality (no coercion) |
| `!==` | Strict inequality | Negation of strict equality |
| `==` | Legacy equality | Parsed; codegen maps to `===` behavior |
| `!=` | Legacy inequality | Parsed; codegen maps to `!==` behavior |
| `?.` | Optional chaining | Safe property access on nullable values |
| `??` | Nullish coalescing | Returns right operand if left is null |
| `!` (postfix) | Null assertion | Asserts nullable value is not null; throws at runtime if null |
| `=>` | Arrow | Defines arrow functions and match arms |
| `...` | Spread/rest | Spreads elements or collects rest parameters |
| `**` | Exponentiation | Power operator |

### 2.8 Whitespace and Line Terminators

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

Whitespace separates tokens but carries no semantic meaning. Line terminators affect automatic semicolon insertion.

---

## 3. Syntax Grammar

### 3.1 Notation

The syntax grammar uses Extended Backus-Naur Form (EBNF). Conventions:

- `::` introduces a production rule
- `::=` introduces a recursive production
- `|` separates alternatives
- `[ ]` marks optional elements
- `( )` groups elements
- `*` means zero or more repetitions
- `+` means one or more repetitions
- `opt` subscript means optional
- **bold** terminals are literal tokens
- *Italic* non-terminals reference other productions
- `one of` lists single-token alternatives

### 3.2 Source File

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

### 3.3 Declarations

#### 3.3.1 Variable Declarations

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

**Examples**:
```ruyi
let x = 42;
const PI = 3.14159;
let name: string = "Ruyi";
let { first, last } = person;
let [head, ...tail] = list;
const { x: a, y: b } = point;
```

#### 3.3.2 Function Declarations

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

**Examples**:
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

#### 3.3.3 Arrow Functions

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

**Examples**:
```ruyi
let double = (x) => x * 2;
let greet = (name) => { print("Hi, " + name); };
let add = (a, b) => a + b;
let fetch = async (url) => await http.get(url);
```

#### 3.3.4 Class Declarations

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

**Examples**:
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

#### 3.3.5 Trait Declarations

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

**Examples**:
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

#### 3.3.6 Type Alias Declarations

```
TypeAliasDeclaration ::
  type BindingIdentifier TypeParametersopt = TypeAnnotation ;
```

**Examples**:
```ruyi
type Result<T, E> = Ok<T, E> | Err<T, E>;
type Callback<T> = fn(T) -> void;
type Point2D = { x: float, y: float };
```

#### 3.3.7 Impl Declarations

Impl blocks provide trait implementations for specific types:

```
ImplDeclaration ::
  impl TypeParametersopt TraitName TypeArgs? for TypeAnnotation { ClassBodyopt }

TypeArgs ::
  < TypeList >
```

**Examples**:
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

#### 3.3.8 Macro Declarations

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

**Examples**:
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

### 3.4 Statements

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

#### 3.4.1 Block Statement

```
BlockStatement ::
  { StatementListopt }

StatementList ::
  StatementListItem
  StatementList StatementListItem
```

#### 3.4.2 Expression Statement

```
ExpressionStatement ::
  Expression ;
```

An expression statement must not begin with `{` or `fn` to avoid ambiguity with block statements and function declarations.

#### 3.4.3 If Statement

```
IfStatement ::
  if ( Expression ) Statement ElseClauseopt

ElseClause ::
  else Statement
```

**Examples**:
```ruyi
if (x > 0) {
  print("positive");
} else if (x < 0) {
  print("negative");
} else {
  print("zero");
}
```

#### 3.4.4 While Statement

```
WhileStatement ::
  while ( Expression ) Statement
```

**Examples**:
```ruyi
while (i < 10) {
  print(i);
  i = i + 1;
}
```

#### 3.4.5 For Statement

```
ForStatement ::
  for ( ForInitializeropt ; Expressionopt ; ForUpdateopt ) Statement

ForInitializer ::
  LexicalDeclaration
  Expression

ForUpdate ::
  Expression
```

**Examples**:
```ruyi
for (let i = 0; i < 10; i = i + 1) {
  print(i);
}

for (let i = items.length - 1; i >= 0; i = i - 1) {
  process(items[i]);
}
```

#### 3.4.6 For-In Statement

```
ForInStatement ::
  for ( let BindingIdentifier in Expression ) Statement
```

Iterates over the keys of an object or indices of an array.

**Examples**:
```ruyi
for (let key in obj) {
  print(key + ": " + obj[key]);
}
```

#### 3.4.7 For-Of Statement

```
ForOfStatement ::
  for ( let BindingIdentifier of Expression ) Statement
  for ( let BindingIdentifier of async Expression ) Statement
```

Iterates over the values of an iterable. The `async` form iterates over async iterables.

**Examples**:
```ruyi
for (let item of items) {
  process(item);
}

for (let line of async readLines(file)) {
  print(line);
}
```

#### 3.4.8 Return Statement

```
ReturnStatement ::
  return Expressionopt ;
```

Returns from the enclosing function. If no expression is provided, returns `null`.

**Examples**:
```ruyi
return 42;
return;
return a + b;
```

#### 3.4.9 Throw Statement

```
ThrowStatement ::
  throw Expression ;
```

Raises an exception. The expression must evaluate to an `Error` or subtype.

**Examples**:
```ruyi
throw Error("something went wrong");
throw TypeError("expected string");
```

#### 3.4.10 Try Statement

```
TryStatement ::
  try Block CatchClauseopt FinallyClauseopt

CatchClause ::
  catch ( BindingPattern TypeAnnotationopt ) Block
  catch Block

FinallyClause ::
  finally Block
```

**Examples**:
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

#### 3.4.11 Match Statement

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

**Examples**:
```ruyi
match (value) {
  1 => { print("one"); }
  2 => { print("two"); }
  n if (n > 10) => { print("big: " + n); }
  _ => { print("other"); }
}
```

#### 3.4.12 Break and Continue

```
BreakStatement ::
  break ;
  break IdentifierName ;

ContinueStatement ::
  continue ;
  continue IdentifierName ;
```

`break` exits the innermost enclosing loop. With a label, it exits the labeled statement. `continue` skips to the next iteration of the innermost enclosing loop.

**Examples**:
```ruyi
outer: for (let i = 0; i < 10; i = i + 1) {
  for (let j = 0; j < 10; j = j + 1) {
    if (j > 5) {
      break outer;
    }
  }
}
```

#### 3.4.13 Yield Statement

```
YieldStatement ::
  yield Expressionopt ;
```

The `yield` statement suspends a generator function and produces a value. Currently parsed as a statement; codegen treats it as a no-op.

**Examples**:
```ruyi
fn* countUp(limit: int) {
  for (let i = 0; i < limit; i = i + 1) {
    yield i;
  }
}
```

#### 3.4.14 Labeled Statement

```
LabeledStatement ::
  IdentifierName : Statement
```

Labels a statement for use with `break` and `continue`.

**Examples**:
```ruyi
loop: while (true) {
  if (done) {
    break loop;
  }
}
```

#### 3.4.15 Empty Statement

```
EmptyStatement ::
  ;
```

### 3.5 Expressions

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

#### 3.5.1 Optional Chaining

```
OptionalChain ::
  ?. IdentifierName
  ?. [ Expression ]
  ?. Arguments
  ?. TemplateLiteral
```

The `?.` operator short-circuits if the left operand is `null`. The entire chain evaluates to `null`.

**Examples**:
```ruyi
let name = user?.profile?.name;
let first = arr?.[0];
let result = obj?.method?.();
```

#### 3.5.2 Nullish Coalescing

```
NullishCoalescingExpression ::
  LogicalOrExpression
  NullishCoalescingExpression ?? LogicalOrExpression
```

The `??` operator returns the right operand if the left operand is `null`. Otherwise returns the left operand.

**Examples**:
```ruyi
let name = user?.name ?? "anonymous";
let count = config.count ?? 0;
let value = maybeNull ?? fallback ?? default;
```

#### 3.5.3 Await Expression

```
AwaitExpression ::
  await UnaryExpression
```

The `await` operator suspends execution of an `async` function until the operand resolves. It may only appear inside `async` functions.

**Examples**:
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

### 3.5.4 If Expression

The `if` construct can be used as an expression that evaluates to a value:

```
IfExpression ::
  if ( Expression ) Expression ElseExpressionopt

ElseExpression ::
  else Expression
```

Unlike the `if` statement, the `if` expression does not use braces for its branches and always produces a value. If no `else` branch is provided and the condition is false, the expression evaluates to `null`.

**Examples**:
```ruyi
let result = if (x > 0) { "positive" } else { "non-positive" };
let max = if (a > b) { a } else { b };
let msg = if (ready) { "go" };  // msg is string? (null if not ready)
```

### 3.5.5 Match Expression

The `match` construct can be used as an expression:

```
MatchExpression ::
  match ( Expression ) { MatchArmsopt }
```

The match expression evaluates to the value of the matched arm's body. All arms must produce compatible types.

**Examples**:
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

### 3.5.6 New Expression

The `new` operator creates an instance of a class:

```
NewExpression ::
  new MemberExpression Arguments
```

**Examples**:
```ruyi
let point = new Point(1.0, 2.0);
let config = new Config({ debug: true });
```

### 3.5.7 Null Assert Expression

The postfix `!` operator asserts that a nullable value is not null:

```
NullAssertExpression ::
  Expression !
```

At runtime, if the value is `null`, a runtime error is thrown. The resulting type is the non-nullable form of the expression's type.

**Examples**:
```ruyi
let name: string? = getUser();
let safe: string = name!;  // throws if name is null
let len = name!.length;    // safe: name! is string
```

### 3.6 Patterns

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

### 3.7 Type Annotations

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

**Built-in type names**:

| Type | Description |
|------|-------------|
| `int` | 64-bit signed integer |
| `float` | 64-bit floating point |
| `bool` | Boolean (true/false) |
| `string` | UTF-8 string |
| `null` | Null type (only value: null) |
| `void` | No return value |
| `dyn` | Dynamic type (runtime checked) |
| `never` | Bottom type (unreachable) |
| `bigint` | Arbitrary precision integer |

**Special types**:

| Type | Description |
|------|-------------|
| `Future<T>` | Represents an async computation producing `T` |
| `dyn TraitName` | Trait object for dynamic dispatch |
| `Array<T>` | Array of elements of type `T` (desugars from `[T]`) |

**Examples**:
```ruyi
let x: int = 42;
let name: string? = null;
let fn: fn(int, int) -> int = add;
let items: Array<string> = [];
let point: { x: float, y: float } = { x: 0.0, y: 0.0 };
let printable: dyn Printable = getPrintable();
let future: Future<string> = fetchData(url);
```

### 3.8 Modules

#### 3.8.1 Import Declarations

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

**Examples**:
```ruyi
import { add, subtract } from "./math";
import * as utils from "./utils";
import HttpClient from "./http";
import HttpClient, { Request, Response } from "./http";
import "./side-effect-module";
```

#### 3.8.2 Export Declarations

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

**Examples**:
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

## 4. Pattern Matching

Ruyi provides first-class pattern matching through the `match` expression and `if-let` statement.

### 4.1 Match Expression

The `match` expression evaluates an expression against a series of patterns. The first matching arm executes.

```ruyi
match (value) {
  0 => { print("zero"); }
  1 | 2 => { print("one or two"); }
  n if (n > 10 && n < 20) => { print("teen: " + n); }
  100 => { print("hundred"); }
  _ => { print("other"); }
}
```

### 4.2 Destructuring in Match

Patterns can destructure objects and arrays:

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

### 4.3 If-Let Statement

The `if-let` statement combines pattern matching with conditional execution:

```
IfLetStatement ::
  if let Pattern = Expression Block ElseClauseopt
```

**Examples**:
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

### 4.4 While-Let Statement

```
WhileLetStatement ::
  while let Pattern = Expression Block
```

**Examples**:
```ruyi
while let Some(item) = iterator.next() {
  process(item);
}
```

---

## 5. Generics

Ruyi supports parametric polymorphism through generics. Generics work with functions, classes, traits, and type aliases.

### 5.1 Type Parameters

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

### 5.2 Generic Functions

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

### 5.3 Generic Classes

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

### 5.4 Trait Implementations

Types implement traits via `impl` blocks:

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

### 5.5 Type Inference with Generics

Ruyi infers generic type parameters from context:

```ruyi
fn wrap<T>(value: T): Option<T> {
  return Option.new(value);
}

let x = wrap(42);       // x: Option<int>
let y = wrap("hello");  // y: Option<string>
```

---

## 6. Null Safety

Ruyi eliminates the billion-dollar mistake through a sound nullable type system. There is no `undefined`. Only `null` exists, and nullable types must be explicitly declared.

### 6.1 Nullable Types

```
NullableType ::
  Type ?
```

A nullable type `T?` can hold values of type `T` or `null`. Non-nullable types cannot hold `null`.

```ruyi
let name: string = "Ruyi";    // cannot be null
let maybe: string? = null;     // can be null
let count: int = 42;           // cannot be null
let maybeCount: int? = null;   // can be null
```

### 6.2 Optional Chaining

```
OptionalChainExpression ::
  MemberExpression ?. IdentifierName
  MemberExpression ?. [ Expression ]
  MemberExpression ?. Arguments
```

The `?.` operator safely accesses properties on nullable values. If the receiver is `null`, the entire expression evaluates to `null` without throwing.

```ruyi
let user: User? = findUser(id);
let name = user?.name;           // string?
let city = user?.address?.city;  // string?
let len = user?.name?.length;    // int?
```

### 6.3 Nullish Coalescing

```
NullishCoalescingExpression ::
  Expression ?? Expression
```

The `??` operator provides a default value for nullable expressions:

```ruyi
let name = user?.name ?? "anonymous";    // string
let count = config.count ?? 0;           // int
let value = maybe ?? fallback ?? default; // T
```

### 6.4 Null Assertion

```
NullAssertion ::
  Expression !
```

The `!` operator asserts that a nullable value is not null. If the value is null at runtime, a runtime error is thrown.

```ruyi
let name: string? = getUser();
let safe: string = name!;  // throws if name is null
```

### 6.5 Type Narrowing

After a null check, the compiler narrows the type within the guarded scope:

```ruyi
let name: string? = getUser();

if (name !== null) {
  // name is narrowed to string here
  print(name.length);
}

// name is string? again here
```

---

## 7. JavaScript Feature Removal

Ruyi removes the following JavaScript features. Each removal includes the rationale and the Ruyi alternative.

### 7.1 Removed Features

| JS feature | Status | Ruyi Alternative | Rationale |
|------------|--------|-------------------|-----------|
| `undefined` | **Removed** | `null` | Two null-like values cause confusion. Ruyi uses a single `null` value. |
| `var` | **Removed** | `let`, `const` | `var` has function-scoped hoisting that causes bugs. Block-scoped `let`/`const` are safer. |
| `==` and `!=` | **Mapped** | `===` and `!==` | Parsed for compatibility; codegen maps to `===`/`!==` behavior. No implicit coercion. |
| Implicit type coercion | **Removed** | Explicit conversion | `"5" + 3` producing `"53"` while `"5" - 3` produces `2` is inconsistent. Ruyi requires explicit type conversion. |
| Prototype chain | **Removed** | `class`, `trait` | Prototype-based inheritance is confusing. Class-based inheritance is clearer and more familiar. |
| `with` statement | **Removed** | None | `with` makes static analysis impossible and introduces scope ambiguity. |
| `arguments` object | **Removed** | Rest parameters `...args` | The `arguments` object is array-like but not an array. Rest parameters are real arrays. |
| Automatic semicolon insertion edge cases | **Reduced** | Clearer ASI rules | Ruyi simplifies ASI to avoid the most surprising cases. |
| Octal literals with leading `0` | **Removed** | `0o` prefix | `0777` being octal but `0999` being decimal is confusing. Explicit `0o` prefix is clear. |
| `function` keyword | **Removed** | `fn` | Shorter, consistent with other declarations. |
| `function*` / generators | **Partial** | `yield` keyword parsed | `yield` is parsed as a keyword and statement; codegen treats it as a no-op. Full generator support is planned. |
| `this` binding complexity | **Simplified** | Lexical `self` | Arrow functions capture `self` lexically. Methods use `self` explicitly. |
| Dynamic property access with arbitrary strings | **Restricted** | Index signatures | Ruyi restricts dynamic property access to typed index signatures. |
| `eval()` | **Removed** | None | `eval` is a security risk and prevents optimization. |
| `delete` on object properties | **Limited** | `null` assignment | `delete` is parsed as a unary operator; full codegen support is limited. Prefer `obj.prop = null`. |
| `typeof` returning `"object"` for `null` | **Fixed** | `typeof null` returns `"null"` | The JS bug where `typeof null === "object"` is corrected. |
| Sparse arrays | **Removed** | Dense arrays with `null` | Sparse arrays have invisible holes. Ruyi arrays are always dense. |
| `Number`, `String`, `Boolean` wrapper objects | **Removed** | Primitive types only | Wrapper objects create confusing identity behavior (`new String("a") !== "a"`). |

### 7.2 Detailed Rationale

#### 7.2.1 `undefined` → `null`

JavaScript has two null-like values: `null` (intentional absence) and `undefined` (unintentional absence). This distinction is rarely useful and causes constant checking for both values.

Ruyi uses a single `null` value. Uninitialized variables default to `null` in dynamic contexts. Missing function parameters are `null`. Object properties that do not exist return `null`.

```ruyi
// JS: two ways to have "nothing"
let a = null;
let b;  // undefined

// Ruyi: one way
let a = null;
let b;  // null
```

#### 7.2.2 `var` → `let` / `const`

`var` declarations are function-scoped and hoisted, leading to confusing behavior:

```javascript
// JS: var leaks out of blocks
for (var i = 0; i < 10; i++) { }
console.log(i); // 10 - i is still accessible!
```

Ruyi only has `let` (mutable, block-scoped) and `const` (immutable, block-scoped):

```ruyi
// Ruyi: block-scoped
for (let i = 0; i < 10; i = i + 1) { }
// i is not accessible here
```

#### 7.2.3 `==` → `===`

JavaScript's `==` performs type coercion before comparison, leading to surprising results:

```javascript
// JS: confusing equality
0 == false       // true
"" == false      // true
[] == false      // true
null == undefined // true
"5" == 5         // true
```

Ruyi removes `==` and `!=` entirely. Only `===` (strict equality) and `!==` (strict inequality) exist:

```ruyi
// Ruyi: strict equality only
0 === false      // false (different types)
"5" === 5        // false (different types)
null === null    // true
```

#### 7.2.4 Implicit Coercion → Explicit Conversion

JavaScript silently converts between types in many contexts:

```javascript
// JS: implicit coercion
"5" + 3    // "53" (string concatenation)
"5" - 3    // 2 (numeric subtraction)
5 + null   // 5 (null coerced to 0)
5 + true   // 6 (true coerced to 1)
```

Ruyi requires explicit type conversion:

```ruyi
// Ruyi: explicit conversion
"5" + toString(3)    // "53"
parseInt("5") - 3    // 2
5 + 0                // 5 (no null coercion)
5 + 1                // 6 (no bool coercion)
```

#### 7.2.5 Prototype Chain → Class/Trait

JavaScript's prototype-based inheritance is powerful but confusing:

```javascript
// JS: prototype inheritance
function Animal(name) { this.name = name; }
Animal.prototype.speak = function() { };

function Dog(name) { Animal.call(this, name); }
Dog.prototype = Object.create(Animal.prototype);
Dog.prototype.bark = function() { };
```

Ruyi uses familiar class syntax:

```ruyi
// Ruyi: class inheritance
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

## Appendix A: Complete Token Reference

### A.1 Keyword Tokens

```
let, const, fn, class, trait, impl, dyn, match, if, else, for, while,
return, throw, try, catch, finally, async, await, import,
export, macro, type, true, false, null, self, super, this,
in, instanceof, typeof, void, delete, as, extends, static,
get, set, new, of, break, continue, yield, _
```

### A.2 Operator Tokens

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

### A.3 Punctuator Tokens

```
{, }, (, ), [, ],
., ,, ;, :, ?,
@, #, ..., ::, $,
<, >
```

### A.4 Literal Forms

```
null, true, false
42, 3.14, 1e10, 0xFF, 0o77, 0b1010, 100n
"hello", 'world', `template ${expr}`
```
let, const, fn, class, trait, match, if, else, for, while,
return, throw, try, catch, finally, async, await, import,
export, macro, type, true, false, null, self, super, this
```

### A.2 Operator Tokens

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

### A.3 Punctuator Tokens

```
{, }, (, ), [, ],
., ,, ;, :, ?,
@, #, ...,
<, >
```

### A.4 Literal Forms

```
null, true, false
42, 3.14, 1e10, 0xFF, 0o77, 0b1010, 100n
"hello", 'world', `template ${expr}`
```

---

## Appendix B: Grammar Summary

### B.1 Declaration Grammar

```
Declaration     → LexicalDeclaration | FunctionDeclaration | ClassDeclaration
                | TraitDeclaration | ImplDeclaration | TypeAliasDeclaration | MacroDeclaration
LexicalDecl     → let BindingList ; | const BindingList ;
FunctionDecl    → fn Identifier TypeParams? ( Params? ) ReturnType? { Body }
ClassDecl       → @Annot* class Identifier TypeParams? extends Expr? { ClassBody }
TraitDecl       → trait Identifier TypeParams? extends TraitList? { TraitBody }
ImplDecl        → impl TypeParams? TraitName TypeArgs? for Type { ClassBody }
TypeAlias       → type Identifier TypeParams? = Type ;
MacroDecl       → macro Identifier { MacroRules }
```

### B.2 Statement Grammar

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

### B.3 Expression Grammar

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
PostfixExpr     → LeftHandSideExpr !          (null assertion)
LeftHandSide    → CallExpr | MemberExpr
CallExpr        → MemberExpr Arguments | CallExpr Arguments | CallExpr [ Expr ]
                | CallExpr . Identifier | CallExpr ?. Identifier
MemberExpr      → PrimaryExpr | MemberExpr [ Expr ] | MemberExpr . Identifier
                | MemberExpr ?. Identifier | MemberExpr TemplateLiteral
PrimaryExpr     → Identifier | Literal | ArrayLiteral | ObjectLiteral
                | FunctionExpr | ClassExpr | ( Expr ) | this | TemplateLiteral
                | if ( Expr ) Expr else Expr  (if-expression)
                | match ( Expr ) { Arms }     (match-expression)
                | new MemberExpr Arguments    (new-expression)
```

### B.4 Pattern Grammar

```
Pattern         → Identifier | Literal | { ObjectPatternFields }
                | [ ArrayPatternElements ] | ... Identifier | Pattern as Identifier
                | Pattern | Pattern | _
```

### B.5 Type Grammar

```
Type            → Identifier | Type? | fn ( Types ) -> Type | Identifier < Types >
                | { TypeFields } | [ Type ] | dyn Identifier | dyn Identifier < Types >
```

### B.6 Module Grammar

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

## Appendix C: Operator Precedence Table

| Precedence | Operator | Description | Associativity |
|------------|----------|-------------|---------------|
| 18 | `.` `?.` `()` `[]` | Member access, call, index | Left |
| 17 | `++` `--` `!` (prefix) `~` `+` `-` `await` `typeof` `void` `delete` | Unary | Right |
| 16 | `**` | Exponentiation | Right |
| 15 | `*` `/` `%` | Multiplicative | Left |
| 14 | `+` `-` | Additive | Left |
| 13 | `<<` `>>` `>>>` | Bitwise shift | Left |
| 12 | `<` `>` `<=` `>=` `in` `instanceof` | Relational | Left |
| 11 | `===` `!==` `==` `!=` | Equality | Left |
| 10 | `&` | Bitwise AND | Left |
| 9 | `^` | Bitwise XOR | Left |
| 8 | `\|` | Bitwise OR | Left |
| 7 | `&&` | Logical AND | Left |
| 6 | `\|\|` | Logical OR | Left |
| 5 | `??` | Nullish coalescing | Left |
| 4 | `?:` | Ternary conditional | Right |
| 3 | `=>` | Arrow function | Right |
| 2 | `=` `+=` `-=` `*=` `/=` `%=` `**=` `&=` `\|=` `^=` `<<=` `>>=` `>>>=` `&&=` `\|\|=` `??=` | Assignment | Right |
| 1 | `,` | Sequence | Left |

**Postfix operators** (applied after all above):

| Operator | Description |
|----------|-------------|
| `!` | Null assertion: `e!` asserts `e` is not null |

---

---

## 8. Type System Semantics

Ruyi uses a **gradual type system** that combines static type checking with dynamic type checking. Programmers can choose to annotate types for compile-time safety, or omit annotations and rely on runtime checks. The system is designed so that static and dynamic typing coexist without contradiction.

### 8.1 Gradual Typing Model

#### 8.1.1 Type Annotation Semantics

Every binding in Ruyi has an associated type. The type is determined by one of two mechanisms:

1. **Explicit annotation**: When a type annotation is provided, the compiler uses that type for static checking.
2. **Implicit inference**: When no annotation is provided, the compiler attempts to infer the type from the initializer. If inference fails, the type defaults to `dyn`.

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

**Examples**:
```ruyi
let x = 42;           // x: int (inferred from literal)
let y: int = 42;      // y: int (explicit annotation)
let z;                // z: dyn (no annotation, no initializer)
let f = (a) => a + 1; // f: fn(dyn) -> dyn (parameter unannotated)
```

#### 8.1.2 The `dyn` Type

`dyn` is the dynamic type. It represents a value whose type is checked at runtime rather than compile time. `dyn` is consistent with every type, meaning:

- Any value can be assigned to `dyn`.
- A value of type `dyn` can be used in any context that expects a specific type, with a runtime check inserted.

**Formal consistency relation** (written as `~`):

```
T ~ dyn    for all types T
dyn ~ T    for all types T
```

This means `dyn` is both a subtype and supertype of every type in the gradual typing sense. However, this does not mean type safety is abandoned. When a `dyn` value flows into a statically-typed context, a **runtime type check** (cast) is inserted.

#### 8.1.3 Runtime Type Checks (Cast Insertion)

When a value of type `dyn` is used in a context expecting a static type `T`, the compiler inserts a runtime check:

```
cast<T>(v: dyn): T
```

The cast operation:
1. Inspects the runtime type tag of `v`.
2. If the runtime type matches `T` (or is a subtype of `T`), returns `v` as type `T`.
3. If the runtime type does not match, throws a `TypeError` at runtime.

**Cast insertion rules**:

| Context | Rule |
|---------|------|
| Function call `f(arg)` where `f: fn(T) -> R` and `arg: dyn` | Insert `cast<T>(arg)` |
| Method call `obj.method()` where `obj: dyn` | Insert runtime method lookup |
| Property access `obj.prop` where `obj: dyn` | Insert runtime property lookup |
| Binary operation `a + b` where either is `dyn` | Insert runtime type check for both operands |
| Return from function `return v` where return type is `T` and `v: dyn` | Insert `cast<T>(v)` |
| Assignment `let x: T = v` where `v: dyn` | Insert `cast<T>(v)` |

#### 8.1.4 Gradual Typing Consistency

The gradual type system satisfies the **gradual guarantee**:

- **Static guarantee**: If a program type-checks without `dyn`, it has the same safety properties as a fully statically-typed program.
- **Dynamic guarantee**: If a program with `dyn` passes all runtime checks, it behaves identically to the same program with all `dyn` replaced by the inferred runtime types.
- **Migration guarantee**: Adding type annotations to a working dynamic program never changes its runtime behavior (unless the annotations are incorrect, in which case a compile-time error is raised).

### 8.2 Type Inference Algorithm

Ruyi uses a **bidirectional type inference** algorithm that combines local type inference with constraint-based solving for generic functions.

#### 8.2.1 Bidirectional Typing

The type checker operates in two modes:

- **Check mode** (`Gamma |- e <= T`): Verify that expression `e` has type `T` in context `Gamma`.
- **Synthesize mode** (`Gamma |- e => T`): Determine the type `T` of expression `e` in context `Gamma`.

**Key rules**:

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

#### 8.2.2 Local Type Inference

For variable declarations without annotations, Ruyi infers types from the initializer:

```ruyi
let x = 42;           // 42 is int literal, so x: int
let y = "hello";      // string literal, so y: string
let z = true;         // bool literal, so z: bool
let arr = [1, 2, 3];  // array of int literals, so arr: Array<int>
```

**Inference rules for literals**:

| Literal | Inferred Type |
|---------|---------------|
| Integer literal (no suffix) | `int` |
| Float literal (with `.` or `e`) | `float` |
| BigInt literal (suffix `n`) | `bigint` |
| String literal | `string` |
| `true` / `false` | `bool` |
| `null` | `null` |
| Array literal `[e1, e2, ...]` | `Array<lub(T1, T2, ...)>` |
| Object literal `{ k1: v1, ... }` | `{ k1: T1, ... }` |

The **least upper bound** (lub) of a set of types is the most specific type that all types in the set can be assigned to:

```
lub(int, int) = int
lub(int, float) = float
lub(T, T) = T
lub(T, dyn) = dyn
lub(dyn, dyn) = dyn
lub(T, U) = dyn    (when T and U are unrelated)
```

#### 8.2.3 Function Return Type Inference

When a function has no return type annotation, Ruyi infers the return type from all `return` statements:

```ruyi
fn add(a: int, b: int) {    // no return annotation
  return a + b;              // a + b: int
}
// Inferred: fn add(a: int, b: int): int
```

If multiple return paths exist, the return type is the lub of all returned types:

```ruyi
fn maybeNumber(flag: bool) {
  if (flag) {
    return 42;               // int
  } else {
    return 3.14;             // float
  }
}
// Inferred: fn maybeNumber(flag: bool): float
```

If a function has no `return` statement, the inferred return type is `void`.

#### 8.2.4 Constraint-Based Inference for Generics

For generic functions, Ruyi collects type constraints during type checking and solves them via unification:

```ruyi
fn map<T, U>(arr: Array<T>, f: fn(T) -> U): Array<U> {
  // ...
}

let result = map([1, 2, 3], (x) => x * 2);
// Constraints: T = int (from array), U = int (from x * 2)
// Result: Array<int>
```

The constraint solver:
1. Creates type variables for each unbound type parameter.
2. Traverses the function body, generating equality constraints.
3. Solves constraints via unification.
4. If constraints are unsatisfiable, reports a type error.
5. If multiple solutions exist, chooses the most specific one.

### 8.3 Type Hierarchy and Subtyping

Ruyi has a structural subtyping system for object types and nominal subtyping for named types.

#### 8.3.1 Subtyping Rules

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

#### 8.3.2 Type Compatibility

Two types are **compatible** if one is a subtype of the other, or if they are both `dyn`:

```
compatible(T, U) = T <: U || U <: T || T = dyn || U = dyn
```

### 8.4 Dynamic Type Runtime Representation

At runtime, `dyn` values carry a **type tag** that identifies their concrete type:

```
DynValue {
  tag: TypeTag,      // enum identifying the concrete type
  value: RawValue,   // the actual value bits
}
```

**TypeTag enumeration**:

```
TypeTag ::
  IntTag          // 64-bit signed integer
  FloatTag        // 64-bit IEEE 754 float
  BoolTag         // boolean
  StringTag       // pointer to string object
  ArrayTag        // pointer to array object
  ObjectTag       // pointer to object
  FunctionTag     // pointer to function closure
  NullTag         // null value
  BigIntTag       // arbitrary precision integer
  ErrorTag        // exception object
  TraitObjectTag  // trait object (vtable + data)
```

Runtime type checks compare the `tag` field against the expected type. For subtyping checks (e.g., `dyn` value assigned to a trait type), the tag is used to look up the trait implementation.

---

## 9. Nullable Type Semantics

### 9.1 Nullable Type Formation

For any type `T`, the nullable type `T?` is formed:

```
T? = T | null
```

`T?` is a distinct type from `T`. The following rules govern nullable types:

```
[Null-Intro]   null : T?                    (for any T)
[Null-Elim]    v : T?    v !== null
               -----------------
               v : T                         (after null check)

[Null-Sub]     T <: T?                      (T is a subtype of T?)
[Null-Double]  (T?)? = T?                   (nullable of nullable is nullable)
```

### 9.2 Optional Chaining Semantics

The `?.` operator provides safe property access on nullable receivers. Its semantics are defined by short-circuit evaluation:

```
e?.prop  =  if (e === null) { null } else { e.prop }
```

The result type of `e?.prop` is always nullable:

```
If   e : T?    and    T.prop : U
Then e?.prop : U?
```

**Chained optional access**:

```
user?.profile?.name
```

Expands to:

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

The entire chain evaluates to `null` if any intermediate value is `null`. The result type is the nullable form of the final property type.

**Optional method call**:

```
obj?.method(args)
```

Expands to:

```
if (obj === null) {
  null
} else {
  obj.method(args)
}
```

The method is only invoked if `obj` is not `null`. The result type is the nullable form of the method's return type.

**Optional indexing**:

```
arr?.[index]
```

Expands to:

```
if (arr === null) {
  null
} else {
  arr[index]
}
```

### 9.3 Nullish Coalescing Semantics

The `??` operator provides a default value for nullable expressions:

```
e1 ?? e2  =  if (e1 !== null) { e1 } else { e2 }
```

**Type derivation**:

```
If   e1 : T?    and    e2 : U    and    T <: U
Then e1 ?? e2 : U
```

The result type is the lub of the non-null form of `e1`'s type and `e2`'s type:

```
type(e1 ?? e2) = lub(nonNull(type(e1)), type(e2))
```

Where `nonNull(T?) = T` and `nonNull(T) = T`.

**Chained coalescing**:

```
a ?? b ?? c
```

Associates left-to-right:

```
(a ?? b) ?? c
```

The result type is the lub of all non-null types in the chain.

### 9.4 Null Assertion

The `!` operator asserts that a nullable value is not null:

```
e!  =  if (e === null) { throw NullAssertionError() } else { e }
```

**Type rule**:

```
If   e : T?
Then e! : T
```

The `!` operator removes the nullable wrapper from the type. At runtime, it performs a null check and throws if the value is null.

### 9.5 Type Narrowing via Control Flow

After a null check, the compiler narrows the type of the checked variable within the guarded scope:

```ruyi
let name: string? = getUser();

if (name !== null) {
  // name is narrowed to string
  print(name.length);    // OK: name is string here
}

// name is string? again here
```

**Narrowing rules**:

| Check | True branch | False branch |
|-------|-------------|--------------|
| `x !== null` | `x` narrowed to `T` | `x` narrowed to `null` |
| `x === null` | `x` narrowed to `null` | `x` narrowed to `T` |
| `x != null` | `x` narrowed to `T` | `x` narrowed to `null` |
| `x == null` | `x` narrowed to `null` | `x` narrowed to `T` |

Narrowing applies to:
- `if` / `else` branches
- Ternary `?:` branches
- `match` arms with null guards
- Loop bodies where the condition includes a null check

Narrowing does **not** persist across function calls or mutable reassignments:

```ruyi
let name: string? = getUser();

if (name !== null) {
  someFunction();     // function call may have side effects
  print(name.length); // still OK: narrowing preserved across pure calls
}

name = getUser();     // reassignment resets narrowing
```

### 9.6 Nullable Types and Generics

Nullable types interact with generics as follows:

```ruyi
class Some<T> {
  value: T;

  fn new(value: T) {
    self.value = value;
  }

  fn unwrap(self): T {
    return self.value;    // directly returns T
  }
}

class None {
  fn unwrap(self): never {
    throw RuntimeError.new("unwrap on None");
  }
}

type Option<T> = Some<T> | None;
```

`Option<T>` and `T?` are distinct. `Option<T>` is a wrapper type that can carry additional methods, while `T?` is a built-in nullable type.

---

## 10. Generics Semantics

### 10.1 Type Parameterization

Generic declarations introduce type parameters that are placeholders for concrete types:

```ruyi
fn identity<T>(x: T): T { return x; }
class Box<T> { value: T; }
trait Iterator<T> { fn next(self): T?; }
```

Type parameters are scoped to their declaration and are replaced with concrete types at instantiation sites.

### 10.2 Trait Bounds

Type parameters can be constrained by trait bounds:

```ruyi
fn max<T: Comparable>(a: T, b: T): T { ... }
fn sort<T: Comparable + Clone>(arr: Array<T>): Array<T> { ... }
```

**Multiple bounds** use the `+` syntax and require the type to implement all listed traits:

```
T: A + B    means    T implements A AND T implements B
```

**Bound semantics**:

When a type parameter `T` has a trait bound `Trait`, the compiler:
1. Verifies that any concrete type substituted for `T` implements `Trait`.
2. Allows method calls defined by `Trait` on values of type `T`.
3. Generates a trait dictionary (vtable pointer) for dynamic dispatch, or monomorphizes for static dispatch.

### 10.3 Monomorphization

Ruyi uses **monomorphization** as the primary code generation strategy for generics. At each call site, the compiler generates a specialized copy of the generic function with concrete types substituted for type parameters.

**Monomorphization process**:

1. **Collection**: During type checking, collect all call sites of generic functions and the concrete types used.
2. **Substitution**: For each unique combination of concrete types, create a specialized version of the generic function.
3. **Code generation**: Generate LLVM IR for each specialized version.
4. **Deduplication**: If the same specialization is used at multiple call sites, generate it only once.

**Example**:

```ruyi
fn identity<T>(x: T): T { return x; }

let a = identity(42);       // generates identity_int(x: int): int
let b = identity("hello");  // generates identity_string(x: string): string
```

Generates:

```ruyi
fn identity_int(x: int): int { return x; }
fn identity_string(x: string): string { return x; }
```

**Monomorphization and trait bounds**:

When a generic function has trait bounds, the monomorphized version includes the trait implementation:

```ruyi
fn max<T: Comparable>(a: T, b: T): T {
  return if a.compare(b) > 0 { a } else { b };
}

let m = max(3, 5);  // generates max_int using int's Comparable impl
```

### 10.4 Generics and Dynamic Types

Generic functions can be called with `dyn` arguments:

```ruyi
fn identity<T>(x: T): T { return x; }

let x: dyn = 42;
let y = identity(x);    // T = dyn, returns dyn
```

When `dyn` is used as a type argument:
1. The generic function is NOT monomorphized. Instead, a single `dyn` version is used.
2. All operations within the function use runtime type checks.
3. Trait bounds on `dyn` are checked at runtime via trait object lookup.

**Interaction rules**:

| Scenario | Behavior |
|----------|----------|
| Generic called with all static types | Monomorphize |
| Generic called with `dyn` type argument | Use dyn version (no monomorphization) |
| Generic with trait bound called with `dyn` | Runtime trait lookup |
| Generic with trait bound called with static type | Monomorphize with static dispatch |

### 10.5 Generic Type Aliases

Type aliases can be generic:

```ruyi
type Result<T, E> = Ok<T, E> | Err<T, E>;
type Callback<T> = fn(T) -> void;
```

Generic type aliases are expanded at use sites:

```
Result<int, Error>  expands to  Ok<int, Error> | Err<int, Error>
```

### 10.6 Variance

Ruyi's generic types have the following variance:

| Type Constructor | Variance | Rule |
|-----------------|----------|------|
| `Array<T>` | Covariant | `Array<S> <: Array<T>` if `S <: T` |
| `fn(T) -> R` | Contravariant in T, Covariant in R | `fn(U) -> S <: fn(T) -> R` if `T <: U` and `S <: R` |
| `Option<T>` | Covariant | `Option<S> <: Option<T>` if `S <: T` |
| `Result<T, E>` | Covariant in both | `Result<S, F> <: Result<T, E>` if `S <: T` and `F <: E` |

---

## 11. Trait Semantics

### 11.1 Trait Declarations

A trait defines a set of methods that a type must implement:

```ruyi
trait Printable {
  fn format(self): string;
}

trait Comparable<T> {
  fn compare(self, other: T): int;
}
```

**Trait semantics**:

1. A trait declaration defines a **contract**, not an implementation.
2. Trait methods may have bodies (default implementations) or be signatures only.
3. Traits can be generic (type parameters on the trait itself).
4. Traits can have **supertraits** via the `extends` clause.

### 11.2 Trait Implementations

Types implement traits via `impl` blocks:

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

**Implementation rules**:

1. **Orphan rule**: An `impl` block must be in the same module as either the trait or the type being implemented. This prevents conflicting implementations.
2. **Coherence**: For any given type and trait, there must be at most one implementation visible in any scope.
3. **Generic impl**: Impl blocks can be generic, with their own type parameters and bounds.

### 11.3 Static vs Dynamic Dispatch

Ruyi supports both static and dynamic dispatch for trait methods.

#### 11.3.1 Static Dispatch (Monomorphized)

When the concrete type is known at compile time, trait method calls use static dispatch:

```ruyi
fn printIt<T: Printable>(value: T) {
  print(value.format());    // static dispatch: T is known
}

printIt("hello");    // calls string.format() directly
printIt(42);         // calls int.format() directly
```

The compiler monomorphizes `printIt` for each concrete type, generating direct function calls with no vtable lookup.

#### 11.3.2 Dynamic Dispatch (Trait Objects)

When the concrete type is not known at compile time, trait method calls use dynamic dispatch via trait objects:

```ruyi
let items: Array<dyn Printable> = ["hello", 42, true];
for (let item of items) {
  print(item.format());    // dynamic dispatch: vtable lookup
}
```

**Trait object representation**:

```
TraitObject {
  data: *void,        // pointer to the concrete value
  vtable: *VTable,    // pointer to the trait's vtable for this type
}

VTable {
  format: fn(*void) -> string,    // function pointer for each trait method
  // ... other methods
}
```

At runtime, the vtable is indexed to find the correct method implementation for the concrete type.

#### 11.3.3 Dispatch Selection Rules

| Context | Dispatch |
|---------|----------|
| Generic function with trait bound | Static (monomorphized) |
| Trait object (`dyn Trait`) | Dynamic (vtable) |
| Direct method call on concrete type | Static (direct call) |
| Method call through `dyn` variable | Dynamic (vtable) |

### 11.4 Default Method Implementations

Traits can provide default implementations for methods:

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

Default methods:
1. Are inherited by all implementations that do not override them.
2. Can call other trait methods (including abstract ones).
3. Are monomorphized when used through static dispatch.
4. Use vtable lookup when used through dynamic dispatch.

### 11.5 Trait Objects and Type Erasure

When a value is cast to a trait object (`dyn Trait`), the concrete type is erased:

```ruyi
let x: string = "hello";
let y: dyn Printable = x;    // type erased to dyn Printable
```

**Type erasure rules**:

1. The trait object retains the concrete value but hides its type.
2. Only methods defined by the trait are accessible through the trait object.
3. The original type can be recovered via pattern matching (see Section 11.6).

### 11.6 Trait Object Downcasting

Trait objects can be downcast to their concrete types:

```ruyi
let y: dyn Printable = "hello";

match (y) {
  s as string => { print("string: " + s); }
  n as int => { print("int: " + n); }
  _ => { print("unknown type"); }
}
```

Downcasting uses runtime type tag comparison. If the tag matches the target type, the value is cast. Otherwise, the match arm is skipped.

---

## 12. Memory Model

### 12.1 Memory Management Strategies

Ruyi supports two memory management strategies:

| Strategy | Default | Use Case |
|----------|---------|----------|
| **GC** (Garbage Collection) | Yes | General-purpose code, rapid development |
| **ARC** (Automatic Reference Counting) | No | Performance-critical paths, deterministic deallocation |

### 12.2 GC Memory Regions

By default, all heap-allocated objects are managed by the garbage collector.

#### 12.2.1 GC Object Layout

```
GC Object Header:
+----------------+----------------+----------------+
| TypeTag (8b)   | Flags (8b)     | Size (16b)     |
+----------------+----------------+----------------+
| Forwarding Ptr (32b) | Reserved (32b)            |
+----------------+----------------+----------------+
| Payload (variable size)                         |
+-------------------------------------------------+
```

**Header fields**:

| Field | Size | Purpose |
|-------|------|---------|
| TypeTag | 8 bits | Identifies the object's type (see Section 8.4) |
| Flags | 8 bits | Bit 0: marked, Bit 1: pinned, Bit 2: in old gen, Bits 3-7: reserved |
| Size | 16 bits | Total object size in bytes (including header) |
| Forwarding Ptr | 32 bits | Used during copying GC to point to new location |
| Reserved | 32 bits | Reserved for future use |

#### 12.2.2 GC Generations

The GC uses a **generational** strategy:

1. **Young generation (nursery)**: New objects are allocated here. Small (typically 1-4 MB). Collected frequently.
2. **Old generation**: Objects that survive multiple young-gen collections are promoted here. Larger (typically 16-64 MB). Collected less frequently.

**Promotion rule**: An object is promoted to the old generation after surviving `N` young-gen collections (default `N = 3`).

#### 12.2.3 GC Collection Algorithm

**Young generation (copying collector)**:
1. Identify root set (stack variables, global variables, registers).
2. Copy live objects from nursery to survivor space.
3. Update all references to point to new locations.
4. Swap nursery and survivor space.

**Old generation (mark-compact collector)**:
1. Mark phase: traverse from roots, mark all reachable objects.
2. Compact phase: move live objects to eliminate fragmentation.
3. Update all references.

**Write barrier**: When a pointer in an old-gen object is updated to point to a young-gen object, the write barrier records this cross-generational reference. This ensures old-gen objects are scanned during young-gen collection.

### 12.3 ARC Memory Regions

Objects can be explicitly allocated with ARC management:

```ruyi
let ptr: Arc<T> = Arc::new(value);
```

#### 12.3.1 ARC Object Layout

```
ARC Object Header:
+----------------+----------------+----------------+
| TypeTag (8b)   | Flags (8b)     | Size (16b)     |
+----------------+----------------+----------------+
| RefCount (32b) | WeakCount (32b)                  |
+----------------+----------------+----------------+
| Payload (variable size)                         |
+-------------------------------------------------+
```

**Reference counting rules**:

1. `Arc::new(value)` creates an object with `RefCount = 1`.
2. `Arc::clone(&ptr)` increments `RefCount` by 1.
3. When `RefCount` reaches 0, the object is deallocated.
4. `Weak<T>` references increment `WeakCount` but not `RefCount`.
5. When both `RefCount` and `WeakCount` reach 0, the memory is freed.

### 12.4 GC/ARC Boundary Rules

GC-managed and ARC-managed objects can reference each other, but with restrictions:

#### 12.4.1 GC referencing ARC

GC objects **can** hold references to ARC objects:

```ruyi
let arcObj: Arc<int> = Arc::new(42);
let gcObj = { value: arcObj };    // OK: GC holds Arc reference
```

The GC treats `Arc<T>` as an opaque value. The Arc's reference count is independent of GC collection.

#### 12.4.2 ARC referencing GC

ARC objects **cannot** directly hold references to GC objects:

```ruyi
let gcObj = { x: 1 };
let arcObj: Arc<SomeType> = Arc::new(gcObj);    // ERROR: ARC cannot hold GC reference
```

**Rationale**: The GC may move objects during collection, invalidating raw pointers held by ARC objects. If an ARC object needs to reference a GC object, it must use a `GcRef<T>` handle:

```ruyi
let gcObj: Gc<MyType> = Gc::new(MyType::new());
let arcObj: Arc<SomeType> = Arc::new({ handle: gcObj.clone() });
```

`GcRef<T>` is a GC-tracked handle that remains valid across collections.

#### 12.4.3 Boundary Summary

| Direction | Allowed | Mechanism |
|-----------|---------|-----------|
| GC -> ARC | Yes | Direct reference (Arc is opaque to GC) |
| ARC -> GC | No (direct) | Must use `GcRef<T>` handle |
| GC -> GC | Yes | Standard GC references |
| ARC -> ARC | Yes | Standard reference counting |

### 12.5 Object Layout and Alignment

All heap objects are aligned to 8-byte boundaries for performance. The minimum object size is 16 bytes (header only).

**Primitive value layout** (on stack or in object payload):

| Type | Size | Alignment |
|------|------|-----------|
| `int` | 8 bytes | 8 |
| `float` | 8 bytes | 8 |
| `bool` | 1 byte | 1 |
| `null` | 0 bytes | 1 |
| `bigint` | variable | 8 |
| Pointer | 8 bytes | 8 |

**String object layout**:

```
String Object:
+----------------+----------------+----------------+
| GC Header (8 bytes)                              |
+----------------+----------------+----------------+
| Length (32b)   | Capacity (32b)                  |
+----------------+----------------+----------------+
| UTF-8 bytes (variable length, null-terminated)   |
+-------------------------------------------------+
```

**Array object layout**:

```
Array Object:
+----------------+----------------+----------------+
| GC Header (8 bytes)                              |
+----------------+----------------+----------------+
| Length (32b)   | Capacity (32b)                  |
+----------------+----------------+----------------+
| Element pointer (8 bytes) -> [T, T, T, ...]      |
+-------------------------------------------------+
```

Arrays store elements contiguously. For arrays of `dyn`, each element includes a type tag.

### 12.6 Memory Safety Guarantees

Ruyi provides the following memory safety guarantees:

1. **No dangling pointers**: GC ensures live objects are never collected. ARC ensures objects are freed only when all references are dropped.
2. **No double-free**: Each object is freed exactly once.
3. **No use-after-free**: Collected/freed objects are never accessed.
4. **No buffer overflow**: Array bounds are checked at runtime (can be optimized away by the compiler when provably safe).
5. **No uninitialized memory**: All variables are initialized before use (compile-time check).

---

## 13. Exception Semantics

### 13.1 Exception Type System

All exceptions in Ruyi are subtypes of the built-in `Error` type:

```
Error
  |- TypeError
  |- RangeError
  |- NullAssertionError
  |- RuntimeError
  |- IOError
  |- CustomError (user-defined)
```

**Exception object layout**:

```
Exception Object:
+----------------+----------------+----------------+
| GC Header (8 bytes)                              |
+----------------+----------------+----------------+
| TypeTag (identifies specific error subtype)      |
+----------------+----------------+----------------+
| message: string                                  |
+----------------+----------------+----------------+
| stackTrace: Array<Frame>                         |
+----------------+----------------+----------------+
```

### 13.2 try/catch/finally Evaluation Order

#### 13.2.1 try Block Evaluation

The `try` block is evaluated first. If no exception is thrown:
1. The `try` block completes normally.
2. The `finally` block (if present) is executed.
3. Control continues after the entire try statement.

If an exception is thrown during `try` block evaluation:
1. Evaluation of the `try` block is interrupted.
2. The exception propagates to the `catch` clause.

#### 13.2.2 catch Clause Evaluation

When an exception reaches a `catch` clause:

1. The exception's type is compared against the catch pattern.
2. If the pattern matches, the exception is bound to the catch variable and the catch block executes.
3. If the pattern does not match, the exception propagates to the next enclosing `catch` or `finally`.

```ruyi
try {
  riskyOperation();
} catch (e: TypeError) {
  // handles TypeError and its subtypes
} catch (e: Error) {
  // handles all other errors
}
```

**Catch pattern matching**:

```
catch (e: T) matches exception E  if  E <: T
```

Multiple catch clauses are tried in order. The first matching clause handles the exception.

#### 13.2.3 finally Block Evaluation

The `finally` block **always** executes, regardless of how the `try` block exits:

| try exit | finally behavior |
|----------|-----------------|
| Normal completion | Executes after try |
| Exception thrown | Executes before exception propagates |
| `return` statement | Executes before return |
| `break` / `continue` | Executes before control transfer |
| Exception in catch | Executes before new exception propagates |

**Evaluation order with try/catch/finally**:

```
try { A } catch (e) { B } finally { C }
```

1. Evaluate `A`.
2. If `A` throws exception `E`:
   a. Match `E` against catch pattern.
   b. If match, evaluate `B`.
   c. If no match, skip `B`, propagate `E`.
3. Evaluate `C` (always).
4. If `B` threw a new exception, propagate it.
5. If `A` threw and no catch matched, propagate original exception.

#### 13.2.4 finally and Exception Suppression

If the `finally` block throws an exception while another exception is already propagating, the `finally` exception **replaces** the original exception:

```ruyi
try {
  throw Error("original");
} finally {
  throw Error("finally");    // this exception replaces "original"
}
// Caught exception: "finally"
```

### 13.3 Exception Propagation

Exceptions propagate up the call stack until a matching `catch` clause is found:

```
fn a() { throw Error("oops"); }
fn b() { a(); }
fn c() {
  try { b(); }
  catch (e: Error) { /* handles it */ }
}
```

**Propagation steps**:

1. Exception is thrown in `a()`.
2. `a()` has no catch, so the exception propagates to `b()`.
3. `b()` has no catch, so the exception propagates to `c()`.
4. `c()` has a matching catch, so the exception is handled.
5. All `finally` blocks on the propagation path execute in order (innermost first).

If no matching `catch` is found at any level, the program terminates with an unhandled exception error.

### 13.4 Exception and Type System

Exceptions interact with the type system as follows:

1. **No checked exceptions**: Ruyi does not require functions to declare which exceptions they throw. All exceptions are unchecked.
2. **`never` return type**: Functions that always throw can be annotated with return type `never`:

```ruyi
fn fail(message: string): never {
  throw Error(message);
}
```

The `never` type is the bottom type. It is a subtype of every type, meaning a `never` expression can be used in any context.

3. **Exception safety in destructors**: When an exception propagates through a scope, all local variables are dropped. If a destructor (drop handler) throws during unwinding, the program aborts (double-panic prevention).

### 13.5 Zero-Cost Exception Implementation

Ruyi exceptions use **zero-cost exception tables** (based on Itanium ABI / DWARF EH):

1. **Normal path**: No overhead when no exception is thrown. The compiler generates exception tables (not inline checks).
2. **Throw path**: `throw` looks up the exception table for the current instruction pointer, finds the nearest landing pad, and jumps to it.
3. **Landing pad**: The landing pad matches the exception type against catch clauses and dispatches accordingly.

This approach ensures that the normal execution path has zero overhead from exception handling.

---

## 14. Async/Await Semantics

### 14.1 Future/Promise Model

Ruyi's async model is based on the **Future** pattern. An `async` function returns a `Future<T>`:

```ruyi
async fn fetchData(url: string): string {
  let response = await http.get(url);
  return response.body;
}

// Call site:
let future: Future<string> = fetchData("https://example.com");
let result: string = await future;
```

**Future semantics**:

1. `Future<T>` represents a computation that will eventually produce a value of type `T`.
2. A `Future` is **lazy**: it does not start executing until awaited or explicitly spawned.
3. `await` suspends the current async function until the future completes.
4. When the future completes, `await` resumes with the result value.

### 14.2 Async Function Transformation

An `async` function is transformed by the compiler into a **state machine**:

```ruyi
// Source:
async fn example(x: int): int {
  let a = await fetchA(x);     // suspension point 1
  let b = await fetchB(a);     // suspension point 2
  return a + b;
}
```

Transformed into:

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
      return Ready(b + a);    // Note: a is captured in state
    }
  }
}
```

**State machine properties**:

1. Each `await` point becomes a state in the state machine.
2. Local variables that live across `await` points are stored in the state enum.
3. The state machine implements the `poll()` method, which returns `Poll<T>`:
   - `Ready(T)`: the computation is complete.
   - `Pending`: the computation is not yet complete, caller should poll again.

### 14.3 Green Thread Scheduling

Ruyi uses a **work-stealing scheduler** for green threads:

#### 14.3.1 Scheduler Architecture

```
+---------------------------------------------------+
|                    Scheduler                       |
|  +-----------+  +-----------+  +-----------+      |
|  | Worker 0  |  | Worker 1  |  | Worker N  |      |
|  | Task Queue|  | Task Queue|  | Task Queue|      |
|  +-----------+  +-----------+  +-----------+      |
+---------------------------------------------------+
```

1. **Workers**: OS threads that execute green threads. Each worker has a local task queue.
2. **Task queue**: Double-ended queue (deque) of ready futures. Workers push to the bottom and pop from the bottom.
3. **Work stealing**: When a worker's queue is empty, it steals tasks from the top of another worker's queue.

#### 14.3.2 Task Lifecycle

1. **Spawn**: `async fn` call creates a future. `spawn(future)` pushes it to the current worker's queue.
2. **Poll**: Worker pops a future and calls `poll()`.
3. **Ready**: If `poll()` returns `Ready`, the future is complete. Result is stored.
4. **Pending**: If `poll()` returns `Pending`, the future is re-queued. The worker executes the next future.
5. **Wake**: When an I/O operation completes, it calls `wake()` on the associated future, re-queuing it.

#### 14.3.3 Blocking Operations

Blocking operations (e.g., synchronous I/O) must not be called from green threads, as they block the worker thread. Instead, use async I/O or `spawn_blocking`:

```ruyi
// Wrong: blocks the worker
let data = fs.readFileSync("file.txt");

// Correct: async I/O
let data = await fs.readFile("file.txt");

// Or: offload to blocking thread pool
let data = await spawn_blocking(|| fs.readFileSync("file.txt"));
```

### 14.4 Async and Exception Interaction

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

**Rules**:

1. If an async function throws, the `Future` completes with an error state.
2. `await` on an errored future re-throws the exception in the awaiting context.
3. Exceptions do not cross task boundaries unless explicitly propagated via `await`.

### 14.5 Async Iterators

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

**AsyncIterator trait**:

```ruyi
trait AsyncIterator<T> {
  fn next(self): Future<T?>;
}
```

`for await` desugars to:

```ruyi
let iter = readLines(file);
while let Some(line) = await iter.next() {
  print(line);
}
```

---

## 15. Module Semantics

### 15.1 Module Structure

Each source file is a module. Modules are organized in a hierarchical namespace based on the file system:

```
src/
  main.ry          -> module main
  utils.ry         -> module utils
  http/
    client.ry      -> module http::client
    server.ry      -> module http::server
```

### 15.2 Import Resolution

Import statements resolve module paths to source files:

```ruyi
import { add, subtract } from "./math";
import * as utils from "./utils";
import HttpClient from "./http/client";
```

**Resolution algorithm**:

1. **Relative paths** (`./` or `../`): Resolved relative to the importing file's directory.
2. **Absolute paths** (no prefix): Resolved from the project's source root.
3. **Standard library paths** (`std::`): Resolved from the standard library.

**Resolution steps**:

1. Convert the module path to a file path:
   - `./math` -> `./math.ry` or `./math/index.ry`
   - `http/client` -> `http/client.ry` or `http/client/index.ry`
2. Check if the file exists.
3. If not found, check for an `index.ry` in a directory with that name.
4. If still not found, report a module resolution error.

### 15.2.1 Standard Library (stdlib)

Ruyi ships with a standard library (`stdlib/`) that provides core types, data structures, and system utilities. The stdlib is located at `$RUYI_HOME/stdlib`.

**RUYI_HOME**:

Ruyi uses the `RUYI_HOME` environment variable to locate its installation directory:

| Path | Description |
|------|-------------|
| `$RUYI_HOME/bin` | Compiler binaries (`ruyic`, etc.) |
| `$RUYI_HOME/stdlib` | Standard library modules |

If `RUYI_HOME` is not set, the compiler falls back to looking for a local `stdlib/` directory relative to the current working directory (useful for development).

**stdlib module layout**:

| Module | Description |
|--------|-------------|
| `core` | Fundamental type methods (trait impl for `string`, `int`, `float`, `bool`, auto-loaded) |
| `option` | `Option<T>` enum (`Some`/`None`) for nullable value handling |
| `result` | `Result<T, E>` enum (`Ok`/`Err`) for error handling |
| `error` | Error hierarchy (`Error`, `TypeError`, `RuntimeError`, `RangeError`, `AssertionError`, `ArgumentError`, `NullError`, `ArithmeticError`, `IteratorError`, `ParseError`) plus `assert()` and `assertNotNull()` |
| `collections` | Generic collections (`Array<T>`, `Map<K, V>`、`Set<T>`) and `Iterator<T>` trait |
| `string` | Standalone string utility functions (`join`, `fromCharCode`, `fromCharCodes`, `concat`, `template`, `processTemplate`) |
| `io` | Console I/O (`readLine`) and file operations (`File.readText`, `File.writeText`, `File.readLines`, `File.exists`, `File.mkdir`, etc.) with async variants |
| `path` | Path manipulation (`Path.join`, `Path.basename`, `Path.dirname`, `Path.extname`, `Path.isAbsolute`, `Path.normalize`, `Path.resolve`, etc.) |
| `process` | Process management (`Process.exec`, `Process.spawn`, `Process.create`), environment variables (`getEnv`, `setEnv`), and system info (`getPID`, `getPlatform`, `getCPUCount`, etc.) |

**Importing stdlib modules**:

```ruyi
// Import stdlib modules by their file name
import { File } from "./io";
import { Path } from "./path";
import { Process, getEnv } from "./process";
import { assert, assertNotNull } from "./error";
import { Option, Some, None } from "./option";
import { Result, Ok, Err } from "./result";
import { Array, Map, Set, Iterator } from "./collections";
```

**Pre-declared built-in symbols**:

The following symbols are available without any import (pre-declared by the type checker):

| Symbol | Type | Description |
|--------|------|-------------|
| `print` | `fn(dyn): void` | Print to stdout (codegen builtin) |
| `spawn` | `fn(dyn): dyn` | Spawn async task |
| `toString` | `fn(dyn): string` | Convert any value to string |
| `Error` | `fn(string): Error` | Error constructor |

### 15.3 Circular Dependency Detection

Ruyi detects circular dependencies at compile time:

```
// a.ry
import { foo } from "./b";

// b.ry
import { bar } from "./a";    // ERROR: circular dependency
```

**Detection algorithm**:

1. Build a **module dependency graph** where nodes are modules and edges are import relationships.
2. Perform a **depth-first search** (DFS) on the graph.
3. If a back edge is found (a node is visited that is already on the current DFS stack), a circular dependency exists.
4. Report the cycle with the full path of modules involved.

**Resolution**: Circular dependencies must be broken by:
- Extracting shared code into a third module.
- Using forward declarations (for types only).
- Restructuring the module hierarchy.

### 15.4 Export Visibility

By default, all top-level declarations in a module are **private** (visible only within the module). The `export` keyword makes them public:

```ruyi
// math.ry
fn add(a: int, b: int): int { ... }       // private
export fn subtract(a: int, b: int): int { ... }  // public
```

**Visibility levels**:

| Level | Keyword | Visible to |
|-------|---------|------------|
| Private | (default) | Current module only |
| Public | `export` | Any module that imports this module |

**Re-exporting**:

```ruyi
export { add, subtract } from "./math";
```

Re-exporting makes the imported names available to modules that import the current module.

### 15.5 Module Initialization

When a module is first imported, its top-level statements execute in order:

```ruyi
// config.ry
export let config = loadConfig();    // executes on first import
```

**Initialization rules**:

1. Each module is initialized exactly once (singleton initialization).
2. Initialization order follows the dependency graph (dependencies initialize before dependents).
3. Circular dependencies are detected before initialization begins.
4. If initialization throws an exception, the program terminates.

### 15.6 Name Resolution and Shadowing

Names are resolved in the following order:

1. Local scope (current block).
2. Function scope (parameters and local variables).
3. Module scope (top-level declarations in the current module).
4. Imported names (from `import` statements).
5. Built-in names (`int`, `string`, `null`, etc.).

**Shadowing**: Inner scopes can shadow outer scope names:

```ruyi
let x = 1;           // module-level x

fn example() {
  let x = 2;         // shadows module-level x
  print(x);          // prints 2
}
```

Shadowing is allowed but generates a warning if the shadowed name is from an imported module.

---

## 16. Macro Semantics

### 16.1 Declarative Macro Expansion

Ruyi macros are **declarative** (pattern-based), similar to Rust's `macro_rules!`. Macros are expanded at compile time, before type checking.

```ruyi
macro debug {
  ($expr) => {
    print("DEBUG: " + stringify($expr) + " = " + $expr);
  }
}

debug(x + 1);    // expands to: print("DEBUG: " + "x + 1" + " = " + (x + 1));
```

### 16.2 Macro Expansion Rules

#### 16.2.1 Pattern Matching

Macro rules are tried in order. The first rule whose pattern matches the input is used:

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

**Pattern elements**:

| Pattern | Matches |
|---------|---------|
| `$name` | Single expression, statement, or token |
| `$(...)` | Repeated pattern |
| `$(...),*` | Zero or more, comma-separated |
| `$(...),+` | One or more, comma-separated |
| `$(...)?` | Zero or one |

#### 16.2.2 Expansion Process

1. **Parse**: The macro invocation is parsed as a sequence of tokens (not as an expression).
2. **Match**: The token sequence is matched against each rule's pattern, in order.
3. **Substitute**: Metavariables (`$name`) are substituted with the matched tokens.
4. **Repeat**: Repetition patterns (`$(...)*`) are expanded for each matched group.
5. **Emit**: The resulting tokens replace the macro invocation in the source.
6. **Re-parse**: The emitted tokens are re-parsed as Ruyi code.

### 16.3 Macro Hygiene

Ruyi macros are **hygienic**: identifiers introduced by a macro do not conflict with identifiers in the calling scope.

```ruyi
macro swap {
  ($a, $b) => {
    let temp = $a;    // 'temp' is hygienic
    $a = $b;
    $b = temp;
  }
}

let temp = 100;
let x = 1;
let y = 2;
swap(x, y);
// 'temp' inside macro does NOT shadow outer 'temp'
// outer 'temp' is still 100
```

**Hygiene implementation**:

1. Each macro expansion is assigned a unique **syntax context** (an integer ID).
2. Identifiers introduced by the macro are tagged with this context.
3. During name resolution, identifiers are matched by both name and context.
4. Identifiers from the calling scope (passed as metavariables) retain their original context.

**Exception**: The `stringify` and `quote` built-in macro functions operate on the surface syntax of their arguments, ignoring hygiene.

### 16.4 Built-in Macro Functions

| Function | Description |
|----------|-------------|
| `stringify($x)` | Converts the matched tokens to a string literal |
| `file!()` | Expands to the current file path (string) |
| `line!()` | Expands to the current line number (int) |
| `column!()` | Expands to the current column number (int) |

### 16.5 Macro Expansion Order

Macros are expanded in a **fixed-point** process:

1. Scan the AST for macro invocations.
2. Expand all found macros.
3. If the expanded output contains new macro invocations, repeat from step 1.
4. Stop when no more macro invocations are found, or when a maximum depth is reached (default: 64).

**Maximum depth**: To prevent infinite expansion, the compiler limits macro expansion depth to 64 levels. If this limit is exceeded, a compile-time error is reported.

### 16.6 Macro and Module Interaction

Macros defined in one module can be used in another:

```ruyi
// macros.ry
export macro debug {
  ($expr) => { print("DEBUG: " + stringify($expr)); }
}

// main.ry
import { debug } from "./macros";
debug(x);    // works
```

**Export rules**:

1. Macros must be explicitly exported with `export macro`.
2. Imported macros are expanded in the context of the importing module.
3. Macro hygiene ensures that exported macros do not accidentally capture names from the importing module.

---

*End of Semantics and Type System Specification*

*End of Lexical and Syntax Specification*
