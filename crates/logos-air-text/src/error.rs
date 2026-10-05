//! Erros e categorias normativas de sintaxe de superfície AIR.

use std::fmt;

/// Categorias normativas de erro na sintaxe de superfície AIR.
///
/// Conforme spec/air-surface.md §1: Syntax, UnknownName, DuplicateName,
/// InvalidVocabulary, InvalidAst, InvalidText, LimitExceeded, UnknownReference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// Erro de sintaxe nos tokens ou na estrutura gramatical.
    Syntax,
    /// Nome de sort, símbolo ou variável não encontrado.
    UnknownName,
    /// Nome duplicado no mesmo namespace ou claims repetidos.
    DuplicateName,
    /// Invariante do vocabulário violada.
    InvalidVocabulary,
    /// Construção da AST inválida.
    InvalidAst,
    /// String com escapes inválidos, controles sem escape, surrogates isolados, não-NFC ou code points não atribuídos.
    InvalidText,
    /// Limite lógico ou textual excedido (profundidade, nós, u64 overflow).
    LimitExceeded,
    /// Referência de fato ou política não encontrada no mapa de referências e não é digest hex válido.
    UnknownReference,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax => f.write_str("Syntax"),
            Self::UnknownName => f.write_str("UnknownName"),
            Self::DuplicateName => f.write_str("DuplicateName"),
            Self::InvalidVocabulary => f.write_str("InvalidVocabulary"),
            Self::InvalidAst => f.write_str("InvalidAst"),
            Self::InvalidText => f.write_str("InvalidText"),
            Self::LimitExceeded => f.write_str("LimitExceeded"),
            Self::UnknownReference => f.write_str("UnknownReference"),
        }
    }
}

/// Erro de parsing ou resolução com categoria e coordenadas de origem 1-based.
///
/// Colunas são contadas em Unicode scalar values (code points `char`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceError {
    /// Categoria normativa do erro.
    pub kind: ErrorKind,
    /// Linha 1-based onde ocorreu o erro.
    pub line: usize,
    /// Coluna 1-based (em Unicode scalar values) onde ocorreu o erro.
    pub column: usize,
    /// Mensagem descritiva (não faz parte da identidade canônica).
    pub message: String,
}

impl SurfaceError {
    /// Constrói um novo erro com categoria, posição 1-based e mensagem descritiva.
    #[must_use]
    pub fn new(kind: ErrorKind, line: usize, column: usize, message: impl Into<String>) -> Self {
        Self {
            kind,
            line,
            column,
            message: message.into(),
        }
    }
}

impl fmt::Display for SurfaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}: {}: {}",
            self.line, self.column, self.kind, self.message
        )
    }
}

impl std::error::Error for SurfaceError {}
