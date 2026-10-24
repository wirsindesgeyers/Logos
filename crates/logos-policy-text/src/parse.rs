//! Escaneador de regras e delegação da fórmula ao parser AIR.

use std::fmt;

use logos_air::Vocabulary;
use logos_air_text::SurfaceError;
use logos_canon::Text;
use logos_policy::{Class, Policy, PolicyError, Rule};

/// Falha ao ler uma política em texto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyTextError {
    /// Estrutura (`policy vN { rule ... }`) malformada.
    Syntax {
        /// Linha 1-based.
        line: usize,
        /// Coluna 1-based, em caracteres.
        column: usize,
        /// Descrição.
        message: String,
    },
    /// A fórmula da regra não passa no parser AIR.
    Formula {
        /// Regra.
        rule: String,
        /// Erro do parser AIR, com posição relativa à fórmula.
        error: SurfaceError,
    },
    /// A política montada foi recusada pela TCB.
    Policy(PolicyError),
}

impl fmt::Display for PolicyTextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax {
                line,
                column,
                message,
            } => write!(f, "[{line}:{column}] Syntax: {message}"),
            Self::Formula { rule, error } => write!(f, "rule {rule}: {error}"),
            Self::Policy(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for PolicyTextError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Ident,
    Str,
    Punct(char),
}

#[derive(Debug, Clone, Copy)]
struct Token {
    kind: Kind,
    start: usize,
    end: usize,
}

struct Source<'a> {
    text: &'a str,
}

impl<'a> Source<'a> {
    fn position(&self, offset: usize) -> (usize, usize) {
        let before = self.text.get(..offset).unwrap_or("");
        let line = before.matches('\n').count() + 1;
        let column = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
        (line, column)
    }

    fn error(&self, offset: usize, message: &str) -> PolicyTextError {
        let (line, column) = self.position(offset);
        PolicyTextError::Syntax {
            line,
            column,
            message: message.to_owned(),
        }
    }

    fn slice(&self, token: Token) -> &'a str {
        self.text.get(token.start..token.end).unwrap_or("")
    }

    fn tokens(&self) -> Result<Vec<Token>, PolicyTextError> {
        let mut tokens = Vec::new();
        let mut chars = self.text.char_indices().peekable();
        while let Some((start, c)) = chars.next() {
            if c.is_whitespace() {
                continue;
            }
            if c == '#' {
                while chars.next_if(|(_, n)| *n != '\n').is_some() {}
                continue;
            }
            if c == '"' {
                let mut closed = None;
                while let Some((i, n)) = chars.next() {
                    match n {
                        '\\' => {
                            chars.next();
                        }
                        '"' => {
                            closed = Some(i + 1);
                            break;
                        }
                        _ => {}
                    }
                }
                let end = closed.ok_or_else(|| self.error(start, "unterminated string"))?;
                tokens.push(Token {
                    kind: Kind::Str,
                    start,
                    end,
                });
                continue;
            }
            if c.is_ascii_alphabetic() || c == '_' {
                let mut end = start + c.len_utf8();
                while let Some((i, n)) =
                    chars.next_if(|(_, n)| n.is_ascii_alphanumeric() || *n == '_')
                {
                    end = i + n.len_utf8();
                }
                tokens.push(Token {
                    kind: Kind::Ident,
                    start,
                    end,
                });
                continue;
            }
            tokens.push(Token {
                kind: Kind::Punct(c),
                start,
                end: start + c.len_utf8(),
            });
        }
        Ok(tokens)
    }
}

/// Decodifica um literal de string (`"..."`) com escapes JSON básicos.
fn unquote(literal: &str) -> Option<String> {
    let inner = literal.strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next()? {
            '"' => out.push('"'),
            '\\' => out.push('\\'),
            '/' => out.push('/'),
            'n' => out.push('\n'),
            't' => out.push('\t'),
            'r' => out.push('\r'),
            'u' => {
                let digits: String = chars.by_ref().take(4).collect();
                if digits.len() != 4 {
                    return None;
                }
                out.push(char::from_u32(u32::from_str_radix(&digits, 16).ok()?)?);
            }
            _ => return None,
        }
    }
    Some(out)
}

fn class_of(name: &str) -> Option<Class> {
    match name {
        "allow" => Some(Class::Allow),
        "deny" => Some(Class::Deny),
        "require" => Some(Class::Require),
        "derive" => Some(Class::Derive),
        _ => None,
    }
}

/// Lê `policy vN { rule ID CLASSE: FÓRMULA [comment "…"] ... }` contra `vocabulary`.
///
/// # Errors
/// Estrutura inválida, fórmula recusada pelo parser AIR ou política recusada pela TCB.
pub fn parse_policy(text: &str, vocabulary: &Vocabulary) -> Result<Policy, PolicyTextError> {
    let source = Source { text };
    let tokens = source.tokens()?;
    let mut at = 0usize;
    let ident = |at: usize, word: &str| {
        tokens
            .get(at)
            .is_some_and(|t| t.kind == Kind::Ident && source.slice(*t) == word)
    };
    let punct = |at: usize, c: char| tokens.get(at).is_some_and(|t| t.kind == Kind::Punct(c));
    let here = |at: usize| tokens.get(at).map_or(text.len(), |t| t.start);

    if !ident(at, "policy") {
        return Err(source.error(here(at), "expected `policy`"));
    }
    at += 1;
    let version = tokens
        .get(at)
        .filter(|t| t.kind == Kind::Ident)
        .and_then(|t| source.slice(*t).strip_prefix('v'))
        .filter(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|d| d.parse::<u64>().ok())
        .ok_or_else(|| source.error(here(at), "expected a version like `v12`"))?;
    at += 1;
    if !punct(at, '{') {
        return Err(source.error(here(at), "expected `{`"));
    }
    at += 1;

    // `rule ID CLASSE :` começa uma regra; `comment "…"` termina a fórmula.
    let rule_start = |at: usize| {
        ident(at, "rule")
            && tokens
                .get(at + 1)
                .is_some_and(|t| matches!(t.kind, Kind::Ident | Kind::Str))
            && tokens
                .get(at + 2)
                .is_some_and(|t| t.kind == Kind::Ident && class_of(source.slice(*t)).is_some())
            && punct(at + 3, ':')
    };
    let comment_start =
        |at: usize| ident(at, "comment") && tokens.get(at + 1).is_some_and(|t| t.kind == Kind::Str);

    let mut rules = Vec::new();
    loop {
        while punct(at, ';') {
            at += 1;
        }
        if punct(at, '}') {
            at += 1;
            break;
        }
        if !rule_start(at) {
            return Err(source.error(here(at), "expected `rule ID CLASS:` or `}`"));
        }
        let id_token = tokens[at + 1];
        let id = match id_token.kind {
            Kind::Str => unquote(source.slice(id_token))
                .ok_or_else(|| source.error(id_token.start, "invalid rule id string"))?,
            _ => source.slice(id_token).to_owned(),
        };
        let class = class_of(source.slice(tokens[at + 2]))
            .ok_or_else(|| source.error(here(at + 2), "unknown rule class"))?;
        at += 4;
        let formula_start = at;
        let mut depth = 0usize;
        while at < tokens.len() {
            let token = tokens[at];
            match token.kind {
                Kind::Punct('(' | '[') => depth += 1,
                Kind::Punct(')' | ']') => depth = depth.saturating_sub(1),
                Kind::Punct('}' | ';') if depth == 0 => break,
                _ if depth == 0 && (rule_start(at) || comment_start(at)) => break,
                _ => {}
            }
            at += 1;
        }
        if at == formula_start {
            return Err(source.error(here(at), "empty rule formula"));
        }
        let first = tokens[formula_start].start;
        let last = tokens[at - 1].end;
        let formula_text = text
            .get(first..last)
            .ok_or_else(|| source.error(first, "invalid formula range"))?;
        let formula = logos_air_text::parse_formula(formula_text, vocabulary).map_err(|error| {
            PolicyTextError::Formula {
                rule: id.clone(),
                error,
            }
        })?;
        let mut comment = String::new();
        if comment_start(at) {
            let literal = tokens[at + 1];
            comment = unquote(source.slice(literal))
                .ok_or_else(|| source.error(literal.start, "invalid comment string"))?;
            at += 2;
        }
        let text_of = |value: &str, offset: usize| {
            Text::new(value)
                .map_err(|_| source.error(offset, "text is not canonical (NFC/assigned)"))
        };
        rules.push(Rule {
            id: text_of(&id, id_token.start)?,
            class,
            formula,
            comment: text_of(&comment, first)?,
        });
    }
    if at != tokens.len() {
        return Err(source.error(here(at), "unexpected input after the closing `}`"));
    }
    Policy::new(version, vocabulary, rules).map_err(PolicyTextError::Policy)
}
