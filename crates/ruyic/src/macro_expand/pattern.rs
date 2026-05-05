use crate::lexer::token::Token;
use crate::macro_expand::{MacroError, MacroResult};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ParsedPattern {
    pub tokens: Vec<PatternToken>,
    pub separators: Vec<Separator>,
    pub repetitions: Vec<RepetitionMode>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternToken {
    Literal(Token),
    MetaVar {
        name: String,
        kind: MetaVarKind,
    },
    Repetition {
        inner: Vec<PatternToken>,
        separator: Option<Separator>,
        mode: RepetitionMode,
    },
    Optional {
        inner: Vec<PatternToken>,
        separator: Option<Separator>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Separator {
    Comma,
    SemiColon,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RepetitionMode {
    ZeroOrMore,
    OneOrMore,
    ZeroOrOne,
}

impl RepetitionMode {
    pub fn allows_zero(&self) -> bool {
        matches!(self, RepetitionMode::ZeroOrMore | RepetitionMode::ZeroOrOne)
    }

    pub fn requires_one(&self) -> bool {
        matches!(self, RepetitionMode::OneOrMore)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MetaVarKind {
    Expr,
    Stmt,
    Pattern,
    Type,
    Ident,
    Any,
}

impl MetaVarKind {
    /// Parses a metavariable kind from a string.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "expr" => Some(MetaVarKind::Expr),
            "stmt" => Some(MetaVarKind::Stmt),
            "pat" | "pattern" => Some(MetaVarKind::Pattern),
            "ty" | "type" => Some(MetaVarKind::Type),
            "ident" => Some(MetaVarKind::Ident),
            _ => None,
        }
    }

    /// Returns the default kind when none is specified.
    pub fn default_for(kind: &str) -> Self {
        match kind {
            "expr" => MetaVarKind::Expr,
            "stmt" => MetaVarKind::Stmt,
            "pat" | "pattern" => MetaVarKind::Pattern,
            "ty" | "type" => MetaVarKind::Type,
            "ident" => MetaVarKind::Ident,
            _ => MetaVarKind::Any,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MatchResult {
    pub captures: HashMap<String, CapturedTokens>,
    pub matched: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CapturedTokens {
    pub tokens: Vec<Token>,
    pub repetitions: Option<Vec<Vec<Token>>>,
}

pub struct PatternMatcher {
    pattern: ParsedPattern,
    pos: usize,
    input: Vec<Token>,
    captures: HashMap<String, CapturedTokens>,
}

impl PatternMatcher {
    pub fn new(pattern: ParsedPattern, input: Vec<Token>) -> Self {
        Self {
            pattern,
            pos: 0,
            input,
            captures: HashMap::new(),
        }
    }

    pub fn match_pattern(&mut self) -> MacroResult<MatchResult> {
        let tokens = self.pattern.tokens.clone();
        if self.match_tokens(&tokens) {
            if self.pos == self.input.len() {
                return Ok(MatchResult {
                    captures: std::mem::take(&mut self.captures),
                    matched: true,
                });
            }
        }
        Ok(MatchResult {
            captures: self.captures.clone(),
            matched: false,
        })
    }

    fn match_tokens(&mut self, tokens: &[PatternToken]) -> bool {
        for token in tokens {
            if !self.match_token(token) {
                return false;
            }
        }
        true
    }

    fn match_token(&mut self, token: &PatternToken) -> bool {
        match token {
            PatternToken::Literal(tok) => self.match_literal(tok),
            PatternToken::MetaVar { name, kind } => self.match_metavar(name, kind),
            PatternToken::Repetition {
                inner,
                separator,
                mode,
            } => self.match_repetition(inner, separator.as_ref(), *mode),
            PatternToken::Optional { inner, separator } => {
                self.match_optional(inner, separator.as_ref())
            }
        }
    }

    fn match_literal(&mut self, expected: &Token) -> bool {
        if let Some(actual) = self.input.get(self.pos) {
            if actual == expected {
                self.pos += 1;
                return true;
            }
        }
        false
    }

    fn match_metavar(&mut self, name: &str, kind: &MetaVarKind) -> bool {
        if let Some(_actual) = self.input.get(self.pos) {
            let captures = match kind {
                MetaVarKind::Expr => self.capture_expr(),
                MetaVarKind::Stmt => self.capture_stmt(),
                MetaVarKind::Pattern => self.capture_pattern(),
                MetaVarKind::Type => self.capture_type(),
                MetaVarKind::Ident => self.capture_ident(),
                MetaVarKind::Any => self.capture_until_delimiter(),
            };

            if let Some(tokens) = captures {
                self.captures.insert(
                    name.to_string(),
                    CapturedTokens {
                        tokens,
                        repetitions: None,
                    },
                );
                return true;
            }
        }
        false
    }

    fn capture_expr(&mut self) -> Option<Vec<Token>> {
        let start = self.pos;
        let mut depth = 0;

        while let Some(token) = self.input.get(self.pos) {
            match token {
                Token::LParen | Token::LBracket | Token::LBrace => depth += 1,
                Token::RParen | Token::RBracket | Token::RBrace => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                }
                Token::SemiColon if depth == 0 => break,
                _ => {}
            }
            self.pos += 1;
        }

        if self.pos > start {
            Some(self.input[start..self.pos].to_vec())
        } else {
            None
        }
    }

    fn capture_stmt(&mut self) -> Option<Vec<Token>> {
        self.capture_expr()
    }

    fn capture_pattern(&mut self) -> Option<Vec<Token>> {
        let start = self.pos;
        let mut depth = 0;

        while let Some(token) = self.input.get(self.pos) {
            match token {
                Token::LParen | Token::LBracket | Token::LBrace => depth += 1,
                Token::RParen | Token::RBracket | Token::RBrace => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                }
                Token::Comma | Token::FatArrow if depth == 0 => break,
                _ => {}
            }
            self.pos += 1;
        }

        if self.pos > start {
            Some(self.input[start..self.pos].to_vec())
        } else {
            None
        }
    }

    fn capture_type(&mut self) -> Option<Vec<Token>> {
        let start = self.pos;

        while let Some(token) = self.input.get(self.pos) {
            match token {
                Token::Ident(_) => {
                    self.pos += 1;
                }
                Token::Question | Token::Less | Token::Greater | Token::Comma => {
                    self.pos += 1;
                }
                Token::LParen | Token::RParen | Token::LBracket | Token::RBracket => {
                    break;
                }
                _ => break,
            }
        }

        if self.pos > start {
            Some(self.input[start..self.pos].to_vec())
        } else {
            None
        }
    }

    fn capture_ident(&mut self) -> Option<Vec<Token>> {
        if let Some(Token::Ident(_)) = self.input.get(self.pos) {
            self.pos += 1;
            Some(self.input[self.pos - 1..self.pos].to_vec())
        } else {
            None
        }
    }

    /// Captures tokens until a delimiter.
    fn capture_until_delimiter(&mut self) -> Option<Vec<Token>> {
        let start = self.pos;

        while let Some(token) = self.input.get(self.pos) {
            match token {
                Token::Comma | Token::SemiColon | Token::RParen => break,
                _ => {
                    self.pos += 1;
                }
            }
        }

        if self.pos > start {
            Some(self.input[start..self.pos].to_vec())
        } else {
            None
        }
    }

    /// Matches a repetition pattern.
    fn match_repetition(
        &mut self,
        inner: &[PatternToken],
        separator: Option<&Separator>,
        mode: RepetitionMode,
    ) -> bool {
        let mut matches = Vec::new();
        let mut first = true;

        loop {
            if let Some(sep) = separator {
                if !first {
                    let sep_token = match sep {
                        Separator::Comma => Token::Comma,
                        Separator::SemiColon => Token::SemiColon,
                    };
                    if !self.match_literal(&sep_token) {
                        break;
                    }
                }
                first = false;
            }

            // Try to match inner pattern
            let _start = self.pos;
            let mut inner_matcher = PatternMatcher {
                pattern: ParsedPattern {
                    tokens: inner.to_vec(),
                    separators: vec![],
                    repetitions: vec![],
                },
                pos: self.pos,
                input: self.input.clone(),
                captures: HashMap::new(),
            };

            if inner_matcher.match_tokens(inner) {
                matches.push(inner_matcher.captures);
                self.pos = inner_matcher.pos;
                continue;
            }

            break;
        }

        if mode.requires_one() && matches.is_empty() {
            return false;
        }

        if !mode.allows_zero() && matches.is_empty() {
            return false;
        }

        true
    }

    fn match_optional(&mut self, inner: &[PatternToken], _separator: Option<&Separator>) -> bool {
        let start = self.pos;
        let mut inner_matcher = PatternMatcher {
            pattern: ParsedPattern {
                tokens: inner.to_vec(),
                separators: vec![],
                repetitions: vec![],
            },
            pos: self.pos,
            input: self.input.clone(),
            captures: HashMap::new(),
        };

        if inner_matcher.match_tokens(inner) {
            self.pos = inner_matcher.pos;
            for (name, cap) in inner_matcher.captures {
                self.captures.insert(name, cap);
            }
            true
        } else {
            self.pos = start;
            true
        }
    }
}

pub fn parse_pattern(tokens: &[Token]) -> MacroResult<ParsedPattern> {
    let mut parser = PatternParser {
        tokens: tokens.to_vec(),
        pos: 0,
    };
    parser.parse()
}

struct PatternParser {
    tokens: Vec<Token>,
    pos: usize,
}

impl PatternParser {
    fn parse(&mut self) -> MacroResult<ParsedPattern> {
        let tokens = self.parse_tokens()?;
        Ok(ParsedPattern {
            tokens,
            separators: vec![],
            repetitions: vec![],
        })
    }

    fn parse_tokens(&mut self) -> MacroResult<Vec<PatternToken>> {
        let mut result = Vec::new();

        while let Some(token) = self.tokens.get(self.pos) {
            match token {
                Token::Dollar => {
                    result.push(self.parse_metavar_or_repetition()?);
                }
                Token::RParen | Token::RBracket => break,
                _ => {
                    result.push(PatternToken::Literal(token.clone()));
                    self.pos += 1;
                }
            }
        }

        Ok(result)
    }

    fn parse_metavar_or_repetition(&mut self) -> MacroResult<PatternToken> {
        self.pos += 1; // consume $

        match self.tokens.get(self.pos) {
            Some(Token::LParen) => {
                self.pos += 1;
                let inner = self.parse_tokens()?;
                self.expect_token(&Token::RParen)?;

                // Check for repetition suffix
                let (separator, mode) = self.parse_repetition_suffix()?;

                Ok(PatternToken::Repetition {
                    inner,
                    separator,
                    mode,
                })
            }
            Some(Token::Ident(name)) => {
                let name = name.clone();
                self.pos += 1;
                let kind = self.parse_metavar_kind()?;
                Ok(PatternToken::MetaVar { name, kind })
            }
            _ => Err(MacroError::InvalidInvocation {
                macro_name: "".to_string(),
                message: "expected identifier or pattern after '$'".to_string(),
            }),
        }
    }

    fn parse_metavar_kind(&mut self) -> MacroResult<MetaVarKind> {
        if let Some(Token::Colon) = self.tokens.get(self.pos) {
            self.pos += 1;
            if let Some(Token::Ident(kind)) = self.tokens.get(self.pos) {
                self.pos += 1;
                Ok(MetaVarKind::parse(kind).unwrap_or(MetaVarKind::Any))
            } else {
                Ok(MetaVarKind::default_for(""))
            }
        } else {
            Ok(MetaVarKind::Any)
        }
    }

    fn parse_repetition_suffix(&mut self) -> MacroResult<(Option<Separator>, RepetitionMode)> {
        let mut separator = None;
        let mut mode = RepetitionMode::ZeroOrMore;

        if let Some(token) = self.tokens.get(self.pos) {
            match token {
                Token::Comma => {
                    separator = Some(Separator::Comma);
                    self.pos += 1;
                }
                Token::SemiColon => {
                    separator = Some(Separator::SemiColon);
                    self.pos += 1;
                }
                _ => {}
            }
        }

        if let Some(token) = self.tokens.get(self.pos) {
            match token {
                Token::Star => {
                    mode = RepetitionMode::ZeroOrMore;
                    self.pos += 1;
                }
                Token::Plus => {
                    mode = RepetitionMode::OneOrMore;
                    self.pos += 1;
                }
                Token::Question => {
                    mode = RepetitionMode::ZeroOrOne;
                    self.pos += 1;
                }
                _ => {}
            }
        }

        Ok((separator, mode))
    }

    fn expect_token(&mut self, expected: &Token) -> MacroResult<()> {
        if let Some(actual) = self.tokens.get(self.pos) {
            if actual == expected {
                self.pos += 1;
                return Ok(());
            }
        }
        Err(MacroError::InvalidInvocation {
            macro_name: "".to_string(),
            message: format!("expected {:?}", expected),
        })
    }
}
