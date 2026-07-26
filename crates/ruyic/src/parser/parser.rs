use crate::lexer::token::{Location, Token, TokenWithLocation};
use crate::lexer::Scanner;
use crate::parser::ast::*;
use crate::parser::error::ParseError;

pub struct Parser {
    tokens: Vec<TokenWithLocation>,
    pos: usize,
    pending_extra_closes: u32,
}

impl Parser {
    pub fn new(source: &str) -> Result<Self, ParseError> {
        let mut scanner = Scanner::new(source);
        let tokens = scanner
            .scan_all()
            .map_err(|e| ParseError::LexerError(e.to_string()))?;
        Ok(Self {
            tokens,
            pos: 0,
            pending_extra_closes: 0,
        })
    }

    pub fn parse(&mut self) -> Result<Program, ParseError> {
        self.parse_program()
    }

    fn current(&self) -> Option<&TokenWithLocation> {
        self.tokens.get(self.pos)
    }

    fn current_token(&self) -> Option<&Token> {
        self.current().map(|t| &t.token)
    }

    #[allow(dead_code)]
    fn peek(&self, offset: usize) -> Option<&TokenWithLocation> {
        self.tokens.get(self.pos + offset)
    }

    #[allow(dead_code)]
    fn peek_token(&self, offset: usize) -> Option<&Token> {
        self.peek(offset).map(|t| &t.token)
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        matches!(self.current_token(), Some(Token::Eof) | None)
    }

    fn location(&self) -> Location {
        self.current()
            .map(|t| t.start)
            .unwrap_or(Location::new(0, 0))
    }

    fn check(&self, token: &Token) -> bool {
        self.current_token() == Some(token)
    }

    fn match_token(&mut self, token: &Token) -> bool {
        if self.check(token) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check_ident(&self, name: &str) -> bool {
        matches!(self.current_token(), Some(Token::Ident(n)) if n == name)
    }

    fn is_builtin_type(name: &str) -> bool {
        matches!(name, "string" | "int" | "float" | "bool" | "byte")
    }

    fn match_ident(&mut self, name: &str) -> bool {
        if self.check_ident(name) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect_keyword_ident(&mut self, name: &'static str) -> Result<(), ParseError> {
        if self.match_ident(name) {
            Ok(())
        } else {
            let loc = self.location();
            let found = self
                .current_token()
                .map(|t| t.name())
                .unwrap_or_else(|| "end of file".into());
            Err(ParseError::ExpectedToken {
                expected: format!("keyword '{name}'"),
                found,
                line: loc.line,
                col: loc.col,
            })
        }
    }

    fn expect(&mut self, token: Token) -> Result<(), ParseError> {
        if self.check(&token) {
            self.advance();
            Ok(())
        } else {
            let loc = self.location();
            let found = self
                .current_token()
                .map(|t| t.name())
                .unwrap_or_else(|| "end of file".into());
            Err(ParseError::ExpectedToken {
                expected: token.name(),
                found,
                line: loc.line,
                col: loc.col,
            })
        }
    }

    fn expect_ident(&mut self) -> Result<String, ParseError> {
        match self.current_token() {
            Some(Token::Ident(name)) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            _ => {
                let loc = self.location();
                let found = self
                    .current_token()
                    .map(|t| t.name())
                    .unwrap_or_else(|| "end of file".into());
                Err(ParseError::ExpectedToken {
                    expected: "identifier".into(),
                    found,
                    line: loc.line,
                    col: loc.col,
                })
            }
        }
    }

    /** Accept an identifier or a contextual keyword as a name (e.g. `fn get()`). */
    fn expect_ident_or_keyword(&mut self) -> Result<String, ParseError> {
        match self.current_token() {
            Some(Token::Ident(name)) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            Some(Token::Get) => {
                self.advance();
                Ok("get".to_string())
            }
            Some(Token::Set) => {
                self.advance();
                Ok("set".to_string())
            }
            Some(Token::New) => {
                self.advance();
                Ok("new".to_string())
            }
            Some(Token::Of) => {
                self.advance();
                Ok("of".to_string())
            }
            Some(Token::Delete) => {
                self.advance();
                Ok("delete".to_string())
            }
            Some(Token::Type) => {
                self.advance();
                Ok("type".to_string())
            }
            _ => {
                let loc = self.location();
                let found = self
                    .current_token()
                    .map(|t| t.name())
                    .unwrap_or_else(|| "end of file".into());
                Err(ParseError::ExpectedToken {
                    expected: "identifier".into(),
                    found,
                    line: loc.line,
                    col: loc.col,
                })
            }
        }
    }

    fn expect_string(&mut self) -> Result<String, ParseError> {
        match self.current_token() {
            Some(Token::String(s)) => {
                let s = s.clone();
                self.advance();
                Ok(s)
            }
            _ => {
                let loc = self.location();
                let found = self
                    .current_token()
                    .map(|t| t.name())
                    .unwrap_or_else(|| "end of file".into());
                Err(ParseError::ExpectedToken {
                    expected: "string literal".into(),
                    found,
                    line: loc.line,
                    col: loc.col,
                })
            }
        }
    }

    fn error(&self, message: impl Into<String>) -> ParseError {
        let loc = self.location();
        ParseError::SyntaxError {
            message: message.into(),
            line: loc.line,
            col: loc.col,
        }
    }

    fn unexpected_eof(&self) -> ParseError {
        let loc = self.location();
        ParseError::UnexpectedEof {
            line: loc.line,
            col: loc.col,
        }
    }

    // Program

    fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut items = Vec::new();
        while !self.is_at_end() {
            items.push(self.parse_module_item()?);
        }
        Ok(Program { items })
    }

    fn parse_annotations(&mut self) -> Vec<String> {
        let mut annotations = Vec::new();
        while self.match_token(&Token::At) {
            if let Some(Token::Ident(name)) = self.current_token() {
                annotations.push(name.clone());
                self.advance();
            }
        }
        annotations
    }

    fn parse_module_item(&mut self) -> Result<ModuleItem, ParseError> {
        match self.current_token() {
            Some(Token::Import) => self.parse_import().map(ModuleItem::Import),
            Some(Token::Export) => self.parse_export().map(ModuleItem::Export),
            Some(Token::Let) | Some(Token::Const) | Some(Token::Fn) | Some(Token::Class)
            | Some(Token::Trait) | Some(Token::Impl) | Some(Token::Type) | Some(Token::Macro)
            | Some(Token::At) | Some(Token::Async) | Some(Token::Extern) => {
                self.parse_declaration().map(ModuleItem::Declaration)
            }
            _ => self.parse_statement().map(ModuleItem::Statement),
        }
    }

    // Import / Export

    fn parse_import(&mut self) -> Result<ImportDecl, ParseError> {
        self.expect(Token::Import)?;
        let mut default = None;
        let mut namespace = None;
        let mut named = Vec::new();
        let source;

        if matches!(self.current_token(), Some(Token::String(_))) {
            // import "...";
            source = self.expect_string()?;
            self.expect(Token::SemiColon)?;
            return Ok(ImportDecl {
                default,
                namespace,
                named,
                source,
            });
        }

        if let Some(Token::Ident(_)) = self.current_token() {
            default = Some(self.expect_ident()?);
            if self.match_token(&Token::Comma) {
                // import X, { ... } from "..."
            } else {
                self.expect_keyword_ident("from")?;
                source = self.expect_string()?;
                self.expect(Token::SemiColon)?;
                return Ok(ImportDecl {
                    default,
                    namespace,
                    named,
                    source,
                });
            }
        }

        if self.match_token(&Token::Star) {
            self.expect(Token::As)?;
            namespace = Some(self.expect_ident()?);
            self.expect_keyword_ident("from")?;
            source = self.expect_string()?;
            self.expect(Token::SemiColon)?;
            return Ok(ImportDecl {
                default,
                namespace,
                named,
                source,
            });
        }

        self.expect(Token::LBrace)?;
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            let name = self.expect_ident()?;
            let alias = if self.match_token(&Token::As) {
                Some(self.expect_ident()?)
            } else {
                None
            };
            named.push(NamedImport { name, alias });
            if !self.match_token(&Token::Comma) {
                break;
            }
        }
        self.expect(Token::RBrace)?;
        self.expect_keyword_ident("from")?;
        source = self.expect_string()?;
        self.expect(Token::SemiColon)?;
        Ok(ImportDecl {
            default,
            namespace,
            named,
            source,
        })
    }

    fn parse_export(&mut self) -> Result<ExportDecl, ParseError> {
        self.expect(Token::Export)?;

        if self.match_ident("default") {
            if self.check(&Token::Fn) {
                let decl = self.parse_fn_declaration()?;
                if let Declaration::Function {
                    name,
                    type_params,
                    params,
                    return_type,
                    body,
                    is_async,
                    ..
                } = decl
                {
                    return Ok(ExportDecl::DefaultFunction {
                        name,
                        type_params,
                        params,
                        return_type,
                        body,
                        is_async,
                    });
                }
                unreachable!()
            }
            if self.check(&Token::Class) || self.check(&Token::At) {
                let decl = self.parse_class_declaration()?;
                if let Declaration::Class {
                    name,
                    type_params,
                    extends,
                    body,
                    annotations,
                } = decl
                {
                    return Ok(ExportDecl::DefaultClass {
                        name,
                        type_params,
                        extends,
                        body,
                        annotations,
                    });
                }
                unreachable!()
            }
            let expr = self.parse_expression()?;
            self.expect(Token::SemiColon)?;
            return Ok(ExportDecl::DefaultExpr(Box::new(expr)));
        }

        if self.match_token(&Token::Star) {
            self.expect_keyword_ident("from")?;
            let source = self.expect_string()?;
            self.expect(Token::SemiColon)?;
            return Ok(ExportDecl::ReExportAll { source });
        }

        if self.check(&Token::LBrace) {
            self.advance();
            let mut items = Vec::new();
            while !self.check(&Token::RBrace) && !self.is_at_end() {
                let name = self.expect_ident()?;
                let alias = if self.match_token(&Token::As) {
                    Some(self.expect_ident()?)
                } else {
                    None
                };
                items.push(NamedExport { name, alias });
                if !self.match_token(&Token::Comma) {
                    break;
                }
            }
            self.expect(Token::RBrace)?;
            if self.match_ident("from") {
                let source = self.expect_string()?;
                self.expect(Token::SemiColon)?;
                return Ok(ExportDecl::ReExportNamed { items, source });
            }
            self.expect(Token::SemiColon)?;
            return Ok(ExportDecl::Named(items));
        }

        let decl = self.parse_declaration()?;
        Ok(ExportDecl::Declaration(decl))
    }

    // Declarations

    fn parse_declaration(&mut self) -> Result<Declaration, ParseError> {
        match self.current_token() {
            Some(Token::Let) => self.parse_let_declaration(),
            Some(Token::Const) => self.parse_const_declaration(),
            Some(Token::Fn) | Some(Token::Async) => self.parse_fn_declaration(),
            Some(Token::At) => self.parse_annotated_declaration(),
            Some(Token::Class) => self.parse_class_declaration(),
            Some(Token::Trait) => self.parse_trait_declaration(),
            Some(Token::Impl) => self.parse_impl_declaration(),
            Some(Token::Type) => self.parse_type_alias(),
            Some(Token::Macro) => self.parse_macro_declaration(),
            Some(Token::Extern) => self.parse_extern_declaration(),
            _ => Err(self.error("expected declaration")),
        }
    }

    fn parse_extern_declaration(&mut self) -> Result<Declaration, ParseError> {
        self.expect(Token::Extern)?;
        self.expect(Token::Fn)?;
        let name = self.expect_ident()?;
        self.expect(Token::LParen)?;
        let params = self.parse_formal_params()?;
        self.expect(Token::RParen)?;
        let return_type = if self.check(&Token::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        self.expect(Token::SemiColon)?;
        Ok(Declaration::ExternFn {
            name,
            params,
            return_type,
        })
    }

    /// Dispatch an annotation-prefixed declaration by lookahead-scanning
    /// through `@name` pairs to decide whether to parse a function or a class.
    /// Also handles `@ann export class/fn` by skipping the `export` keyword.
    fn parse_annotated_declaration(&mut self) -> Result<Declaration, ParseError> {
        let mut scan = self.pos;
        while matches!(self.tokens.get(scan).map(|t| &t.token), Some(Token::At)) {
            scan += 1;
            if matches!(
                self.tokens.get(scan).map(|t| &t.token),
                Some(Token::Ident(_))
            ) {
                scan += 1;
            } else {
                break;
            }
        }
        // Skip `export` keyword if present (e.g., `@arc export class Buffer`)
        if matches!(self.tokens.get(scan).map(|t| &t.token), Some(Token::Export)) {
            scan += 1;
        }
        match self.tokens.get(scan).map(|t| &t.token) {
            Some(Token::Class) => self.parse_class_declaration(),
            _ => self.parse_fn_declaration(),
        }
    }

    fn parse_let_declaration(&mut self) -> Result<Declaration, ParseError> {
        self.expect(Token::Let)?;
        let bindings = self.parse_binding_list()?;
        self.expect(Token::SemiColon)?;
        Ok(Declaration::Let(bindings))
    }

    fn parse_const_declaration(&mut self) -> Result<Declaration, ParseError> {
        self.expect(Token::Const)?;
        let bindings = self.parse_binding_list()?;
        self.expect(Token::SemiColon)?;
        Ok(Declaration::Const(bindings))
    }

    fn parse_binding_list(&mut self) -> Result<Vec<Binding>, ParseError> {
        // Ruyi only supports a single binding per `let`/`const` declaration.
        // Multiple comma-separated bindings (`let x = 1, y = 2;`) are a
        // syntax error — the caller's `expect(;)` will reject the comma.
        let pattern = self.parse_pattern()?;
        let ty = if self.check(&Token::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        let init = if self.match_token(&Token::Assign) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };
        Ok(vec![Binding { pattern, init, ty }])
    }

    fn parse_fn_declaration(&mut self) -> Result<Declaration, ParseError> {
        let annotations = self.parse_annotations();
        // Skip `export` keyword if present (e.g., `@inline export fn foo()`)
        self.match_token(&Token::Export);
        let is_async = self.match_token(&Token::Async);
        self.expect(Token::Fn)?;
        // Skip generator marker `*` if present: `fn* name()`
        self.match_token(&Token::Star);
        let name = self.expect_ident_or_keyword()?;
        let type_params = self.parse_type_params()?;
        self.expect(Token::LParen)?;
        let params = self.parse_formal_params()?;
        self.expect(Token::RParen)?;
        let return_type = if self.check(&Token::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        self.expect(Token::LBrace)?;
        let body = self.parse_function_body()?;
        self.expect(Token::RBrace)?;
        Ok(Declaration::Function {
            name,
            type_params,
            params,
            return_type,
            body,
            is_async,
            annotations,
        })
    }

    fn parse_class_declaration(&mut self) -> Result<Declaration, ParseError> {
        let annotations = self.parse_annotations();
        // Skip `export` keyword if present (e.g., `@arc export class Buffer`)
        self.match_token(&Token::Export);
        self.expect(Token::Class)?;
        let name = self.expect_ident()?;
        let type_params = self.parse_type_params()?;
        let extends = if self.match_token(&Token::Extends) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };
        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            body.push(self.parse_class_element()?);
        }
        self.expect(Token::RBrace)?;
        Ok(Declaration::Class {
            name,
            type_params,
            extends,
            body,
            annotations,
        })
    }

    fn parse_trait_declaration(&mut self) -> Result<Declaration, ParseError> {
        self.expect(Token::Trait)?;
        let name = self.expect_ident()?;
        let type_params = self.parse_type_params()?;
        let supertraits = if self.match_token(&Token::Extends) {
            let mut traits = vec![self.expect_ident()?];
            while self.match_token(&Token::Comma) {
                traits.push(self.expect_ident()?);
            }
            traits
        } else {
            Vec::new()
        };
        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            body.push(self.parse_trait_element()?);
        }
        self.expect(Token::RBrace)?;
        Ok(Declaration::Trait {
            name,
            type_params,
            supertraits,
            body,
        })
    }

    fn parse_impl_declaration(&mut self) -> Result<Declaration, ParseError> {
        self.expect(Token::Impl)?;
        let type_params = self.parse_type_params()?;
        let trait_name = self.expect_ident()?;
        let trait_args = if self.check(&Token::Less) {
            self.parse_type_args()?
        } else {
            Vec::new()
        };
        self.expect(Token::For)?;
        let for_type = self.parse_type()?;
        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            body.push(self.parse_class_element()?);
        }
        self.expect(Token::RBrace)?;
        Ok(Declaration::Impl {
            type_params,
            trait_name,
            trait_args,
            for_type,
            body,
        })
    }

    fn parse_type_alias(&mut self) -> Result<Declaration, ParseError> {
        self.expect(Token::Type)?;
        let name = self.expect_ident()?;
        let type_params = self.parse_type_params()?;
        self.expect(Token::Assign)?;
        let ty = self.parse_type()?;
        self.expect(Token::SemiColon)?;
        Ok(Declaration::TypeAlias {
            name,
            type_params,
            ty,
        })
    }

    fn parse_macro_declaration(&mut self) -> Result<Declaration, ParseError> {
        self.expect(Token::Macro)?;
        let name = self.expect_ident()?;
        self.expect(Token::LBrace)?;
        let mut rules = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            self.expect(Token::LParen)?;
            let pattern = self.parse_macro_tokens_until(&Token::RParen)?;
            self.expect(Token::RParen)?;
            self.expect(Token::FatArrow)?;
            self.expect(Token::LBrace)?;
            let body = self.parse_macro_tokens_until(&Token::RBrace)?;
            self.expect(Token::RBrace)?;
            rules.push(MacroRule { pattern, body });
        }
        self.expect(Token::RBrace)?;
        Ok(Declaration::Macro { name, rules })
    }

    fn parse_macro_tokens_until(&mut self, end: &Token) -> Result<Vec<Token>, ParseError> {
        let mut tokens = Vec::new();
        let mut depth = 0;
        while !self.is_at_end() {
            if depth == 0 && self.check(end) {
                break;
            }
            let token = self.current_token().unwrap().clone();
            match &token {
                Token::LParen | Token::LBracket | Token::LBrace => depth += 1,
                Token::RParen | Token::RBracket | Token::RBrace => {
                    if depth > 0 {
                        depth -= 1;
                    } else {
                        break;
                    }
                }
                _ => {}
            }
            tokens.push(token);
            self.advance();
        }
        Ok(tokens)
    }

    // Class / Trait elements

    fn parse_class_element(&mut self) -> Result<ClassElement, ParseError> {
        if self.match_token(&Token::SemiColon) {
            return Ok(ClassElement::Empty);
        }
        let is_static = self.match_token(&Token::Static);

        if self.match_token(&Token::Get) {
            let name = self.parse_property_name()?;
            self.expect(Token::LParen)?;
            self.expect(Token::RParen)?;
            let return_type = if self.check(&Token::Colon) {
                Some(self.parse_type_annotation()?)
            } else {
                None
            };
            self.expect(Token::LBrace)?;
            let body = self.parse_function_body()?;
            self.expect(Token::RBrace)?;
            return Ok(ClassElement::Method {
                name,
                type_params: vec![],
                params: vec![],
                return_type,
                body,
                is_async: false,
                is_static,
                is_getter: true,
                is_setter: false,
            });
        }

        if self.match_token(&Token::Set) {
            let name = self.parse_property_name()?;
            self.expect(Token::LParen)?;
            let params = vec![self.parse_formal_param()?];
            self.expect(Token::RParen)?;
            self.expect(Token::LBrace)?;
            let body = self.parse_function_body()?;
            self.expect(Token::RBrace)?;
            return Ok(ClassElement::Method {
                name,
                type_params: vec![],
                params,
                return_type: None,
                body,
                is_async: false,
                is_static,
                is_getter: false,
                is_setter: true,
            });
        }

        let is_async = self.match_token(&Token::Async);
        if self.check(&Token::Fn) {
            self.advance();
            let name = self.parse_property_name()?;
            let type_params = self.parse_type_params()?;
            self.expect(Token::LParen)?;
            let params = self.parse_formal_params()?;
            self.expect(Token::RParen)?;
            let return_type = if self.check(&Token::Colon) {
                Some(self.parse_type_annotation()?)
            } else {
                None
            };
            self.expect(Token::LBrace)?;
            let body = self.parse_function_body()?;
            self.expect(Token::RBrace)?;
            return Ok(ClassElement::Method {
                name,
                type_params,
                params,
                return_type,
                body,
                is_async,
                is_static,
                is_getter: false,
                is_setter: false,
            });
        }

        // Field
        let name = self.parse_property_name()?;
        let ty = if self.check(&Token::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        let init = if self.match_token(&Token::Assign) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };
        self.expect(Token::SemiColon)?;
        Ok(ClassElement::Field {
            name,
            ty,
            init,
            is_static,
        })
    }

    fn parse_trait_element(&mut self) -> Result<TraitElement, ParseError> {
        if self.match_token(&Token::SemiColon) {
            return Ok(TraitElement::Empty);
        }
        let is_static = self.match_token(&Token::Static);
        if self.check(&Token::Fn) {
            self.advance();
            let name = self.parse_property_name()?;
            let type_params = self.parse_type_params()?;
            self.expect(Token::LParen)?;
            let params = self.parse_formal_params()?;
            self.expect(Token::RParen)?;
            let return_type = if self.check(&Token::Colon) {
                Some(self.parse_type_annotation()?)
            } else {
                None
            };
            let body = if self.check(&Token::LBrace) {
                self.advance();
                let stmts = self.parse_function_body()?;
                self.expect(Token::RBrace)?;
                Some(stmts)
            } else {
                self.expect(Token::SemiColon)?;
                None
            };
            return Ok(TraitElement::Method {
                name,
                type_params,
                params,
                return_type,
                body,
                is_static,
            });
        }
        let name = self.parse_property_name()?;
        self.expect(Token::Colon)?;
        let ty = self.parse_type_annotation()?;
        self.expect(Token::SemiColon)?;
        Ok(TraitElement::Field { name, ty })
    }

    fn parse_property_name(&mut self) -> Result<PropertyName, ParseError> {
        match self.current_token() {
            Some(Token::Ident(name)) => {
                let n = name.clone();
                self.advance();
                Ok(PropertyName::Ident(n))
            }
            Some(Token::New) => {
                self.advance();
                Ok(PropertyName::Ident("new".to_string()))
            }
            Some(Token::SelfKw) => {
                self.advance();
                Ok(PropertyName::Ident("self".to_string()))
            }
            Some(Token::Get) => {
                self.advance();
                Ok(PropertyName::Ident("get".to_string()))
            }
            Some(Token::Set) => {
                self.advance();
                Ok(PropertyName::Ident("set".to_string()))
            }
            Some(Token::Delete) => {
                self.advance();
                Ok(PropertyName::Ident("delete".to_string()))
            }
            Some(Token::Type) => {
                self.advance();
                Ok(PropertyName::Ident("type".to_string()))
            }
            Some(Token::Of) => {
                self.advance();
                Ok(PropertyName::Ident("of".to_string()))
            }
            Some(Token::String(s)) => {
                let s = s.clone();
                self.advance();
                Ok(PropertyName::String(s))
            }
            Some(Token::Int(i)) => {
                let i = *i;
                self.advance();
                Ok(PropertyName::Number(i as f64))
            }
            Some(Token::Float(f)) => {
                let f = *f;
                self.advance();
                Ok(PropertyName::Number(f))
            }
            Some(Token::LBracket) => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(Token::RBracket)?;
                Ok(PropertyName::Computed(Box::new(expr)))
            }
            _ => Err(self.error("expected property name")),
        }
    }

    // Statements

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        match self.current_token() {
            Some(Token::LBrace) => self.parse_block_statement(),
            Some(Token::If) => self.parse_if_statement(),
            Some(Token::While) => self.parse_while_statement(),
            Some(Token::For) => self.parse_for_statement(),
            Some(Token::Return) => self.parse_return_statement(),
            Some(Token::Throw) => self.parse_throw_statement(),
            Some(Token::Yield) => self.parse_yield_statement(),
            Some(Token::Try) => self.parse_try_statement(),
            Some(Token::Match) => self.parse_match_statement(),
            Some(Token::SemiColon) => {
                self.advance();
                Ok(Statement::Empty)
            }
            Some(Token::Let) | Some(Token::Const) | Some(Token::Class) | Some(Token::Fn)
            | Some(Token::Async) => {
                let decl = self.parse_declaration()?;
                Ok(Statement::Declaration(decl))
            }
            Some(Token::Ident(_)) => {
                let name = match self.current_token() {
                    Some(Token::Ident(n)) => n.clone(),
                    _ => unreachable!(),
                };
                self.advance();
                if self.check(&Token::Colon) {
                    self.advance();
                    let body = self.parse_statement()?;
                    Ok(Statement::Labeled {
                        label: name,
                        body: Box::new(body),
                    })
                } else if name == "loop" && self.check(&Token::LBrace) {
                    // `loop { ... }` → `while (true) { ... }`
                    let body = Box::new(self.parse_block_statement()?);
                    Ok(Statement::While {
                        condition: Box::new(Expr::BooleanLiteral(true)),
                        body,
                    })
                } else {
                    self.pos -= 1;
                    self.parse_expression_statement()
                }
            }
            _ => {
                if self.check(&Token::Break) {
                    self.parse_break_statement()
                } else if self.check(&Token::Continue) {
                    self.parse_continue_statement()
                } else {
                    self.parse_expression_statement()
                }
            }
        }
    }

    fn parse_block_statement(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::LBrace)?;
        let mut stmts = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            stmts.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;
        Ok(Statement::Block(stmts))
    }

    fn parse_expression_statement(&mut self) -> Result<Statement, ParseError> {
        let expr = self.parse_expression()?;
        // Semicolon is optional if followed by '}' (e.g., in if-expression blocks)
        if !self.check(&Token::RBrace) {
            self.expect(Token::SemiColon)?;
        }
        Ok(Statement::Expression(Box::new(expr)))
    }

    fn parse_if_statement(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::If)?;
        if self.match_token(&Token::Let) {
            let pattern = self.parse_pattern()?;
            self.expect(Token::Assign)?;
            let value = self.parse_expression()?;
            let then_branch = Box::new(self.parse_block_statement()?);
            let else_branch = if self.match_token(&Token::Else) {
                Some(Box::new(self.parse_statement()?))
            } else {
                None
            };
            return Ok(Statement::IfLet {
                pattern,
                value: Box::new(value),
                then_branch,
                else_branch,
            });
        }
        self.expect(Token::LParen)?;
        let condition = self.parse_expression()?;
        self.expect(Token::RParen)?;
        let then_branch = Box::new(self.parse_statement()?);
        let else_branch = if self.match_token(&Token::Else) {
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };
        Ok(Statement::If {
            condition: Box::new(condition),
            then_branch,
            else_branch,
        })
    }

    fn parse_while_statement(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::While)?;
        if self.match_token(&Token::Let) {
            let pattern = self.parse_pattern()?;
            self.expect(Token::Assign)?;
            let value = self.parse_expression()?;
            let body = Box::new(self.parse_block_statement()?);
            return Ok(Statement::WhileLet {
                pattern,
                value: Box::new(value),
                body,
            });
        }
        self.expect(Token::LParen)?;
        let condition = self.parse_expression()?;
        self.expect(Token::RParen)?;
        let body = Box::new(self.parse_statement()?);
        Ok(Statement::While {
            condition: Box::new(condition),
            body,
        })
    }

    fn parse_for_statement(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::For)?;
        let is_for_await = self.match_token(&Token::Await);
        self.expect(Token::LParen)?;

        if self.check(&Token::Let) {
            let saved = self.pos;
            self.advance();
            if let Some(Token::Ident(_)) = self.current_token() {
                self.advance();
                if self.check(&Token::In) || self.check(&Token::Of) {
                    // for-in or for-of
                    self.pos = saved;
                    self.advance();
                    let variable = self.expect_ident()?;
                    if self.match_token(&Token::In) {
                        let iterable = self.parse_expression()?;
                        self.expect(Token::RParen)?;
                        let body = Box::new(self.parse_statement()?);
                        return Ok(Statement::ForIn {
                            variable,
                            iterable: Box::new(iterable),
                            body,
                        });
                    }
                    if self.match_token(&Token::Of) {
                        let is_async = is_for_await || self.match_token(&Token::Async);
                        let iterable = self.parse_expression()?;
                        self.expect(Token::RParen)?;
                        let body = Box::new(self.parse_statement()?);
                        return Ok(Statement::ForOf {
                            variable,
                            iterable: Box::new(iterable),
                            body,
                            is_async,
                        });
                    }
                }
            }
            self.pos = saved;
        }

        let init = if self.check(&Token::SemiColon) {
            self.advance();
            None
        } else if self.check(&Token::Let) || self.check(&Token::Const) {
            Some(ForInit::VarDecl(self.parse_declaration()?))
        } else {
            let expr = self.parse_expression()?;
            self.expect(Token::SemiColon)?;
            Some(ForInit::Expr(Box::new(expr)))
        };
        let condition = if self.check(&Token::SemiColon) {
            None
        } else {
            Some(Box::new(self.parse_expression()?))
        };
        self.expect(Token::SemiColon)?;
        let update = if self.check(&Token::RParen) {
            None
        } else {
            Some(Box::new(self.parse_expression()?))
        };
        self.expect(Token::RParen)?;
        let body = Box::new(self.parse_statement()?);
        Ok(Statement::For {
            init,
            condition,
            update,
            body,
        })
    }

    fn parse_return_statement(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::Return)?;
        let value =
            if self.check(&Token::SemiColon) || self.check(&Token::RBrace) || self.is_at_end() {
                None
            } else {
                Some(Box::new(self.parse_expression()?))
            };
        self.expect(Token::SemiColon)?;
        Ok(Statement::Return(value))
    }

    fn parse_yield_statement(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::Yield)?;
        let value =
            if self.check(&Token::SemiColon) || self.check(&Token::RBrace) || self.is_at_end() {
                None
            } else {
                Some(Box::new(self.parse_expression()?))
            };
        self.expect(Token::SemiColon)?;
        Ok(Statement::Yield(value))
    }

    fn parse_throw_statement(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::Throw)?;
        let value = self.parse_expression()?;
        self.expect(Token::SemiColon)?;
        Ok(Statement::Throw(Box::new(value)))
    }

    fn parse_try_statement(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::Try)?;
        self.expect(Token::LBrace)?;
        let body = self.parse_function_body()?;
        self.expect(Token::RBrace)?;

        let mut catches = Vec::new();
        while self.match_token(&Token::Catch) {
            let pattern = if self.match_token(&Token::LParen) {
                let pat = self.parse_pattern()?;
                let ty = if self.check(&Token::Colon) {
                    Some(self.parse_type_annotation()?)
                } else {
                    None
                };
                self.expect(Token::RParen)?;
                Some((pat, ty))
            } else {
                None
            };
            self.expect(Token::LBrace)?;
            let catch_body = self.parse_function_body()?;
            self.expect(Token::RBrace)?;
            catches.push(CatchClause {
                pattern: pattern.as_ref().map(|(p, _)| p.clone()),
                ty: pattern.and_then(|(_, t)| t),
                body: catch_body,
            });
        }

        let finally = if self.match_token(&Token::Finally) {
            self.expect(Token::LBrace)?;
            let fbody = self.parse_function_body()?;
            self.expect(Token::RBrace)?;
            Some(fbody)
        } else {
            None
        };

        Ok(Statement::Try {
            body,
            catch: catches,
            finally,
        })
    }

    fn parse_match_statement(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::Match)?;
        self.expect(Token::LParen)?;
        let value = self.parse_expression()?;
        self.expect(Token::RParen)?;
        self.expect(Token::LBrace)?;
        let mut arms = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            arms.push(self.parse_match_arm()?);
            if !self.match_token(&Token::Comma) {
                // optional comma
            }
        }
        self.expect(Token::RBrace)?;
        Ok(Statement::Match {
            value: Box::new(value),
            arms,
        })
    }

    fn parse_match_arm(&mut self) -> Result<MatchArm, ParseError> {
        let pattern = self.parse_pattern()?;
        let guard = if self.match_token(&Token::If) {
            self.expect(Token::LParen)?;
            let expr = self.parse_expression()?;
            self.expect(Token::RParen)?;
            Some(Box::new(expr))
        } else {
            None
        };
        self.expect(Token::FatArrow)?;
        let body = if self.check(&Token::LBrace) {
            self.advance();
            let mut stmts = Vec::new();
            while !self.check(&Token::RBrace) && !self.is_at_end() {
                match self.current_token() {
                    // Tokens that start keyword-based statements (delegate to parse_statement)
                    Some(Token::LBrace)
                    | Some(Token::If)
                    | Some(Token::While)
                    | Some(Token::For)
                    | Some(Token::Return)
                    | Some(Token::Throw)
                    | Some(Token::Try)
                    | Some(Token::Match)
                    | Some(Token::SemiColon)
                    | Some(Token::Let)
                    | Some(Token::Const)
                    | Some(Token::Break)
                    | Some(Token::Continue) => {
                        stmts.push(self.parse_statement()?);
                    }
                    // Everything else is an expression; semicolon is optional
                    // so that the last expression in a match arm block can omit it
                    _ => {
                        let expr = self.parse_expression()?;
                        if self.check(&Token::SemiColon) {
                            self.advance();
                        }
                        stmts.push(Statement::Expression(Box::new(expr)));
                    }
                }
            }
            self.expect(Token::RBrace)?;
            stmts
        } else {
            vec![Statement::Expression(Box::new(self.parse_expression()?))]
        };
        Ok(MatchArm {
            pattern,
            guard,
            body,
        })
    }

    fn parse_break_statement(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::Break)?;
        let label = if let Some(Token::Ident(name)) = self.current_token() {
            let name = name.clone();
            self.advance();
            Some(name)
        } else {
            None
        };
        self.expect(Token::SemiColon)?;
        Ok(Statement::Break(label))
    }

    fn parse_continue_statement(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::Continue)?;
        let label = if let Some(Token::Ident(name)) = self.current_token() {
            let name = name.clone();
            self.advance();
            Some(name)
        } else {
            None
        };
        self.expect(Token::SemiColon)?;
        Ok(Statement::Continue(label))
    }

    fn parse_function_body(&mut self) -> Result<Vec<Statement>, ParseError> {
        let mut stmts = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            stmts.push(self.parse_statement()?);
        }
        Ok(stmts)
    }

    // Expressions

    pub fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_expr_bp(0)
    }

    fn parse_expr_bp(&mut self, min_bp: u8) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_prefix()?;

        loop {
            if self.check(&Token::Not) {
                self.advance();
                lhs = Expr::NullAssert(Box::new(lhs));
                continue;
            }

            let op = match self.current_token() {
                Some(t) => t.clone(),
                None => break,
            };

            let (l_bp, r_bp) = match infix_binding_power(&op) {
                Some(bp) => bp,
                None => break,
            };

            if l_bp < min_bp {
                break;
            }

            self.advance();

            // Ternary conditional
            if op == Token::Question {
                let then_branch = self.parse_expr_bp(r_bp)?;
                self.expect(Token::Colon)?;
                let else_branch = self.parse_expr_bp(r_bp)?;
                lhs = Expr::Conditional {
                    condition: Box::new(lhs),
                    then_branch: Box::new(then_branch),
                    else_branch: Box::new(else_branch),
                };
                continue;
            }

            // Assignment
            if let Some(assign_op) = token_to_assign_op(&op) {
                let rhs = self.parse_expr_bp(r_bp)?;
                lhs = Expr::Assignment {
                    left: Box::new(lhs),
                    op: assign_op,
                    right: Box::new(rhs),
                };
                continue;
            }

            // Arrow function (when we see => after a single identifier or parenthesized params)
            if op == Token::FatArrow {
                // Convert lhs to params
                let params = expr_to_arrow_params(lhs)?;
                let return_type = if self.check(&Token::Colon) {
                    Some(self.parse_type_annotation()?)
                } else {
                    None
                };
                let body = if self.check(&Token::LBrace) {
                    self.advance();
                    let stmts = self.parse_function_body()?;
                    self.expect(Token::RBrace)?;
                    ArrowBody::Block(stmts)
                } else {
                    ArrowBody::Expr(Box::new(self.parse_expr_bp(r_bp)?))
                };
                lhs = Expr::ArrowFunction {
                    params,
                    return_type,
                    body,
                    is_async: false,
                };
                continue;
            }

            // Member access, call, index
            if op == Token::Dot || op == Token::OptChain {
                let property = match self.current_token() {
                    Some(Token::Ident(name)) => {
                        let n = name.clone();
                        self.advance();
                        MemberProperty::Ident(n)
                    }
                    Some(Token::Int(i)) => {
                        let n = i.to_string();
                        self.advance();
                        MemberProperty::Ident(n)
                    }
                    Some(Token::New) => {
                        self.advance();
                        MemberProperty::Ident("new".to_string())
                    }
                    Some(Token::SelfKw) => {
                        self.advance();
                        MemberProperty::Ident("self".to_string())
                    }
                    Some(Token::Underscore) => {
                        self.advance();
                        // Check if followed by identifier (e.g., _map)
                        if let Some(Token::Ident(name)) = self.current_token() {
                            let n = format!("_{}", name);
                            self.advance();
                            MemberProperty::Ident(n)
                        } else {
                            MemberProperty::Ident("_".to_string())
                        }
                    }
                    Some(Token::Get) => {
                        self.advance();
                        MemberProperty::Ident("get".to_string())
                    }
                    Some(Token::Set) => {
                        self.advance();
                        MemberProperty::Ident("set".to_string())
                    }
                    Some(Token::Delete) => {
                        self.advance();
                        MemberProperty::Ident("delete".to_string())
                    }
                    Some(Token::Type) => {
                        self.advance();
                        MemberProperty::Ident("type".to_string())
                    }
                    Some(Token::Of) => {
                        self.advance();
                        MemberProperty::Ident("of".to_string())
                    }
                    _ => return Err(self.error("expected identifier after '.'")),
                };
                lhs = Expr::Member {
                    object: Box::new(lhs),
                    property,
                    optional: op == Token::OptChain,
                };
                continue;
            }

            if op == Token::LParen {
                let mut args = Vec::new();
                while !self.check(&Token::RParen) && !self.is_at_end() {
                    if self.match_token(&Token::Spread) {
                        let expr = self.parse_expression()?;
                        args.push(Argument::Spread(Box::new(expr)));
                    } else {
                        let expr = self.parse_expression()?;
                        args.push(Argument::Expr(Box::new(expr)));
                    }
                    if !self.match_token(&Token::Comma) {
                        break;
                    }
                }
                self.expect(Token::RParen)?;
                lhs = Expr::Call {
                    callee: Box::new(lhs),
                    args,
                };
                continue;
            }

            if op == Token::LBracket {
                let index = self.parse_expression()?;
                self.expect(Token::RBracket)?;
                lhs = Expr::Member {
                    object: Box::new(lhs),
                    property: MemberProperty::Expr(Box::new(index)),
                    optional: false,
                };
                continue;
            }

            // Optional call: ?.(
            if op == Token::OptChain && self.check(&Token::LParen) {
                let args = self.parse_arguments()?;
                lhs = Expr::OptionalCall {
                    callee: Box::new(lhs),
                    args,
                };
                continue;
            }

            // Optional index: ?.[
            if op == Token::OptChain && self.check(&Token::LBracket) {
                self.advance();
                let index = self.parse_expression()?;
                self.expect(Token::RBracket)?;
                lhs = Expr::Member {
                    object: Box::new(lhs),
                    property: MemberProperty::Expr(Box::new(index)),
                    optional: true,
                };
                continue;
            }

            // Binary operators
            if let Some(bin_op) = token_to_binary_op(&op) {
                let rhs = self.parse_expr_bp(r_bp)?;
                lhs = Expr::Binary {
                    op: bin_op,
                    left: Box::new(lhs),
                    right: Box::new(rhs),
                };
                continue;
            }

            break;
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<Expr, ParseError> {
        let op = match self.current_token() {
            Some(t) => t.clone(),
            None => return Err(self.unexpected_eof()),
        };

        if let Some(unary_op) = token_to_prefix_op(&op) {
            self.advance();
            let operand = self.parse_expr_bp(170)?;
            return Ok(Expr::Unary {
                op: unary_op,
                operand: Box::new(operand),
            });
        }

        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match self.current_token() {
            Some(Token::Ident(name)) => {
                let n = name.clone();
                self.advance();
                Ok(Expr::Identifier(n))
            }
            Some(Token::Int(i)) => {
                let i = *i;
                self.advance();
                Ok(Expr::IntLiteral(i))
            }
            Some(Token::Float(f)) => {
                let f = *f;
                self.advance();
                Ok(Expr::FloatLiteral(f))
            }
            Some(Token::String(s)) => {
                let s = s.clone();
                self.advance();
                Ok(Expr::StringLiteral(s))
            }
            Some(Token::BigInt(s)) => {
                let s = s.clone();
                self.advance();
                Ok(Expr::BigIntLiteral(s))
            }
            Some(Token::True) => {
                self.advance();
                Ok(Expr::BooleanLiteral(true))
            }
            Some(Token::False) => {
                self.advance();
                Ok(Expr::BooleanLiteral(false))
            }
            Some(Token::Null) => {
                self.advance();
                Ok(Expr::NullLiteral)
            }
            Some(Token::This) => {
                self.advance();
                Ok(Expr::This)
            }
            Some(Token::Super) => {
                self.advance();
                Ok(Expr::Super)
            }
            Some(Token::SelfKw) => {
                self.advance();
                Ok(Expr::SelfExpr)
            }
            Some(Token::TemplateString(s)) => {
                let s = s.clone();
                self.advance();
                if self.check(&Token::TemplateExprStart) {
                    self.parse_template_literal(s)
                } else {
                    Ok(Expr::StringLiteral(s))
                }
            }
            Some(Token::LBracket) => self.parse_array_literal(),
            Some(Token::LBrace) => self.parse_object_literal(),
            Some(Token::LParen) => self.parse_grouping_or_arrow(),
            Some(Token::Fn) => self.parse_function_expression(),
            Some(Token::Class) => self.parse_class_expression(),
            Some(Token::New) => self.parse_new_expression(),
            Some(Token::Match) => self.parse_match_expression(),
            Some(Token::If) => self.parse_if_expression(),
            Some(Token::Async) => {
                self.advance();
                if self.check(&Token::Fn) {
                    self.parse_async_function_expression()
                } else if self.check(&Token::LParen) {
                    // async arrow: async (params) => body
                    let params_expr = self.parse_primary()?;
                    if self.match_token(&Token::FatArrow) {
                        let params = expr_to_arrow_params(params_expr)?;
                        let return_type = if self.check(&Token::Colon) {
                            Some(self.parse_type_annotation()?)
                        } else {
                            None
                        };
                        let body = if self.check(&Token::LBrace) {
                            self.advance();
                            let stmts = self.parse_function_body()?;
                            self.expect(Token::RBrace)?;
                            ArrowBody::Block(stmts)
                        } else {
                            ArrowBody::Expr(Box::new(self.parse_expression()?))
                        };
                        Ok(Expr::ArrowFunction {
                            params,
                            return_type,
                            body,
                            is_async: true,
                        })
                    } else {
                        Err(self.error("expected '=>' after async parameters"))
                    }
                } else {
                    Err(self.error("expected 'fn' or '(' after 'async'"))
                }
            }
            _ => {
                let loc = self.location();
                let found = self
                    .current_token()
                    .map(|t| t.name())
                    .unwrap_or_else(|| "end of file".into());
                Err(ParseError::UnexpectedToken {
                    token: found,
                    line: loc.line,
                    col: loc.col,
                })
            }
        }
    }

    fn parse_template_literal(&mut self, first_part: String) -> Result<Expr, ParseError> {
        let mut parts = vec![TemplatePart::String(first_part)];
        while self.match_token(&Token::TemplateExprStart) {
            let expr = self.parse_expression()?;
            parts.push(TemplatePart::Expr(Box::new(expr)));
            self.expect(Token::TemplateExprEnd)?;
            if let Some(Token::TemplateString(s)) = self.current_token() {
                let s = s.clone();
                self.advance();
                parts.push(TemplatePart::String(s));
            } else {
                return Err(self.error("expected template string part after interpolation"));
            }
        }
        Ok(Expr::TemplateLiteral(parts))
    }

    fn parse_array_literal(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::LBracket)?;
        let mut elements = Vec::new();
        while !self.check(&Token::RBracket) && !self.is_at_end() {
            if self.match_token(&Token::Comma) {
                elements.push(ArrayElement::Elision);
                continue;
            }
            if self.match_token(&Token::Spread) {
                let expr = self.parse_expression()?;
                elements.push(ArrayElement::Spread(Box::new(expr)));
            } else {
                let expr = self.parse_expression()?;
                elements.push(ArrayElement::Expr(Box::new(expr)));
            }
            if !self.match_token(&Token::Comma) {
                break;
            }
        }
        self.expect(Token::RBracket)?;
        Ok(Expr::ArrayLiteral(elements))
    }

    fn parse_object_literal(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::LBrace)?;
        let mut properties = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            if self.match_token(&Token::Spread) {
                let expr = self.parse_expression()?;
                properties.push(ObjectProperty::Spread(Box::new(expr)));
            } else {
                let key = self.parse_property_name()?;
                if self.match_token(&Token::Colon) {
                    let value = self.parse_expression()?;
                    properties.push(ObjectProperty::Property {
                        key,
                        value: Box::new(value),
                    });
                } else if let PropertyName::Ident(name) = &key {
                    properties.push(ObjectProperty::Shorthand(name.clone()));
                } else {
                    return Err(self.error("expected ':' in object property"));
                }
            }
            if !self.match_token(&Token::Comma) {
                break;
            }
        }
        self.expect(Token::RBrace)?;
        Ok(Expr::ObjectLiteral(properties))
    }

    fn parse_grouping_or_arrow(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::LParen)?;
        if self.check(&Token::RParen) {
            self.advance();
            return Ok(Expr::Sequence(vec![]));
        }

        // Check if this is typed arrow function parameters: (x: int, y: string)
        if let Some(Token::Ident(_)) = self.current_token() {
            let saved_pos = self.pos;
            let first_name = match self.current_token() {
                Some(Token::Ident(n)) => n.clone(),
                _ => unreachable!(),
            };
            self.advance();

            // If identifier is followed by ':', we're parsing typed params
            if self.check(&Token::Colon) {
                self.advance();
                let first_type = Some(self.parse_type()?);
                let mut params = vec![(first_name, first_type)];

                while self.check(&Token::Comma) {
                    self.advance();
                    let name = match self.current_token() {
                        Some(Token::Ident(n)) => n.clone(),
                        _ => {
                            self.pos = saved_pos;
                            let expr = self.parse_expression()?;
                            self.expect(Token::RParen)?;
                            return Ok(Expr::Grouping(Box::new(expr)));
                        }
                    };
                    self.advance();
                    let ty = if self.check(&Token::Colon) {
                        self.advance();
                        Some(self.parse_type()?)
                    } else {
                        None
                    };
                    params.push((name, ty));
                }

                if self.check(&Token::RParen) {
                    self.advance();
                    return Ok(Expr::ArrowParams(params));
                }
            }

            // Not typed params, backtrack and parse as expression
            self.pos = saved_pos;
        }

        let first = self.parse_expression()?;
        if self.match_token(&Token::Comma) {
            let mut exprs = vec![first];
            while !self.check(&Token::RParen) && !self.is_at_end() {
                exprs.push(self.parse_expression()?);
                if !self.match_token(&Token::Comma) {
                    break;
                }
            }
            self.expect(Token::RParen)?;
            return Ok(Expr::Sequence(exprs));
        }
        self.expect(Token::RParen)?;
        Ok(Expr::Grouping(Box::new(first)))
    }

    fn parse_function_expression(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::Fn)?;
        let name = if let Some(Token::Ident(_)) = self.current_token() {
            Some(self.expect_ident()?)
        } else {
            None
        };
        let type_params = self.parse_type_params()?;
        self.expect(Token::LParen)?;
        let params = self.parse_formal_params()?;
        self.expect(Token::RParen)?;
        let return_type = if self.check(&Token::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        self.expect(Token::LBrace)?;
        let body = self.parse_function_body()?;
        self.expect(Token::RBrace)?;
        Ok(Expr::Function {
            name,
            type_params,
            params,
            return_type,
            body,
            is_async: false,
        })
    }

    fn parse_async_function_expression(&mut self) -> Result<Expr, ParseError> {
        let expr = self.parse_function_expression()?;
        if let Expr::Function {
            name,
            type_params,
            params,
            return_type,
            body,
            ..
        } = expr
        {
            Ok(Expr::Function {
                name,
                type_params,
                params,
                return_type,
                body,
                is_async: true,
            })
        } else {
            unreachable!()
        }
    }

    fn parse_class_expression(&mut self) -> Result<Expr, ParseError> {
        let annotations = self.parse_annotations();
        self.expect(Token::Class)?;
        let name = if let Some(Token::Ident(_)) = self.current_token() {
            Some(self.expect_ident()?)
        } else {
            None
        };
        let type_params = self.parse_type_params()?;
        let extends = if self.match_token(&Token::Extends) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };
        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            body.push(self.parse_class_element()?);
        }
        self.expect(Token::RBrace)?;
        Ok(Expr::Class {
            name,
            type_params,
            extends,
            body,
            annotations,
        })
    }

    fn parse_new_expression(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::New)?;
        let class_name = match self.current_token() {
            Some(Token::Ident(_)) => self.expect_ident()?,
            _ => return Err(self.error("expected class name after `new`")),
        };
        // Constructor arguments: `new Foo(args)`. The parentheses are
        // optional — `new Foo` and `new Foo()` both construct with no
        // arguments. Arguments are forwarded to the class's `new` method
        // (the constructor) by codegen.
        let args = if self.check(&Token::LParen) {
            self.parse_arguments()?
        } else {
            Vec::new()
        };
        Ok(Expr::New {
            callee: Box::new(Expr::Identifier(class_name)),
            args,
        })
    }

    fn parse_match_expression(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::Match)?;
        self.expect(Token::LParen)?;
        let value = self.parse_expression()?;
        self.expect(Token::RParen)?;
        self.expect(Token::LBrace)?;
        let mut arms = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            arms.push(self.parse_match_arm()?);
            if !self.match_token(&Token::Comma) {
                // optional comma
            }
        }
        self.expect(Token::RBrace)?;
        Ok(Expr::Match {
            value: Box::new(value),
            arms,
        })
    }

    fn parse_if_expression(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::If)?;
        self.expect(Token::LParen)?;
        let condition = self.parse_expression()?;
        self.expect(Token::RParen)?;
        self.expect(Token::LBrace)?;
        let then_stmts = self.parse_function_body()?;
        self.expect(Token::RBrace)?;
        let then_branch = if then_stmts.len() == 1 {
            match then_stmts.into_iter().next().unwrap() {
                Statement::Expression(e) => *e,
                _stmt => Expr::Grouping(Box::new(Expr::Identifier("__block__".into()))),
            }
        } else {
            Expr::Grouping(Box::new(Expr::Identifier("__block__".into()))) // placeholder - not ideal
        };
        let else_branch = if self.match_token(&Token::Else) {
            if self.check(&Token::If) {
                // else if → 递归解析嵌套 if 表达式
                Some(Box::new(self.parse_if_expression()?))
            } else {
                self.expect(Token::LBrace)?;
                let else_stmts = self.parse_function_body()?;
                self.expect(Token::RBrace)?;
                if else_stmts.len() == 1 {
                    match else_stmts.into_iter().next().unwrap() {
                        Statement::Expression(e) => Some(Box::new(*e)),
                        _ => Some(Box::new(Expr::NullLiteral)),
                    }
                } else {
                    Some(Box::new(Expr::NullLiteral))
                }
            }
        } else {
            None
        };
        Ok(Expr::If {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch,
        })
    }

    fn parse_arguments(&mut self) -> Result<Vec<Argument>, ParseError> {
        self.expect(Token::LParen)?;
        let mut args = Vec::new();
        while !self.check(&Token::RParen) && !self.is_at_end() {
            if self.match_token(&Token::Spread) {
                let expr = self.parse_expression()?;
                args.push(Argument::Spread(Box::new(expr)));
            } else {
                let expr = self.parse_expression()?;
                args.push(Argument::Expr(Box::new(expr)));
            }
            if !self.match_token(&Token::Comma) {
                break;
            }
        }
        self.expect(Token::RParen)?;
        Ok(args)
    }

    // Type annotations

    fn parse_type_annotation(&mut self) -> Result<TypeAnnotation, ParseError> {
        self.expect(Token::Colon)?;
        self.parse_type()
    }

    fn parse_type(&mut self) -> Result<TypeAnnotation, ParseError> {
        let mut parts: Vec<TypeAnnotation> = Vec::new();
        parts.push(self.parse_postfix_type()?);
        while self.match_token(&Token::Pipe) {
            parts.push(self.parse_postfix_type()?);
        }
        if parts.len() == 1 {
            Ok(parts.into_iter().next().unwrap())
        } else {
            Ok(TypeAnnotation::Union(parts))
        }
    }

    fn parse_postfix_type(&mut self) -> Result<TypeAnnotation, ParseError> {
        let mut ty = self.parse_primary_type()?;
        if self.match_token(&Token::Question) {
            ty = TypeAnnotation::Nullable(Box::new(ty));
        }
        while self.match_token(&Token::LBracket) {
            self.expect(Token::RBracket)?;
            ty = TypeAnnotation::Array(Box::new(ty));
        }
        Ok(ty)
    }

    fn parse_primary_type(&mut self) -> Result<TypeAnnotation, ParseError> {
        match self.current_token() {
            Some(Token::Void) => {
                self.advance();
                Ok(TypeAnnotation::Identifier("void".to_string()))
            }
            Some(Token::Null) => {
                self.advance();
                Ok(TypeAnnotation::Identifier("null".to_string()))
            }
            Some(Token::Dyn) => {
                self.advance();
                // Check if next token could start a type (for `dyn Trait` syntax).
                // If not, treat as bare `dyn` (any dynamic type).
                //
                // R2 / v0.5.9: LBrace intentionally NOT in this list. In a
                // function declaration like `fn f(): dyn { ... }`, the `{`
                // after `dyn` opens the function body, NOT an anonymous
                // object type. Treating it as `Dyn({...})` would make the
                // parser try to interpret statement-level keywords (e.g.
                // `return`) as object field names. Anonymous object types
                // (`dyn { field: T }`) are not used by any code in
                // stdlib/ or examples/, so dropping LBrace from the
                // type-starter set is safe.
                if matches!(
                    self.current_token(),
                    Some(Token::Ident(_))
                        | Some(Token::Fn)
                        | Some(Token::LBracket)
                        | Some(Token::LParen)
                        | Some(Token::Void)
                        | Some(Token::Null)
                ) {
                    let inner = self.parse_type()?;
                    Ok(TypeAnnotation::Dyn(Box::new(inner)))
                } else {
                    // Bare dyn — represents any dynamic type
                    Ok(TypeAnnotation::Dyn(Box::new(TypeAnnotation::Builtin(
                        "dyn".to_string(),
                    ))))
                }
            }
            Some(Token::Ident(name)) => {
                let name = name.clone();
                self.advance();
                if self.check(&Token::Less) {
                    // Generic type
                    let args = self.parse_type_args()?;
                    Ok(TypeAnnotation::Generic { base: name, args })
                } else if Self::is_builtin_type(&name) {
                    Ok(TypeAnnotation::Builtin(name))
                } else {
                    Ok(TypeAnnotation::Identifier(name))
                }
            }
            Some(Token::Fn) => {
                self.advance();
                self.expect(Token::LParen)?;
                let mut params = Vec::new();
                while !self.check(&Token::RParen) && !self.is_at_end() {
                    params.push(self.parse_type()?);
                    if !self.match_token(&Token::Comma) {
                        break;
                    }
                }
                self.expect(Token::RParen)?;
                // Accept both `->` (FatArrow) and `:` as return type separator
                if !self.match_token(&Token::FatArrow) && !self.match_token(&Token::Colon) {
                    return Err(self.error("expected '->' or ':' after function type parameters"));
                }
                let return_type = Box::new(self.parse_type()?);
                Ok(TypeAnnotation::Function {
                    params,
                    return_type,
                })
            }
            Some(Token::LBrace) => {
                self.advance();
                let mut fields = Vec::new();
                while !self.check(&Token::RBrace) && !self.is_at_end() {
                    if self.check(&Token::Fn) {
                        // Method signature: fn name(params): return_type
                        self.advance();
                        let name = self.expect_ident()?;
                        self.expect(Token::LParen)?;
                        let mut params = Vec::new();
                        while !self.check(&Token::RParen) && !self.is_at_end() {
                            params.push(self.parse_type()?);
                            if !self.match_token(&Token::Comma) {
                                break;
                            }
                        }
                        self.expect(Token::RParen)?;
                        if !self.match_token(&Token::FatArrow) && !self.match_token(&Token::Colon) {
                            return Err(self.error("expected '->' or ':' after method parameters"));
                        }
                        let return_type = Box::new(self.parse_type()?);
                        fields.push(TypeField {
                            name,
                            ty: TypeAnnotation::Function {
                                params,
                                return_type,
                            },
                        });
                    } else {
                        let name = self.expect_ident_or_keyword()?;
                        self.expect(Token::Colon)?;
                        let ty = self.parse_type()?;
                        fields.push(TypeField { name, ty });
                    }
                    if !self.match_token(&Token::Comma) {
                        break;
                    }
                }
                self.expect(Token::RBrace)?;
                Ok(TypeAnnotation::Object(fields))
            }
            Some(Token::LBracket) => {
                self.advance();
                let ty = self.parse_type()?;
                self.expect(Token::RBracket)?;
                Ok(TypeAnnotation::Array(Box::new(ty)))
            }
            Some(Token::LParen) => {
                self.advance();
                let mut types = Vec::new();
                while !self.check(&Token::RParen) && !self.is_at_end() {
                    types.push(self.parse_type()?);
                    if !self.match_token(&Token::Comma) {
                        break;
                    }
                }
                self.expect(Token::RParen)?;
                if types.len() == 1 {
                    Ok(types.into_iter().next().unwrap())
                } else {
                    Ok(TypeAnnotation::Tuple(types))
                }
            }
            Some(Token::Star) => {
                self.advance();
                // 解析 `*mut T` 或 `*const T` 指针类型（用于 FFI extern 声明）
                let qualifier = if self.check_ident("mut") {
                    self.advance();
                    "mut"
                } else if self.check_ident("const") {
                    self.advance();
                    "const"
                } else {
                    "mut"
                };
                let inner = self.parse_postfix_type()?;
                let inner_str = match &inner {
                    TypeAnnotation::Identifier(s) => s.clone(),
                    TypeAnnotation::Builtin(s) => s.clone(),
                    _ => "void".to_string(),
                };
                Ok(TypeAnnotation::Identifier(format!(
                    "*{} {}",
                    qualifier, inner_str
                )))
            }
            _ => Err(self.error("expected type")),
        }
    }

    fn parse_type_args(&mut self) -> Result<Vec<TypeAnnotation>, ParseError> {
        self.expect(Token::Less)?;
        let mut args = Vec::new();
        let mut depth: i32 = 1;
        while depth > 0 && !self.is_at_end() {
            match self.current_token() {
                Some(Token::Greater) => {
                    self.advance();
                    depth -= 1;
                }
                Some(Token::Shr) | Some(Token::ShrAssign) => {
                    self.advance();
                    depth -= 2;
                    self.pending_extra_closes += 1;
                }
                Some(Token::Comma) => {
                    if depth == 1 {
                        self.advance();
                    }
                }
                _ => {
                    if depth == 1 {
                        args.push(self.parse_type()?);
                        if self.pending_extra_closes > 0 {
                            depth -= self.pending_extra_closes as i32;
                            self.pending_extra_closes = 0;
                        }
                    }
                }
            }
        }
        Ok(args)
    }

    fn parse_type_params(&mut self) -> Result<Vec<TypeParam>, ParseError> {
        if !self.check(&Token::Less) {
            return Ok(vec![]);
        }
        self.advance();
        let mut params = Vec::new();
        let mut depth: i32 = 1;
        while depth > 0 && !self.is_at_end() {
            match self.current_token() {
                Some(Token::Greater) => {
                    self.advance();
                    depth -= 1;
                }
                Some(Token::Shr) | Some(Token::ShrAssign) => {
                    self.advance();
                    depth -= 2;
                    self.pending_extra_closes += 1;
                }
                Some(Token::Comma) => {
                    if depth == 1 {
                        self.advance();
                    }
                }
                _ => {
                    if depth == 1 {
                        let name = self.expect_ident()?;
                        let mut bounds = Vec::new();
                        if self.match_token(&Token::Colon) {
                            bounds.push(self.expect_ident()?);
                            while self.match_token(&Token::Plus) {
                                bounds.push(self.expect_ident()?);
                            }
                        }
                        params.push(TypeParam { name, bounds });
                        if self.pending_extra_closes > 0 {
                            depth -= self.pending_extra_closes as i32;
                            self.pending_extra_closes = 0;
                        }
                    }
                }
            }
        }
        Ok(params)
    }

    // Parameters

    fn parse_formal_params(&mut self) -> Result<Vec<Param>, ParseError> {
        let mut params = Vec::new();
        while !self.check(&Token::RParen) && !self.is_at_end() {
            params.push(self.parse_formal_param()?);
            if !self.match_token(&Token::Comma) {
                break;
            }
        }
        Ok(params)
    }

    fn parse_formal_param(&mut self) -> Result<Param, ParseError> {
        let is_rest = self.match_token(&Token::Spread);
        let pattern = self.parse_pattern()?;
        // Optional parameter marker: `name?: type` (R2 / v0.5.9).
        // The `?` is consumed here so the colon-to-type transition
        // below sees `:` directly. Typechecker/codegen treat the
        // argument as nullable at the call site.
        let is_optional = self.match_token(&Token::Question);
        let ty = if self.check(&Token::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        let init = if self.match_token(&Token::Assign) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };
        Ok(Param {
            pattern,
            ty,
            init,
            is_rest,
            is_optional,
        })
    }
}

// ── Pattern parsing (delegated to pattern.rs) ────────────────

impl Parser {
    pub fn parse_pattern(&mut self) -> Result<Pattern, ParseError> {
        self.parse_or_pattern()
    }

    fn parse_or_pattern(&mut self) -> Result<Pattern, ParseError> {
        let mut patterns = vec![self.parse_base_pattern()?];
        while self.match_token(&Token::Pipe) {
            patterns.push(self.parse_base_pattern()?);
        }
        if patterns.len() == 1 {
            Ok(patterns.into_iter().next().unwrap())
        } else {
            Ok(Pattern::Or(patterns))
        }
    }

    fn parse_base_pattern(&mut self) -> Result<Pattern, ParseError> {
        if self.match_token(&Token::Underscore) {
            // Wait, Token doesn't have Underscore! _ is an Ident
            return Ok(Pattern::Wildcard);
        }
        if self.match_token(&Token::Spread) {
            let name = self.expect_ident()?;
            return Ok(Pattern::Rest(name));
        }
        if self.check(&Token::LBrace) {
            return self.parse_object_pattern();
        }
        if self.check(&Token::LBracket) {
            return self.parse_array_pattern();
        }
        if let Some(lit) = self.try_parse_literal_pattern()? {
            return Ok(lit);
        }
        if self.match_token(&Token::SelfKw) {
            return Ok(Pattern::Identifier("self".to_string()));
        }
        let name = self.expect_ident()?;
        if self.match_token(&Token::As) {
            let alias = self.expect_ident()?;
            return Ok(Pattern::As(Box::new(Pattern::Identifier(name)), alias));
        }
        Ok(Pattern::Identifier(name))
    }

    fn parse_object_pattern(&mut self) -> Result<Pattern, ParseError> {
        self.expect(Token::LBrace)?;
        let mut fields = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            if self.match_token(&Token::Spread) {
                let name = self.expect_ident()?;
                fields.push(ObjectPatternField::Rest(name));
            } else {
                let key = self.expect_ident()?;
                if self.match_token(&Token::Colon) {
                    let pattern = self.parse_pattern()?;
                    fields.push(ObjectPatternField::Property { key, pattern });
                } else if self.match_token(&Token::Assign) {
                    let default = self.parse_expression()?;
                    fields.push(ObjectPatternField::ShorthandDefault(key, Box::new(default)));
                } else {
                    fields.push(ObjectPatternField::Shorthand(key));
                }
            }
            if !self.match_token(&Token::Comma) {
                break;
            }
        }
        self.expect(Token::RBrace)?;
        Ok(Pattern::Object(fields))
    }

    fn parse_array_pattern(&mut self) -> Result<Pattern, ParseError> {
        self.expect(Token::LBracket)?;
        let mut elements = Vec::new();
        while !self.check(&Token::RBracket) && !self.is_at_end() {
            if self.match_token(&Token::Comma) {
                elements.push(ArrayPatternElement::Elision);
                continue;
            }
            if self.match_token(&Token::Spread) {
                let pat = self.parse_pattern()?;
                elements.push(ArrayPatternElement::Rest(pat));
            } else {
                let pat = self.parse_pattern()?;
                if self.match_token(&Token::Assign) {
                    let default = self.parse_expression()?;
                    elements.push(ArrayPatternElement::Default(pat, Box::new(default)));
                } else {
                    elements.push(ArrayPatternElement::Pattern(pat));
                }
            }
            if !self.match_token(&Token::Comma) {
                break;
            }
        }
        self.expect(Token::RBracket)?;
        Ok(Pattern::Array(elements))
    }

    fn try_parse_literal_pattern(&mut self) -> Result<Option<Pattern>, ParseError> {
        let pat = match self.current_token() {
            Some(Token::Int(i)) => {
                let i = *i;
                self.advance();
                Some(Pattern::Literal(Box::new(Expr::IntLiteral(i))))
            }
            Some(Token::BigInt(s)) => {
                let s = s.clone();
                self.advance();
                Some(Pattern::Literal(Box::new(Expr::BigIntLiteral(s))))
            }
            Some(Token::Float(f)) => {
                let f = *f;
                self.advance();
                Some(Pattern::Literal(Box::new(Expr::FloatLiteral(f))))
            }
            Some(Token::String(s)) => {
                let s = s.clone();
                self.advance();
                Some(Pattern::Literal(Box::new(Expr::StringLiteral(s))))
            }
            Some(Token::True) => {
                self.advance();
                Some(Pattern::Literal(Box::new(Expr::BooleanLiteral(true))))
            }
            Some(Token::False) => {
                self.advance();
                Some(Pattern::Literal(Box::new(Expr::BooleanLiteral(false))))
            }
            Some(Token::Null) => {
                self.advance();
                Some(Pattern::Literal(Box::new(Expr::NullLiteral)))
            }
            _ => None,
        };
        Ok(pat)
    }
}

// ── Operator binding powers ──────────────────────────────────

fn infix_binding_power(op: &Token) -> Option<(u8, u8)> {
    match op {
        Token::Assign
        | Token::PlusAssign
        | Token::MinusAssign
        | Token::StarAssign
        | Token::SlashAssign
        | Token::PercentAssign
        | Token::PowerAssign
        | Token::AmpAssign
        | Token::PipeAssign
        | Token::CaretAssign
        | Token::ShlAssign
        | Token::ShrAssign
        | Token::UShrAssign
        | Token::AndAssign
        | Token::OrAssign
        | Token::NullishAssign => {
            Some((21, 20)) // right-associative
        }
        Token::FatArrow => Some((31, 30)), // right-associative
        Token::Question => Some((41, 40)), // right-associative (ternary)
        Token::Nullish => Some((50, 51)),
        Token::Or => Some((60, 61)),
        Token::And => Some((70, 71)),
        Token::Pipe => Some((80, 81)),
        Token::Caret => Some((90, 91)),
        Token::Amp => Some((100, 101)),
        Token::Equals | Token::NotEquals | Token::StrictEquals | Token::StrictNotEquals => {
            Some((110, 111))
        }
        Token::Less
        | Token::Greater
        | Token::LessEq
        | Token::GreaterEq
        | Token::In
        | Token::Instanceof => Some((120, 121)),
        Token::Shl | Token::Shr | Token::UShr => Some((130, 131)),
        Token::Plus | Token::Minus => Some((140, 141)),
        Token::Star | Token::Slash | Token::Percent => Some((150, 151)),
        Token::Power => Some((161, 160)), // right-associative
        Token::Dot | Token::OptChain => Some((180, 181)),
        Token::LParen => Some((180, 181)),   // call
        Token::LBracket => Some((180, 181)), // index
        _ => None,
    }
}

fn token_to_binary_op(op: &Token) -> Option<BinaryOp> {
    match op {
        Token::StrictEquals => Some(BinaryOp::StrictEquals),
        Token::StrictNotEquals => Some(BinaryOp::StrictNotEquals),
        Token::Equals => Some(BinaryOp::Equals),
        Token::NotEquals => Some(BinaryOp::NotEquals),
        Token::Less => Some(BinaryOp::Less),
        Token::Greater => Some(BinaryOp::Greater),
        Token::LessEq => Some(BinaryOp::LessEq),
        Token::GreaterEq => Some(BinaryOp::GreaterEq),
        Token::In => Some(BinaryOp::In),
        Token::Instanceof => Some(BinaryOp::Instanceof),
        Token::Plus => Some(BinaryOp::Plus),
        Token::Minus => Some(BinaryOp::Minus),
        Token::Star => Some(BinaryOp::Star),
        Token::Slash => Some(BinaryOp::Slash),
        Token::Percent => Some(BinaryOp::Percent),
        Token::Power => Some(BinaryOp::Power),
        Token::Shl => Some(BinaryOp::Shl),
        Token::Shr => Some(BinaryOp::Shr),
        Token::UShr => Some(BinaryOp::UShr),
        Token::Amp => Some(BinaryOp::Amp),
        Token::Pipe => Some(BinaryOp::Pipe),
        Token::Caret => Some(BinaryOp::Caret),
        Token::And => Some(BinaryOp::And),
        Token::Or => Some(BinaryOp::Or),
        Token::Nullish => Some(BinaryOp::Nullish),
        _ => None,
    }
}

fn token_to_assign_op(op: &Token) -> Option<AssignOp> {
    match op {
        Token::Assign => Some(AssignOp::Assign),
        Token::PlusAssign => Some(AssignOp::PlusAssign),
        Token::MinusAssign => Some(AssignOp::MinusAssign),
        Token::StarAssign => Some(AssignOp::StarAssign),
        Token::SlashAssign => Some(AssignOp::SlashAssign),
        Token::PercentAssign => Some(AssignOp::PercentAssign),
        Token::PowerAssign => Some(AssignOp::PowerAssign),
        Token::AmpAssign => Some(AssignOp::AmpAssign),
        Token::PipeAssign => Some(AssignOp::PipeAssign),
        Token::CaretAssign => Some(AssignOp::CaretAssign),
        Token::ShlAssign => Some(AssignOp::ShlAssign),
        Token::ShrAssign => Some(AssignOp::ShrAssign),
        Token::UShrAssign => Some(AssignOp::UShrAssign),
        Token::AndAssign => Some(AssignOp::AndAssign),
        Token::OrAssign => Some(AssignOp::OrAssign),
        Token::NullishAssign => Some(AssignOp::NullishAssign),
        _ => None,
    }
}

fn token_to_prefix_op(op: &Token) -> Option<UnaryOp> {
    match op {
        Token::Plus => Some(UnaryOp::Plus),
        Token::Minus => Some(UnaryOp::Minus),
        Token::Not => Some(UnaryOp::Not),
        Token::Tilde => Some(UnaryOp::Tilde),
        Token::Increment => Some(UnaryOp::PreIncrement),
        Token::Decrement => Some(UnaryOp::PreDecrement),
        Token::Typeof => Some(UnaryOp::Typeof),
        Token::Void => Some(UnaryOp::Void),
        Token::Delete => Some(UnaryOp::Delete),
        Token::Await => Some(UnaryOp::Await),
        _ => None,
    }
}

fn expr_to_arrow_params(expr: Expr) -> Result<Vec<Param>, ParseError> {
    match expr {
        Expr::Identifier(name) => Ok(vec![Param {
            pattern: Pattern::Identifier(name),
            ty: None,
            init: None,
            is_rest: false,
            is_optional: false,
        }]),
        Expr::Sequence(exprs) => {
            let mut params = Vec::new();
            for e in exprs {
                match e {
                    Expr::Identifier(name) => params.push(Param {
                        pattern: Pattern::Identifier(name),
                        ty: None,
                        init: None,
                        is_rest: false,
                        is_optional: false,
                    }),
                    _ => {
                        return Err(ParseError::SyntaxError {
                            message: "invalid arrow function parameter".into(),
                            line: 0,
                            col: 0,
                        })
                    }
                }
            }
            Ok(params)
        }
        Expr::Grouping(boxed) => match *boxed {
            Expr::Sequence(exprs) => {
                let mut params = Vec::new();
                for e in exprs {
                    match e {
                        Expr::Identifier(name) => params.push(Param {
                            pattern: Pattern::Identifier(name),
                            ty: None,
                            init: None,
                            is_rest: false,
                            is_optional: false,
                        }),
                        _ => {
                            return Err(ParseError::SyntaxError {
                                message: "invalid arrow function parameter".into(),
                                line: 0,
                                col: 0,
                            })
                        }
                    }
                }
                Ok(params)
            }
            Expr::Identifier(name) => Ok(vec![Param {
                pattern: Pattern::Identifier(name),
                ty: None,
                init: None,
                is_rest: false,
                is_optional: false,
            }]),
            _ => Err(ParseError::SyntaxError {
                message: "invalid arrow function parameters".into(),
                line: 0,
                col: 0,
            }),
        },
        Expr::ArrowParams(params) => Ok(params
            .into_iter()
            .map(|(name, ty)| Param {
                pattern: Pattern::Identifier(name),
                ty,
                init: None,
                is_rest: false,
                is_optional: false,
            })
            .collect()),
        _ => Err(ParseError::SyntaxError {
            message: "invalid arrow function parameters".into(),
            line: 0,
            col: 0,
        }),
    }
}
