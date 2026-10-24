//! Conteúdo externo atribuído (6.6, ADR0068): `Extracted` é premissa "fonte diz φ", nunca fato.
//!
//! `φ` precisa ser um literal `Says`-lifted: um átomo (ou sua negação) de predicado
//! declarado `Anchored(ingest, says-lift/v1)` cujo primeiro argumento é a constante de sort
//! `Source` que nomeia a fonte e cujos demais argumentos são fechados. Como `Says_P` e `P` são
//! símbolos distintos, a premissa nunca satisfaz uma obrigação que exige `P`: só uma regra de
//! política explícita (`Says_P(s, x) ∧ Externo(s) → P(x)`) faz a ponte.

use logos_air::{FormulaNode, Grounding, Limits, SymbolResult, Term, TermNode, Vocabulary};
use logos_canon::Text;
use logos_crypto::{Hash, KeyId, Signer, hash};
use logos_ledger::{Event, EventBody, Ledger};

use crate::TransportError as Error;

/// `AttestorId` dos símbolos `Says`-lifted.
pub const SAYS_ATTESTOR: &str = "ingest";
/// Procedimento dos símbolos `Says`-lifted.
pub const SAYS_PROCEDURE: &str = "says-lift/v1";
/// Sort das fontes.
pub const SOURCE_SORT: &str = "Source";
/// Tópico de confiança que o extrator precisa ter.
pub const EXTRACTION_TOPIC: &str = "extraction";

fn is_ground(term: &Term) -> bool {
    term.nodes()
        .iter()
        .all(|node| matches!(node, TermNode::App { .. }))
}

/// Confere que `formula` é `Says_P(source, t…)` ou `¬Says_P(source, t…)` em `vocabulary`.
fn check_says_literal(
    vocabulary: &Vocabulary,
    formula: &logos_air::Formula,
    source: &str,
) -> Result<(), Error> {
    let atom = match formula.nodes() {
        [atom @ FormulaNode::Pred { .. }] | [FormulaNode::Not, atom @ FormulaNode::Pred { .. }] => {
            atom
        }
        _ => return Err(Error::Invalid("extracted formula must be a Says literal")),
    };
    let FormulaNode::Pred { symbol, args } = atom else {
        return Err(Error::Invalid("extracted formula must be a Says literal"));
    };
    let declared = vocabulary
        .symbols()
        .iter()
        .find(|d| d.signature == *symbol)
        .ok_or(Error::Invalid(
            "extracted predicate is not in the vocabulary",
        ))?;
    match &declared.grounding {
        Grounding::Anchored {
            attestor,
            procedure,
        } if attestor.0.as_str() == SAYS_ATTESTOR && procedure.as_str() == SAYS_PROCEDURE => {}
        _ => {
            return Err(Error::Invalid(
                "extracted predicate is not a Says-lifted symbol",
            ));
        }
    }
    let (Some(first_sort), Some(first)) = (symbol.arity.first(), args.first()) else {
        return Err(Error::Invalid("Says predicate needs a source argument"));
    };
    if first_sort.name.as_str() != SOURCE_SORT {
        return Err(Error::Invalid("Says first argument must have sort Source"));
    }
    match first.nodes() {
        [
            TermNode::App {
                symbol: constant,
                args: 0,
            },
        ] if constant.name.as_str() == source
            && matches!(&constant.result, SymbolResult::Sort(s) if s.name.as_str() == SOURCE_SORT) =>
            {}
        _ => {
            return Err(Error::Invalid(
                "Says first argument must be the declared source constant",
            ));
        }
    }
    if !args.iter().all(is_ground) {
        return Err(Error::Invalid("extracted formula must be closed"));
    }
    Ok(())
}

/// Registra `Extracted { source diz formula }` extraída do conteúdo de hash `content`.
///
/// `extractor` é a identidade que o kernel autenticou para o extrator (a conexão/UID, na
/// Fase 11); precisa de `Trusted(extractor, "extraction")` vigente e não pode estar revogado.
///
/// # Errors
/// Extrator sem confiança ou revogado; vocabulário ausente; fórmula mal tipada ou que não é
/// literal `Says`-lifted fechado da fonte; I/O do ledger. Recusa não grava evento.
pub fn register<S: Signer>(
    ledger: &mut Ledger<'_, S>,
    extractor: KeyId,
    vocabulary: Hash<Vocabulary>,
    source: &str,
    content: [u8; 32],
    formula: logos_air::Formula,
) -> Result<Hash<Event>, Error> {
    if ledger.live_index()?.issuer_revoked(&extractor)? {
        return Err(Error::Unauthorized);
    }
    if ledger
        .live_index()?
        .trust_grant(&extractor, EXTRACTION_TOPIC)?
        .is_none()
    {
        return Err(Error::Unauthorized);
    }
    let bytes = ledger
        .cas()
        .get(&vocabulary)?
        .ok_or(Error::Invalid("extraction vocabulary missing"))?;
    let decoded = Vocabulary::decode_with_limits(&bytes, Limits::default())
        .map_err(|_| Error::Invalid("extraction vocabulary"))?;
    logos_air::check(&decoded, &formula).map_err(|_| Error::Invalid("extracted formula sort"))?;
    check_says_literal(&decoded, &formula, source)?;
    Ok(hash(&ledger.append(EventBody::Extracted {
        vocabulary,
        source: Text::new(source)?,
        content,
        formula,
        extractor,
    })?)?)
}
