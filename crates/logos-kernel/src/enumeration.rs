//! Fonte de sort enumerado (6.1b, ADR0062): `DomainSource::Fact(r)` só vale com
//! um fato registrado, vivo e aplicável cuja fórmula é a conjunção fecho ∧ nomes
//! únicos de V, com `r = h(conjunção)`, citado como premissa de O. Política fica
//! recusada até a 7.3.

use logos_air::{DomainSource, Sort, SortDomain, Vocabulary};
use logos_crypto::hash;
use logos_ground::{GroundError, domain_axioms};
use logos_obligation::{EntryKind, LedgerView, Obligation};

use crate::TransportError as Error;

/// Confere cada sort enumerado de V contra as premissas de O e a view que as gerou.
pub(crate) fn check_sources(
    vocabulary: &Vocabulary,
    obligation: &Obligation,
    view: &impl LedgerView,
) -> Result<(), Error> {
    for decl in vocabulary.sorts() {
        let SortDomain::Enumerated { domain_source, .. } = &decl.domain else {
            continue;
        };
        let DomainSource::Fact(expected) = domain_source else {
            return Err(Error::Invalid("unresolved enumeration source"));
        };
        let attested = domain_axioms(
            vocabulary,
            &Sort {
                name: decl.name.clone(),
            },
        )
        .and_then(|axioms| axioms.conjunction())
        .map_err(|error| match error {
            GroundError::Unknown(_) => Error::Limit("enumeration axioms"),
            _ => Error::Invalid("enumeration axioms"),
        })?;
        if hash(&attested)?.as_bytes() != &expected.0 {
            return Err(Error::Invalid(
                "enumeration source does not match vocabulary",
            ));
        }
        let mut found = false;
        for (reference, formula) in obligation.premises() {
            if *formula != attested {
                continue;
            }
            let entry = view
                .lookup(*reference)
                .map_err(|_| Error::Invalid("enumeration source lookup"))?
                .ok_or(Error::Invalid("enumeration source lookup"))?;
            // A mesma fórmula como hipótese ou Verified não autentica a fonte.
            if entry.kind != EntryKind::Fact {
                return Err(Error::Invalid(
                    "enumeration source is not a registered fact",
                ));
            }
            found = true;
        }
        if !found {
            return Err(Error::Invalid("missing enumeration source fact"));
        }
    }
    Ok(())
}
