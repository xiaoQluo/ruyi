use crate::lexer::error::LexerError;
use crate::lexer::token::{Location, Token, TokenWithLocation};

pub struct Scanner {
    input: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    template_context: bool,
    pending_template_part: bool,
    pending_dq_string: bool,
    dq_in_interp: bool,
}

impl Scanner {
    pub fn new(source: &str) -> Self {
        Self {
            input: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            template_context: false,
            pending_template_part: false,
            pending_dq_string: false,
            dq_in_interp: false,
        }
    }

    pub fn next_token(&mut self) -> Result<TokenWithLocation, LexerError> {
        if self.pending_template_part {
            self.pending_template_part = false;
            return self.scan_template_part();
        }

        if self.pending_dq_string {
            return self.scan_dq_string_part();
        }

        self.skip_whitespace_and_comments()?;

        if self.is_at_end() {
            if self.template_context {
                return Err(LexerError::UnterminatedTemplate {
                    line: self.line,
                    col: self.col,
                });
            }
            return Ok(self.make_token(Token::Eof));
        }

        if (self.template_context || self.dq_in_interp) && self.current_char() == '}' {
            let start = self.current_location();
            self.advance();
            self.dq_in_interp = false;
            self.pending_dq_string = true;
            return Ok(TokenWithLocation {
                token: Token::TemplateExprEnd,
                start,
                end: self.current_location(),
            });
        }

        let start_loc = self.current_location();
        let ch = self.current_char();

        let token = match ch {
            '$' if self.peek_char(1) == '{' => {
                self.advance();
                self.advance();
                Token::TemplateExprStart
            }
            '$' => {
                self.advance();
                Token::Dollar
            }
            'a'..='z' | 'A'..='Z' | '_' => self.scan_ident_or_keyword(),
            '0'..='9' => self.scan_number()?,
            '"' => return self.scan_string('"'),
            '\'' => return self.scan_string('\''),
            '`' => {
                self.template_context = true;
                self.advance();
                return self.scan_template_part();
            }
            '+' => {
                self.advance();
                if self.match_char('=') {
                    Token::PlusAssign
                } else if self.match_char('+') {
                    Token::Increment
                } else {
                    Token::Plus
                }
            }
            '-' => {
                self.advance();
                if self.match_char('=') {
                    Token::MinusAssign
                } else if self.match_char('>') {
                    Token::FatArrow
                } else if self.match_char('-') {
                    Token::Decrement
                } else {
                    Token::Minus
                }
            }
            '*' => {
                self.advance();
                if self.match_char('=') {
                    Token::StarAssign
                } else if self.match_char('*') {
                    if self.match_char('=') {
                        Token::PowerAssign
                    } else {
                        Token::Power
                    }
                } else {
                    Token::Star
                }
            }
            '/' => {
                self.advance();
                if self.match_char('=') {
                    Token::SlashAssign
                } else {
                    Token::Slash
                }
            }
            '%' => {
                self.advance();
                if self.match_char('=') {
                    Token::PercentAssign
                } else {
                    Token::Percent
                }
            }
            '=' => {
                self.advance();
                if self.match_char('=') {
                    if self.match_char('=') {
                        Token::StrictEquals
                    } else {
                        Token::Equals
                    }
                } else if self.match_char('>') {
                    Token::FatArrow
                } else {
                    Token::Assign
                }
            }
            '!' => {
                self.advance();
                if self.match_char('=') {
                    if self.match_char('=') {
                        Token::StrictNotEquals
                    } else {
                        Token::NotEquals
                    }
                } else {
                    Token::Not
                }
            }
            '<' => {
                self.advance();
                if self.match_char('=') {
                    Token::LessEq
                } else if self.match_char('<') {
                    if self.match_char('=') {
                        Token::ShlAssign
                    } else {
                        Token::Shl
                    }
                } else {
                    Token::Less
                }
            }
            '>' => {
                self.advance();
                if self.match_char('=') {
                    Token::GreaterEq
                } else if self.match_char('>') {
                    if self.match_char('=') {
                        Token::ShrAssign
                    } else if self.match_char('>') {
                        if self.match_char('=') {
                            Token::UShrAssign
                        } else {
                            Token::UShr
                        }
                    } else {
                        Token::Shr
                    }
                } else {
                    Token::Greater
                }
            }
            '&' => {
                self.advance();
                if self.match_char('=') {
                    Token::AmpAssign
                } else if self.match_char('&') {
                    if self.match_char('=') {
                        Token::AndAssign
                    } else {
                        Token::And
                    }
                } else {
                    Token::Amp
                }
            }
            '|' => {
                self.advance();
                if self.match_char('=') {
                    Token::PipeAssign
                } else if self.match_char('|') {
                    if self.match_char('=') {
                        Token::OrAssign
                    } else {
                        Token::Or
                    }
                } else {
                    Token::Pipe
                }
            }
            '^' => {
                self.advance();
                if self.match_char('=') {
                    Token::CaretAssign
                } else {
                    Token::Caret
                }
            }
            '~' => {
                self.advance();
                Token::Tilde
            }
            '?' => {
                self.advance();
                if self.match_char('?') {
                    if self.match_char('=') {
                        Token::NullishAssign
                    } else {
                        Token::Nullish
                    }
                } else if self.match_char('.') {
                    Token::OptChain
                } else {
                    Token::Question
                }
            }
            '.' => {
                self.advance();
                if self.current_char() == '.' && self.peek_char(1) == '.' {
                    self.advance();
                    self.advance();
                    Token::Spread
                } else {
                    Token::Dot
                }
            }
            ':' => {
                self.advance();
                if self.match_char(':') {
                    Token::DoubleColon
                } else {
                    Token::Colon
                }
            }
            ';' => {
                self.advance();
                Token::SemiColon
            }
            ',' => {
                self.advance();
                Token::Comma
            }
            '(' => {
                self.advance();
                Token::LParen
            }
            ')' => {
                self.advance();
                Token::RParen
            }
            '{' => {
                self.advance();
                Token::LBrace
            }
            '}' => {
                self.advance();
                Token::RBrace
            }
            '[' => {
                self.advance();
                Token::LBracket
            }
            ']' => {
                self.advance();
                Token::RBracket
            }
            '@' => {
                self.advance();
                Token::At
            }
            '#' => {
                self.advance();
                Token::Hash
            }
            _ => {
                return Err(LexerError::InvalidCharacter {
                    ch,
                    line: self.line,
                    col: self.col,
                });
            }
        };

        let end_loc = self.current_location();
        Ok(TokenWithLocation {
            token,
            start: start_loc,
            end: end_loc,
        })
    }

    pub fn scan_all(&mut self) -> Result<Vec<TokenWithLocation>, LexerError> {
        let mut tokens = Vec::new();
        loop {
            let t = self.next_token()?;
            let is_eof = matches!(t.token, Token::Eof);
            tokens.push(t);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    fn current_char(&self) -> char {
        self.input.get(self.pos).copied().unwrap_or('\0')
    }

    fn peek_char(&self, offset: usize) -> char {
        self.input.get(self.pos + offset).copied().unwrap_or('\0')
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn advance(&mut self) -> char {
        let ch = self.current_char();
        if !self.is_at_end() {
            if ch == '\n' {
                self.line += 1;
                self.col = 1;
            } else if ch == '\r' {
                if self.peek_char(1) != '\n' {
                    self.line += 1;
                    self.col = 1;
                }
            } else {
                self.col += 1;
            }
            self.pos += 1;
        }
        ch
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.current_char() == expected {
            self.advance();
            true
        } else {
            false
        }
    }

    fn current_location(&self) -> Location {
        Location::new(self.line, self.col)
    }

    fn make_token(&self, token: Token) -> TokenWithLocation {
        let loc = self.current_location();
        TokenWithLocation {
            token,
            start: loc,
            end: loc,
        }
    }

    fn skip_whitespace_and_comments(&mut self) -> Result<(), LexerError> {
        while !self.is_at_end() {
            match self.current_char() {
                ' ' | '\t' | '\r' | '\n' | '\x0C' => {
                    self.advance();
                }
                '/' if self.peek_char(1) == '/' => {
                    self.skip_line_comment();
                }
                '/' if self.peek_char(1) == '*' => {
                    self.skip_block_comment()?;
                }
                _ => break,
            }
        }
        Ok(())
    }

    fn skip_line_comment(&mut self) {
        while !self.is_at_end() && self.current_char() != '\n' {
            self.advance();
        }
    }

    fn skip_block_comment(&mut self) -> Result<(), LexerError> {
        let start_line = self.line;
        let start_col = self.col;
        self.advance();
        self.advance();
        while !self.is_at_end() {
            if self.current_char() == '*' && self.peek_char(1) == '/' {
                self.advance();
                self.advance();
                return Ok(());
            }
            self.advance();
        }
        Err(LexerError::UnterminatedComment {
            line: start_line,
            col: start_col,
        })
    }

    fn scan_ident_or_keyword(&mut self) -> Token {
        let start = self.pos;
        self.advance();
        while !self.is_at_end() && self.is_ident_part(self.current_char()) {
            self.advance();
        }
        let ident: String = self.input[start..self.pos].iter().collect();
        Self::resolve_keyword(&ident)
    }

    fn is_ident_part(&self, ch: char) -> bool {
        matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_')
    }

    fn resolve_keyword(ident: &str) -> Token {
        match ident {
            "let" => Token::Let,
            "const" => Token::Const,
            "fn" => Token::Fn,
            "class" => Token::Class,
            "trait" => Token::Trait,
            "impl" => Token::Impl,
            "dyn" => Token::Dyn,
            "match" => Token::Match,
            "if" => Token::If,
            "else" => Token::Else,
            "for" => Token::For,
            "while" => Token::While,
            "return" => Token::Return,
            "throw" => Token::Throw,
            "try" => Token::Try,
            "catch" => Token::Catch,
            "finally" => Token::Finally,
            "async" => Token::Async,
            "await" => Token::Await,
            "import" => Token::Import,
            "export" => Token::Export,
            "extern" => Token::Extern,
            "macro" => Token::Macro,
            "type" => Token::Type,
            "true" => Token::True,
            "false" => Token::False,
            "null" => Token::Null,
            "self" => Token::SelfKw,
            "super" => Token::Super,
            "this" => Token::This,
            "in" => Token::In,
            "instanceof" => Token::Instanceof,
            "typeof" => Token::Typeof,
            "void" => Token::Void,
            "delete" => Token::Delete,
            "as" => Token::As,

            "extends" => Token::Extends,
            "static" => Token::Static,
            "get" => Token::Get,
            "set" => Token::Set,
            "new" => Token::New,
            "of" => Token::Of,
            "break" => Token::Break,
            "continue" => Token::Continue,
            "yield" => Token::Yield,
            "_" => Token::Underscore,
            _ => Token::Ident(ident.to_string()),
        }
    }

    fn scan_number(&mut self) -> Result<Token, LexerError> {
        let start = self.pos;
        let start_line = self.line;
        let start_col = self.col;

        if self.current_char() == '0' {
            let next = self.peek_char(1);
            match next {
                'x' | 'X' => {
                    self.advance();
                    self.advance();
                    return self.scan_hex_number(start_line, start_col);
                }
                'o' | 'O' => {
                    self.advance();
                    self.advance();
                    return self.scan_octal_number(start_line, start_col);
                }
                'b' | 'B' => {
                    self.advance();
                    self.advance();
                    return self.scan_binary_number(start_line, start_col);
                }
                _ => {}
            }
        }

        while !self.is_at_end() && self.current_char().is_ascii_digit() {
            self.advance();
        }

        let mut is_float = false;
        if self.current_char() == '.' && self.peek_char(1).is_ascii_digit() {
            is_float = true;
            self.advance();
            while !self.is_at_end() && self.current_char().is_ascii_digit() {
                self.advance();
            }
        }

        if self.current_char() == 'e' || self.current_char() == 'E' {
            is_float = true;
            self.advance();
            if self.current_char() == '+' || self.current_char() == '-' {
                self.advance();
            }
            if !self.current_char().is_ascii_digit() {
                return Err(LexerError::InvalidNumber {
                    line: start_line,
                    col: start_col,
                    msg: "expected digits after exponent".into(),
                });
            }
            while !self.is_at_end() && self.current_char().is_ascii_digit() {
                self.advance();
            }
        }

        let raw: String = self.input[start..self.pos].iter().collect();

        if self.current_char() == 'n' {
            self.advance();
            return Ok(Token::BigInt(raw));
        }

        if is_float {
            match raw.parse::<f64>() {
                Ok(f) => Ok(Token::Float(f)),
                Err(_) => Err(LexerError::InvalidNumber {
                    line: start_line,
                    col: start_col,
                    msg: format!("cannot parse float '{}'", raw),
                }),
            }
        } else {
            match raw.parse::<i64>() {
                Ok(i) => Ok(Token::Int(i)),
                Err(_) => Err(LexerError::InvalidNumber {
                    line: start_line,
                    col: start_col,
                    msg: format!("cannot parse integer '{}'", raw),
                }),
            }
        }
    }

    fn scan_hex_number(
        &mut self,
        start_line: usize,
        start_col: usize,
    ) -> Result<Token, LexerError> {
        let start = self.pos;
        if !self.current_char().is_ascii_hexdigit() {
            return Err(LexerError::InvalidNumber {
                line: start_line,
                col: start_col,
                msg: "expected hex digits after 0x".into(),
            });
        }
        while !self.is_at_end() && self.current_char().is_ascii_hexdigit() {
            self.advance();
        }
        let raw: String = self.input[start..self.pos].iter().collect();
        if self.current_char() == 'n' {
            self.advance();
            return Ok(Token::BigInt(format!("0x{}", raw)));
        }
        match i64::from_str_radix(&raw, 16) {
            Ok(i) => Ok(Token::Int(i)),
            Err(_) => Err(LexerError::InvalidNumber {
                line: start_line,
                col: start_col,
                msg: format!("cannot parse hex integer '{}'", raw),
            }),
        }
    }

    fn scan_octal_number(
        &mut self,
        start_line: usize,
        start_col: usize,
    ) -> Result<Token, LexerError> {
        let start = self.pos;
        if !matches!(self.current_char(), '0'..='7') {
            return Err(LexerError::InvalidNumber {
                line: start_line,
                col: start_col,
                msg: "expected octal digits after 0o".into(),
            });
        }
        while !self.is_at_end() && matches!(self.current_char(), '0'..='7') {
            self.advance();
        }
        let raw: String = self.input[start..self.pos].iter().collect();
        if self.current_char() == 'n' {
            self.advance();
            return Ok(Token::BigInt(format!("0o{}", raw)));
        }
        match i64::from_str_radix(&raw, 8) {
            Ok(i) => Ok(Token::Int(i)),
            Err(_) => Err(LexerError::InvalidNumber {
                line: start_line,
                col: start_col,
                msg: format!("cannot parse octal integer '{}'", raw),
            }),
        }
    }

    fn scan_binary_number(
        &mut self,
        start_line: usize,
        start_col: usize,
    ) -> Result<Token, LexerError> {
        let start = self.pos;
        if !matches!(self.current_char(), '0' | '1') {
            return Err(LexerError::InvalidNumber {
                line: start_line,
                col: start_col,
                msg: "expected binary digits after 0b".into(),
            });
        }
        while !self.is_at_end() && matches!(self.current_char(), '0' | '1') {
            self.advance();
        }
        let raw: String = self.input[start..self.pos].iter().collect();
        if self.current_char() == 'n' {
            self.advance();
            return Ok(Token::BigInt(format!("0b{}", raw)));
        }
        match i64::from_str_radix(&raw, 2) {
            Ok(i) => Ok(Token::Int(i)),
            Err(_) => Err(LexerError::InvalidNumber {
                line: start_line,
                col: start_col,
                msg: format!("cannot parse binary integer '{}'", raw),
            }),
        }
    }

    fn scan_string(&mut self, quote: char) -> Result<TokenWithLocation, LexerError> {
        let start_line = self.line;
        let start_col = self.col;
        self.advance();
        let mut result = String::new();

        while !self.is_at_end() && self.current_char() != quote {
            if self.current_char() == '$' && self.peek_char(1) == '{' {
                self.pending_dq_string = true;
                let loc = self.current_location();
                return Ok(TokenWithLocation {
                    token: Token::TemplateString(result),
                    start: loc,
                    end: loc,
                });
            }
            if self.current_char() == '\\' {
                let escaped = self.scan_escape_sequence(start_line, start_col)?;
                result.push_str(&escaped);
            } else if self.current_char() == '\n' {
                return Err(LexerError::UnterminatedString {
                    line: start_line,
                    col: start_col,
                });
            } else {
                result.push(self.advance());
            }
        }

        if self.is_at_end() {
            return Err(LexerError::UnterminatedString {
                line: start_line,
                col: start_col,
            });
        }

        self.advance();
        Ok(self.make_token(Token::String(result)))
    }

    fn scan_escape_sequence(
        &mut self,
        start_line: usize,
        start_col: usize,
    ) -> Result<String, LexerError> {
        self.advance();
        let ch = self.current_char();
        let escaped = match ch {
            'n' => "\n".to_string(),
            'r' => "\r".to_string(),
            't' => "\t".to_string(),
            'b' => "\x08".to_string(),
            'f' => "\x0C".to_string(),
            'v' => "\x0B".to_string(),
            '0' => "\0".to_string(),
            '"' => "\"".to_string(),
            '\'' => "\'".to_string(),
            '\\' => "\\".to_string(),
            'x' => {
                self.advance();
                let d1 = self.current_char();
                self.advance();
                let d2 = self.current_char();
                if !d1.is_ascii_hexdigit() || !d2.is_ascii_hexdigit() {
                    return Err(LexerError::InvalidEscape {
                        line: start_line,
                        col: start_col,
                    });
                }
                let hex: String = self.input[self.pos - 1..=self.pos].iter().collect();
                self.advance();
                match u8::from_str_radix(&hex, 16) {
                    Ok(b) => String::from_utf8_lossy(&[b]).to_string(),
                    Err(_) => {
                        return Err(LexerError::InvalidEscape {
                            line: start_line,
                            col: start_col,
                        })
                    }
                }
            }
            'u' => {
                self.advance();
                if self.current_char() == '{' {
                    self.advance();
                    let ustart = self.pos;
                    while !self.is_at_end() && self.current_char().is_ascii_hexdigit() {
                        self.advance();
                    }
                    let hex: String = self.input[ustart..self.pos].iter().collect();
                    if self.current_char() != '}' || hex.is_empty() {
                        return Err(LexerError::InvalidEscape {
                            line: start_line,
                            col: start_col,
                        });
                    }
                    self.advance();
                    match u32::from_str_radix(&hex, 16) {
                        Ok(cp) => match char::from_u32(cp) {
                            Some(c) => c.to_string(),
                            None => {
                                return Err(LexerError::InvalidEscape {
                                    line: start_line,
                                    col: start_col,
                                })
                            }
                        },
                        Err(_) => {
                            return Err(LexerError::InvalidEscape {
                                line: start_line,
                                col: start_col,
                            })
                        }
                    }
                } else {
                    let mut hex = String::new();
                    for _ in 0..4 {
                        let c = self.current_char();
                        if !c.is_ascii_hexdigit() {
                            return Err(LexerError::InvalidEscape {
                                line: start_line,
                                col: start_col,
                            });
                        }
                        hex.push(c);
                        self.advance();
                    }
                    match u32::from_str_radix(&hex, 16) {
                        Ok(cp) => match char::from_u32(cp) {
                            Some(c) => c.to_string(),
                            None => {
                                return Err(LexerError::InvalidEscape {
                                    line: start_line,
                                    col: start_col,
                                })
                            }
                        },
                        Err(_) => {
                            return Err(LexerError::InvalidEscape {
                                line: start_line,
                                col: start_col,
                            })
                        }
                    }
                }
            }
            _ => {
                return Err(LexerError::InvalidEscape {
                    line: start_line,
                    col: start_col,
                })
            }
        };
        if !matches!(ch, 'x' | 'u') {
            self.advance();
        }
        Ok(escaped)
    }

    fn scan_template_part(&mut self) -> Result<TokenWithLocation, LexerError> {
        let start_line = self.line;
        let start_col = self.col;
        let mut result = String::new();

        while !self.is_at_end() {
            if self.current_char() == '`' {
                self.advance();
                self.template_context = false;
                self.pending_template_part = false;
                return Ok(TokenWithLocation {
                    token: Token::TemplateString(result),
                    start: Location::new(start_line, start_col),
                    end: self.current_location(),
                });
            }
            if self.current_char() == '$' && self.peek_char(1) == '{' {
                return Ok(TokenWithLocation {
                    token: Token::TemplateString(result),
                    start: Location::new(start_line, start_col),
                    end: self.current_location(),
                });
            }
            if self.current_char() == '\\' {
                let escaped = self.scan_escape_sequence(start_line, start_col)?;
                result.push_str(&escaped);
            } else if self.current_char() == '\n' {
                result.push('\n');
                self.advance();
            } else {
                result.push(self.advance());
            }
        }

        Err(LexerError::UnterminatedTemplate {
            line: start_line,
            col: start_col,
        })
    }

    /// Scan content following a `${...}` interpolation in a double-quoted
    /// string. Returns a `TemplateExprStart` (another interpolation), or a
    /// `TemplateString` (more content / end of string).
    fn scan_dq_string_part(&mut self) -> Result<TokenWithLocation, LexerError> {
        if self.current_char() == '"' {
            self.advance();
            self.pending_dq_string = false;
            return Ok(self.make_token(Token::TemplateString(String::new())));
        }

        if self.current_char() == '$' && self.peek_char(1) == '{' {
            self.advance();
            self.advance();
            self.dq_in_interp = true;
            self.pending_dq_string = false;
            return Ok(self.make_token(Token::TemplateExprStart));
        }

        let start_line = self.line;
        let start_col = self.col;
        let mut result = String::new();
        while !self.is_at_end() && self.current_char() != '"' {
            if self.current_char() == '$' && self.peek_char(1) == '{' {
                return Ok(self.make_token(Token::TemplateString(result)));
            }
            if self.current_char() == '\\' {
                let escaped = self.scan_escape_sequence(start_line, start_col)?;
                result.push_str(&escaped);
            } else {
                result.push(self.advance());
            }
        }

        if self.is_at_end() {
            return Err(LexerError::UnterminatedString {
                line: start_line,
                col: start_col,
            });
        }

        self.advance();
        self.pending_dq_string = false;
        Ok(self.make_token(Token::TemplateString(result)))
    }
}
