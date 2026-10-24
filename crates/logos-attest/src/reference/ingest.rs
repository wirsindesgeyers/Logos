//! `ingest`: atesta a proveniência dos bytes de um conteúdo externo, não o significado.
//!
//! Para o conteúdo `c` da fonte `s`, afirma `content_of(s, h(c)) ∧ origin_is(s, origem)`.
//! Também fornece o levantamento `Says` (6.6, ADR 0068): para cada predicado `P(a…)` do
//! vocabulário, `says_P(Source, a…)`, um símbolo distinto de `P` e, por isso, impossível de
//! confundir com um fato: só uma regra de política explícita leva de `says_P` a `P`.

use std::{fs, io::Read, path::PathBuf};

use logos_air::{
    Formula, FormulaNode, Limits, SortDecl, SortDomain, Symbol, SymbolDecl, Vocabulary,
};
use logos_canon::Text;
use logos_crypto::ExternalDigest;
use logos_kernel::extracted::{SAYS_ATTESTOR, SAYS_PROCEDURE, SOURCE_SORT};

use crate::sdk::{
    AttestError, Attester, Observation, anchored, atom, constant, predicate, resource_version,
    sort, until,
};

/// Identificador do atestador (também o `AttestorId` dos símbolos `says_*`).
pub const ID: &str = SAYS_ATTESTOR;
/// Procedimento de `content_of` e `origin_is`.
pub const PROCEDURE: &str = "ingest/content-origin/v1";
/// Procedimento dos símbolos levantados `says_*`.
pub const SAYS: &str = SAYS_PROCEDURE;
/// Tópico atestado.
pub const TOPIC: &str = "ingestion";
/// Teto de leitura do conteúdo.
pub const MAX_BYTES: u64 = 64 * 1024 * 1024;

/// Origem declarada do conteúdo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Fora do sistema (e-mail, web, documento recebido).
    External,
    /// Produzido dentro do sistema.
    Internal,
    /// Digitado pelo usuário.
    User,
}

impl Origin {
    /// Nome da constante no vocabulário.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::External => "origin_external",
            Self::Internal => "origin_internal",
            Self::User => "origin_user",
        }
    }
}

/// Nome da constante de um digest de conteúdo.
#[must_use]
pub fn digest_name(digest: &ExternalDigest) -> String {
    format!("sha256:{}", digest.to_hex())
}

/// Levanta `P(a…)` para `says_P(Source, a…)`, `Anchored(ingest, says-lift/v1)`.
///
/// # Errors
/// Nome inválido ou `pred` que não é predicado.
pub fn says_lift(pred: &Symbol) -> Result<SymbolDecl, AttestError> {
    if pred.kind != logos_air::SymbolKind::Pred {
        return Err(AttestError::Invalid(
            "only predicates can be lifted to Says",
        ));
    }
    let mut arity = vec![sort(SOURCE_SORT)?];
    arity.extend(pred.arity.iter().cloned());
    anchored(
        predicate(&format!("says_{}", pred.name.as_str()), arity)?,
        ID,
        SAYS,
        &format!("a fonte diz: {}", pred.name.as_str()),
    )
}

/// Declaração de uma fonte (constante de sort `Source`) para o vocabulário.
///
/// # Errors
/// Nome inválido.
pub fn source_decl(name: &str) -> Result<SymbolDecl, AttestError> {
    anchored(
        constant(name, &sort(SOURCE_SORT)?)?,
        ID,
        PROCEDURE,
        "fonte de conteúdo externo",
    )
}

/// Sorts abertos usados pela ingestão.
///
/// # Errors
/// Nome inválido.
pub fn sorts() -> Result<Vec<SortDecl>, AttestError> {
    ["Source", "Digest", "Origin"]
        .into_iter()
        .map(|n| {
            Ok(SortDecl {
                name: Text::new(n)?,
                domain: SortDomain::Open,
            })
        })
        .collect()
}

/// Símbolos da ingestão para a fonte e o digest dados.
///
/// # Errors
/// Nomes inválidos.
pub fn declarations(
    source: &str,
    digest: &ExternalDigest,
) -> Result<(Vec<SortDecl>, Vec<SymbolDecl>), AttestError> {
    let (source_sort, digest_sort, origin_sort) =
        (sort("Source")?, sort("Digest")?, sort("Origin")?);
    let mut decls = vec![
        source_decl(source)?,
        anchored(
            constant(&digest_name(digest), &digest_sort)?,
            ID,
            PROCEDURE,
            "digest SHA-256 do conteúdo ingerido",
        )?,
    ];
    for origin in [Origin::External, Origin::Internal, Origin::User] {
        decls.push(anchored(
            constant(origin.name(), &origin_sort)?,
            ID,
            PROCEDURE,
            "origem declarada do conteúdo",
        )?);
    }
    decls.push(anchored(
        predicate("content_of", vec![source_sort.clone(), digest_sort])?,
        ID,
        PROCEDURE,
        "o conteúdo da fonte tem este digest SHA-256",
    )?);
    decls.push(anchored(
        predicate("origin_is", vec![source_sort, origin_sort])?,
        ID,
        PROCEDURE,
        "a fonte tem esta origem",
    )?);
    Ok((sorts()?, decls))
}

/// Ingere um arquivo de conteúdo como a fonte `source`.
#[derive(Debug)]
pub struct Ingest {
    source: String,
    path: PathBuf,
    origin: Origin,
    ttl_secs: u64,
}

impl Ingest {
    /// `source` nomeia a fonte no vocabulário; `path` é o conteúdo bruto.
    pub fn new(
        source: impl Into<String>,
        path: impl Into<PathBuf>,
        origin: Origin,
        ttl_secs: u64,
    ) -> Self {
        Self {
            source: source.into(),
            path: path.into(),
            origin,
            ttl_secs,
        }
    }
}

fn conjunction(parts: &[&Formula]) -> Result<Formula, AttestError> {
    let mut nodes = vec![FormulaNode::And(parts.len())];
    for part in parts {
        nodes.extend_from_slice(part.nodes());
    }
    Ok(Formula::new(nodes, Limits::default())?)
}

impl Attester for Ingest {
    fn id(&self) -> &str {
        ID
    }
    fn topic(&self) -> &str {
        TOPIC
    }
    fn procedure(&self) -> &'static str {
        PROCEDURE
    }
    fn observe(&mut self, now: u64) -> Result<Observation, AttestError> {
        let meta = fs::symlink_metadata(&self.path)?;
        if !meta.is_file() || meta.len() > MAX_BYTES {
            return Err(AttestError::Observation(
                "content must be a regular file up to 64 MiB (no symbolic links)".into(),
            ));
        }
        let mut bytes = Vec::new();
        fs::File::open(&self.path)?
            .take(MAX_BYTES)
            .read_to_end(&mut bytes)?;
        let digest = ExternalDigest::of(&bytes);
        let (sorts, decls) = declarations(&self.source, &digest)?;
        let find = |name: &str| {
            decls
                .iter()
                .find(|d| d.signature.name.as_str() == name)
                .map(|d| d.signature.clone())
                .ok_or(AttestError::Invalid("symbol missing from declarations"))
        };
        let (source, content_digest, origin) = (
            find(&self.source)?,
            find(&digest_name(&digest))?,
            find(self.origin.name())?,
        );
        let content = atom(&find("content_of")?, &[&source, &content_digest])?;
        let origin_fact = atom(&find("origin_is")?, &[&source, &origin])?;
        Ok(Observation {
            vocabulary: Vocabulary::new(1, sorts, decls)?,
            formula: conjunction(&[&content, &origin_fact])?,
            version: Some(resource_version(
                &self.source,
                &format!("sha256={};size={}", digest.to_hex(), bytes.len()),
            )?),
            valid: until(now, self.ttl_secs)?,
        })
    }
}
