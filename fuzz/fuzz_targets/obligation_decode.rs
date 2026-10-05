//! Fuzz target para o decodificador contextual de `obligation/v1`.
//!
//! Contrato normativo: `spec/obligation.md` §2; ADR 0016; etapa 2.3.
//!
//! Testa a robustez do parser contra bytes arbitrários sob vocabulário fixo delimitado,
//! conferindo:
//! - Decodificação com hash recalculado dos bytes originais;
//! - Rejeição imediata por [`DecodeError::HashMismatch`] quando o hash é incorreto;
//! - Idempotência e integridade sobre fixtures válidas geradas pela TCB;
//! - Comportamento com perturbações pontuais em fixtures válidas.

#![no_main]

use libfuzzer_sys::fuzz_target;
use logos_air::{
    Formula, FormulaNode, Limits, Sort, SortDecl, SortDomain, Symbol, SymbolDecl, SymbolKind,
    SymbolResult, Vocabulary,
};
use logos_canon::{Text, to_canon};
use logos_crypto::{Hash, hash, hash_raw};
use logos_obligation::{
    DecodeError, DecodeLimits, EntryKind, LedgerEntry, LedgerError, LedgerView, Obligation,
    PremiseRef, Proposal, decode_for_check, generate,
};
use std::sync::LazyLock;

/// Vocabulário fixo representativo delimitado:
/// Sorts S e T (Open), constante c: -> S, predicado P: S, T -> Bool.
static FIXED_VOCAB: LazyLock<Vocabulary> = LazyLock::new(|| {
    let s = Sort {
        name: Text::new("S").expect("text S"),
    };
    let t = Sort {
        name: Text::new("T").expect("text T"),
    };

    let s_decl = SortDecl {
        name: s.name.clone(),
        domain: SortDomain::Open,
    };
    let t_decl = SortDecl {
        name: t.name.clone(),
        domain: SortDomain::Open,
    };

    let c = Symbol {
        name: Text::new("c").expect("text c"),
        kind: SymbolKind::Const,
        arity: vec![],
        result: SymbolResult::Sort(s.clone()),
    };
    let c_decl = SymbolDecl {
        signature: c,
        grounding: logos_air::Grounding::Glossed(Text::new("const c").expect("gloss")),
        gloss_pt: Text::new("constante c").expect("gloss pt"),
    };

    let p = Symbol {
        name: Text::new("P").expect("text P"),
        kind: SymbolKind::Pred,
        arity: vec![s, t],
        result: SymbolResult::Bool,
    };
    let p_decl = SymbolDecl {
        signature: p,
        grounding: logos_air::Grounding::Glossed(Text::new("pred P").expect("gloss")),
        gloss_pt: Text::new("predicado P").expect("gloss pt"),
    };

    Vocabulary::new(1, vec![s_decl, t_decl], vec![c_decl, p_decl])
        .expect("valid representative vocabulary")
});

/// LedgerView mínimo em memória para geração de fixture válida.
struct StaticLedger;

impl LedgerView for StaticLedger {
    fn current_epoch(&self) -> u64 {
        1
    }

    fn lookup(&self, reference: PremiseRef) -> Result<Option<LedgerEntry<'_>>, LedgerError> {
        if reference.event_hash() != &[0x11; 32] && reference.event_hash() != &[0x22; 32] {
            return Ok(None);
        }
        Ok(Some(LedgerEntry {
            kind: EntryKind::Fact,
            epoch: 1,
            retracted: false,
            vocabulary: &FIXED_VOCAB,
            formula: &FIXED_PREMISE,
        }))
    }
}

static FIXED_PREMISE: LazyLock<Formula> = LazyLock::new(|| {
    Formula::new(vec![FormulaNode::True], Limits::default()).expect("valid premise")
});
/// Fixture válida gerada via TCB determinística de obligation/v1.
static VALID_FIXTURE: LazyLock<(Vec<u8>, Hash<Obligation>, Formula)> = LazyLock::new(|| {
    let goal =
        Formula::new(vec![FormulaNode::True], Limits::default()).expect("valid tautology formula");
    let proposal = Proposal {
        vocabulary: &FIXED_VOCAB,
        goal: &goal,
        cites: &[
            PremiseRef::from_event_hash([0x11; 32]),
            PremiseRef::from_event_hash([0x22; 32]),
        ],
    };
    let obligation = generate(proposal, &StaticLedger).expect("valid obligation generate");
    let bytes = to_canon(&obligation).expect("obligation to_canon");
    let hash = obligation.hash().expect("obligation hash");
    (bytes, hash, goal)
});

fuzz_target!(|data: &[u8]| {
    // 1. Limites estritos para fuzzer (evita alocações desproporcionais)
    let tight_limits = DecodeLimits {
        max_bytes: 4096,
        max_premises: 32,
        air: Limits {
            max_depth: 16,
            max_nodes: 64,
        },
    };

    // 2. Decodificação de bytes arbitrários com hash recalculado
    let recomputed_h = hash_raw::<Obligation>(data);
    let res = decode_for_check(data, recomputed_h, &FIXED_VOCAB, tight_limits);

    // Se aceito, valida invariantes normativas do CheckedObligation
    if let Ok(checked) = res {
        let expected_vocab_h = hash(&*FIXED_VOCAB).expect("vocab hash");
        assert_eq!(
            checked.vocab(),
            expected_vocab_h,
            "vocab hash do objeto aceito deve ser identico ao vocab fornecido"
        );
        assert!(
            checked.premises().len() <= tight_limits.max_premises,
            "quantidade de premissas deve respeitar o limite"
        );
        assert!(
            checked.goal().depth() <= tight_limits.air.max_depth,
            "profundidade da meta deve respeitar o limite"
        );
        let total_nodes = checked.goal().node_count()
            + checked
                .premises()
                .iter()
                .map(|(_, f)| f.node_count())
                .sum::<usize>();
        assert!(
            total_nodes <= tight_limits.air.max_nodes,
            "total de nós deve respeitar o limite agregado"
        );
        for (_, premise) in checked.premises() {
            assert!(premise.depth() <= tight_limits.air.max_depth);
        }

        // Referências de premissas estritamente crescentes
        for window in checked.premises().windows(2) {
            assert!(
                window[0].0 < window[1].0,
                "premissas aceitas devem ser estritamente crescentes e únicas"
            );
        }
    }

    // 3. Decodificação com hash incorreto (inversão de bits no primeiro byte)
    // Se o tamanho estiver dentro de max_bytes, DEVE recusar com HashMismatch
    let mut wrong_bytes = *recomputed_h.as_bytes();
    wrong_bytes[0] ^= 0xff;
    let wrong_h = Hash::<Obligation>::from_bytes(wrong_bytes);

    let res_wrong = decode_for_check(data, wrong_h, &FIXED_VOCAB, tight_limits);
    if data.len() <= tight_limits.max_bytes {
        assert!(
            matches!(res_wrong, Err(DecodeError::HashMismatch)),
            "entrada com hash incorreto deve ser rejeitada com HashMismatch: {res_wrong:?}"
        );
    }

    // 4. Validação da fixture canônica gerada (idempotência básica)
    let (valid_bytes, valid_hash, valid_goal) = &*VALID_FIXTURE;
    let default_limits = DecodeLimits::default();
    let checked_valid = decode_for_check(valid_bytes, *valid_hash, &FIXED_VOCAB, default_limits)
        .expect("fixture canônica válida deve ser aceita");
    assert_eq!(checked_valid.goal(), valid_goal);
    assert_eq!(checked_valid.premises().len(), 2);

    // 5. Perturbação de fixture canônica com bytes da entrada do fuzzer
    if !data.is_empty() {
        let mut mutated = valid_bytes.clone();
        let idx = (data[0] as usize) % mutated.len();
        mutated[idx] ^= data.get(1).copied().unwrap_or(0x55);

        // Com hash recalculado sobre a mutação: exercita o parser em estruturas quase válidas
        let mutated_h = hash_raw::<Obligation>(&mutated);
        let _ = decode_for_check(&mutated, mutated_h, &FIXED_VOCAB, tight_limits);

        // Com hash original da fixture: se os bytes mudaram e cabem no limite, deve falhar com HashMismatch
        if mutated != *valid_bytes && mutated.len() <= tight_limits.max_bytes {
            let res_mut_wrong = decode_for_check(&mutated, *valid_hash, &FIXED_VOCAB, tight_limits);
            assert!(
                matches!(res_mut_wrong, Err(DecodeError::HashMismatch)),
                "mutação com hash desatualizado deve falhar com HashMismatch"
            );
        }
    }
});
