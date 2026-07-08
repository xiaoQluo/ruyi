use ruyic::lexer::{LexerError, Scanner, Token, TokenWithLocation};

fn token_kinds(source: &str) -> Vec<Token> {
    let mut scanner = Scanner::new(source);
    scanner
        .scan_all()
        .unwrap()
        .into_iter()
        .map(|t| t.token)
        .collect()
}

fn tokens(source: &str) -> Vec<TokenWithLocation> {
    let mut scanner = Scanner::new(source);
    scanner.scan_all().unwrap()
}

fn assert_tokens(source: &str, expected: &[Token]) {
    let actual = token_kinds(source);
    assert_eq!(actual, expected, "Token mismatch for source: {}", source);
}

// ── Keywords ─────────────────────────────────────────────────

#[test]
fn test_keywords() {
    assert_tokens(
        "let const fn class trait match if else for while return throw try catch finally async await import export macro type",
        &[
            Token::Let, Token::Const, Token::Fn, Token::Class, Token::Trait,
            Token::Match, Token::If, Token::Else, Token::For, Token::While,
            Token::Return, Token::Throw, Token::Try, Token::Catch, Token::Finally,
            Token::Async, Token::Await, Token::Import, Token::Export, Token::Macro,
            Token::Type, Token::Eof,
        ],
    );
}

#[test]
fn test_boolean_and_null_literals() {
    assert_tokens(
        "true false null",
        &[Token::True, Token::False, Token::Null, Token::Eof],
    );
}

#[test]
fn test_special_identifiers() {
    assert_tokens(
        "self super this",
        &[Token::SelfKw, Token::Super, Token::This, Token::Eof],
    );
}

#[test]
fn test_operator_keywords() {
    assert_tokens(
        "in instanceof typeof void delete",
        &[
            Token::In,
            Token::Instanceof,
            Token::Typeof,
            Token::Void,
            Token::Delete,
            Token::Eof,
        ],
    );
}

// ── Identifiers ──────────────────────────────────────────────

#[test]
fn test_identifiers() {
    assert_tokens(
        "x _myVar firstName camelCaseName",
        &[
            Token::Ident("x".into()),
            Token::Ident("_myVar".into()),
            Token::Ident("firstName".into()),
            Token::Ident("camelCaseName".into()),
            Token::Eof,
        ],
    );
}

#[test]
fn test_identifier_not_keyword() {
    assert_tokens(
        "letx xlet",
        &[
            Token::Ident("letx".into()),
            Token::Ident("xlet".into()),
            Token::Eof,
        ],
    );
}

// ── Operators ────────────────────────────────────────────────

#[test]
fn test_comparison_operators() {
    assert_tokens(
        "=== !== == != < > <= >=",
        &[
            Token::StrictEquals,
            Token::StrictNotEquals,
            Token::Equals,
            Token::NotEquals,
            Token::Less,
            Token::Greater,
            Token::LessEq,
            Token::GreaterEq,
            Token::Eof,
        ],
    );
}

#[test]
fn test_arithmetic_operators() {
    assert_tokens(
        "+ - * / % **",
        &[
            Token::Plus,
            Token::Minus,
            Token::Star,
            Token::Slash,
            Token::Percent,
            Token::Power,
            Token::Eof,
        ],
    );
}

#[test]
fn test_bitwise_operators() {
    assert_tokens(
        "& | ^ ~ << >> >>>",
        &[
            Token::Amp,
            Token::Pipe,
            Token::Caret,
            Token::Tilde,
            Token::Shl,
            Token::Shr,
            Token::UShr,
            Token::Eof,
        ],
    );
}

#[test]
fn test_logical_operators() {
    assert_tokens(
        "&& || ?? !",
        &[
            Token::And,
            Token::Or,
            Token::Nullish,
            Token::Not,
            Token::Eof,
        ],
    );
}

#[test]
fn test_null_safety_operators() {
    assert_tokens(
        "?. ? . ...",
        &[
            Token::OptChain,
            Token::Question,
            Token::Dot,
            Token::Spread,
            Token::Eof,
        ],
    );
}

#[test]
fn test_assignment_operators() {
    assert_tokens(
        "= += -= *= /= %= **= &= |= ^= <<= >>= >>>= &&= ||= ??=",
        &[
            Token::Assign,
            Token::PlusAssign,
            Token::MinusAssign,
            Token::StarAssign,
            Token::SlashAssign,
            Token::PercentAssign,
            Token::PowerAssign,
            Token::AmpAssign,
            Token::PipeAssign,
            Token::CaretAssign,
            Token::ShlAssign,
            Token::ShrAssign,
            Token::UShrAssign,
            Token::AndAssign,
            Token::OrAssign,
            Token::NullishAssign,
            Token::Eof,
        ],
    );
}

#[test]
fn test_arrow_and_inc_dec() {
    assert_tokens(
        "=> ++ --",
        &[
            Token::FatArrow,
            Token::Increment,
            Token::Decrement,
            Token::Eof,
        ],
    );
}

// ── Delimiters ───────────────────────────────────────────────

#[test]
fn test_delimiters() {
    assert_tokens(
        "( ) [ ] { } , ; : :: @ #",
        &[
            Token::LParen,
            Token::RParen,
            Token::LBracket,
            Token::RBracket,
            Token::LBrace,
            Token::RBrace,
            Token::Comma,
            Token::SemiColon,
            Token::Colon,
            Token::DoubleColon,
            Token::At,
            Token::Hash,
            Token::Eof,
        ],
    );
}

// ── Numeric literals ─────────────────────────────────────────

#[test]
fn test_decimal_integers() {
    assert_tokens(
        "42 0 123",
        &[Token::Int(42), Token::Int(0), Token::Int(123), Token::Eof],
    );
}

#[test]
fn test_floats() {
    let toks = token_kinds("3.14 0.5 1.0");
    assert!(matches!(toks[0], Token::Float(f) if (f - 3.14).abs() < 0.0001));
    assert!(matches!(toks[1], Token::Float(f) if (f - 0.5).abs() < 0.0001));
    assert!(matches!(toks[2], Token::Float(f) if (f - 1.0).abs() < 0.0001));
    assert!(matches!(toks[3], Token::Eof));
}

#[test]
fn test_scientific_notation() {
    let toks = token_kinds("1e10 2.5e-3 3E+5");
    assert!(matches!(toks[0], Token::Float(f) if (f - 1e10).abs() < 0.1));
    assert!(matches!(toks[1], Token::Float(f) if (f - 2.5e-3).abs() < 0.0001));
    assert!(matches!(toks[2], Token::Float(f) if (f - 3e5).abs() < 0.1));
    assert!(matches!(toks[3], Token::Eof));
}

#[test]
fn test_hex_numbers() {
    assert_tokens(
        "0xFF 0x0 0xABC",
        &[Token::Int(255), Token::Int(0), Token::Int(2748), Token::Eof],
    );
}

#[test]
fn test_hex_uppercase_prefix() {
    assert_tokens("0Xff", &[Token::Int(255), Token::Eof]);
}

#[test]
fn test_octal_numbers() {
    assert_tokens("0o77 0o0", &[Token::Int(63), Token::Int(0), Token::Eof]);
}

#[test]
fn test_binary_numbers() {
    assert_tokens("0b1010 0b0", &[Token::Int(10), Token::Int(0), Token::Eof]);
}

#[test]
fn test_bigint_literal() {
    assert_tokens("100n", &[Token::BigInt("100".into()), Token::Eof]);
}

#[test]
fn test_hex_bigint() {
    assert_tokens("0xFFn", &[Token::BigInt("0xFF".into()), Token::Eof]);
}

// ── String literals ──────────────────────────────────────────

#[test]
fn test_double_quoted_string() {
    assert_tokens(r#""hello""#, &[Token::String("hello".into()), Token::Eof]);
}

#[test]
fn test_single_quoted_string() {
    assert_tokens(r#"'world'"#, &[Token::String("world".into()), Token::Eof]);
}

#[test]
fn test_string_with_escapes() {
    let toks = token_kinds(r#""line1\nline2\ttab""#);
    assert_eq!(toks[0], Token::String("line1\nline2\ttab".into()));
}

#[test]
fn test_string_with_unicode_escape() {
    let toks = token_kinds(r#""\u{1F600}""#);
    assert_eq!(toks[0], Token::String("😀".into()));
}

#[test]
fn test_string_with_hex_escape() {
    let toks = token_kinds(r#""\x41""#);
    assert_eq!(toks[0], Token::String("A".into()));
}

#[test]
fn test_string_with_4digit_unicode() {
    let toks = token_kinds(r#""\u0041""#);
    assert_eq!(toks[0], Token::String("A".into()));
}

// ── Template strings ─────────────────────────────────────────

#[test]
fn test_simple_template_string() {
    let toks = token_kinds("`hello`");
    assert_eq!(toks[0], Token::TemplateString("hello".into()));
}

#[test]
fn test_template_string_with_interpolation() {
    let toks = token_kinds("`hello ${name}`");
    assert_eq!(toks[0], Token::TemplateString("hello ".into()));
    assert_eq!(toks[1], Token::TemplateExprStart);
    assert_eq!(toks[2], Token::Ident("name".into()));
    assert_eq!(toks[3], Token::TemplateExprEnd);
    assert_eq!(toks[4], Token::TemplateString("".into()));
}

#[test]
fn test_template_string_multi_interpolation() {
    let toks = token_kinds("`a ${x} b ${y} c`");
    assert_eq!(toks[0], Token::TemplateString("a ".into()));
    assert_eq!(toks[1], Token::TemplateExprStart);
    assert_eq!(toks[2], Token::Ident("x".into()));
    assert_eq!(toks[3], Token::TemplateExprEnd);
    assert_eq!(toks[4], Token::TemplateString(" b ".into()));
    assert_eq!(toks[5], Token::TemplateExprStart);
    assert_eq!(toks[6], Token::Ident("y".into()));
    assert_eq!(toks[7], Token::TemplateExprEnd);
    assert_eq!(toks[8], Token::TemplateString(" c".into()));
}

#[test]
fn test_template_string_multiline() {
    let toks = token_kinds("`line1\nline2`");
    assert_eq!(toks[0], Token::TemplateString("line1\nline2".into()));
}

// ── Comments ─────────────────────────────────────────────────

#[test]
fn test_line_comment() {
    assert_tokens(
        "let x // this is a comment\n42",
        &[
            Token::Let,
            Token::Ident("x".into()),
            Token::Int(42),
            Token::Eof,
        ],
    );
}

#[test]
fn test_block_comment() {
    assert_tokens(
        "let /* middle */ x",
        &[Token::Let, Token::Ident("x".into()), Token::Eof],
    );
}

#[test]
fn test_block_comment_multiline() {
    assert_tokens(
        "let /* \n multiline \n */ x",
        &[Token::Let, Token::Ident("x".into()), Token::Eof],
    );
}

#[test]
fn test_doc_comment() {
    assert_tokens(
        "/** doc */ let x",
        &[Token::Let, Token::Ident("x".into()), Token::Eof],
    );
}

// ── Location tracking ────────────────────────────────────────

#[test]
fn test_location_tracking() {
    let toks = tokens("let\nx");
    assert_eq!(toks[0].start.line, 1);
    assert_eq!(toks[0].start.col, 1);
    assert_eq!(toks[1].start.line, 2);
    assert_eq!(toks[1].start.col, 1);
}

#[test]
fn test_location_after_comment() {
    let toks = tokens("// comment\nlet");
    assert_eq!(toks[0].start.line, 2);
    assert_eq!(toks[0].start.col, 1);
}

// ── Error cases ──────────────────────────────────────────────

#[test]
fn test_invalid_character() {
    let mut scanner = Scanner::new("let €");
    scanner.next_token().unwrap(); // let
    let err = scanner.next_token().unwrap_err();
    assert!(matches!(
        err,
        LexerError::InvalidCharacter {
            ch: '€',
            line: 1,
            col: 5
        }
    ));
}

#[test]
fn test_unterminated_string() {
    let mut scanner = Scanner::new(r#""hello"#);
    let err = scanner.next_token().unwrap_err();
    assert!(matches!(
        err,
        LexerError::UnterminatedString { line: 1, col: 1 }
    ));
}

#[test]
fn test_unterminated_block_comment() {
    let mut scanner = Scanner::new("/* hello");
    let err = scanner.next_token().unwrap_err();
    assert!(matches!(
        err,
        LexerError::UnterminatedComment { line: 1, col: 1 }
    ));
}

#[test]
fn test_unterminated_template() {
    let mut scanner = Scanner::new("`hello");
    let err = scanner.next_token().unwrap_err();
    assert!(matches!(
        err,
        LexerError::UnterminatedTemplate { line: 1, col: 2 }
    ));
}

#[test]
fn test_invalid_escape() {
    let mut scanner = Scanner::new(r#""\z""#);
    let err = scanner.next_token().unwrap_err();
    assert!(matches!(err, LexerError::InvalidEscape { line: 1, col: 1 }));
}

#[test]
fn test_invalid_number_no_digits_after_prefix() {
    let mut scanner = Scanner::new("0x");
    let err = scanner.next_token().unwrap_err();
    assert!(matches!(
        err,
        LexerError::InvalidNumber {
            line: 1,
            col: 1,
            ..
        }
    ));
}

#[test]
fn test_invalid_number_bad_exponent() {
    let mut scanner = Scanner::new("1e");
    let err = scanner.next_token().unwrap_err();
    assert!(matches!(
        err,
        LexerError::InvalidNumber {
            line: 1,
            col: 1,
            ..
        }
    ));
}

// ── Complex / realistic snippets ─────────────────────────────

#[test]
fn test_variable_declaration() {
    assert_tokens(
        "let x = 42;",
        &[
            Token::Let,
            Token::Ident("x".into()),
            Token::Assign,
            Token::Int(42),
            Token::SemiColon,
            Token::Eof,
        ],
    );
}

#[test]
fn test_function_declaration() {
    assert_tokens(
        "fn add(a: int, b: int): int { return a + b; }",
        &[
            Token::Fn,
            Token::Ident("add".into()),
            Token::LParen,
            Token::Ident("a".into()),
            Token::Colon,
            Token::Ident("int".into()),
            Token::Comma,
            Token::Ident("b".into()),
            Token::Colon,
            Token::Ident("int".into()),
            Token::RParen,
            Token::Colon,
            Token::Ident("int".into()),
            Token::LBrace,
            Token::Return,
            Token::Ident("a".into()),
            Token::Plus,
            Token::Ident("b".into()),
            Token::SemiColon,
            Token::RBrace,
            Token::Eof,
        ],
    );
}

#[test]
fn test_optional_chaining() {
    assert_tokens(
        "user?.name",
        &[
            Token::Ident("user".into()),
            Token::OptChain,
            Token::Ident("name".into()),
            Token::Eof,
        ],
    );
}

#[test]
fn test_nullish_coalescing() {
    assert_tokens(
        "val ?? default",
        &[
            Token::Ident("val".into()),
            Token::Nullish,
            Token::Ident("default".into()),
            Token::Eof,
        ],
    );
}

#[test]
fn test_arrow_function() {
    assert_tokens(
        "(x) => x * 2",
        &[
            Token::LParen,
            Token::Ident("x".into()),
            Token::RParen,
            Token::FatArrow,
            Token::Ident("x".into()),
            Token::Star,
            Token::Int(2),
            Token::Eof,
        ],
    );
}

#[test]
fn test_spread_operator() {
    assert_tokens(
        "[a, ...rest]",
        &[
            Token::LBracket,
            Token::Ident("a".into()),
            Token::Comma,
            Token::Spread,
            Token::Ident("rest".into()),
            Token::RBracket,
            Token::Eof,
        ],
    );
}

#[test]
fn test_match_expression() {
    assert_tokens(
        "match (value) { 1 => \"one\", _ => \"other\" }",
        &[
            Token::Match,
            Token::LParen,
            Token::Ident("value".into()),
            Token::RParen,
            Token::LBrace,
            Token::Int(1),
            Token::FatArrow,
            Token::String("one".into()),
            Token::Comma,
            Token::Underscore,
            Token::FatArrow,
            Token::String("other".into()),
            Token::RBrace,
            Token::Eof,
        ],
    );
}

#[test]
fn test_generic_function() {
    assert_tokens(
        "fn identity<T>(x: T): T",
        &[
            Token::Fn,
            Token::Ident("identity".into()),
            Token::Less,
            Token::Ident("T".into()),
            Token::Greater,
            Token::LParen,
            Token::Ident("x".into()),
            Token::Colon,
            Token::Ident("T".into()),
            Token::RParen,
            Token::Colon,
            Token::Ident("T".into()),
            Token::Eof,
        ],
    );
}

#[test]
fn test_trait_declaration() {
    assert_tokens(
        "trait Printable { fn format(self): string; }",
        &[
            Token::Trait,
            Token::Ident("Printable".into()),
            Token::LBrace,
            Token::Fn,
            Token::Ident("format".into()),
            Token::LParen,
            Token::SelfKw,
            Token::RParen,
            Token::Colon,
            Token::Ident("string".into()),
            Token::SemiColon,
            Token::RBrace,
            Token::Eof,
        ],
    );
}

#[test]
fn test_whitespace_between_tokens() {
    assert_tokens(
        "let   x\t=\n42",
        &[
            Token::Let,
            Token::Ident("x".into()),
            Token::Assign,
            Token::Int(42),
            Token::Eof,
        ],
    );
}

#[test]
fn test_eof_only() {
    assert_tokens("", &[Token::Eof]);
}
