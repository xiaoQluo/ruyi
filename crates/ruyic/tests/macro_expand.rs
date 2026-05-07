use ruyic::macro_expand::{expand_macros, MacroRegistry};
use ruyic::parser::*;

fn parse_ok(source: &str) -> Program {
    let mut parser = Parser::new(source).expect("lexer should not fail");
    parser.parse().expect("parse should succeed")
}

fn parse_err(source: &str) -> ParseError {
    let mut parser = Parser::new(source).expect("lexer should not fail");
    parser.parse().expect_err("parse should fail")
}

fn single_item(source: &str) -> ModuleItem {
    let program = parse_ok(source);
    assert_eq!(program.items.len(), 1, "expected exactly one module item");
    program.items.into_iter().next().unwrap()
}

fn single_decl(source: &str) -> Declaration {
    match single_item(source) {
        ModuleItem::Declaration(d) => d,
        other => panic!("expected declaration, got {:?}", other),
    }
}

// ── Macro declaration parsing ─────────────────────────────────

#[test]
fn test_macro_declaration_simple() {
    let decl = single_decl(
        r#"
        macro debug {
            ($expr) => {
                print($expr);
            }
        }
    "#,
    );
    match decl {
        Declaration::Macro { name, rules } => {
            assert_eq!(name, "debug");
            assert_eq!(rules.len(), 1);
        }
        _ => panic!("expected macro declaration"),
    }
}

#[test]
fn test_macro_declaration_multiple_rules() {
    let decl = single_decl(
        r#"
        macro vec {
            () => { [] }
            ($elem) => { [$elem] }
            ($($elem),*) => { [$($elem),*] }
        }
    "#,
    );
    match decl {
        Declaration::Macro { name, rules } => {
            assert_eq!(name, "vec");
            assert_eq!(rules.len(), 3);
        }
        _ => panic!("expected macro declaration"),
    }
}

// ── Macro expansion ─────────────────────────────────────────────

#[test]
fn test_macro_expand_basic() {
    let source = r#"
        macro hello {
            () => { print("hello"); }
        }
        hello();
    "#;
    let program = parse_ok(source);
    let registry = MacroRegistry::with_builtins();
    let result = expand_macros(&program, &registry);
    assert!(
        result.is_ok(),
        "macro expansion should succeed: {:?}",
        result
    );
}

#[test]
fn test_macro_expand_with_arg() {
    let source = r#"
        macro debug {
            ($x) => { print($x); }
        }
        debug(42);
    "#;
    let program = parse_ok(source);
    let registry = MacroRegistry::with_builtins();
    let result = expand_macros(&program, &registry);
    assert!(
        result.is_ok(),
        "macro expansion should succeed: {:?}",
        result
    );
}

#[test]
fn test_macro_expand_multiple_rules() {
    let source = r#"
        macro one_or_two {
            () => { 1 }
            ($x) => { $x }
        }
        let a = one_or_two();
        let b = one_or_two(42);
    "#;
    let program = parse_ok(source);
    let registry = MacroRegistry::with_builtins();
    let result = expand_macros(&program, &registry);
    assert!(
        result.is_ok(),
        "macro expansion should succeed: {:?}",
        result
    );
}

// ── Built-in macros ─────────────────────────────────────────────

#[test]
fn test_builtin_todo() {
    let source = "todo!();";
    let program = parse_ok(source);
    let registry = MacroRegistry::with_builtins();
    let result = expand_macros(&program, &registry);
    assert!(result.is_ok(), "todo! should expand: {:?}", result);
}

#[test]
fn test_builtin_unreachable() {
    let source = "unreachable!();";
    let program = parse_ok(source);
    let registry = MacroRegistry::with_builtins();
    let result = expand_macros(&program, &registry);
    assert!(result.is_ok(), "unreachable! should expand: {:?}", result);
}

// ── Macro errors ───────────────────────────────────────────────

#[test]
fn test_macro_undefined() {
    let source = "foo();";
    let program = parse_ok(source);
    let registry = MacroRegistry::with_builtins();
    let result = expand_macros(&program, &registry);
    assert!(
        result.is_ok(),
        "undefined macro should not error if not called"
    );
}

#[test]
fn test_macro_registry_contains() {
    let registry = MacroRegistry::with_builtins();
    assert!(registry.contains("todo"));
    assert!(registry.contains("unreachable"));
    assert!(registry.contains("stringify"));
    assert!(!registry.contains("nonexistent"));
}

#[test]
fn test_macro_registry_user_macros() {
    let mut registry = MacroRegistry::with_builtins();
    let source = r#"
        macro mymacro {
            ($x) => { $x }
        }
    "#;
    let program = parse_ok(source);
    for item in program.items {
        if let ModuleItem::Declaration(Declaration::Macro { name, .. }) = item {
            let macros = registry.get_macro(&name);
            assert!(macros.is_some(), "user macro should be registered");
        }
    }
}

// ── Hygiene ────────────────────────────────────────────────────

#[test]
fn test_macro_hygiene_context() {
    use ruyic::macro_expand::hygiene::{HygieneContext, StandardHygieneContext, SyntaxContext};

    let mut ctx = StandardHygieneContext::new();
    let ident = ctx.fresh_ident("temp");
    assert!(ident.contains("__hygiene_"));
    assert!(ident.contains("temp"));
}

#[test]
fn test_macro_hygiene_unique_contexts() {
    use ruyic::macro_expand::hygiene::SyntaxContext;

    let ctx1 = SyntaxContext::new();
    let ctx2 = SyntaxContext::new();
    assert_ne!(ctx1, ctx2);
    assert_ne!(ctx1, SyntaxContext::global());
    assert_eq!(SyntaxContext::global(), SyntaxContext::global());
}

// ── Pattern matching ─────────────────────────────────────────────

#[test]
fn test_pattern_meta_var() {
    use ruyic::macro_expand::pattern::{parse_pattern, PatternToken};

    let tokens = vec![
        ruyic::lexer::token::Token::LParen,
        ruyic::lexer::token::Token::Dollar,
        ruyic::lexer::token::Token::Ident("x".to_string()),
        ruyic::lexer::token::Token::RParen,
    ];
    let result = parse_pattern(&tokens);
    assert!(result.is_ok());
    let pattern = result.unwrap();
    assert!(!pattern.tokens.is_empty());
}

#[test]
fn test_pattern_repetition() {
    use ruyic::macro_expand::pattern::{parse_pattern, RepetitionMode};

    let tokens = vec![
        ruyic::lexer::token::Token::Dollar,
        ruyic::lexer::token::Token::LParen,
        ruyic::lexer::token::Token::Ident("x".to_string()),
        ruyic::lexer::token::Token::RParen,
        ruyic::lexer::token::Token::Comma,
        ruyic::lexer::token::Token::Star,
    ];
    let result = parse_pattern(&tokens);
    assert!(result.is_ok());
}

// ── Expansion depth ───────────────────────────────────────────

#[test]
fn test_expansion_depth_limit() {
    use ruyic::macro_expand::{MacroError, MAX_EXPANSION_DEPTH};

    assert_eq!(MAX_EXPANSION_DEPTH, 128);
}
