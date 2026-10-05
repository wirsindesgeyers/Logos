//! Lexer para a sintaxe de superfície AIR (air-text/v1).

use crate::error::{ErrorKind, SurfaceError};
use logos_canon::{CanonError, Text};

/// Coordenadas 1-based de início de um token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Linha 1-based.
    pub line: usize,
    /// Coluna 1-based (em Unicode scalar values).
    pub column: usize,
}

/// Tokens léxicos da gramática air-text/v1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token<'src> {
    // Palavras reservadas
    /// Palavra-chave `vocab`.
    Vocab,
    /// Palavra-chave `sort`.
    Sort,
    /// Palavra-chave `enum`.
    Enum,
    /// Palavra-chave `from`.
    From,
    /// Palavra-chave `fact`.
    Fact,
    /// Palavra-chave `policy`.
    Policy,
    /// Palavra-chave `const`.
    Const,
    /// Palavra-chave `func`.
    Func,
    /// Palavra-chave `pred`.
    Pred,
    /// Palavra-chave `decl`.
    Decl,
    /// Palavra-chave `gloss`.
    Gloss,
    /// Palavra-chave `glossed`.
    Glossed,
    /// Palavra-chave `anchored`.
    Anchored,
    /// Palavra-chave `defined`.
    Defined,
    /// Palavra-chave `claim`.
    Claim,
    /// Palavra-chave `cites`.
    Cites,
    /// Palavra-chave `true`.
    True,
    /// Palavra-chave `false`.
    False,
    /// Palavra-chave `eq`.
    Eq,
    /// Palavra-chave `not`.
    Not,
    /// Palavra-chave `and`.
    And,
    /// Palavra-chave `or`.
    Or,
    /// Palavra-chave `implies`.
    Implies,
    /// Palavra-chave `iff`.
    Iff,
    /// Palavra-chave `forall`.
    Forall,
    /// Palavra-chave `exists`.
    Exists,
    /// Palavra-chave `var`.
    Var,
    /// Palavra-chave `app`.
    App,
    /// Palavra-chave `symbol`.
    Symbol,
    /// Palavra-chave `bool`.
    Bool,

    // Pontuação
    /// Delimitador `{`.
    OpenBrace,
    /// Delimitador `}`.
    CloseBrace,
    /// Delimitador `(`.
    OpenParen,
    /// Delimitador `)`.
    CloseParen,
    /// Delimitador `[`.
    OpenBracket,
    /// Delimitador `]`.
    CloseBracket,
    /// Pontuação `,`.
    Comma,
    /// Pontuação `:`.
    Colon,
    /// Pontuação `.`.
    Dot,
    /// Pontuação `;`.
    Semi,
    /// Pontuação `=`.
    Equals,
    /// Operador `->`.
    Arrow,
    /// Operador `<->`.
    DoubleArrow,

    // Literais e identificadores
    /// Inteiro unsigned decimal `u64`.
    Integer(u64),
    /// Identificador sem aspas.
    Ident(&'src str),
    /// String JSON validada como texto canônico.
    String(Text),
    /// Fim de arquivo.
    Eof,
}

/// Token acompanhado de suas coordenadas de origem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpannedToken<'src> {
    /// Token reconhecido.
    pub token: Token<'src>,
    /// Coordenadas de início do token.
    pub span: Span,
}

/// Tokeniza o texto de entrada em uma lista completa de tokens com coordenadas.
///
/// # Errors
/// Retorna [`SurfaceError`] para caracteres não reconhecidos, literais malformados,
/// strings com surrogates isolados/não-NFC ou inteiros que excedam `u64::MAX`.
pub fn lex(input: &str) -> Result<Vec<SpannedToken<'_>>, SurfaceError> {
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();

    loop {
        let spanned = lexer.next_token()?;
        let is_eof = spanned.token == Token::Eof;
        tokens.push(spanned);
        if is_eof {
            break;
        }
    }

    Ok(tokens)
}

struct Lexer<'a> {
    input: &'a str,
    byte_pos: usize,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            byte_pos: 0,
            line: 1,
            column: 1,
        }
    }

    fn current_span(&self) -> Span {
        Span {
            line: self.line,
            column: self.column,
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.byte_pos..].chars().next()
    }

    fn peek2_chars(&self) -> (Option<char>, Option<char>) {
        let mut it = self.input[self.byte_pos..].chars();
        (it.next(), it.next())
    }

    fn peek3_chars(&self) -> (Option<char>, Option<char>, Option<char>) {
        let mut it = self.input[self.byte_pos..].chars();
        (it.next(), it.next(), it.next())
    }

    fn next_char(&mut self) -> Option<char> {
        let c = self.peek_char()?;
        self.byte_pos += c.len_utf8();
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(c)
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek_char() {
                Some(c) if c.is_whitespace() => {
                    self.next_char();
                }
                Some('#') => {
                    self.next_char();
                    while let Some(c) = self.peek_char() {
                        if c == '\n' {
                            break;
                        }
                        self.next_char();
                    }
                }
                _ => break,
            }
        }
    }

    fn next_token(&mut self) -> Result<SpannedToken<'a>, SurfaceError> {
        self.skip_whitespace_and_comments();
        let span = self.current_span();

        let c = match self.peek_char() {
            None => {
                return Ok(SpannedToken {
                    token: Token::Eof,
                    span,
                });
            }
            Some(c) => c,
        };

        // Números inteiros unsigned
        if c.is_ascii_digit() {
            let num = self.read_decimal_u64(span, "integer literal")?;
            return Ok(SpannedToken {
                token: Token::Integer(num),
                span,
            });
        }

        // Strings JSON
        if c == '"' {
            let text = self.read_string(span)?;
            return Ok(SpannedToken {
                token: Token::String(text),
                span,
            });
        }

        // Identificadores e palavras-chave
        if c == '_' || c.is_ascii_alphabetic() {
            let ident = self.read_ident();
            let token = match ident {
                "vocab" => Token::Vocab,
                "sort" => Token::Sort,
                "enum" => Token::Enum,
                "from" => Token::From,
                "fact" => Token::Fact,
                "policy" => Token::Policy,
                "const" => Token::Const,
                "func" => Token::Func,
                "pred" => Token::Pred,
                "decl" => Token::Decl,
                "gloss" => Token::Gloss,
                "glossed" => Token::Glossed,
                "anchored" => Token::Anchored,
                "defined" => Token::Defined,
                "claim" => Token::Claim,
                "cites" => Token::Cites,
                "true" => Token::True,
                "false" => Token::False,
                "eq" => Token::Eq,
                "not" => Token::Not,
                "and" => Token::And,
                "or" => Token::Or,
                "implies" => Token::Implies,
                "iff" => Token::Iff,
                "forall" => Token::Forall,
                "exists" => Token::Exists,
                "var" => Token::Var,
                "app" => Token::App,
                "symbol" => Token::Symbol,
                "bool" => Token::Bool,
                _ => Token::Ident(ident),
            };
            return Ok(SpannedToken { token, span });
        }

        // Pontuação e operadores
        match c {
            '{' => {
                self.next_char();
                Ok(SpannedToken {
                    token: Token::OpenBrace,
                    span,
                })
            }
            '}' => {
                self.next_char();
                Ok(SpannedToken {
                    token: Token::CloseBrace,
                    span,
                })
            }
            '(' => {
                self.next_char();
                Ok(SpannedToken {
                    token: Token::OpenParen,
                    span,
                })
            }
            ')' => {
                self.next_char();
                Ok(SpannedToken {
                    token: Token::CloseParen,
                    span,
                })
            }
            '[' => {
                self.next_char();
                Ok(SpannedToken {
                    token: Token::OpenBracket,
                    span,
                })
            }
            ']' => {
                self.next_char();
                Ok(SpannedToken {
                    token: Token::CloseBracket,
                    span,
                })
            }
            ',' => {
                self.next_char();
                Ok(SpannedToken {
                    token: Token::Comma,
                    span,
                })
            }
            ':' => {
                self.next_char();
                Ok(SpannedToken {
                    token: Token::Colon,
                    span,
                })
            }
            '.' => {
                self.next_char();
                Ok(SpannedToken {
                    token: Token::Dot,
                    span,
                })
            }
            ';' => {
                self.next_char();
                Ok(SpannedToken {
                    token: Token::Semi,
                    span,
                })
            }
            '=' => {
                self.next_char();
                Ok(SpannedToken {
                    token: Token::Equals,
                    span,
                })
            }
            '-' => {
                let (_, second) = self.peek2_chars();
                if second == Some('>') {
                    self.next_char();
                    self.next_char();
                    Ok(SpannedToken {
                        token: Token::Arrow,
                        span,
                    })
                } else {
                    self.next_char();
                    Err(SurfaceError::new(
                        ErrorKind::Syntax,
                        span.line,
                        span.column,
                        "unexpected character '-', expected '->'",
                    ))
                }
            }
            '<' => {
                let (_, second, third) = self.peek3_chars();
                if second == Some('-') && third == Some('>') {
                    self.next_char();
                    self.next_char();
                    self.next_char();
                    Ok(SpannedToken {
                        token: Token::DoubleArrow,
                        span,
                    })
                } else {
                    self.next_char();
                    Err(SurfaceError::new(
                        ErrorKind::Syntax,
                        span.line,
                        span.column,
                        "unexpected character '<', expected '<->'",
                    ))
                }
            }
            _ => {
                self.next_char();
                Err(SurfaceError::new(
                    ErrorKind::Syntax,
                    span.line,
                    span.column,
                    format!("unrecognized token '{c}'"),
                ))
            }
        }
    }

    fn read_decimal_u64(&mut self, span: Span, desc: &str) -> Result<u64, SurfaceError> {
        let mut value = 0_u64;
        while let Some(c) = self.peek_char() {
            if !c.is_ascii_digit() {
                break;
            }
            let digit = u64::from(c as u32) - u64::from(b'0');
            value = value
                .checked_mul(10)
                .and_then(|n| n.checked_add(digit))
                .ok_or_else(|| {
                    SurfaceError::new(
                        ErrorKind::LimitExceeded,
                        span.line,
                        span.column,
                        format!("{desc} exceeds u64::MAX"),
                    )
                })?;
            self.next_char();
        }
        Ok(value)
    }

    fn read_ident(&mut self) -> &'a str {
        let start = self.byte_pos;
        while let Some(c) = self.peek_char() {
            if c == '_' || c.is_ascii_alphanumeric() {
                self.next_char();
            } else {
                break;
            }
        }
        &self.input[start..self.byte_pos]
    }

    fn read_string(&mut self, start_span: Span) -> Result<Text, SurfaceError> {
        self.next_char(); // consome aspas de abertura
        let mut buf = String::new();

        loop {
            let ch_span = self.current_span();
            match self.next_char() {
                None => {
                    return Err(SurfaceError::new(
                        ErrorKind::Syntax,
                        start_span.line,
                        start_span.column,
                        "unterminated string literal",
                    ));
                }
                Some('"') => break,
                Some(c) if (c as u32) < 0x20 => {
                    return Err(SurfaceError::new(
                        ErrorKind::InvalidText,
                        ch_span.line,
                        ch_span.column,
                        "unescaped control character in string literal",
                    ));
                }
                Some('\\') => {
                    let esc_span = self.current_span();
                    match self.next_char() {
                        None => {
                            return Err(SurfaceError::new(
                                ErrorKind::Syntax,
                                start_span.line,
                                start_span.column,
                                "unterminated escape sequence in string literal",
                            ));
                        }
                        Some('"') => buf.push('"'),
                        Some('\\') => buf.push('\\'),
                        Some('/') => buf.push('/'),
                        Some('b') => buf.push('\x08'),
                        Some('f') => buf.push('\x0C'),
                        Some('n') => buf.push('\n'),
                        Some('r') => buf.push('\r'),
                        Some('t') => buf.push('\t'),
                        Some('u') => {
                            let code = self.read_hex4(esc_span)?;
                            if (0xD800..=0xDBFF).contains(&code) {
                                // High surrogate: deve ser imediatamente seguido por \uXXXX (Low surrogate)
                                let (first, second) = self.peek2_chars();
                                if first == Some('\\') && second == Some('u') {
                                    self.next_char();
                                    self.next_char();
                                    let low_span = self.current_span();
                                    let low = self.read_hex4(low_span)?;
                                    if (0xDC00..=0xDFFF).contains(&low) {
                                        let cp = 0x10000
                                            + (((u32::from(code) - 0xD800) << 10)
                                                | (u32::from(low) - 0xDC00));
                                        if let Some(ch) = char::from_u32(cp) {
                                            buf.push(ch);
                                        } else {
                                            return Err(SurfaceError::new(
                                                ErrorKind::InvalidText,
                                                esc_span.line,
                                                esc_span.column,
                                                "invalid surrogate pair code point",
                                            ));
                                        }
                                    } else {
                                        return Err(SurfaceError::new(
                                            ErrorKind::InvalidText,
                                            esc_span.line,
                                            esc_span.column,
                                            "isolated high surrogate in string literal",
                                        ));
                                    }
                                } else {
                                    return Err(SurfaceError::new(
                                        ErrorKind::InvalidText,
                                        esc_span.line,
                                        esc_span.column,
                                        "isolated high surrogate in string literal",
                                    ));
                                }
                            } else if (0xDC00..=0xDFFF).contains(&code) {
                                return Err(SurfaceError::new(
                                    ErrorKind::InvalidText,
                                    esc_span.line,
                                    esc_span.column,
                                    "isolated low surrogate in string literal",
                                ));
                            } else if let Some(ch) = char::from_u32(u32::from(code)) {
                                buf.push(ch);
                            } else {
                                return Err(SurfaceError::new(
                                    ErrorKind::InvalidText,
                                    esc_span.line,
                                    esc_span.column,
                                    "invalid unicode code point",
                                ));
                            }
                        }
                        Some(other) => {
                            return Err(SurfaceError::new(
                                ErrorKind::Syntax,
                                esc_span.line,
                                esc_span.column,
                                format!("invalid escape sequence '\\{other}' in string literal"),
                            ));
                        }
                    }
                }
                Some(c) => buf.push(c),
            }
        }

        Text::new(buf).map_err(|e| match e {
            CanonError::UnassignedCodePoint => SurfaceError::new(
                ErrorKind::InvalidText,
                start_span.line,
                start_span.column,
                "unassigned code point in Unicode 16",
            ),
            CanonError::NotNfc => SurfaceError::new(
                ErrorKind::InvalidText,
                start_span.line,
                start_span.column,
                "string is not in Unicode NFC normal form",
            ),
            _ => SurfaceError::new(
                ErrorKind::InvalidText,
                start_span.line,
                start_span.column,
                e.to_string(),
            ),
        })
    }

    fn read_hex4(&mut self, span: Span) -> Result<u16, SurfaceError> {
        let mut value = 0_u16;
        for _ in 0..4 {
            let digit = match self.next_char() {
                Some(c @ '0'..='9') => c as u16 - u16::from(b'0'),
                Some(c @ 'a'..='f') => c as u16 - u16::from(b'a') + 10,
                Some(c @ 'A'..='F') => c as u16 - u16::from(b'A') + 10,
                _ => {
                    return Err(SurfaceError::new(
                        ErrorKind::Syntax,
                        span.line,
                        span.column,
                        "expected 4 hex digits after \\u",
                    ));
                }
            };
            value = (value << 4) | digit;
        }
        Ok(value)
    }
}
