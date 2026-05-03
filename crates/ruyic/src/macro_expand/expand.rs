use crate::lexer::token::Token;
use crate::macro_expand::hygiene::StandardHygieneContext;
use crate::macro_expand::pattern::{parse_pattern, PatternMatcher};
use crate::macro_expand::{MacroError, MacroRegistry, MacroResult, MacroRule, MAX_EXPANSION_DEPTH};
use crate::parser::ast::{Argument, MatchArm};
use crate::parser::ast::{Declaration, Expr, ForInit, ModuleItem, Program, Statement};

pub struct MacroExpander {
    registry: MacroRegistry,
    depth: usize,
    hygiene_ctx: StandardHygieneContext,
}

impl MacroExpander {
    pub fn new(registry: &MacroRegistry) -> Self {
        Self {
            registry: registry.clone(),
            depth: 0,
            hygiene_ctx: StandardHygieneContext::new(),
        }
    }

    pub fn expand_program(&mut self, program: &Program) -> MacroResult<Program> {
        let mut expanded_items = Vec::new();

        for item in &program.items {
            let expanded = self.expand_module_item(item)?;
            expanded_items.push(expanded);
        }

        Ok(Program {
            items: expanded_items,
        })
    }

    fn expand_module_item(&mut self, item: &ModuleItem) -> MacroResult<ModuleItem> {
        match item {
            ModuleItem::Declaration(decl) => {
                let expanded = self.expand_declaration(decl)?;
                Ok(ModuleItem::Declaration(expanded))
            }
            ModuleItem::Statement(stmt) => {
                let expanded = self.expand_statement(stmt)?;
                Ok(ModuleItem::Statement(expanded))
            }
            ModuleItem::Import(import) => Ok(ModuleItem::Import(import.clone())),
            ModuleItem::Export(export) => Ok(ModuleItem::Export(export.clone())),
        }
    }

    fn expand_declaration(&mut self, decl: &Declaration) -> MacroResult<Declaration> {
        match decl {
            Declaration::Macro { name, rules } => {
                self.registry.add_macro(name.clone(), rules.clone());
                Ok(decl.clone())
            }
            _ => Ok(decl.clone()),
        }
    }

    fn expand_statement(&mut self, stmt: &Statement) -> MacroResult<Statement> {
        match stmt {
            Statement::Expression(expr) => {
                let expanded = self.expand_expression(expr)?;
                Ok(Statement::Expression(Box::new(expanded)))
            }
            Statement::Block(stmts) => {
                let mut expanded_stmts = Vec::new();
                for s in stmts {
                    expanded_stmts.push(self.expand_statement(s)?);
                }
                Ok(Statement::Block(expanded_stmts))
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond = self.expand_expression(condition)?;
                let then_b = Box::new(self.expand_statement(then_branch)?);
                let else_b = match else_branch {
                    Some(e) => Some(Box::new(self.expand_statement(e)?)),
                    None => None,
                };
                Ok(Statement::If {
                    condition: Box::new(cond),
                    then_branch: then_b,
                    else_branch: else_b,
                })
            }
            Statement::While { condition, body } => {
                let cond = self.expand_expression(condition)?;
                let b = Box::new(self.expand_statement(body)?);
                Ok(Statement::While {
                    condition: Box::new(cond),
                    body: b,
                })
            }
            Statement::For {
                init,
                condition,
                update,
                body,
            } => {
                let init_expanded = match init {
                    Some(ForInit::Expr(e)) => {
                        Some(ForInit::Expr(Box::new(self.expand_expression(e)?)))
                    }
                    Some(ForInit::VarDecl(d)) => {
                        Some(ForInit::VarDecl(self.expand_declaration(d)?))
                    }
                    None => None,
                };
                let cond_expanded = match condition {
                    Some(e) => Some(Box::new(self.expand_expression(e)?)),
                    None => None,
                };
                let update_expanded = match update {
                    Some(e) => Some(Box::new(self.expand_expression(e)?)),
                    None => None,
                };
                let body_expanded = Box::new(self.expand_statement(body)?);
                Ok(Statement::For {
                    init: init_expanded,
                    condition: cond_expanded,
                    update: update_expanded,
                    body: body_expanded,
                })
            }
            Statement::Return(expr) => {
                let expanded = match expr {
                    Some(e) => Some(Box::new(self.expand_expression(e)?)),
                    None => None,
                };
                Ok(Statement::Return(expanded))
            }
            Statement::Throw(expr) => {
                let expanded = self.expand_expression(expr)?;
                Ok(Statement::Throw(Box::new(expanded)))
            }
            Statement::Try {
                body,
                catch,
                finally,
            } => {
                let body_expanded: Result<Vec<Statement>, MacroError> =
                    body.iter().map(|s| self.expand_statement(s)).collect();
                let catch_expanded = catch
                    .as_ref()
                    .map(|c| -> Result<crate::parser::ast::CatchClause, MacroError> {
                        let body: Result<Vec<Statement>, MacroError> =
                            c.body.iter().map(|s| self.expand_statement(s)).collect();
                        Ok(crate::parser::ast::CatchClause {
                            pattern: c.pattern.clone(),
                            ty: c.ty.clone(),
                            body: body?,
                        })
                    })
                    .transpose()?;
                let finally_expanded = finally
                    .as_ref()
                    .map(|f| -> Result<Vec<Statement>, MacroError> {
                        f.iter().map(|s| self.expand_statement(s)).collect()
                    })
                    .transpose()?;
                Ok(Statement::Try {
                    body: body_expanded?,
                    catch: catch_expanded,
                    finally: finally_expanded,
                })
            }
            Statement::Match { value, arms } => {
                let value_expanded = self.expand_expression(value)?;
                let arms_expanded: Result<Vec<MatchArm>, MacroError> = arms
                    .iter()
                    .map(|arm| {
                        let body: Result<Vec<Statement>, MacroError> =
                            arm.body.iter().map(|s| self.expand_statement(s)).collect();
                        Ok(crate::parser::ast::MatchArm {
                            pattern: arm.pattern.clone(),
                            guard: arm.guard.clone(),
                            body: body?,
                        })
                    })
                    .collect();
                Ok(Statement::Match {
                    value: Box::new(value_expanded),
                    arms: arms_expanded?,
                })
            }
            Statement::Declaration(decl) => {
                let expanded = self.expand_declaration(decl)?;
                Ok(Statement::Declaration(expanded))
            }
            _ => Ok(stmt.clone()),
        }
    }

    fn expand_expression(&mut self, expr: &Expr) -> MacroResult<Expr> {
        match expr {
            Expr::Call { callee, args } => {
                let callee_expanded = self.expand_expression(callee)?;
                let args_expanded: Result<Vec<Argument>, MacroError> = args
                    .iter()
                    .map(|a| match a {
                        Argument::Expr(e) => {
                            Ok(Argument::Expr(Box::new(self.expand_expression(e)?)))
                        }
                        Argument::Spread(e) => {
                            Ok(Argument::Spread(Box::new(self.expand_expression(e)?)))
                        }
                    })
                    .collect();

                if let Expr::Identifier(name) = &callee_expanded {
                    if self.registry.contains(name.as_str()) && !is_buildin_expr(expr) {
                        return self.expand_macro_call(name.as_str(), &args_expanded?);
                    }
                }

                Ok(Expr::Call {
                    callee: Box::new(callee_expanded),
                    args: args_expanded?,
                })
            }
            Expr::Block(stmts) => {
                let stmts_expanded: Result<Vec<Statement>, MacroError> =
                    stmts.iter().map(|s| self.expand_statement(s)).collect();
                Ok(Expr::Block(stmts_expanded?))
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond = self.expand_expression(condition)?;
                let then_b = Box::new(self.expand_expression(then_branch)?);
                let else_b = match else_branch {
                    Some(e) => Some(Box::new(self.expand_expression(e)?)),
                    None => None,
                };
                Ok(Expr::If {
                    condition: Box::new(cond),
                    then_branch: then_b,
                    else_branch: else_b,
                })
            }
            Expr::Match { value, arms } => {
                let value_expanded = self.expand_expression(value)?;
                let arms_expanded: Result<Vec<MatchArm>, MacroError> = arms
                    .iter()
                    .map(|arm| {
                        let body = self.expand_expression(&Expr::Block(arm.body.clone()))?;
                        if let Expr::Block(stmts) = body {
                            Ok(crate::parser::ast::MatchArm {
                                pattern: arm.pattern.clone(),
                                guard: arm.guard.clone(),
                                body: stmts,
                            })
                        } else {
                            Ok(crate::parser::ast::MatchArm {
                                pattern: arm.pattern.clone(),
                                guard: arm.guard.clone(),
                                body: vec![Statement::Expression(Box::new(body))],
                            })
                        }
                    })
                    .collect();
                Ok(Expr::Match {
                    value: Box::new(value_expanded),
                    arms: arms_expanded?,
                })
            }
            Expr::Binary { op, left, right } => {
                let left_expanded = self.expand_expression(left)?;
                let right_expanded = self.expand_expression(right)?;
                Ok(Expr::Binary {
                    op: op.clone(),
                    left: Box::new(left_expanded),
                    right: Box::new(right_expanded),
                })
            }
            Expr::Unary { op, operand } => {
                let operand_expanded = self.expand_expression(operand)?;
                Ok(Expr::Unary {
                    op: op.clone(),
                    operand: Box::new(operand_expanded),
                })
            }
            Expr::Assignment { left, op, right } => {
                let left_expanded = self.expand_expression(left)?;
                let right_expanded = self.expand_expression(right)?;
                Ok(Expr::Assignment {
                    left: Box::new(left_expanded),
                    op: op.clone(),
                    right: Box::new(right_expanded),
                })
            }
            _ => Ok(expr.clone()),
        }
    }

    fn expand_macro_call(
        &mut self,
        name: &str,
        args: &[crate::parser::ast::Argument],
    ) -> MacroResult<Expr> {
        if self.depth >= MAX_EXPANSION_DEPTH {
            return Err(MacroError::ExpansionDepthExceeded {
                macro_name: name.to_string(),
                depth: self.depth,
            });
        }

        self.depth += 1;

        let arg_tokens = args_to_tokens(args);
        let expansion = if let Some(rules) = self.registry.get_macro(name) {
            let rules = rules.to_vec();
            self.expand_with_rules(name, &rules, &arg_tokens)?
        } else if let Some(builtin) = self.registry.get_builtin(name) {
            (builtin.expand)(&arg_tokens, &self.hygiene_ctx)?
        } else {
            return Err(MacroError::NoMatchingRule {
                macro_name: name.to_string(),
                location: "unknown".to_string(),
            });
        };

        self.depth -= 1;

        let expanded_tokens = apply_template(
            &expansion,
            &std::collections::HashMap::new(),
            &self.hygiene_ctx,
        );
        let source = tokens_to_source(&expanded_tokens);

        let mut parser =
            crate::parser::Parser::new(&source).map_err(|e| MacroError::InvalidInvocation {
                macro_name: name.to_string(),
                message: e.to_string(),
            })?;
        let program = parser.parse().map_err(|e| MacroError::InvalidInvocation {
            macro_name: name.to_string(),
            message: e.to_string(),
        })?;

        if let Some(ModuleItem::Statement(Statement::Expression(e))) = program.items.first() {
            self.expand_expression(e)
        } else if let Some(ModuleItem::Declaration(d)) = program.items.first() {
            if let Declaration::Macro { .. } = d {
                Err(MacroError::NestedMacroDefinition {
                    location: "unknown".to_string(),
                })
            } else {
                Ok(Expr::Block(vec![Statement::Declaration(d.clone())]))
            }
        } else if let Some(ModuleItem::Statement(s)) = program.items.first() {
            match s {
                Statement::Expression(e) => self.expand_expression(e),
                _ => {
                    let expanded = self.expand_statement(s)?;
                    Ok(Expr::Block(vec![expanded]))
                }
            }
        } else {
            Ok(Expr::Block(vec![]))
        }
    }

    fn expand_with_rules(
        &mut self,
        _name: &str,
        rules: &[MacroRule],
        input: &[Token],
    ) -> MacroResult<Vec<Token>> {
        for rule in rules {
            let pattern = parse_pattern(&rule.pattern)?;
            let mut matcher = PatternMatcher::new(pattern, input.to_vec());
            if let Ok(result) = matcher.match_pattern() {
                if result.matched {
                    return Ok(rule.body.clone());
                }
            }
        }
        Err(MacroError::NoMatchingRule {
            macro_name: _name.to_string(),
            location: "unknown".to_string(),
        })
    }
}

fn is_buildin_expr(expr: &Expr) -> bool {
    matches!(expr, Expr::Identifier(_) | Expr::Call { .. })
}

fn args_to_tokens(args: &[crate::parser::ast::Argument]) -> Vec<Token> {
    let mut tokens = Vec::new();
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            tokens.push(Token::Comma);
        }
        match arg {
            crate::parser::ast::Argument::Expr(e) => {
                tokens.extend(expr_to_tokens(e));
            }
            crate::parser::ast::Argument::Spread(e) => {
                tokens.push(Token::Spread);
                tokens.extend(expr_to_tokens(e));
            }
        }
    }
    tokens
}

fn expr_to_tokens(expr: &Expr) -> Vec<Token> {
    match expr {
        Expr::IntLiteral(i) => vec![Token::Int(*i)],
        Expr::FloatLiteral(f) => vec![Token::Float(*f)],
        Expr::StringLiteral(s) => vec![Token::String(s.clone())],
        Expr::BooleanLiteral(b) => vec![if *b { Token::True } else { Token::False }],
        Expr::NullLiteral => vec![Token::Null],
        Expr::Identifier(name) => vec![Token::Ident(name.clone())],
        Expr::Binary { op, left, right } => {
            let mut tokens = expr_to_tokens(left);
            tokens.push(binary_op_to_token(op));
            tokens.extend(expr_to_tokens(right));
            tokens
        }
        Expr::Unary { op, operand } => {
            let mut tokens = vec![unary_op_to_token(op)];
            tokens.extend(expr_to_tokens(operand));
            tokens
        }
        Expr::Call { callee, args } => {
            let mut tokens = expr_to_tokens(callee);
            tokens.push(Token::LParen);
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    tokens.push(Token::Comma);
                }
                match arg {
                    crate::parser::ast::Argument::Expr(e) => tokens.extend(expr_to_tokens(e)),
                    crate::parser::ast::Argument::Spread(e) => {
                        tokens.push(Token::Spread);
                        tokens.extend(expr_to_tokens(e));
                    }
                }
            }
            tokens.push(Token::RParen);
            tokens
        }
        Expr::Member {
            object, property, ..
        } => {
            let mut tokens = expr_to_tokens(object);
            tokens.push(Token::Dot);
            match property {
                crate::parser::ast::MemberProperty::Ident(name) => {
                    tokens.push(Token::Ident(name.clone()));
                }
                crate::parser::ast::MemberProperty::Expr(e) => {
                    tokens.push(Token::LBracket);
                    tokens.extend(expr_to_tokens(e));
                    tokens.push(Token::RBracket);
                }
            }
            tokens
        }
        Expr::ArrayLiteral(elements) => {
            let mut tokens = vec![Token::LBracket];
            for (i, el) in elements.iter().enumerate() {
                if i > 0 {
                    tokens.push(Token::Comma);
                }
                match el {
                    crate::parser::ast::ArrayElement::Expr(e) => tokens.extend(expr_to_tokens(e)),
                    crate::parser::ast::ArrayElement::Spread(e) => {
                        tokens.push(Token::Spread);
                        tokens.extend(expr_to_tokens(e));
                    }
                    crate::parser::ast::ArrayElement::Elision => {}
                }
            }
            tokens.push(Token::RBracket);
            tokens
        }
        Expr::ObjectLiteral(properties) => {
            let mut tokens = vec![Token::LBrace];
            for (i, prop) in properties.iter().enumerate() {
                if i > 0 {
                    tokens.push(Token::Comma);
                }
                match prop {
                    crate::parser::ast::ObjectProperty::Property { key, value } => {
                        match key {
                            crate::parser::ast::PropertyName::Ident(name) => {
                                tokens.push(Token::Ident(name.clone()));
                            }
                            crate::parser::ast::PropertyName::String(s) => {
                                tokens.push(Token::String(s.clone()));
                            }
                            crate::parser::ast::PropertyName::Number(n) => {
                                tokens.push(Token::Float(*n));
                            }
                            crate::parser::ast::PropertyName::Computed(e) => {
                                tokens.push(Token::LBracket);
                                tokens.extend(expr_to_tokens(e));
                                tokens.push(Token::RBracket);
                            }
                        }
                        tokens.push(Token::Colon);
                        tokens.extend(expr_to_tokens(value));
                    }
                    crate::parser::ast::ObjectProperty::Shorthand(name) => {
                        tokens.push(Token::Ident(name.clone()));
                    }
                    crate::parser::ast::ObjectProperty::Spread(e) => {
                        tokens.push(Token::Spread);
                        tokens.extend(expr_to_tokens(e));
                    }
                    crate::parser::ast::ObjectProperty::ComputedProperty { key, value } => {
                        tokens.push(Token::LBracket);
                        tokens.extend(expr_to_tokens(key));
                        tokens.push(Token::RBracket);
                        tokens.push(Token::Colon);
                        tokens.extend(expr_to_tokens(value));
                    }
                }
            }
            tokens.push(Token::RBrace);
            tokens
        }
        _ => vec![],
    }
}

fn binary_op_to_token(op: &crate::parser::ast::BinaryOp) -> Token {
    match op {
        crate::parser::ast::BinaryOp::StrictEquals => Token::StrictEquals,
        crate::parser::ast::BinaryOp::StrictNotEquals => Token::StrictNotEquals,
        crate::parser::ast::BinaryOp::Equals => Token::Equals,
        crate::parser::ast::BinaryOp::NotEquals => Token::NotEquals,
        crate::parser::ast::BinaryOp::Less => Token::Less,
        crate::parser::ast::BinaryOp::Greater => Token::Greater,
        crate::parser::ast::BinaryOp::LessEq => Token::LessEq,
        crate::parser::ast::BinaryOp::GreaterEq => Token::GreaterEq,
        crate::parser::ast::BinaryOp::In => Token::In,
        crate::parser::ast::BinaryOp::Instanceof => Token::Instanceof,
        crate::parser::ast::BinaryOp::Plus => Token::Plus,
        crate::parser::ast::BinaryOp::Minus => Token::Minus,
        crate::parser::ast::BinaryOp::Star => Token::Star,
        crate::parser::ast::BinaryOp::Slash => Token::Slash,
        crate::parser::ast::BinaryOp::Percent => Token::Percent,
        crate::parser::ast::BinaryOp::Power => Token::Power,
        crate::parser::ast::BinaryOp::Shl => Token::Shl,
        crate::parser::ast::BinaryOp::Shr => Token::Shr,
        crate::parser::ast::BinaryOp::UShr => Token::UShr,
        crate::parser::ast::BinaryOp::Amp => Token::Amp,
        crate::parser::ast::BinaryOp::Pipe => Token::Pipe,
        crate::parser::ast::BinaryOp::Caret => Token::Caret,
        crate::parser::ast::BinaryOp::And => Token::And,
        crate::parser::ast::BinaryOp::Or => Token::Or,
        crate::parser::ast::BinaryOp::Nullish => Token::Nullish,
    }
}

fn unary_op_to_token(op: &crate::parser::ast::UnaryOp) -> Token {
    match op {
        crate::parser::ast::UnaryOp::Plus => Token::Plus,
        crate::parser::ast::UnaryOp::Minus => Token::Minus,
        crate::parser::ast::UnaryOp::Not => Token::Not,
        crate::parser::ast::UnaryOp::Tilde => Token::Tilde,
        crate::parser::ast::UnaryOp::PreIncrement => Token::Increment,
        crate::parser::ast::UnaryOp::PreDecrement => Token::Decrement,
        crate::parser::ast::UnaryOp::Typeof => Token::Typeof,
        crate::parser::ast::UnaryOp::Void => Token::Void,
        crate::parser::ast::UnaryOp::Delete => Token::Delete,
        crate::parser::ast::UnaryOp::Await => Token::Await,
    }
}

fn apply_template(
    template: &[Token],
    captures: &std::collections::HashMap<String, crate::macro_expand::pattern::CapturedTokens>,
    _ctx: &StandardHygieneContext,
) -> Vec<Token> {
    let mut result = Vec::new();
    let mut i = 0;

    while i < template.len() {
        if template[i] == Token::Dollar {
            if let Some(Token::Ident(name)) = template.get(i + 1) {
                if let Some(cap) = captures.get(name) {
                    result.extend(cap.tokens.clone());
                    i += 2;
                    continue;
                }
            }
        }
        result.push(template[i].clone());
        i += 1;
    }

    result
}

fn tokens_to_source(tokens: &[Token]) -> String {
    tokens
        .iter()
        .map(|t| token_to_source(t))
        .collect::<Vec<_>>()
        .join(" ")
}

fn token_to_source(token: &Token) -> String {
    match token {
        Token::Int(i) => i.to_string(),
        Token::Float(f) => f.to_string(),
        Token::String(s) => format!("\"{}\"", s),
        Token::Ident(s) => s.clone(),
        Token::True => "true".to_string(),
        Token::False => "false".to_string(),
        Token::Null => "null".to_string(),
        Token::LParen => "(".to_string(),
        Token::RParen => ")".to_string(),
        Token::LBracket => "[".to_string(),
        Token::RBracket => "]".to_string(),
        Token::LBrace => "{".to_string(),
        Token::RBrace => "}".to_string(),
        Token::Comma => ",".to_string(),
        Token::SemiColon => ";".to_string(),
        Token::Colon => ":".to_string(),
        Token::Dot => ".".to_string(),
        Token::Assign => "=".to_string(),
        Token::Plus => "+".to_string(),
        Token::Minus => "-".to_string(),
        Token::Star => "*".to_string(),
        Token::Slash => "/".to_string(),
        Token::Percent => "%".to_string(),
        Token::StrictEquals => "===".to_string(),
        Token::StrictNotEquals => "!==".to_string(),
        Token::Equals => "==".to_string(),
        Token::NotEquals => "!=".to_string(),
        Token::Less => "<".to_string(),
        Token::Greater => ">".to_string(),
        Token::LessEq => "<=".to_string(),
        Token::GreaterEq => ">=".to_string(),
        Token::And => "&&".to_string(),
        Token::Or => "||".to_string(),
        Token::Not => "!".to_string(),
        Token::Question => "?".to_string(),
        Token::Spread => "...".to_string(),
        Token::FatArrow => "=>".to_string(),
        Token::DoubleColon => "::".to_string(),
        Token::Dollar => "$".to_string(),
        _ => format!("{:?}", token),
    }
}
