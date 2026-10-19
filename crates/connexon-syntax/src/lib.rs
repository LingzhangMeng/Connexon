//! Connexon syntax: lexer, AST and a hand-written Pratt-style parser.
//!
//! Grammar (Phase 0):
//! ```text
//! program    := (separator* statement)* separator*
//! statement  := 'let' IDENT '=' expr
//!             | 'fn' IDENT '(' params? ')' '=' expr
//!             | expr
//! expr       := pipe
//! pipe       := or ('|>' postfix)*
//! or         := and (('||' | 'or') and)*
//! and        := comparison (('&&' | 'and') comparison)*
//! comparison := additive (('==' | '!=' | '<' | '<=' | '>' | '>=') additive)*
//! additive   := multiplicative (('+' | '-') multiplicative)*
//! mult       := unary (('*' | '/' | '%') unary)*
//! unary      := ('-' | '!' | 'not') unary | postfix
//! postfix    := primary (call | '.' IDENT)*
//! primary    := NUMBER | STRING | 'true' | 'false' | 'null'
//!             | IDENT ('::' IDENT)? call?
//!             | '(' expr ')' | '[' items? ']' | '{' fields? '}'
//!             | 'if' expr 'then' expr 'else' expr
//! ```

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Ident(String),
    Number(f64),
    Str(String),
    Let,
    Fn,
    If,
    Then,
    Else,
    True,
    False,
    Null,
    Pipe,
    ColonColon,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Dot,
    Colon,
    Semicolon,
    Newline,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Assign,
    EqEq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
    Not,
    Eof,
}

impl TokenKind {
    pub fn describe(&self) -> String {
        match self {
            TokenKind::Ident(name) => format!("identifier `{name}`"),
            TokenKind::Number(value) => format!("number `{value}`"),
            TokenKind::Str(value) => format!("string `{value}`"),
            TokenKind::Newline => "newline".to_string(),
            TokenKind::Eof => "end of input".to_string(),
            other => format!("`{}`", other.symbol()),
        }
    }

    fn symbol(&self) -> &'static str {
        match self {
            TokenKind::Let => "let",
            TokenKind::Fn => "fn",
            TokenKind::If => "if",
            TokenKind::Then => "then",
            TokenKind::Else => "else",
            TokenKind::True => "true",
            TokenKind::False => "false",
            TokenKind::Null => "null",
            TokenKind::Pipe => "|>",
            TokenKind::ColonColon => "::",
            TokenKind::LParen => "(",
            TokenKind::RParen => ")",
            TokenKind::LBracket => "[",
            TokenKind::RBracket => "]",
            TokenKind::LBrace => "{",
            TokenKind::RBrace => "}",
            TokenKind::Comma => ",",
            TokenKind::Dot => ".",
            TokenKind::Colon => ":",
            TokenKind::Semicolon => ";",
            TokenKind::Newline => "\\n",
            TokenKind::Plus => "+",
            TokenKind::Minus => "-",
            TokenKind::Star => "*",
            TokenKind::Slash => "/",
            TokenKind::Percent => "%",
            TokenKind::Assign => "=",
            TokenKind::EqEq => "==",
            TokenKind::NotEq => "!=",
            TokenKind::Lt => "<",
            TokenKind::LtEq => "<=",
            TokenKind::Gt => ">",
            TokenKind::GtEq => ">=",
            TokenKind::And => "&&",
            TokenKind::Or => "||",
            TokenKind::Not => "!",
            TokenKind::Ident(_) | TokenKind::Number(_) | TokenKind::Str(_) | TokenKind::Eof => "",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum SyntaxError {
    #[error("{message} at line {line}, column {column}")]
    Unexpected {
        message: String,
        line: usize,
        column: usize,
    },
    #[error("unexpected end of input")]
    Eof,
}

fn unexpected(message: impl Into<String>, line: usize, column: usize) -> SyntaxError {
    SyntaxError::Unexpected {
        message: message.into(),
        line,
        column,
    }
}

pub fn lex(source: &str) -> Result<Vec<Token>, SyntaxError> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut index = 0usize;
    let mut line = 1usize;
    let mut column = 1usize;

    while index < chars.len() {
        let ch = chars[index];
        match ch {
            ' ' | '\t' | '\r' => {
                index += 1;
                column += 1;
            }
            '\n' => {
                tokens.push(Token {
                    kind: TokenKind::Newline,
                    line,
                    column,
                });
                index += 1;
                line += 1;
                column = 1;
            }
            '#' => {
                let start = index;
                while index < chars.len() && chars[index] != '\n' {
                    index += 1;
                }
                column += index - start;
            }
            '"' => {
                let start_line = line;
                let start_column = column;
                let start = index;
                index += 1;
                let mut value = String::new();
                let mut closed = false;
                while index < chars.len() {
                    let current = chars[index];
                    if current == '"' {
                        index += 1;
                        closed = true;
                        break;
                    }
                    if current == '\n' {
                        return Err(unexpected("unterminated string", line, column));
                    }
                    if current == '\\' {
                        index += 1;
                        let escaped = chars.get(index).copied().ok_or_else(|| {
                            unexpected("unterminated escape sequence", line, column)
                        })?;
                        match escaped {
                            'n' => value.push('\n'),
                            't' => value.push('\t'),
                            'r' => value.push('\r'),
                            '\\' => value.push('\\'),
                            '"' => value.push('"'),
                            other => {
                                return Err(unexpected(
                                    format!("unknown escape `\\{other}`"),
                                    line,
                                    column,
                                ))
                            }
                        }
                        index += 1;
                    } else {
                        value.push(current);
                        index += 1;
                    }
                }
                if !closed {
                    return Err(unexpected("unterminated string", start_line, start_column));
                }
                column += index - start;
                tokens.push(Token {
                    kind: TokenKind::Str(value),
                    line: start_line,
                    column: start_column,
                });
            }
            '0'..='9' => {
                let start = index;
                let start_column = column;
                while index < chars.len() && chars[index].is_ascii_digit() {
                    index += 1;
                }
                if index + 1 < chars.len()
                    && chars[index] == '.'
                    && chars[index + 1].is_ascii_digit()
                {
                    index += 1;
                    while index < chars.len() && chars[index].is_ascii_digit() {
                        index += 1;
                    }
                }
                if index < chars.len() && (chars[index] == 'e' || chars[index] == 'E') {
                    let mut look = index + 1;
                    if look < chars.len() && (chars[look] == '+' || chars[look] == '-') {
                        look += 1;
                    }
                    if look < chars.len() && chars[look].is_ascii_digit() {
                        index = look;
                        while index < chars.len() && chars[index].is_ascii_digit() {
                            index += 1;
                        }
                    }
                }
                let text: String = chars[start..index].iter().collect();
                let value = text.parse::<f64>().map_err(|_| {
                    unexpected(format!("invalid number `{text}`"), line, start_column)
                })?;
                column += index - start;
                tokens.push(Token {
                    kind: TokenKind::Number(value),
                    line,
                    column: start_column,
                });
            }
            'a'..='z' | 'A'..='Z' | '_' => {
                let start = index;
                let start_column = column;
                index += 1;
                while index < chars.len()
                    && (chars[index].is_ascii_alphanumeric() || chars[index] == '_')
                {
                    index += 1;
                }
                let text: String = chars[start..index].iter().collect();
                column += index - start;
                tokens.push(Token {
                    kind: keyword(&text),
                    line,
                    column: start_column,
                });
            }
            _ => {
                let start_column = column;
                let next = chars.get(index + 1).copied();
                let (kind, consumed) = match ch {
                    '(' => (TokenKind::LParen, 1),
                    ')' => (TokenKind::RParen, 1),
                    '[' => (TokenKind::LBracket, 1),
                    ']' => (TokenKind::RBracket, 1),
                    '{' => (TokenKind::LBrace, 1),
                    '}' => (TokenKind::RBrace, 1),
                    ',' => (TokenKind::Comma, 1),
                    '.' => (TokenKind::Dot, 1),
                    ';' => (TokenKind::Semicolon, 1),
                    '+' => (TokenKind::Plus, 1),
                    '-' => (TokenKind::Minus, 1),
                    '*' => (TokenKind::Star, 1),
                    '/' => (TokenKind::Slash, 1),
                    '%' => (TokenKind::Percent, 1),
                    ':' => match next {
                        Some(':') => (TokenKind::ColonColon, 2),
                        _ => (TokenKind::Colon, 1),
                    },
                    '=' => match next {
                        Some('=') => (TokenKind::EqEq, 2),
                        _ => (TokenKind::Assign, 1),
                    },
                    '!' => match next {
                        Some('=') => (TokenKind::NotEq, 2),
                        _ => (TokenKind::Not, 1),
                    },
                    '<' => match next {
                        Some('=') => (TokenKind::LtEq, 2),
                        _ => (TokenKind::Lt, 1),
                    },
                    '>' => match next {
                        Some('=') => (TokenKind::GtEq, 2),
                        _ => (TokenKind::Gt, 1),
                    },
                    '&' => match next {
                        Some('&') => (TokenKind::And, 2),
                        _ => {
                            return Err(unexpected("expected `&&`", line, start_column));
                        }
                    },
                    '|' => match next {
                        Some('|') => (TokenKind::Or, 2),
                        Some('>') => (TokenKind::Pipe, 2),
                        _ => {
                            return Err(unexpected("expected `||` or `|>`", line, start_column));
                        }
                    },
                    other => {
                        return Err(unexpected(
                            format!("unexpected character `{other}`"),
                            line,
                            start_column,
                        ))
                    }
                };
                index += consumed;
                column += consumed;
                tokens.push(Token {
                    kind,
                    line,
                    column: start_column,
                });
            }
        }
    }

    tokens.push(Token {
        kind: TokenKind::Eof,
        line,
        column,
    });
    Ok(tokens)
}

fn keyword(text: &str) -> TokenKind {
    match text {
        "let" => TokenKind::Let,
        "fn" => TokenKind::Fn,
        "if" => TokenKind::If,
        "then" => TokenKind::Then,
        "else" => TokenKind::Else,
        "true" => TokenKind::True,
        "false" => TokenKind::False,
        "null" => TokenKind::Null,
        "and" => TokenKind::And,
        "or" => TokenKind::Or,
        "not" => TokenKind::Not,
        _ => TokenKind::Ident(text.to_string()),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Ident(String),
    Array(Vec<Expr>),
    Record(Vec<(String, Expr)>),
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        namespace: Option<String>,
        callee: Box<Expr>,
        args: Vec<Arg>,
    },
    Field {
        base: Box<Expr>,
        name: String,
    },
    Pipe {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    If {
        cond: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Positional(Expr),
    Named(String, Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let {
        name: String,
        value: Expr,
    },
    FnDef {
        name: String,
        params: Vec<String>,
        body: Expr,
    },
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Program {
    pub statements: Vec<Stmt>,
}

pub fn parse(source: &str) -> Result<Program, SyntaxError> {
    let tokens = lex(source)?;
    Parser::new(tokens).parse_program()
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    fn peek(&self) -> &TokenKind {
        &self.tokens[self.position].kind
    }

    fn peek_token(&self) -> &Token {
        &self.tokens[self.position]
    }

    fn peek_at(&self, offset: usize) -> TokenKind {
        self.tokens
            .get(self.position + offset)
            .map(|token| token.kind.clone())
            .unwrap_or(TokenKind::Eof)
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens[self.position].clone();
        if self.position + 1 < self.tokens.len() {
            self.position += 1;
        }
        token
    }

    fn check(&self, kind: &TokenKind) -> bool {
        self.peek() == kind
    }

    fn eat(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: &TokenKind, what: &str) -> Result<Token, SyntaxError> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            let token = self.peek_token();
            Err(unexpected(
                format!("expected {what}, found {}", token.kind.describe()),
                token.line,
                token.column,
            ))
        }
    }

    fn expect_ident(&mut self) -> Result<String, SyntaxError> {
        let token = self.peek_token().clone();
        match token.kind {
            TokenKind::Ident(name) => {
                self.advance();
                Ok(name)
            }
            other => Err(unexpected(
                format!("expected identifier, found {}", other.describe()),
                token.line,
                token.column,
            )),
        }
    }

    fn skip_separators(&mut self) {
        while matches!(self.peek(), TokenKind::Newline | TokenKind::Semicolon) {
            self.advance();
        }
    }

    fn parse_program(mut self) -> Result<Program, SyntaxError> {
        let mut statements = Vec::new();
        self.skip_separators();
        while !matches!(self.peek(), TokenKind::Eof) {
            statements.push(self.parse_statement()?);
            if !matches!(
                self.peek(),
                TokenKind::Eof | TokenKind::Newline | TokenKind::Semicolon
            ) {
                let token = self.peek_token();
                return Err(unexpected(
                    format!(
                        "expected newline or `;` after statement, found {}",
                        token.kind.describe()
                    ),
                    token.line,
                    token.column,
                ));
            }
            self.skip_separators();
        }
        Ok(Program { statements })
    }

    fn parse_statement(&mut self) -> Result<Stmt, SyntaxError> {
        match self.peek() {
            TokenKind::Let => {
                self.advance();
                let name = self.expect_ident()?;
                self.expect(&TokenKind::Assign, "`=`")?;
                let value = self.parse_expr()?;
                Ok(Stmt::Let { name, value })
            }
            TokenKind::Fn => {
                self.advance();
                let name = self.expect_ident()?;
                self.expect(&TokenKind::LParen, "`(`")?;
                let mut params = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    loop {
                        params.push(self.expect_ident()?);
                        if self.eat(&TokenKind::Comma) {
                            continue;
                        }
                        break;
                    }
                }
                self.expect(&TokenKind::RParen, "`)`")?;
                self.expect(&TokenKind::Assign, "`=`")?;
                let body = self.parse_expr()?;
                Ok(Stmt::FnDef { name, params, body })
            }
            _ => Ok(Stmt::Expr(self.parse_expr()?)),
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, SyntaxError> {
        self.parse_pipe()
    }

    fn parse_pipe(&mut self) -> Result<Expr, SyntaxError> {
        let mut expression = self.parse_or()?;
        while self.eat(&TokenKind::Pipe) {
            let right = self.parse_postfix()?;
            expression = Expr::Pipe {
                left: Box::new(expression),
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn parse_or(&mut self) -> Result<Expr, SyntaxError> {
        let mut expression = self.parse_and()?;
        while matches!(self.peek(), TokenKind::Or) {
            self.advance();
            let right = self.parse_and()?;
            expression = Expr::Binary {
                op: BinaryOp::Or,
                left: Box::new(expression),
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn parse_and(&mut self) -> Result<Expr, SyntaxError> {
        let mut expression = self.parse_comparison()?;
        while matches!(self.peek(), TokenKind::And) {
            self.advance();
            let right = self.parse_comparison()?;
            expression = Expr::Binary {
                op: BinaryOp::And,
                left: Box::new(expression),
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn parse_comparison(&mut self) -> Result<Expr, SyntaxError> {
        let mut expression = self.parse_additive()?;
        loop {
            let op = match self.peek() {
                TokenKind::EqEq => BinaryOp::Eq,
                TokenKind::NotEq => BinaryOp::NotEq,
                TokenKind::Lt => BinaryOp::Lt,
                TokenKind::LtEq => BinaryOp::LtEq,
                TokenKind::Gt => BinaryOp::Gt,
                TokenKind::GtEq => BinaryOp::GtEq,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            expression = Expr::Binary {
                op,
                left: Box::new(expression),
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn parse_additive(&mut self) -> Result<Expr, SyntaxError> {
        let mut expression = self.parse_multiplicative()?;
        loop {
            let op = match self.peek() {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            expression = Expr::Binary {
                op,
                left: Box::new(expression),
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, SyntaxError> {
        let mut expression = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                TokenKind::Percent => BinaryOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            expression = Expr::Binary {
                op,
                left: Box::new(expression),
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn parse_unary(&mut self) -> Result<Expr, SyntaxError> {
        match self.peek() {
            TokenKind::Minus => {
                self.advance();
                Ok(Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(self.parse_unary()?),
                })
            }
            TokenKind::Not => {
                self.advance();
                Ok(Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(self.parse_unary()?),
                })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, SyntaxError> {
        let mut expression = self.parse_primary()?;
        loop {
            if self.eat(&TokenKind::LParen) {
                let args = self.parse_arguments()?;
                expression = Expr::Call {
                    namespace: None,
                    callee: Box::new(expression),
                    args,
                };
            } else if self.eat(&TokenKind::Dot) {
                let name = self.expect_ident()?;
                expression = Expr::Field {
                    base: Box::new(expression),
                    name,
                };
            } else {
                break;
            }
        }
        Ok(expression)
    }

    fn parse_arguments(&mut self) -> Result<Vec<Arg>, SyntaxError> {
        let mut args = Vec::new();
        if self.eat(&TokenKind::RParen) {
            return Ok(args);
        }
        loop {
            if let TokenKind::Ident(name) = self.peek().clone() {
                if matches!(self.peek_at(1), TokenKind::Colon) {
                    self.advance();
                    self.advance();
                    let value = self.parse_expr()?;
                    args.push(Arg::Named(name, value));
                } else {
                    args.push(Arg::Positional(self.parse_expr()?));
                }
            } else {
                args.push(Arg::Positional(self.parse_expr()?));
            }
            if self.eat(&TokenKind::Comma) {
                if self.check(&TokenKind::RParen) {
                    break;
                }
                continue;
            }
            break;
        }
        self.expect(&TokenKind::RParen, "`)`")?;
        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expr, SyntaxError> {
        let token = self.peek_token().clone();
        match token.kind {
            TokenKind::Number(value) => {
                self.advance();
                Ok(Expr::Number(value))
            }
            TokenKind::Str(value) => {
                self.advance();
                Ok(Expr::String(value))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::Bool(true))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::Bool(false))
            }
            TokenKind::Null => {
                self.advance();
                Ok(Expr::Null)
            }
            TokenKind::If => {
                self.advance();
                let cond = self.parse_expr()?;
                self.expect(&TokenKind::Then, "`then`")?;
                let then_branch = self.parse_expr()?;
                self.expect(&TokenKind::Else, "`else`")?;
                let else_branch = self.parse_expr()?;
                Ok(Expr::If {
                    cond: Box::new(cond),
                    then_branch: Box::new(then_branch),
                    else_branch: Box::new(else_branch),
                })
            }
            TokenKind::LParen => {
                self.advance();
                let expression = self.parse_expr()?;
                self.expect(&TokenKind::RParen, "`)`")?;
                Ok(expression)
            }
            TokenKind::LBracket => {
                self.advance();
                let mut items = Vec::new();
                if !self.check(&TokenKind::RBracket) {
                    loop {
                        items.push(self.parse_expr()?);
                        if self.eat(&TokenKind::Comma) {
                            if self.check(&TokenKind::RBracket) {
                                break;
                            }
                            continue;
                        }
                        break;
                    }
                }
                self.expect(&TokenKind::RBracket, "`]`")?;
                Ok(Expr::Array(items))
            }
            TokenKind::LBrace => {
                self.advance();
                let mut fields = Vec::new();
                if !self.check(&TokenKind::RBrace) {
                    loop {
                        let key = match self.peek().clone() {
                            TokenKind::Ident(name) => {
                                self.advance();
                                name
                            }
                            TokenKind::Str(name) => {
                                self.advance();
                                name
                            }
                            other => {
                                let token = self.peek_token();
                                return Err(unexpected(
                                    format!("expected record key, found {}", other.describe()),
                                    token.line,
                                    token.column,
                                ));
                            }
                        };
                        self.expect(&TokenKind::Colon, "`:`")?;
                        let value = self.parse_expr()?;
                        fields.push((key, value));
                        if self.eat(&TokenKind::Comma) {
                            if self.check(&TokenKind::RBrace) {
                                break;
                            }
                            continue;
                        }
                        break;
                    }
                }
                self.expect(&TokenKind::RBrace, "`}`")?;
                Ok(Expr::Record(fields))
            }
            TokenKind::Ident(name) => {
                self.advance();
                if self.eat(&TokenKind::ColonColon) {
                    let function = self.expect_ident()?;
                    self.expect(&TokenKind::LParen, "`(`")?;
                    let args = self.parse_arguments()?;
                    Ok(Expr::Call {
                        namespace: Some(name),
                        callee: Box::new(Expr::Ident(function)),
                        args,
                    })
                } else if self.eat(&TokenKind::LParen) {
                    let args = self.parse_arguments()?;
                    Ok(Expr::Call {
                        namespace: None,
                        callee: Box::new(Expr::Ident(name)),
                        args,
                    })
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            other => Err(unexpected(
                format!("expected expression, found {}", other.describe()),
                token.line,
                token.column,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_arithmetic_and_comments() {
        let tokens = lex("1 + 2 * 3 # comment\n").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Number(1.0)));
        assert!(matches!(tokens[1].kind, TokenKind::Plus));
        assert!(matches!(tokens[2].kind, TokenKind::Number(2.0)));
        assert!(matches!(tokens[3].kind, TokenKind::Star));
        assert!(matches!(tokens[4].kind, TokenKind::Number(3.0)));
        assert!(matches!(tokens[5].kind, TokenKind::Newline));
        assert!(matches!(tokens[6].kind, TokenKind::Eof));
    }

    #[test]
    fn parses_precedence() {
        let program = parse("1 + 2 * 3").unwrap();
        match &program.statements[0] {
            Stmt::Expr(Expr::Binary { op, right, .. }) => {
                assert_eq!(*op, BinaryOp::Add);
                assert!(matches!(
                    **right,
                    Expr::Binary {
                        op: BinaryOp::Mul,
                        ..
                    }
                ));
            }
            other => panic!("unexpected AST: {other:?}"),
        }
    }

    #[test]
    fn parses_pipe_chain() {
        let program = parse("let x = [1, 2, 3] |> length()").unwrap();
        match &program.statements[0] {
            Stmt::Let { value, .. } => match value {
                Expr::Pipe { left, right } => {
                    assert!(matches!(**left, Expr::Array(_)));
                    assert!(matches!(**right, Expr::Call { .. }));
                }
                other => panic!("expected pipe, got {other:?}"),
            },
            other => panic!("expected let, got {other:?}"),
        }
    }

    #[test]
    fn parses_namespaced_call() {
        let program = parse("r::call(\"mean\", [1, 2, 3])").unwrap();
        match &program.statements[0] {
            Stmt::Expr(Expr::Call {
                namespace, args, ..
            }) => {
                assert_eq!(namespace.as_deref(), Some("r"));
                assert_eq!(args.len(), 2);
            }
            other => panic!("expected call, got {other:?}"),
        }
    }

    #[test]
    fn parses_function_definition() {
        let program = parse("fn add(a, b) = a + b").unwrap();
        match &program.statements[0] {
            Stmt::FnDef { name, params, .. } => {
                assert_eq!(name, "add");
                assert_eq!(params, &vec!["a".to_string(), "b".to_string()]);
            }
            other => panic!("expected fn, got {other:?}"),
        }
    }

    #[test]
    fn parses_records_and_fields() {
        let program = parse("let r = {gene: \"G1\", padj: 0.01}\nlet x = r.padj").unwrap();
        assert_eq!(program.statements.len(), 2);
        match &program.statements[1] {
            Stmt::Let {
                value: Expr::Field { name, .. },
                ..
            } => assert_eq!(name, "padj"),
            other => panic!("expected field access, got {other:?}"),
        }
    }

    #[test]
    fn reports_unexpected_character() {
        let error = lex("1 @ 2").unwrap_err();
        assert!(format!("{error}").contains("unexpected character"));
    }
}
