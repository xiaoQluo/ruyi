/**
 * Token definitions for the Ruyi lexical analyzer.
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */

/// Source location (line and column) for error reporting and AST mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    pub line: usize,
    pub col: usize,
}

impl Location {
    pub fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

/// A token with its source location.
#[derive(Debug, Clone, PartialEq)]
pub struct TokenWithLocation {
    pub token: Token,
    pub start: Location,
    pub end: Location,
}

/// All token types in the Ruyi language.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // ── Keywords ───────────────────────────────────────────────
    Let,
    Const,
    Fn,
    Class,
    Trait,
    Match,
    If,
    Else,
    For,
    While,
    Return,
    Throw,
    Try,
    Catch,
    Finally,
    Async,
    Await,
    Import,
    Export,
    Macro,
    Type,
    /// Boolean literal `true`
    True,
    /// Boolean literal `false`
    False,
    /// Null literal `null`
    Null,
    /// `self` reference
    SelfKw,
    /// `super` reference
    Super,
    /// `this` reference
    This,
    /// `in` operator
    In,
    /// `instanceof` operator
    Instanceof,
    /// `typeof` operator
    Typeof,
    /// `void` operator
    Void,
    /// `delete` operator
    Delete,
    /// `as` keyword
    As,
    /// `from` keyword
    From,
    /// `extends` keyword
    Extends,
    /// `impl` keyword
    Impl,
    /// `dyn` keyword
    Dyn,
    /// `static` keyword
    Static,
    /// `get` keyword
    Get,
    /// `set` keyword
    Set,
    /// `new` keyword
    New,
    /// `of` keyword
    Of,
    /// `break` keyword
    Break,
    /// `continue` keyword
    Continue,
    /// `_` wildcard
    Underscore,

    // ── Identifiers ────────────────────────────────────────────
    Ident(String),

    // ── Literals ───────────────────────────────────────────────
    /// Integer literal (decimal, hex, octal, binary)
    Int(i64),
    /// Big integer literal (value, stored as string to preserve precision)
    BigInt(String),
    /// Floating-point literal
    Float(f64),
    /// String literal (single or double quoted)
    String(String),
    /// Template string literal (backtick quoted)
    TemplateString(String),
    /// Template string interpolation start `${`
    TemplateExprStart,
    /// Template string interpolation end `}`
    TemplateExprEnd,

    // ── Comparison operators ───────────────────────────────────
    /// `===`
    StrictEquals,
    /// `!==`
    StrictNotEquals,
    /// `==`
    Equals,
    /// `!=`
    NotEquals,
    /// `<`
    Less,
    /// `>`
    Greater,
    /// `<=`
    LessEq,
    /// `>=`
    GreaterEq,

    // ── Arithmetic operators ───────────────────────────────────
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `%`
    Percent,
    /// `**`
    Power,

    // ── Bitwise operators ──────────────────────────────────────
    /// `&`
    Amp,
    /// `|`
    Pipe,
    /// `^`
    Caret,
    /// `~`
    Tilde,
    /// `<<`
    Shl,
    /// `>>`
    Shr,
    /// `>>>`
    UShr,

    // ── Logical operators ──────────────────────────────────────
    /// `&&`
    And,
    /// `||`
    Or,
    /// `??`
    Nullish,
    /// `!`
    Not,

    // ── Null-safety / access operators ─────────────────────────
    /// `?.`
    OptChain,
    /// `?`
    Question,
    /// `.`
    Dot,
    /// `...`
    Spread,

    // ── Assignment operators ───────────────────────────────────
    /// `=`
    Assign,
    /// `+=`
    PlusAssign,
    /// `-=`
    MinusAssign,
    /// `*=`
    StarAssign,
    /// `/=`
    SlashAssign,
    /// `%=`
    PercentAssign,
    /// `**=`
    PowerAssign,
    /// `&=`
    AmpAssign,
    /// `|=`
    PipeAssign,
    /// `^=`
    CaretAssign,
    /// `<<=`
    ShlAssign,
    /// `>>=`
    ShrAssign,
    /// `>>>=`
    UShrAssign,
    /// `&&=`
    AndAssign,
    /// `||=`
    OrAssign,
    /// `??=`
    NullishAssign,

    // ── Arrow / increment ──────────────────────────────────────
    /// `=>`
    FatArrow,
    /// `++`
    Increment,
    /// `--`
    Decrement,

    // ── Delimiters / punctuators ───────────────────────────────
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `,`
    Comma,
    /// `;`
    SemiColon,
    /// `:`
    Colon,
    /// `::`
    DoubleColon,
    /// `@`
    At,
    /// `#`
    Hash,
    /// `$`
    Dollar,

    // ── Special ────────────────────────────────────────────────
    /// End of file
    Eof,
}

impl Token {
    /// Returns a human-readable name for the token (used in error messages).
    pub fn name(&self) -> String {
        match self {
            Token::Let => "keyword 'let'".into(),
            Token::Const => "keyword 'const'".into(),
            Token::Fn => "keyword 'fn'".into(),
            Token::Class => "keyword 'class'".into(),
            Token::Trait => "keyword 'trait'".into(),
            Token::Impl => "keyword 'impl'".into(),
            Token::Dyn => "keyword 'dyn'".into(),
            Token::Match => "keyword 'match'".into(),
            Token::If => "keyword 'if'".into(),
            Token::Else => "keyword 'else'".into(),
            Token::For => "keyword 'for'".into(),
            Token::While => "keyword 'while'".into(),
            Token::Return => "keyword 'return'".into(),
            Token::Throw => "keyword 'throw'".into(),
            Token::Try => "keyword 'try'".into(),
            Token::Catch => "keyword 'catch'".into(),
            Token::Finally => "keyword 'finally'".into(),
            Token::Async => "keyword 'async'".into(),
            Token::Await => "keyword 'await'".into(),
            Token::Import => "keyword 'import'".into(),
            Token::Export => "keyword 'export'".into(),
            Token::Macro => "keyword 'macro'".into(),
            Token::Type => "keyword 'type'".into(),
            Token::True => "literal 'true'".into(),
            Token::False => "literal 'false'".into(),
            Token::Null => "literal 'null'".into(),
            Token::SelfKw => "keyword 'self'".into(),
            Token::Super => "keyword 'super'".into(),
            Token::This => "keyword 'this'".into(),
            Token::In => "keyword 'in'".into(),
            Token::Instanceof => "keyword 'instanceof'".into(),
            Token::Typeof => "keyword 'typeof'".into(),
            Token::Void => "keyword 'void'".into(),
            Token::Delete => "keyword 'delete'".into(),
            Token::As => "keyword 'as'".into(),
            Token::From => "keyword 'from'".into(),
            Token::Extends => "keyword 'extends'".into(),
            Token::Static => "keyword 'static'".into(),
            Token::Get => "keyword 'get'".into(),
            Token::Set => "keyword 'set'".into(),
            Token::New => "keyword 'new'".into(),
            Token::Of => "keyword 'of'".into(),
            Token::Break => "keyword 'break'".into(),
            Token::Continue => "keyword 'continue'".into(),
            Token::Underscore => "'_'".into(),
            Token::Ident(_) => "identifier".into(),
            Token::Int(_) => "integer literal".into(),
            Token::BigInt(_) => "big integer literal".into(),
            Token::Float(_) => "float literal".into(),
            Token::String(_) => "string literal".into(),
            Token::TemplateString(_) => "template string literal".into(),
            Token::TemplateExprStart => "'${'".into(),
            Token::TemplateExprEnd => "'}'".into(),
            Token::StrictEquals => "'==='".into(),
            Token::StrictNotEquals => "'!=='".into(),
            Token::Equals => "'=='".into(),
            Token::NotEquals => "'!='".into(),
            Token::Less => "'<'".into(),
            Token::Greater => "'>'".into(),
            Token::LessEq => "'<='".into(),
            Token::GreaterEq => "'>='".into(),
            Token::Plus => "'+'".into(),
            Token::Minus => "'-'".into(),
            Token::Star => "'*'".into(),
            Token::Slash => "'/'".into(),
            Token::Percent => "'%'".into(),
            Token::Power => "'**'".into(),
            Token::Amp => "'&'".into(),
            Token::Pipe => "'|'".into(),
            Token::Caret => "'^'".into(),
            Token::Tilde => "'~'".into(),
            Token::Shl => "'<<'".into(),
            Token::Shr => "'>>'".into(),
            Token::UShr => "'>>>'".into(),
            Token::And => "'&&'".into(),
            Token::Or => "'||'".into(),
            Token::Nullish => "'??'".into(),
            Token::Not => "'!'".into(),
            Token::OptChain => "'?.'".into(),
            Token::Question => "'?'".into(),
            Token::Dot => "'.'".into(),
            Token::Spread => "'...'".into(),
            Token::Assign => "'='".into(),
            Token::PlusAssign => "'+='".into(),
            Token::MinusAssign => "'-='".into(),
            Token::StarAssign => "'*='".into(),
            Token::SlashAssign => "'/='".into(),
            Token::PercentAssign => "'%='".into(),
            Token::PowerAssign => "'**='".into(),
            Token::AmpAssign => "'&='".into(),
            Token::PipeAssign => "'|='".into(),
            Token::CaretAssign => "'^='".into(),
            Token::ShlAssign => "'<<='".into(),
            Token::ShrAssign => "'>>='".into(),
            Token::UShrAssign => "'>>>='".into(),
            Token::AndAssign => "'&&='".into(),
            Token::OrAssign => "'||='".into(),
            Token::NullishAssign => "'??='".into(),
            Token::FatArrow => "'=>'".into(),
            Token::Increment => "'++'".into(),
            Token::Decrement => "'--'".into(),
            Token::LParen => "'('".into(),
            Token::RParen => "')'".into(),
            Token::LBracket => "'['".into(),
            Token::RBracket => "']'".into(),
            Token::LBrace => "'{'".into(),
            Token::RBrace => "'}'".into(),
            Token::Comma => "','".into(),
            Token::SemiColon => "';'".into(),
            Token::Colon => "':'".into(),
            Token::DoubleColon => "'::'".into(),
            Token::At => "'@'".into(),
            Token::Hash => "'#'".into(),
            Token::Dollar => "'$'".into(),
            Token::Eof => "end of file".into(),
        }
    }

    /// Returns true if this token is a keyword.
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            Token::Let
                | Token::Const
                | Token::Fn
                | Token::Class
            | Token::Trait
            | Token::Impl
            | Token::Dyn
            | Token::Match
                | Token::If
                | Token::Else
                | Token::For
                | Token::While
                | Token::Return
                | Token::Throw
                | Token::Try
                | Token::Catch
                | Token::Finally
                | Token::Async
                | Token::Await
                | Token::Import
                | Token::Export
                | Token::Macro
                | Token::Type
                | Token::True
                | Token::False
                | Token::Null
                | Token::SelfKw
                | Token::Super
                | Token::This
                | Token::In
                | Token::Instanceof
            | Token::Typeof
            | Token::Void
            | Token::Delete
            | Token::As
            | Token::From
            | Token::Extends
            | Token::Static
            | Token::Get
            | Token::Set
            | Token::New
            | Token::Of
            | Token::Break
            | Token::Continue
            | Token::Underscore
        )
    }
}
