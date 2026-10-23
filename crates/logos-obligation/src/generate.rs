//! Algoritmo de geração determinística de obrigações canônicas.
//!
//! Contrato normativo: `spec/obligation.md` §3; ADR 0012.

use logos_air::Compatibility;

use crate::error::Reject;
use crate::types::{EntryKind, LedgerView, Obligation, PremiseRef, Proposal};

/// Gera uma obrigação canônica selada $\Gamma \vdash \varphi$ a partir de uma proposta e um snapshot do ledger.
///
/// Implementa o algoritmo normativo de `spec/obligation.md` §3 e ADR 0012:
///
/// 1. Verifica sorts do objetivo (`goal`) contra o vocabulário alvo;
/// 2. Calcula $h(V_{\text{alvo}})$;
/// 3. Ordena e deduplica as referências citadas; lê a época corrente do snapshot uma única vez;
/// 4. Para cada referência na ordem canônica estritamente crescente:
///    - Consulta no snapshot (`lookup`);
///    - Confere não-retratação, época de aplicabilidade e categoria citável;
///    - Confere compatibilidade direcional ($V_{\text{alvo}} \ge V_{\text{origem}}$);
///    - Verifica sorts da fórmula da premissa contra o vocabulário alvo;
/// 5. Constrói e retorna a obrigação canônica imutável.
///
/// # Errors
/// Retorna [`Reject`] na primeira condição inválida encontrada de acordo com a precedência
/// normativa, diagnosticando referências inválidas pela menor referência ordenada.
pub fn generate(proposal: Proposal<'_>, view: &impl LedgerView) -> Result<Obligation, Reject> {
    generate_with(proposal, &[], view)
}

/// Gera uma obrigação especulativa (Etapa 5.8, ADR 0058): $\Gamma$ inclui as
/// citações e as bases `speculative`, cada base com a fórmula do seu `Pending`.
///
/// Mesmo algoritmo de [`generate`] sobre a união ordenada e deduplicada; uma
/// referência de `speculative` precisa ter kind [`EntryKind::Pending`], e as demais
/// continuam exigindo kind citável. Qualquer outro kind é `NotCitable`.
///
/// # Errors
/// As mesmas recusas de [`generate`], na mesma precedência.
pub fn generate_speculative(
    proposal: Proposal<'_>,
    speculative: &[PremiseRef],
    view: &impl LedgerView,
) -> Result<Obligation, Reject> {
    generate_with(proposal, speculative, view)
}

fn generate_with(
    proposal: Proposal<'_>,
    speculative: &[PremiseRef],
    view: &impl LedgerView,
) -> Result<Obligation, Reject> {
    // 1. check(V_alvo, goal); falha => Reject::GoalSort(SortError)
    logos_air::check(proposal.vocabulary, proposal.goal).map_err(Reject::GoalSort)?;

    // 2. Calcular h(V_alvo); erro canônico => Reject::Canon(CanonError)
    let vocab_hash = logos_crypto::hash(proposal.vocabulary).map_err(Reject::Canon)?;

    // 3. Copiar referências, ordenar e deduplicar; ler época do snapshot uma vez.
    let mut refs = proposal.cites.to_vec();
    refs.extend_from_slice(speculative);
    refs.sort_unstable();
    refs.dedup();
    let current_epoch = view.current_epoch();

    // 4..7. Processamento das referências ordenadas e deduplicadas.
    let mut premises = Vec::with_capacity(refs.len());
    for reference in refs {
        // 4. lookup; erro => Ledger, None => UnknownCitation
        let entry = match view.lookup(reference) {
            Ok(Some(entry)) => entry,
            Ok(None) => return Err(Reject::UnknownCitation(reference)),
            Err(error) => return Err(Reject::Ledger { reference, error }),
        };

        // 5. Se retracted => Retracted; época diferente => InvalidEpoch; kind fora de citáveis => NotCitable
        if entry.retracted {
            return Err(Reject::Retracted(reference));
        }

        if entry.expired {
            return Err(Reject::Expired(reference));
        }

        if entry.epoch != current_epoch {
            return Err(Reject::InvalidEpoch {
                reference,
                expected: current_epoch,
                actual: entry.epoch,
            });
        }

        let eligible = if speculative.contains(&reference) {
            entry.kind == EntryKind::Pending
        } else {
            entry.kind.is_citable()
        };
        if !eligible {
            return Err(Reject::NotCitable {
                reference,
                kind: entry.kind,
            });
        }

        // 6. Exigir V_alvo.compatibility_with(V_origem) == Compatible (inclui igualdade)
        // Atalho ptr::eq para mesmo vocabulário evita varredura redundante
        let target_vocab = proposal.vocabulary;
        let origin_vocab = entry.vocabulary;
        let compatible = core::ptr::eq(target_vocab, origin_vocab)
            || target_vocab.compatibility_with(origin_vocab) == Compatibility::Compatible;
        if !compatible {
            return Err(Reject::IncompatibleVocabulary(reference));
        }

        // 7. check(V_alvo, premise.formula); falha => PremiseSort. Copiar fórmula aceita para armazenamento próprio.
        logos_air::check(target_vocab, entry.formula)
            .map_err(|error| Reject::PremiseSort { reference, error })?;

        premises.push((reference, entry.formula.clone()));
    }

    // 8. Construir O privado com lógica, encoder 1, h(V_alvo), premissas e goal próprios.
    Ok(Obligation::new(
        proposal.goal.logic_id(),
        1,
        vocab_hash,
        premises,
        proposal.goal.clone(),
    ))
}
