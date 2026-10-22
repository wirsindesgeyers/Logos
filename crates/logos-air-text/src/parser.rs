//! Parser para a sintaxe de superfície AIR (air-text/v1).

use crate::error::{ErrorKind, SurfaceError};
use crate::lexer::{Span, SpannedToken, Token, lex};
use crate::types::{Claim, Document, References};
use logos_air::{
    AirError, AttestorId, DomainSource, FactRef, Formula, FormulaNode, Grounding, Limits,
    PolicyRef, Sort, SortDecl, SortDomain, Symbol, SymbolDecl, SymbolKind, SymbolResult, Term,
    TermNode, Vocabulary, VocabularyError,
};
use logos_canon::Text;
use std::collections::{BTreeMap, BTreeSet};

const MAX_TEXT_NESTING: usize = 4096;

/// Faz o parsing de uma fórmula de superfície em texto, usando o vocabulário para resolução.
///
/// # Errors
/// Retorna [`SurfaceError`] para erros de sintaxe, nomes desconhecidos, texto inválido
/// ou limites excedidos (profundidade 128, nós 100.000).
pub fn parse_formula(text: &str, vocabulary: &Vocabulary) -> Result<Formula, SurfaceError> {
    let tokens = lex(text)?;
    let sig_map: BTreeMap<&str, &Symbol> = vocabulary
        .symbols()
        .iter()
        .map(|d| (d.signature.name.as_str(), &d.signature))
        .collect();

    let mut stream = TokenStream::new(&tokens);
    let mut pool = FormulaPool::new();
    let mut binders = Vec::new();
    let root = stream.parse_formula_expr(&mut pool, &mut binders, &sig_map, 0)?;

    if stream.peek_token() != &Token::Eof {
        let span = stream.peek_span();
        return Err(SurfaceError::new(
            ErrorKind::Syntax,
            span.line,
            span.column,
            "unexpected trailing tokens after formula; expected EOF",
        ));
    }

    pool_to_formula(pool, root, stream.peek_span())
}

/// Faz o parsing de um documento AIR de superfície contendo vocabulário e claims.
///
/// Executa coleta em duas passagens para permitir forward references em corpos
/// `defined(...)`, valida unicidade de nomes e checa invariantes de `vocabulary/v1`.
///
/// # Errors
/// Retorna [`SurfaceError`] com a categoria e posição de qualquer violação sintática,
/// referência ausente, nome desconhecido/duplicado ou invariante descumprida.
pub fn parse_document(text: &str, references: &References) -> Result<Document, SurfaceError> {
    let tokens = lex(text)?;
    let mut stream = TokenStream::new(&tokens);
    let doc = stream.parse_document_internal(references)?;

    if stream.peek_token() != &Token::Eof {
        let span = stream.peek_span();
        return Err(SurfaceError::new(
            ErrorKind::Syntax,
            span.line,
            span.column,
            "unexpected trailing tokens after document; expected EOF",
        ));
    }

    Ok(doc)
}

fn parse_hex32(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let mut bytes = [0u8; 32];
    let (pairs, _) = s.as_bytes().as_chunks::<2>();
    for (byte_slot, pair) in bytes.iter_mut().zip(pairs) {
        let high = char::from(pair[0]).to_digit(16)?;
        let low = char::from(pair[1]).to_digit(16)?;
        *byte_slot = u8::try_from(high * 16 + low).ok()?;
    }
    Some(bytes)
}

fn resolve_reference(
    name: &str,
    references: &References,
    span: Span,
) -> Result<[u8; 32], SurfaceError> {
    if let Some(bytes) = references.get(name) {
        return Ok(*bytes);
    }
    if let Some(bytes) = parse_hex32(name) {
        return Ok(bytes);
    }
    Err(SurfaceError::new(
        ErrorKind::UnknownReference,
        span.line,
        span.column,
        format!(
            "reference '{name}' not found in references map and is not a valid 64-character hex digest"
        ),
    ))
}

fn map_air_error(error: AirError, span: Span) -> SurfaceError {
    match error {
        AirError::DepthExceeded { .. } | AirError::NodesExceeded { .. } => SurfaceError::new(
            ErrorKind::LimitExceeded,
            span.line,
            span.column,
            error.to_string(),
        ),
        AirError::Canon(canon_err) => match canon_err {
            logos_canon::CanonError::UnassignedCodePoint | logos_canon::CanonError::NotNfc => {
                SurfaceError::new(
                    ErrorKind::InvalidText,
                    span.line,
                    span.column,
                    canon_err.to_string(),
                )
            }
            logos_canon::CanonError::DepthExceeded => SurfaceError::new(
                ErrorKind::LimitExceeded,
                span.line,
                span.column,
                canon_err.to_string(),
            ),
            _ => SurfaceError::new(
                ErrorKind::InvalidAst,
                span.line,
                span.column,
                canon_err.to_string(),
            ),
        },
        _ => SurfaceError::new(
            ErrorKind::InvalidAst,
            span.line,
            span.column,
            error.to_string(),
        ),
    }
}

fn map_vocab_error(error: VocabularyError, span: Span) -> SurfaceError {
    match error {
        VocabularyError::DuplicateSort | VocabularyError::DuplicateSymbol => SurfaceError::new(
            ErrorKind::DuplicateName,
            span.line,
            span.column,
            error.to_string(),
        ),
        VocabularyError::UnknownSort | VocabularyError::UnknownSymbol => SurfaceError::new(
            ErrorKind::UnknownName,
            span.line,
            span.column,
            error.to_string(),
        ),
        VocabularyError::Air(air_err) => map_air_error(air_err, span),
        VocabularyError::Canon(canon_err) => match canon_err {
            logos_canon::CanonError::UnassignedCodePoint | logos_canon::CanonError::NotNfc => {
                SurfaceError::new(
                    ErrorKind::InvalidText,
                    span.line,
                    span.column,
                    canon_err.to_string(),
                )
            }
            logos_canon::CanonError::DepthExceeded => SurfaceError::new(
                ErrorKind::LimitExceeded,
                span.line,
                span.column,
                canon_err.to_string(),
            ),
            _ => SurfaceError::new(
                ErrorKind::InvalidVocabulary,
                span.line,
                span.column,
                canon_err.to_string(),
            ),
        },
        VocabularyError::SignatureMismatch
        | VocabularyError::InvalidDomainConstant
        | VocabularyError::CircularDefinition
        | VocabularyError::VersionOverflow
        | VocabularyError::InvalidRenderTemplate => SurfaceError::new(
            ErrorKind::InvalidVocabulary,
            span.line,
            span.column,
            error.to_string(),
        ),
    }
}

fn resolve_binder(binders: &[(&str, Sort)], name: &str) -> Option<(u64, Sort)> {
    for (i, (b_name, sort)) in binders.iter().rev().enumerate() {
        if *b_name == name {
            return Some((i as u64, sort.clone()));
        }
    }
    None
}

enum AstNode {
    True,
    False,
    Eq(Term, Term),
    Pred { symbol: Symbol, args: Vec<Term> },
    Not(usize),
    And(usize, usize),
    AndN(Vec<usize>),
    Or(usize, usize),
    OrN(Vec<usize>),
    Implies(usize, usize),
    Iff(usize, usize),
    Forall(Sort, usize),
    Exists(Sort, usize),
}

struct FormulaPool {
    nodes: Vec<AstNode>,
}

impl FormulaPool {
    fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    fn push(&mut self, node: AstNode) -> usize {
        let idx = self.nodes.len();
        self.nodes.push(node);
        idx
    }
}

fn pool_to_formula(
    mut pool: FormulaPool,
    root: usize,
    span: Span,
) -> Result<Formula, SurfaceError> {
    let mut formula_nodes = Vec::with_capacity(pool.nodes.len());
    let mut work_stack = vec![root];

    while let Some(node_id) = work_stack.pop() {
        let node = std::mem::replace(&mut pool.nodes[node_id], AstNode::True);
        match node {
            AstNode::True => formula_nodes.push(FormulaNode::True),
            AstNode::False => formula_nodes.push(FormulaNode::False),
            AstNode::Eq(left, right) => formula_nodes.push(FormulaNode::Eq(left, right)),
            AstNode::Pred { symbol, args } => {
                formula_nodes.push(FormulaNode::Pred { symbol, args })
            }
            AstNode::Not(child) => {
                formula_nodes.push(FormulaNode::Not);
                work_stack.push(child);
            }
            AstNode::And(left, right) => {
                formula_nodes.push(FormulaNode::And(2));
                work_stack.push(right);
                work_stack.push(left);
            }
            AstNode::AndN(children) => {
                formula_nodes.push(FormulaNode::And(children.len()));
                for child in children.into_iter().rev() {
                    work_stack.push(child);
                }
            }
            AstNode::Or(left, right) => {
                formula_nodes.push(FormulaNode::Or(2));
                work_stack.push(right);
                work_stack.push(left);
            }
            AstNode::OrN(children) => {
                formula_nodes.push(FormulaNode::Or(children.len()));
                for child in children.into_iter().rev() {
                    work_stack.push(child);
                }
            }
            AstNode::Implies(left, right) => {
                formula_nodes.push(FormulaNode::Implies);
                work_stack.push(right);
                work_stack.push(left);
            }
            AstNode::Iff(left, right) => {
                formula_nodes.push(FormulaNode::Iff);
                work_stack.push(right);
                work_stack.push(left);
            }
            AstNode::Forall(sort, body) => {
                formula_nodes.push(FormulaNode::Forall(sort));
                work_stack.push(body);
            }
            AstNode::Exists(sort, body) => {
                formula_nodes.push(FormulaNode::Exists(sort));
                work_stack.push(body);
            }
        }
    }

    Formula::new(formula_nodes, Limits::default()).map_err(|e| map_air_error(e, span))
}

struct TokenStream<'a> {
    tokens: &'a [SpannedToken<'a>],
    end: &'a SpannedToken<'a>,
    pos: usize,
    nesting: usize,
}

impl<'a> TokenStream<'a> {
    fn new(tokens: &'a [SpannedToken<'a>]) -> Self {
        // Lexer always appends the source EOF token.
        let end = &tokens[tokens.len() - 1];
        Self::with_end(&tokens[..tokens.len() - 1], end)
    }

    fn with_end(tokens: &'a [SpannedToken<'a>], end: &'a SpannedToken<'a>) -> Self {
        Self {
            tokens,
            end,
            pos: 0,
            nesting: 0,
        }
    }

    fn peek(&self) -> &'a SpannedToken<'a> {
        self.tokens.get(self.pos).unwrap_or(self.end)
    }

    fn peek_token(&self) -> &'a Token<'a> {
        &self.peek().token
    }

    fn peek_span(&self) -> Span {
        self.peek().span
    }

    fn peek2_token(&self) -> Option<&'a Token<'a>> {
        if self.pos + 1 < self.tokens.len() {
            Some(&self.tokens[self.pos + 1].token)
        } else {
            None
        }
    }

    fn next(&mut self) -> &'a SpannedToken<'a> {
        let token = self.peek();
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        token
    }

    fn expect(&mut self, expected: &Token<'_>) -> Result<Span, SurfaceError> {
        let tok = self.peek();
        if &tok.token == expected {
            let span = tok.span;
            self.next();
            Ok(span)
        } else {
            Err(SurfaceError::new(
                ErrorKind::Syntax,
                tok.span.line,
                tok.span.column,
                format!("expected {expected:?}, found {:?}", tok.token),
            ))
        }
    }

    fn expect_name_ref(&mut self) -> Result<(&'a str, Span), SurfaceError> {
        let tok = self.peek();
        match &tok.token {
            Token::Ident(s) => {
                let span = tok.span;
                let s_ref = *s;
                self.next();
                Ok((s_ref, span))
            }
            Token::String(text) => {
                let span = tok.span;
                let s_ref = text.as_str();
                self.next();
                Ok((s_ref, span))
            }
            Token::Vocab
            | Token::Sort
            | Token::Enum
            | Token::From
            | Token::Fact
            | Token::Policy
            | Token::Const
            | Token::Func
            | Token::Pred
            | Token::Decl
            | Token::Gloss
            | Token::Glossed
            | Token::Anchored
            | Token::Defined
            | Token::Claim
            | Token::Cites
            | Token::True
            | Token::False
            | Token::Eq
            | Token::Not
            | Token::And
            | Token::Or
            | Token::Implies
            | Token::Iff
            | Token::Forall
            | Token::Exists
            | Token::Var
            | Token::App
            | Token::Symbol
            | Token::Bool => Err(SurfaceError::new(
                ErrorKind::Syntax,
                tok.span.line,
                tok.span.column,
                "reserved keyword cannot be used as a name without quotes",
            )),
            _ => Err(SurfaceError::new(
                ErrorKind::Syntax,
                tok.span.line,
                tok.span.column,
                "expected identifier or string literal as name",
            )),
        }
    }

    fn expect_name(&mut self) -> Result<(Text, Span), SurfaceError> {
        let tok = self.peek();
        match &tok.token {
            Token::Ident(s) => {
                let span = tok.span;
                let text = Text::new(*s).map_err(|e| {
                    SurfaceError::new(
                        ErrorKind::InvalidText,
                        span.line,
                        span.column,
                        e.to_string(),
                    )
                })?;
                self.next();
                Ok((text, span))
            }
            Token::String(text) => {
                let span = tok.span;
                let t = text.clone();
                self.next();
                Ok((t, span))
            }
            Token::Vocab
            | Token::Sort
            | Token::Enum
            | Token::From
            | Token::Fact
            | Token::Policy
            | Token::Const
            | Token::Func
            | Token::Pred
            | Token::Decl
            | Token::Gloss
            | Token::Glossed
            | Token::Anchored
            | Token::Defined
            | Token::Claim
            | Token::Cites
            | Token::True
            | Token::False
            | Token::Eq
            | Token::Not
            | Token::And
            | Token::Or
            | Token::Implies
            | Token::Iff
            | Token::Forall
            | Token::Exists
            | Token::Var
            | Token::App
            | Token::Symbol
            | Token::Bool => Err(SurfaceError::new(
                ErrorKind::Syntax,
                tok.span.line,
                tok.span.column,
                "reserved keyword cannot be used as a name without quotes",
            )),
            _ => Err(SurfaceError::new(
                ErrorKind::Syntax,
                tok.span.line,
                tok.span.column,
                "expected identifier or string literal as name",
            )),
        }
    }

    fn expect_string_literal(&mut self) -> Result<(Text, Span), SurfaceError> {
        let tok = self.peek();
        match &tok.token {
            Token::String(text) => {
                let span = tok.span;
                let t = text.clone();
                self.next();
                Ok((t, span))
            }
            _ => Err(SurfaceError::new(
                ErrorKind::Syntax,
                tok.span.line,
                tok.span.column,
                "expected string literal in double quotes",
            )),
        }
    }

    fn enter_nesting(&mut self, span: Span) -> Result<(), SurfaceError> {
        self.nesting += 1;
        if self.nesting > MAX_TEXT_NESTING {
            Err(SurfaceError::new(
                ErrorKind::LimitExceeded,
                span.line,
                span.column,
                "textual nesting depth limit exceeded",
            ))
        } else {
            Ok(())
        }
    }

    fn exit_nesting(&mut self) {
        if self.nesting > 0 {
            self.nesting -= 1;
        }
    }

    fn parse_symbol(&mut self) -> Result<Symbol, SurfaceError> {
        self.expect(&Token::Symbol)?;
        self.expect(&Token::OpenParen)?;
        let (name, _) = self.expect_name()?;
        self.expect(&Token::Comma)?;
        let kind = match self.peek_token() {
            Token::Const => {
                self.next();
                SymbolKind::Const
            }
            Token::Func => {
                self.next();
                SymbolKind::Func
            }
            Token::Pred => {
                self.next();
                SymbolKind::Pred
            }
            _ => {
                let span = self.peek_span();
                return Err(SurfaceError::new(
                    ErrorKind::Syntax,
                    span.line,
                    span.column,
                    "expected 'const', 'func', or 'pred'",
                ));
            }
        };
        self.expect(&Token::Comma)?;
        self.expect(&Token::OpenBracket)?;
        let mut arity = Vec::new();
        if self.peek_token() != &Token::CloseBracket {
            let (sort_name, _) = self.expect_name()?;
            arity.push(Sort { name: sort_name });
            while self.peek_token() == &Token::Comma {
                self.next();
                let (sort_name, _) = self.expect_name()?;
                arity.push(Sort { name: sort_name });
            }
        }
        self.expect(&Token::CloseBracket)?;
        self.expect(&Token::Comma)?;
        let result = if self.peek_token() == &Token::Bool {
            self.next();
            SymbolResult::Bool
        } else {
            let (sort_name, _) = self.expect_name()?;
            SymbolResult::Sort(Sort { name: sort_name })
        };
        self.expect(&Token::CloseParen)?;
        Ok(Symbol {
            name,
            kind,
            arity,
            result,
        })
    }

    fn parse_term_into(
        &mut self,
        out: &mut Vec<TermNode>,
        binders: &[(&str, Sort)],
        sig_map: &BTreeMap<&str, &Symbol>,
    ) -> Result<(), SurfaceError> {
        enum TermFrame {
            ExplicitApp { app_idx: usize, arg_count: usize },
            NamedApp { app_idx: usize, arg_count: usize },
        }

        let mut stack: Vec<TermFrame> = Vec::new();

        loop {
            let span = self.peek_span();
            match self.peek_token() {
                Token::Var => {
                    self.enter_nesting(span)?;
                    self.next();
                    self.expect(&Token::OpenParen)?;
                    let index = match self.peek_token() {
                        Token::Integer(n) => {
                            let val = *n;
                            self.next();
                            val
                        }
                        _ => {
                            let s = self.peek_span();
                            return Err(SurfaceError::new(
                                ErrorKind::Syntax,
                                s.line,
                                s.column,
                                "expected integer variable index",
                            ));
                        }
                    };
                    self.expect(&Token::Comma)?;
                    let (sort_name, _) = self.expect_name()?;
                    self.expect(&Token::CloseParen)?;
                    self.exit_nesting();
                    out.push(TermNode::Var {
                        index,
                        sort: Sort { name: sort_name },
                    });
                }
                Token::App => {
                    self.enter_nesting(span)?;
                    self.next();
                    self.expect(&Token::OpenParen)?;
                    let symbol = self.parse_symbol()?;
                    self.expect(&Token::Comma)?;
                    self.expect(&Token::OpenBracket)?;
                    let app_idx = out.len();
                    out.push(TermNode::App { symbol, args: 0 });
                    if self.peek_token() == &Token::CloseBracket {
                        self.next();
                        self.expect(&Token::CloseParen)?;
                        self.exit_nesting();
                    } else {
                        stack.push(TermFrame::ExplicitApp {
                            app_idx,
                            arg_count: 0,
                        });
                        continue;
                    }
                }
                _ => {
                    let (name_str, name_span) = self.expect_name_ref()?;
                    if self.peek_token() == &Token::OpenParen {
                        self.enter_nesting(name_span)?;
                        self.next();
                        let symbol = sig_map.get(name_str).copied().cloned().ok_or_else(|| {
                            SurfaceError::new(
                                ErrorKind::UnknownName,
                                name_span.line,
                                name_span.column,
                                format!("unknown symbol '{name_str}'"),
                            )
                        })?;
                        let app_idx = out.len();
                        out.push(TermNode::App { symbol, args: 0 });
                        if self.peek_token() == &Token::CloseParen {
                            self.next();
                            self.exit_nesting();
                        } else {
                            stack.push(TermFrame::NamedApp {
                                app_idx,
                                arg_count: 0,
                            });
                            continue;
                        }
                    } else if let Some((index, sort)) = resolve_binder(binders, name_str) {
                        out.push(TermNode::Var { index, sort });
                    } else if let Some(symbol) = sig_map.get(name_str).copied() {
                        out.push(TermNode::App {
                            symbol: symbol.clone(),
                            args: 0,
                        });
                    } else {
                        return Err(SurfaceError::new(
                            ErrorKind::UnknownName,
                            name_span.line,
                            name_span.column,
                            format!("unknown variable or symbol '{name_str}'"),
                        ));
                    }
                }
            }

            loop {
                if stack.is_empty() {
                    return Ok(());
                }

                match stack.last_mut().expect("stack not empty") {
                    TermFrame::ExplicitApp {
                        app_idx: _,
                        arg_count,
                    } => {
                        *arg_count += 1;
                        if self.peek_token() == &Token::Comma {
                            self.next();
                            break;
                        } else {
                            self.expect(&Token::CloseBracket)?;
                            self.expect(&Token::CloseParen)?;
                            self.exit_nesting();
                            let frame = stack.pop().expect("stack not empty");
                            if let TermFrame::ExplicitApp { app_idx, arg_count } = frame
                                && let TermNode::App { ref mut args, .. } = out[app_idx]
                            {
                                *args = arg_count;
                            }
                        }
                    }
                    TermFrame::NamedApp {
                        app_idx: _,
                        arg_count,
                    } => {
                        *arg_count += 1;
                        if self.peek_token() == &Token::Comma {
                            self.next();
                            break;
                        } else {
                            self.expect(&Token::CloseParen)?;
                            self.exit_nesting();
                            let frame = stack.pop().expect("stack not empty");
                            if let TermFrame::NamedApp { app_idx, arg_count } = frame
                                && let TermNode::App { ref mut args, .. } = out[app_idx]
                            {
                                *args = arg_count;
                            }
                        }
                    }
                }
            }
        }
    }

    fn parse_embedded_term(
        &mut self,
        binders: &[(&str, Sort)],
        sig_map: &BTreeMap<&str, &Symbol>,
    ) -> Result<Term, SurfaceError> {
        let span = self.peek_span();
        let mut nodes = Vec::new();
        self.parse_term_into(&mut nodes, binders, sig_map)?;
        Term::new(nodes, Limits::default()).map_err(|e| map_air_error(e, span))
    }

    fn parse_formula_expr(
        &mut self,
        pool: &mut FormulaPool,
        binders: &mut Vec<(&'a str, Sort)>,
        sig_map: &BTreeMap<&str, &Symbol>,
        initial_min_bp: u8,
    ) -> Result<usize, SurfaceError> {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum InfixOp {
            Iff,
            Implies,
            Or,
            And,
            None,
        }

        enum ResumeFrame {
            InfixRight {
                left: usize,
                op: InfixOp,
                min_bp: u8,
            },
            UnaryNot {
                min_bp: u8,
            },
            GroupClose {
                min_bp: u8,
            },
            QuantifierScope {
                is_forall: bool,
                sort: Sort,
                saved_binders_len: usize,
                min_bp: u8,
            },
            ExplicitNot {
                min_bp: u8,
            },
            ExplicitBinaryLeft {
                is_implies: bool,
                min_bp: u8,
            },
            ExplicitBinaryRight {
                is_implies: bool,
                left: usize,
                min_bp: u8,
            },
            ExplicitNaryList {
                is_and: bool,
                children: Vec<usize>,
                min_bp: u8,
            },
        }

        enum ParseState {
            StartExpr { min_bp: u8 },
            HaveExpr { left: usize, min_bp: u8 },
        }

        let mut resume_stack: Vec<ResumeFrame> = Vec::new();
        let mut state = ParseState::StartExpr {
            min_bp: initial_min_bp,
        };

        loop {
            match state {
                ParseState::StartExpr { min_bp } => {
                    let span = self.peek_span();
                    match self.peek_token() {
                        Token::Not if self.peek2_token() != Some(&Token::OpenParen) => {
                            self.enter_nesting(span)?;
                            self.next();
                            resume_stack.push(ResumeFrame::UnaryNot { min_bp });
                            state = ParseState::StartExpr { min_bp: 5 };
                        }
                        Token::OpenParen => {
                            self.enter_nesting(span)?;
                            self.next();
                            resume_stack.push(ResumeFrame::GroupClose { min_bp });
                            state = ParseState::StartExpr { min_bp: 0 };
                        }
                        Token::Forall => {
                            self.enter_nesting(span)?;
                            self.next();
                            let (var_name, _) = self.expect_name_ref()?;
                            self.expect(&Token::Colon)?;
                            let (sort_name, _) = self.expect_name()?;
                            self.expect(&Token::Dot)?;
                            let sort = Sort { name: sort_name };
                            let saved_binders_len = binders.len();
                            binders.push((var_name, sort.clone()));
                            resume_stack.push(ResumeFrame::QuantifierScope {
                                is_forall: true,
                                sort,
                                saved_binders_len,
                                min_bp,
                            });
                            state = ParseState::StartExpr { min_bp: 0 };
                        }
                        Token::Exists => {
                            self.enter_nesting(span)?;
                            self.next();
                            let (var_name, _) = self.expect_name_ref()?;
                            self.expect(&Token::Colon)?;
                            let (sort_name, _) = self.expect_name()?;
                            self.expect(&Token::Dot)?;
                            let sort = Sort { name: sort_name };
                            let saved_binders_len = binders.len();
                            binders.push((var_name, sort.clone()));
                            resume_stack.push(ResumeFrame::QuantifierScope {
                                is_forall: false,
                                sort,
                                saved_binders_len,
                                min_bp,
                            });
                            state = ParseState::StartExpr { min_bp: 0 };
                        }
                        Token::Not if self.peek2_token() == Some(&Token::OpenParen) => {
                            self.enter_nesting(span)?;
                            self.next();
                            self.expect(&Token::OpenParen)?;
                            resume_stack.push(ResumeFrame::ExplicitNot { min_bp });
                            state = ParseState::StartExpr { min_bp: 0 };
                        }
                        Token::Implies if self.peek2_token() == Some(&Token::OpenParen) => {
                            self.enter_nesting(span)?;
                            self.next();
                            self.expect(&Token::OpenParen)?;
                            resume_stack.push(ResumeFrame::ExplicitBinaryLeft {
                                is_implies: true,
                                min_bp,
                            });
                            state = ParseState::StartExpr { min_bp: 0 };
                        }
                        Token::Iff if self.peek2_token() == Some(&Token::OpenParen) => {
                            self.enter_nesting(span)?;
                            self.next();
                            self.expect(&Token::OpenParen)?;
                            resume_stack.push(ResumeFrame::ExplicitBinaryLeft {
                                is_implies: false,
                                min_bp,
                            });
                            state = ParseState::StartExpr { min_bp: 0 };
                        }
                        Token::And if self.peek2_token() == Some(&Token::OpenParen) => {
                            self.enter_nesting(span)?;
                            self.next();
                            self.expect(&Token::OpenParen)?;
                            self.expect(&Token::OpenBracket)?;
                            if self.peek_token() == &Token::CloseBracket {
                                self.next();
                                self.expect(&Token::CloseParen)?;
                                self.exit_nesting();
                                let node = pool.push(AstNode::AndN(Vec::new()));
                                state = ParseState::HaveExpr { left: node, min_bp };
                            } else {
                                resume_stack.push(ResumeFrame::ExplicitNaryList {
                                    is_and: true,
                                    children: Vec::new(),
                                    min_bp,
                                });
                                state = ParseState::StartExpr { min_bp: 0 };
                            }
                        }
                        Token::Or if self.peek2_token() == Some(&Token::OpenParen) => {
                            self.enter_nesting(span)?;
                            self.next();
                            self.expect(&Token::OpenParen)?;
                            self.expect(&Token::OpenBracket)?;
                            if self.peek_token() == &Token::CloseBracket {
                                self.next();
                                self.expect(&Token::CloseParen)?;
                                self.exit_nesting();
                                let node = pool.push(AstNode::OrN(Vec::new()));
                                state = ParseState::HaveExpr { left: node, min_bp };
                            } else {
                                resume_stack.push(ResumeFrame::ExplicitNaryList {
                                    is_and: false,
                                    children: Vec::new(),
                                    min_bp,
                                });
                                state = ParseState::StartExpr { min_bp: 0 };
                            }
                        }
                        Token::True => {
                            self.next();
                            let node = pool.push(AstNode::True);
                            state = ParseState::HaveExpr { left: node, min_bp };
                        }
                        Token::False => {
                            self.next();
                            let node = pool.push(AstNode::False);
                            state = ParseState::HaveExpr { left: node, min_bp };
                        }
                        Token::Eq => {
                            self.enter_nesting(span)?;
                            self.next();
                            self.expect(&Token::OpenParen)?;
                            let left = self.parse_embedded_term(binders, sig_map)?;
                            self.expect(&Token::Comma)?;
                            let right = self.parse_embedded_term(binders, sig_map)?;
                            self.expect(&Token::CloseParen)?;
                            self.exit_nesting();
                            let node = pool.push(AstNode::Eq(left, right));
                            state = ParseState::HaveExpr { left: node, min_bp };
                        }
                        Token::Pred => {
                            self.enter_nesting(span)?;
                            self.next();
                            self.expect(&Token::OpenParen)?;
                            let symbol = self.parse_symbol()?;
                            self.expect(&Token::Comma)?;
                            self.expect(&Token::OpenBracket)?;
                            let mut args = Vec::new();
                            if self.peek_token() != &Token::CloseBracket {
                                args.push(self.parse_embedded_term(binders, sig_map)?);
                                while self.peek_token() == &Token::Comma {
                                    self.next();
                                    args.push(self.parse_embedded_term(binders, sig_map)?);
                                }
                            }
                            self.expect(&Token::CloseBracket)?;
                            self.expect(&Token::CloseParen)?;
                            self.exit_nesting();
                            let node = pool.push(AstNode::Pred { symbol, args });
                            state = ParseState::HaveExpr { left: node, min_bp };
                        }
                        Token::Var | Token::App => {
                            let left = self.parse_embedded_term(binders, sig_map)?;
                            self.expect(&Token::Equals)?;
                            let right = self.parse_embedded_term(binders, sig_map)?;
                            let node = pool.push(AstNode::Eq(left, right));
                            state = ParseState::HaveExpr { left: node, min_bp };
                        }
                        _ => {
                            let (name_str, name_span) = self.expect_name_ref()?;
                            if self.peek_token() == &Token::OpenParen {
                                self.enter_nesting(name_span)?;
                                self.next();
                                let mut raw_args: Vec<Vec<TermNode>> = Vec::new();
                                if self.peek_token() != &Token::CloseParen {
                                    let mut arg_nodes = Vec::new();
                                    self.parse_term_into(&mut arg_nodes, binders, sig_map)?;
                                    raw_args.push(arg_nodes);
                                    while self.peek_token() == &Token::Comma {
                                        self.next();
                                        let mut arg_nodes = Vec::new();
                                        self.parse_term_into(&mut arg_nodes, binders, sig_map)?;
                                        raw_args.push(arg_nodes);
                                    }
                                }
                                self.expect(&Token::CloseParen)?;
                                self.exit_nesting();

                                if self.peek_token() == &Token::Equals {
                                    self.next();
                                    let right = self.parse_embedded_term(binders, sig_map)?;
                                    let symbol =
                                        sig_map.get(name_str).copied().cloned().ok_or_else(
                                            || {
                                                SurfaceError::new(
                                                    ErrorKind::UnknownName,
                                                    name_span.line,
                                                    name_span.column,
                                                    format!("unknown symbol '{name_str}'"),
                                                )
                                            },
                                        )?;
                                    let mut left_nodes = Vec::new();
                                    left_nodes.push(TermNode::App {
                                        symbol,
                                        args: raw_args.len(),
                                    });
                                    for arg_vec in raw_args {
                                        left_nodes.extend(arg_vec);
                                    }
                                    let left = Term::new(left_nodes, Limits::default())
                                        .map_err(|e| map_air_error(e, name_span))?;
                                    let node = pool.push(AstNode::Eq(left, right));
                                    state = ParseState::HaveExpr { left: node, min_bp };
                                } else {
                                    let symbol = sig_map
                                        .get(name_str)
                                        .copied()
                                        .cloned()
                                        .ok_or_else(|| {
                                            SurfaceError::new(
                                                ErrorKind::UnknownName,
                                                name_span.line,
                                                name_span.column,
                                                format!("unknown predicate symbol '{name_str}'"),
                                            )
                                        })?;
                                    let mut terms = Vec::with_capacity(raw_args.len());
                                    for arg_vec in raw_args {
                                        let term = Term::new(arg_vec, Limits::default())
                                            .map_err(|e| map_air_error(e, name_span))?;
                                        terms.push(term);
                                    }
                                    let node = pool.push(AstNode::Pred {
                                        symbol,
                                        args: terms,
                                    });
                                    state = ParseState::HaveExpr { left: node, min_bp };
                                }
                            } else if self.peek_token() == &Token::Equals {
                                self.next();
                                let right = self.parse_embedded_term(binders, sig_map)?;
                                let left = if let Some((index, sort)) =
                                    resolve_binder(binders, name_str)
                                {
                                    Term::new(
                                        vec![TermNode::Var { index, sort }],
                                        Limits::default(),
                                    )
                                    .map_err(|e| map_air_error(e, name_span))?
                                } else if let Some(symbol) = sig_map.get(name_str).copied() {
                                    Term::new(
                                        vec![TermNode::App {
                                            symbol: symbol.clone(),
                                            args: 0,
                                        }],
                                        Limits::default(),
                                    )
                                    .map_err(|e| map_air_error(e, name_span))?
                                } else {
                                    return Err(SurfaceError::new(
                                        ErrorKind::UnknownName,
                                        name_span.line,
                                        name_span.column,
                                        format!("unknown variable or symbol '{name_str}'"),
                                    ));
                                };
                                let node = pool.push(AstNode::Eq(left, right));
                                state = ParseState::HaveExpr { left: node, min_bp };
                            } else {
                                let symbol =
                                    sig_map.get(name_str).copied().cloned().ok_or_else(|| {
                                        SurfaceError::new(
                                            ErrorKind::UnknownName,
                                            name_span.line,
                                            name_span.column,
                                            format!("unknown predicate symbol '{name_str}'"),
                                        )
                                    })?;
                                let node = pool.push(AstNode::Pred {
                                    symbol,
                                    args: Vec::new(),
                                });
                                state = ParseState::HaveExpr { left: node, min_bp };
                            }
                        }
                    }
                }
                ParseState::HaveExpr { left, min_bp } => {
                    let (op_prec, next_min_prec, op) = match self.peek_token() {
                        Token::DoubleArrow => (1, 2, InfixOp::Iff),
                        Token::Arrow => (2, 2, InfixOp::Implies),
                        Token::Or => (3, 4, InfixOp::Or),
                        Token::And => (4, 5, InfixOp::And),
                        _ => (0, 0, InfixOp::None),
                    };

                    if op != InfixOp::None && op_prec >= min_bp {
                        let span = self.peek_span();
                        self.enter_nesting(span)?;
                        self.next();
                        resume_stack.push(ResumeFrame::InfixRight { left, op, min_bp });
                        state = ParseState::StartExpr {
                            min_bp: next_min_prec,
                        };
                    } else {
                        match resume_stack.pop() {
                            None => return Ok(left),
                            Some(frame) => match frame {
                                ResumeFrame::InfixRight {
                                    left: prev_left,
                                    op,
                                    min_bp: orig_min_bp,
                                } => {
                                    self.exit_nesting();
                                    let new_left = match op {
                                        InfixOp::Iff => pool.push(AstNode::Iff(prev_left, left)),
                                        InfixOp::Implies => {
                                            pool.push(AstNode::Implies(prev_left, left))
                                        }
                                        InfixOp::Or => pool.push(AstNode::Or(prev_left, left)),
                                        InfixOp::And => pool.push(AstNode::And(prev_left, left)),
                                        InfixOp::None => unreachable!(),
                                    };
                                    state = ParseState::HaveExpr {
                                        left: new_left,
                                        min_bp: orig_min_bp,
                                    };
                                }
                                ResumeFrame::UnaryNot {
                                    min_bp: orig_min_bp,
                                } => {
                                    self.exit_nesting();
                                    let new_left = pool.push(AstNode::Not(left));
                                    state = ParseState::HaveExpr {
                                        left: new_left,
                                        min_bp: orig_min_bp,
                                    };
                                }
                                ResumeFrame::GroupClose {
                                    min_bp: orig_min_bp,
                                } => {
                                    self.expect(&Token::CloseParen)?;
                                    self.exit_nesting();
                                    state = ParseState::HaveExpr {
                                        left,
                                        min_bp: orig_min_bp,
                                    };
                                }
                                ResumeFrame::QuantifierScope {
                                    is_forall,
                                    sort,
                                    saved_binders_len,
                                    min_bp: orig_min_bp,
                                } => {
                                    self.exit_nesting();
                                    binders.truncate(saved_binders_len);
                                    let new_left = if is_forall {
                                        pool.push(AstNode::Forall(sort, left))
                                    } else {
                                        pool.push(AstNode::Exists(sort, left))
                                    };
                                    state = ParseState::HaveExpr {
                                        left: new_left,
                                        min_bp: orig_min_bp,
                                    };
                                }
                                ResumeFrame::ExplicitNot {
                                    min_bp: orig_min_bp,
                                } => {
                                    self.expect(&Token::CloseParen)?;
                                    self.exit_nesting();
                                    let new_left = pool.push(AstNode::Not(left));
                                    state = ParseState::HaveExpr {
                                        left: new_left,
                                        min_bp: orig_min_bp,
                                    };
                                }
                                ResumeFrame::ExplicitBinaryLeft {
                                    is_implies,
                                    min_bp: orig_min_bp,
                                } => {
                                    self.expect(&Token::Comma)?;
                                    resume_stack.push(ResumeFrame::ExplicitBinaryRight {
                                        is_implies,
                                        left,
                                        min_bp: orig_min_bp,
                                    });
                                    state = ParseState::StartExpr { min_bp: 0 };
                                }
                                ResumeFrame::ExplicitBinaryRight {
                                    is_implies,
                                    left: left_child,
                                    min_bp: orig_min_bp,
                                } => {
                                    self.expect(&Token::CloseParen)?;
                                    self.exit_nesting();
                                    let new_left = if is_implies {
                                        pool.push(AstNode::Implies(left_child, left))
                                    } else {
                                        pool.push(AstNode::Iff(left_child, left))
                                    };
                                    state = ParseState::HaveExpr {
                                        left: new_left,
                                        min_bp: orig_min_bp,
                                    };
                                }
                                ResumeFrame::ExplicitNaryList {
                                    is_and,
                                    mut children,
                                    min_bp: orig_min_bp,
                                } => {
                                    children.push(left);
                                    if self.peek_token() == &Token::Comma {
                                        self.next();
                                        resume_stack.push(ResumeFrame::ExplicitNaryList {
                                            is_and,
                                            children,
                                            min_bp: orig_min_bp,
                                        });
                                        state = ParseState::StartExpr { min_bp: 0 };
                                    } else {
                                        self.expect(&Token::CloseBracket)?;
                                        self.expect(&Token::CloseParen)?;
                                        self.exit_nesting();
                                        let new_left = if is_and {
                                            pool.push(AstNode::AndN(children))
                                        } else {
                                            pool.push(AstNode::OrN(children))
                                        };
                                        state = ParseState::HaveExpr {
                                            left: new_left,
                                            min_bp: orig_min_bp,
                                        };
                                    }
                                }
                            },
                        }
                    }
                }
            }
        }
    }

    fn parse_document_internal(
        &mut self,
        references: &References,
    ) -> Result<Document, SurfaceError> {
        let vocab_span = self.peek_span();
        self.expect(&Token::Vocab)?;
        let version = match self.peek_token() {
            Token::Ident(s)
                if s.starts_with('v')
                    && s.len() > 1
                    && s[1..].chars().all(|c| c.is_ascii_digit()) =>
            {
                let s_span = self.peek_span();
                let digits = &s[1..];
                let val = digits.parse::<u64>().map_err(|_| {
                    SurfaceError::new(
                        ErrorKind::LimitExceeded,
                        s_span.line,
                        s_span.column,
                        "vocabulary version exceeds u64::MAX",
                    )
                })?;
                self.next();
                val
            }
            _ => {
                let s = self.peek_span();
                return Err(SurfaceError::new(
                    ErrorKind::Syntax,
                    s.line,
                    s.column,
                    "expected vocabulary version token like 'v1'",
                ));
            }
        };

        self.expect(&Token::OpenBrace)?;

        let mut raw_sorts = Vec::new();
        let mut raw_symbols = Vec::new();
        let mut render_pt = None;
        let mut seen_sorts = BTreeSet::new();
        let mut seen_symbols = BTreeSet::new();

        while self.peek_token() != &Token::CloseBrace {
            match self.peek_token() {
                Token::Ident("render_pt") => {
                    let span = self.peek_span();
                    if render_pt.is_some() {
                        return Err(SurfaceError::new(
                            ErrorKind::InvalidAst,
                            span.line,
                            span.column,
                            "duplicate render_pt",
                        ));
                    }
                    self.next();
                    self.expect(&Token::OpenBracket)?;
                    let mut templates = Vec::new();
                    if self.peek_token() != &Token::CloseBracket {
                        templates.push(self.expect_string_literal()?.0);
                        while self.peek_token() == &Token::Comma {
                            self.next();
                            templates.push(self.expect_string_literal()?.0);
                        }
                    }
                    self.expect(&Token::CloseBracket)?;
                    render_pt = Some(templates);
                }
                Token::Sort => {
                    self.next();
                    let (name, name_span) = self.expect_name()?;
                    if !seen_sorts.insert(name.clone()) {
                        return Err(SurfaceError::new(
                            ErrorKind::DuplicateName,
                            name_span.line,
                            name_span.column,
                            format!("duplicate sort name '{}'", name.as_str()),
                        ));
                    }

                    if self.peek_token() == &Token::Equals {
                        self.next();
                        self.expect(&Token::Enum)?;
                        self.expect(&Token::OpenBracket)?;
                        let mut constants = Vec::new();
                        if self.peek_token() != &Token::CloseBracket {
                            constants.push(self.expect_name()?);
                            while self.peek_token() == &Token::Comma {
                                self.next();
                                constants.push(self.expect_name()?);
                            }
                        }
                        self.expect(&Token::CloseBracket)?;
                        self.expect(&Token::From)?;
                        let is_fact = match self.peek_token() {
                            Token::Fact => {
                                self.next();
                                true
                            }
                            Token::Policy => {
                                self.next();
                                false
                            }
                            _ => {
                                let s = self.peek_span();
                                return Err(SurfaceError::new(
                                    ErrorKind::Syntax,
                                    s.line,
                                    s.column,
                                    "expected 'fact' or 'policy' after 'from'",
                                ));
                            }
                        };
                        let (ref_name, ref_span) = self.expect_name()?;
                        let digest = resolve_reference(ref_name.as_str(), references, ref_span)?;
                        let domain_source = if is_fact {
                            DomainSource::Fact(FactRef(digest))
                        } else {
                            DomainSource::Policy(PolicyRef(digest))
                        };
                        raw_sorts.push(RawSort::Enum {
                            name,
                            constants,
                            domain_source,
                        });
                    } else {
                        raw_sorts.push(RawSort::Open { name });
                    }
                }
                Token::Const => {
                    self.next();
                    let (name, name_span) = self.expect_name()?;
                    if !seen_symbols.insert(name.clone()) {
                        return Err(SurfaceError::new(
                            ErrorKind::DuplicateName,
                            name_span.line,
                            name_span.column,
                            format!("duplicate symbol name '{}'", name.as_str()),
                        ));
                    }
                    self.expect(&Token::Colon)?;
                    let (sort_name, _) = self.expect_name()?;
                    let signature = Symbol {
                        name,
                        kind: SymbolKind::Const,
                        arity: Vec::new(),
                        result: SymbolResult::Sort(Sort { name: sort_name }),
                    };
                    let raw_grounding = self.parse_optional_grounding()?;
                    let gloss = self.parse_optional_gloss()?;
                    raw_symbols.push(RawSymbol {
                        signature,
                        span: name_span,
                        raw_grounding,
                        gloss,
                    });
                }
                Token::Func => {
                    self.next();
                    let (name, name_span) = self.expect_name()?;
                    if !seen_symbols.insert(name.clone()) {
                        return Err(SurfaceError::new(
                            ErrorKind::DuplicateName,
                            name_span.line,
                            name_span.column,
                            format!("duplicate symbol name '{}'", name.as_str()),
                        ));
                    }
                    self.expect(&Token::OpenParen)?;
                    let mut arity = Vec::new();
                    if self.peek_token() != &Token::CloseParen {
                        let (s, _) = self.expect_name()?;
                        arity.push(Sort { name: s });
                        while self.peek_token() == &Token::Comma {
                            self.next();
                            let (s, _) = self.expect_name()?;
                            arity.push(Sort { name: s });
                        }
                    }
                    self.expect(&Token::CloseParen)?;
                    self.expect(&Token::Colon)?;
                    let (ret_sort, _) = self.expect_name()?;
                    let signature = Symbol {
                        name,
                        kind: SymbolKind::Func,
                        arity,
                        result: SymbolResult::Sort(Sort { name: ret_sort }),
                    };
                    let raw_grounding = self.parse_optional_grounding()?;
                    let gloss = self.parse_optional_gloss()?;
                    raw_symbols.push(RawSymbol {
                        signature,
                        span: name_span,
                        raw_grounding,
                        gloss,
                    });
                }
                Token::Pred => {
                    self.next();
                    let (name, name_span) = self.expect_name()?;
                    if !seen_symbols.insert(name.clone()) {
                        return Err(SurfaceError::new(
                            ErrorKind::DuplicateName,
                            name_span.line,
                            name_span.column,
                            format!("duplicate symbol name '{}'", name.as_str()),
                        ));
                    }
                    self.expect(&Token::OpenParen)?;
                    let mut arity = Vec::new();
                    if self.peek_token() != &Token::CloseParen {
                        let (s, _) = self.expect_name()?;
                        arity.push(Sort { name: s });
                        while self.peek_token() == &Token::Comma {
                            self.next();
                            let (s, _) = self.expect_name()?;
                            arity.push(Sort { name: s });
                        }
                    }
                    self.expect(&Token::CloseParen)?;
                    let signature = Symbol {
                        name,
                        kind: SymbolKind::Pred,
                        arity,
                        result: SymbolResult::Bool,
                    };
                    let raw_grounding = self.parse_optional_grounding()?;
                    let gloss = self.parse_optional_gloss()?;
                    raw_symbols.push(RawSymbol {
                        signature,
                        span: name_span,
                        raw_grounding,
                        gloss,
                    });
                }
                Token::Decl => {
                    self.next();
                    let sym_span = self.peek_span();
                    let signature = self.parse_symbol()?;
                    if !seen_symbols.insert(signature.name.clone()) {
                        return Err(SurfaceError::new(
                            ErrorKind::DuplicateName,
                            sym_span.line,
                            sym_span.column,
                            format!("duplicate symbol name '{}'", signature.name.as_str()),
                        ));
                    }
                    let raw_grounding = self.parse_required_grounding()?;
                    self.expect(&Token::Gloss)?;
                    let (gloss, _) = self.expect_string_literal()?;
                    raw_symbols.push(RawSymbol {
                        signature,
                        span: sym_span,
                        raw_grounding,
                        gloss,
                    });
                }
                _ => {
                    let s = self.peek_span();
                    return Err(SurfaceError::new(
                        ErrorKind::Syntax,
                        s.line,
                        s.column,
                        "expected declaration ('sort', 'const', 'func', 'pred', 'decl') or '}'",
                    ));
                }
            }

            if self.peek_token() == &Token::Semi {
                self.next();
            }
        }

        self.expect(&Token::CloseBrace)?;

        // Signatures and pending grounding are disjoint fields: borrow the former
        // while resolving the latter, then move declarations into Vocabulary.
        let mut sig_map = BTreeMap::new();
        let mut definitions = Vec::new();
        for rs in &mut raw_symbols {
            sig_map.insert(rs.signature.name.as_str(), &rs.signature);
            if matches!(rs.raw_grounding, RawGrounding::Defined(..)) {
                definitions.push(&mut rs.raw_grounding);
            }
        }

        let mut sorts = Vec::with_capacity(raw_sorts.len());
        for rs in raw_sorts {
            match rs {
                RawSort::Open { name } => {
                    sorts.push(SortDecl {
                        name,
                        domain: SortDomain::Open,
                    });
                }
                RawSort::Enum {
                    name,
                    constants,
                    domain_source,
                } => {
                    let mut const_symbols = Vec::with_capacity(constants.len());
                    for (c_name, c_span) in constants {
                        if let Some(sym) = sig_map.get(c_name.as_str()).copied() {
                            const_symbols.push(sym.clone());
                        } else {
                            return Err(SurfaceError::new(
                                ErrorKind::UnknownName,
                                c_span.line,
                                c_span.column,
                                format!("unknown constant '{}' in enum", c_name.as_str()),
                            ));
                        }
                    }
                    sorts.push(SortDecl {
                        name,
                        domain: SortDomain::Enumerated {
                            constants: const_symbols,
                            domain_source,
                        },
                    });
                }
            }
        }

        for raw_grounding in definitions {
            if let RawGrounding::Defined(tokens, end, span) = raw_grounding {
                let mut def_stream = TokenStream::with_end(tokens, end);
                let mut pool = FormulaPool::new();
                let mut binders = Vec::new();
                let root = def_stream.parse_formula_expr(&mut pool, &mut binders, &sig_map, 0)?;
                if def_stream.pos != def_stream.tokens.len() {
                    let s = def_stream.peek_span();
                    return Err(SurfaceError::new(
                        ErrorKind::Syntax,
                        s.line,
                        s.column,
                        "unexpected tokens in defined formula",
                    ));
                }
                *raw_grounding = RawGrounding::Resolved(pool_to_formula(pool, root, *span)?);
            }
        }

        // Claims
        let mut claims = Vec::new();
        let mut seen_claims = BTreeSet::new();

        while self.peek_token() == &Token::Claim {
            self.next();
            let (claim_name, claim_span) = self.expect_name()?;
            if !seen_claims.insert(claim_name.clone()) {
                return Err(SurfaceError::new(
                    ErrorKind::DuplicateName,
                    claim_span.line,
                    claim_span.column,
                    format!("duplicate claim name '{}'", claim_name.as_str()),
                ));
            }
            self.expect(&Token::Colon)?;
            let mut pool = FormulaPool::new();
            let mut binders = Vec::new();
            let root = self.parse_formula_expr(&mut pool, &mut binders, &sig_map, 0)?;
            let formula = pool_to_formula(pool, root, claim_span)?;

            let cites = if self.peek_token() == &Token::Cites {
                self.next();
                self.expect(&Token::OpenBracket)?;
                let mut list = Vec::new();
                if self.peek_token() != &Token::CloseBracket {
                    let (cite, _) = self.expect_name()?;
                    list.push(cite);
                    while self.peek_token() == &Token::Comma {
                        self.next();
                        let (cite, _) = self.expect_name()?;
                        list.push(cite);
                    }
                }
                self.expect(&Token::CloseBracket)?;
                list
            } else {
                Vec::new()
            };

            if self.peek_token() == &Token::Semi {
                self.next();
            }

            claims.push(Claim {
                name: claim_name,
                formula,
                cites,
            });
        }

        let mut final_symbols = Vec::with_capacity(raw_symbols.len());
        for rs in raw_symbols {
            let grounding = match rs.raw_grounding {
                RawGrounding::Glossed(text) => Grounding::Glossed(text),
                RawGrounding::Anchored {
                    attestor,
                    procedure,
                } => Grounding::Anchored {
                    attestor: AttestorId(attestor),
                    procedure,
                },
                RawGrounding::Resolved(formula) => Grounding::Defined(formula),
                RawGrounding::Defined(..) => {
                    return Err(SurfaceError::new(
                        ErrorKind::InvalidAst,
                        vocab_span.line,
                        vocab_span.column,
                        "definition was not resolved",
                    ));
                }
            };
            final_symbols.push(SymbolDecl {
                signature: rs.signature,
                grounding,
                gloss_pt: rs.gloss,
            });
        }

        let vocabulary = match render_pt {
            None => Vocabulary::new(version, sorts, final_symbols),
            Some(templates) => {
                Vocabulary::new_with_render_pt(version, sorts, final_symbols, templates)
            }
        }
        .map_err(|e| map_vocab_error(e, vocab_span))?;

        Ok(Document { vocabulary, claims })
    }

    fn parse_optional_grounding(&mut self) -> Result<RawGrounding<'a>, SurfaceError> {
        if self.peek_token() == &Token::Equals {
            self.parse_required_grounding()
        } else {
            let empty = Text::new("").map_err(|e| {
                SurfaceError::new(
                    ErrorKind::InvalidText,
                    self.peek_span().line,
                    self.peek_span().column,
                    e.to_string(),
                )
            })?;
            Ok(RawGrounding::Glossed(empty))
        }
    }

    fn parse_required_grounding(&mut self) -> Result<RawGrounding<'a>, SurfaceError> {
        self.expect(&Token::Equals)?;
        match self.peek_token() {
            Token::Glossed => {
                self.next();
                self.expect(&Token::OpenParen)?;
                let (text, _) = self.expect_string_literal()?;
                self.expect(&Token::CloseParen)?;
                Ok(RawGrounding::Glossed(text))
            }
            Token::Anchored => {
                self.next();
                self.expect(&Token::OpenParen)?;
                let (attestor, _) = self.expect_string_literal()?;
                self.expect(&Token::Comma)?;
                let (procedure, _) = self.expect_string_literal()?;
                self.expect(&Token::CloseParen)?;
                Ok(RawGrounding::Anchored {
                    attestor,
                    procedure,
                })
            }
            Token::Defined => {
                let def_span = self.peek_span();
                self.next();
                self.expect(&Token::OpenParen)?;
                let start_pos = self.pos;
                let mut paren_depth = 1;
                while paren_depth > 0 {
                    match self.peek_token() {
                        Token::OpenParen => {
                            paren_depth += 1;
                            self.next();
                        }
                        Token::CloseParen => {
                            paren_depth -= 1;
                            if paren_depth > 0 {
                                self.next();
                            }
                        }
                        Token::Eof => {
                            let s = self.peek_span();
                            return Err(SurfaceError::new(
                                ErrorKind::Syntax,
                                s.line,
                                s.column,
                                "unclosed parenthesis in defined grounding",
                            ));
                        }
                        _ => {
                            self.next();
                        }
                    }
                }
                let end_pos = self.pos;
                let end = self.peek();
                self.expect(&Token::CloseParen)?;
                let slice = &self.tokens[start_pos..end_pos];
                Ok(RawGrounding::Defined(slice, end, def_span))
            }
            _ => {
                let s = self.peek_span();
                Err(SurfaceError::new(
                    ErrorKind::Syntax,
                    s.line,
                    s.column,
                    "expected 'glossed', 'anchored', or 'defined' in grounding",
                ))
            }
        }
    }

    fn parse_optional_gloss(&mut self) -> Result<Text, SurfaceError> {
        if self.peek_token() == &Token::Gloss {
            self.next();
            let (gloss, _) = self.expect_string_literal()?;
            Ok(gloss)
        } else {
            Text::new("").map_err(|e| {
                SurfaceError::new(
                    ErrorKind::InvalidText,
                    self.peek_span().line,
                    self.peek_span().column,
                    e.to_string(),
                )
            })
        }
    }
}

enum RawSort {
    Open {
        name: Text,
    },
    Enum {
        name: Text,
        constants: Vec<(Text, Span)>,
        domain_source: DomainSource,
    },
}

enum RawGrounding<'a> {
    Glossed(Text),
    Anchored { attestor: Text, procedure: Text },
    Defined(&'a [SpannedToken<'a>], &'a SpannedToken<'a>, Span),
    Resolved(Formula),
}

struct RawSymbol<'a> {
    signature: Symbol,
    #[allow(dead_code)]
    span: Span,
    raw_grounding: RawGrounding<'a>,
    gloss: Text,
}
