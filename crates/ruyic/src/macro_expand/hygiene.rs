use crate::lexer::token::Token;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SyntaxContext(u32);

impl SyntaxContext {
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    pub fn global() -> Self {
        Self(0)
    }
}

impl Default for SyntaxContext {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HygienicToken {
    pub token: Token,
    pub context: SyntaxContext,
}

impl HygienicToken {
    pub fn new(token: Token, context: SyntaxContext) -> Self {
        Self { token, context }
    }

    pub fn with_global_context(token: Token) -> Self {
        Self::new(token, SyntaxContext::global())
    }
}

pub trait HygieneContext {
    fn current_context(&self) -> SyntaxContext;
    fn apply_context(&self, token: Token) -> Token;
    fn fresh_ident(&mut self, base: &str) -> String;
}

#[derive(Debug, Clone)]
pub struct StandardHygieneContext {
    current: SyntaxContext,
    counter: u32,
}

impl StandardHygieneContext {
    pub fn new() -> Self {
        Self {
            current: SyntaxContext::new(),
            counter: 0,
        }
    }

    pub fn for_macro() -> Self {
        Self::new()
    }
}

impl Default for StandardHygieneContext {
    fn default() -> Self {
        Self::new()
    }
}

impl HygieneContext for StandardHygieneContext {
    fn current_context(&self) -> SyntaxContext {
        self.current
    }

    fn apply_context(&self, token: Token) -> Token {
        HygienicToken::new(token, self.current).token
    }

    fn fresh_ident(&mut self, base: &str) -> String {
        self.counter += 1;
        format!("__hygiene_{}_{}", base, self.counter)
    }
}

pub fn apply_hygiene(tokens: &[Token], context: SyntaxContext) -> Vec<Token> {
    tokens
        .iter()
        .map(|t| {
            if matches!(t, Token::Ident(_)) {
                HygienicToken::new(t.clone(), context).token
            } else {
                t.clone()
            }
        })
        .collect()
}

pub fn contexts_compatible(a: SyntaxContext, b: SyntaxContext) -> bool {
    a == b || a == SyntaxContext::global() || b == SyntaxContext::global()
}
