/**
 * Abstract Syntax Tree node types for the Ruyi parser.
 *
 * Covers expressions, statements, declarations, patterns, and type annotations.
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */
use crate::lexer::token::Token;

// ── Program ──────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub items: Vec<ModuleItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModuleItem {
    Import(ImportDecl),
    Export(ExportDecl),
    Statement(Statement),
    Declaration(Declaration),
}

// ── Declarations ─────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Declaration {
    Let(Vec<Binding>),
    Const(Vec<Binding>),
Function {
        name: String,
        type_params: Vec<TypeParam>,
        params: Vec<Param>,
        return_type: Option<TypeAnnotation>,
        body: Vec<Statement>,
        is_async: bool,
        annotations: Vec<String>,
    },
    Class {
        name: String,
        type_params: Vec<TypeParam>,
        extends: Option<Box<Expr>>,
        body: Vec<ClassElement>,
        annotations: Vec<String>,
    },
    Trait {
        name: String,
        type_params: Vec<TypeParam>,
        supertraits: Vec<String>,
        body: Vec<TraitElement>,
    },
    Impl {
        type_params: Vec<TypeParam>,
        trait_name: String,
        trait_args: Vec<TypeAnnotation>,
        for_type: TypeAnnotation,
        body: Vec<ClassElement>,
    },
    TypeAlias {
        name: String,
        type_params: Vec<TypeParam>,
        ty: TypeAnnotation,
    },
    Macro {
        name: String,
        rules: Vec<MacroRule>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    pub pattern: Pattern,
    pub init: Option<Box<Expr>>,
    pub ty: Option<TypeAnnotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub pattern: Pattern,
    pub ty: Option<TypeAnnotation>,
    pub init: Option<Box<Expr>>,
    pub is_rest: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeParam {
    pub name: String,
    pub bounds: Vec<String>,
}

// ── Statements ───────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Block(Vec<Statement>),
    Expression(Box<Expr>),
    If {
        condition: Box<Expr>,
        then_branch: Box<Statement>,
        else_branch: Option<Box<Statement>>,
    },
    IfLet {
        pattern: Pattern,
        value: Box<Expr>,
        then_branch: Box<Statement>,
        else_branch: Option<Box<Statement>>,
    },
    While {
        condition: Box<Expr>,
        body: Box<Statement>,
    },
    WhileLet {
        pattern: Pattern,
        value: Box<Expr>,
        body: Box<Statement>,
    },
    For {
        init: Option<ForInit>,
        condition: Option<Box<Expr>>,
        update: Option<Box<Expr>>,
        body: Box<Statement>,
    },
    ForIn {
        variable: String,
        iterable: Box<Expr>,
        body: Box<Statement>,
    },
    ForOf {
        variable: String,
        iterable: Box<Expr>,
        body: Box<Statement>,
        is_async: bool,
    },
    Return(Option<Box<Expr>>),
    Throw(Box<Expr>),
    Try {
        body: Vec<Statement>,
        catch: Vec<CatchClause>,
        finally: Option<Vec<Statement>>,
    },
    Match {
        value: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    Break(Option<String>),
    Continue(Option<String>),
    Yield(Option<Box<Expr>>),
    Labeled {
        label: String,
        body: Box<Statement>,
    },
    Declaration(Declaration),
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ForInit {
    VarDecl(Declaration),
    Expr(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CatchClause {
    pub pattern: Option<Pattern>,
    pub ty: Option<TypeAnnotation>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Box<Expr>>,
    pub body: Vec<Statement>,
}

// ── Expressions ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Identifier(String),
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    BigIntLiteral(String),
    BooleanLiteral(bool),
    NullLiteral,
    TemplateLiteral(Vec<TemplatePart>),
    ArrayLiteral(Vec<ArrayElement>),
    ObjectLiteral(Vec<ObjectProperty>),
    This,
    Super,
    SelfExpr,
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Argument>,
    },
    Member {
        object: Box<Expr>,
        property: MemberProperty,
        optional: bool,
    },
    OptionalCall {
        callee: Box<Expr>,
        args: Vec<Argument>,
    },
    Conditional {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },
    Assignment {
        left: Box<Expr>,
        op: AssignOp,
        right: Box<Expr>,
    },
    ArrowFunction {
        params: Vec<Param>,
        return_type: Option<TypeAnnotation>,
        body: ArrowBody,
        is_async: bool,
    },
    /// Intermediate representation for arrow function typed parameters.
    /// Used during parsing to capture `(x: int, y: string)` before conversion to `ArrowFunction`.
    ArrowParams(Vec<(String, Option<TypeAnnotation>)>),
    Await(Box<Expr>),
    Sequence(Vec<Expr>),
    Function {
        name: Option<String>,
        type_params: Vec<TypeParam>,
        params: Vec<Param>,
        return_type: Option<TypeAnnotation>,
        body: Vec<Statement>,
        is_async: bool,
    },
    Class {
        name: Option<String>,
        type_params: Vec<TypeParam>,
        extends: Option<Box<Expr>>,
        body: Vec<ClassElement>,
        annotations: Vec<String>,
    },
    New {
        callee: Box<Expr>,
        args: Vec<Argument>,
    },
    Match {
        value: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
    },
    Grouping(Box<Expr>),
    Block(Vec<Statement>),
    NullAssert(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TemplatePart {
    String(String),
    Expr(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArrayElement {
    Expr(Box<Expr>),
    Spread(Box<Expr>),
    Elision,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObjectProperty {
    Property { key: PropertyName, value: Box<Expr> },
    Shorthand(String),
    Spread(Box<Expr>),
    ComputedProperty { key: Box<Expr>, value: Box<Expr> },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Argument {
    Expr(Box<Expr>),
    Spread(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum MemberProperty {
    Ident(String),
    Expr(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssignOp {
    Assign,
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,
    PercentAssign,
    PowerAssign,
    AmpAssign,
    PipeAssign,
    CaretAssign,
    ShlAssign,
    ShrAssign,
    UShrAssign,
    AndAssign,
    OrAssign,
    NullishAssign,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    StrictEquals,
    StrictNotEquals,
    Equals,
    NotEquals,
    Less,
    Greater,
    LessEq,
    GreaterEq,
    In,
    Instanceof,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Power,
    Shl,
    Shr,
    UShr,
    Amp,
    Pipe,
    Caret,
    And,
    Or,
    Nullish,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Plus,
    Minus,
    Not,
    Tilde,
    PreIncrement,
    PreDecrement,
    Typeof,
    Void,
    Delete,
    Await,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArrowBody {
    Expr(Box<Expr>),
    Block(Vec<Statement>),
}

// ── Class / Trait elements ───────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ClassElement {
    Method {
        name: PropertyName,
        type_params: Vec<TypeParam>,
        params: Vec<Param>,
        return_type: Option<TypeAnnotation>,
        body: Vec<Statement>,
        is_async: bool,
        is_static: bool,
        is_getter: bool,
        is_setter: bool,
    },
    Field {
        name: PropertyName,
        ty: Option<TypeAnnotation>,
        init: Option<Box<Expr>>,
        is_static: bool,
    },
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TraitElement {
    Method {
        name: PropertyName,
        type_params: Vec<TypeParam>,
        params: Vec<Param>,
        return_type: Option<TypeAnnotation>,
        body: Option<Vec<Statement>>,
    },
    Field {
        name: PropertyName,
        ty: TypeAnnotation,
    },
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PropertyName {
    Ident(String),
    String(String),
    Number(f64),
    Computed(Box<Expr>),
}

// ── Macro ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct MacroRule {
    pub pattern: Vec<Token>,
    pub body: Vec<Token>,
}

// ── Patterns ─────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Identifier(String),
    Literal(Box<Expr>),
    Object(Vec<ObjectPatternField>),
    Array(Vec<ArrayPatternElement>),
    Rest(String),
    As(Box<Pattern>, String),
    Or(Vec<Pattern>),
    Wildcard,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObjectPatternField {
    Property { key: String, pattern: Pattern },
    Shorthand(String),
    Rest(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArrayPatternElement {
    Pattern(Pattern),
    Rest(Pattern),
    Elision,
}

// ── Type Annotations ─────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum TypeAnnotation {
    Identifier(String),
    Builtin(String), // string, int, float, bool, etc.
    Nullable(Box<TypeAnnotation>),
    Function {
        params: Vec<TypeAnnotation>,
        return_type: Box<TypeAnnotation>,
    },
    Generic {
        base: String,
        args: Vec<TypeAnnotation>,
    },
    Object(Vec<TypeField>),
    Array(Box<TypeAnnotation>),
    Tuple(Vec<TypeAnnotation>),
    Dyn(Box<TypeAnnotation>),
    Union(Vec<TypeAnnotation>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeField {
    pub name: String,
    pub ty: TypeAnnotation,
}

// ── Module declarations ──────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ImportDecl {
    pub default: Option<String>,
    pub namespace: Option<String>,
    pub named: Vec<NamedImport>,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NamedImport {
    pub name: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExportDecl {
    Named(Vec<NamedExport>),
    ReExportAll {
        source: String,
    },
    ReExportNamed {
        items: Vec<NamedExport>,
        source: String,
    },
    Declaration(Declaration),
    DefaultExpr(Box<Expr>),
    DefaultFunction {
        name: String,
        type_params: Vec<TypeParam>,
        params: Vec<Param>,
        return_type: Option<TypeAnnotation>,
        body: Vec<Statement>,
        is_async: bool,
    },
    DefaultClass {
        name: String,
        type_params: Vec<TypeParam>,
        extends: Option<Box<Expr>>,
        body: Vec<ClassElement>,
        annotations: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct NamedExport {
    pub name: String,
    pub alias: Option<String>,
}
