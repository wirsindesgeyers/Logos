//! `enum-source`: atesta o fecho de um domínio ("os usuários são exatamente {…}").
//!
//! O fato afirma a conjunção fecho ∧ nomes distintos do sort enumerado e é a fonte
//! (`DomainSource::Fact`) que o kernel exige para usar o sort (6.1b, ADR 0062).

use logos_air::{DomainSource, FactRef, SortDecl, SortDomain, Symbol, SymbolDecl, Vocabulary};
use logos_canon::Text;
use logos_crypto::hash;

use crate::sdk::{
    AttestError, Attester, Observation, anchored, constant, resource_version, sort, until,
};

/// Identificador do atestador.
pub const ID: &str = "enum-source";
/// Procedimento (documento `docs/attesters/enum-source.md`).
pub const PROCEDURE: &str = "enum-source/closure/v1";
/// Tópico atestado.
pub const TOPIC: &str = "domain-closure";

/// Atesta o fecho de um sort enumerado.
#[derive(Debug)]
pub struct EnumSource {
    sort_name: String,
    members: Vec<String>,
    ttl_secs: u64,
}

impl EnumSource {
    /// Sort `sort_name` com exatamente `members` (ordem preservada).
    pub fn new(
        sort_name: impl Into<String>,
        members: impl IntoIterator<Item = impl Into<String>>,
        ttl_secs: u64,
    ) -> Self {
        Self {
            sort_name: sort_name.into(),
            members: members.into_iter().map(Into::into).collect(),
            ttl_secs,
        }
    }
}

fn build(sort_name: &str, members: &[String], source: FactRef) -> Result<Vocabulary, AttestError> {
    let of = sort(sort_name)?;
    let constants: Vec<Symbol> = members
        .iter()
        .map(|m| constant(m, &of))
        .collect::<Result<_, _>>()?;
    let decls: Vec<SymbolDecl> = constants
        .iter()
        .map(|c| {
            anchored(
                c.clone(),
                ID,
                PROCEDURE,
                "membro do domínio fechado pelo atestador",
            )
        })
        .collect::<Result<_, _>>()?;
    Ok(Vocabulary::new(
        1,
        vec![SortDecl {
            name: Text::new(sort_name)?,
            domain: SortDomain::Enumerated {
                constants,
                domain_source: DomainSource::Fact(source),
            },
        }],
        decls,
    )?)
}

/// Vocabulário do sort e a fórmula que o fato de fonte afirma. `FactRef` é o hash da
/// fórmula (não do fato), o que desfaz o ciclo vocabulário ↔ fato.
///
/// # Errors
/// Nomes inválidos, constantes repetidas, limites de AIR.
pub fn closure(
    sort_name: &str,
    members: &[String],
) -> Result<(Vocabulary, logos_air::Formula), AttestError> {
    let draft = build(sort_name, members, FactRef([0; 32]))?;
    let conjunction = logos_ground::domain_axioms(&draft, &sort(sort_name)?)
        .and_then(|axioms| axioms.conjunction())
        .map_err(|e| AttestError::Observation(format!("closure formula: {e:?}")))?;
    let source = FactRef(*hash(&conjunction)?.as_bytes());
    let vocabulary = build(sort_name, members, source)?;
    let check = logos_ground::domain_axioms(&vocabulary, &sort(sort_name)?)
        .and_then(|axioms| axioms.conjunction())
        .map_err(|e| AttestError::Observation(format!("closure formula: {e:?}")))?;
    if check != conjunction {
        return Err(AttestError::Observation(
            "closure formula depends on its own source reference".into(),
        ));
    }
    Ok((vocabulary, conjunction))
}

impl Attester for EnumSource {
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
        let (vocabulary, formula) = closure(&self.sort_name, &self.members)?;
        Ok(Observation {
            vocabulary,
            formula,
            version: Some(resource_version(
                &self.sort_name,
                &format!("members={}", self.members.join(",")),
            )?),
            valid: until(now, self.ttl_secs)?,
        })
    }
}
