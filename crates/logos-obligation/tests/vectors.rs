//! Verificação dos 20 vetores canônicos de obligation/v1 contra a TCB.
//!
//! Contrato normativo: `spec/obligation.md` §2 e §5; ADR 0012.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;
use std::fmt::Write;

use logos_air::{Formula, LogicId, Vocabulary};
use logos_canon::{from_canon, to_canon};
use logos_obligation::{
    EntryKind, LedgerEntry, LedgerError, LedgerView, PremiseRef, Proposal, generate,
};
use serde_json::Value;

fn unhex(hex: &str) -> Vec<u8> {
    assert!(
        hex.len().is_multiple_of(2),
        "hex string length must be even, got {}",
        hex.len()
    );
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .unwrap_or_else(|e| panic!("invalid hex byte at {i}: {e}"))
        })
        .collect()
}

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        write!(&mut s, "{:02x}", b).expect("hex write");
    }
    s
}

fn parse_ref(hex: &str) -> PremiseRef {
    let bytes = unhex(hex);
    assert_eq!(bytes.len(), 32, "PremiseRef must be exactly 32 bytes");
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    PremiseRef::from_event_hash(arr)
}

fn parse_kind(s: &str) -> EntryKind {
    match s {
        "Fact" => EntryKind::Fact,
        "Policy" => EntryKind::Policy,
        "Verified" => EntryKind::Verified,
        "Assumption" => EntryKind::Assumption,
        "Pending" => EntryKind::Pending,
        "Refuted" => EntryKind::Refuted,
        "Unknown" => EntryKind::Unknown,
        "Rejected" => EntryKind::Rejected,
        "Invalidated" => EntryKind::Invalidated,
        "Proposal" => EntryKind::Proposal,
        other => panic!("unknown EntryKind string: {other}"),
    }
}

struct InMemoryLedgerView<'a> {
    current_epoch: u64,
    entries: HashMap<PremiseRef, LedgerEntry<'a>>,
}

impl<'a> LedgerView for InMemoryLedgerView<'a> {
    fn current_epoch(&self) -> u64 {
        self.current_epoch
    }

    fn lookup(&self, reference: PremiseRef) -> Result<Option<LedgerEntry<'_>>, LedgerError> {
        Ok(self.entries.get(&reference).cloned())
    }
}

#[test]
fn test_all_20_obligation_vectors() {
    let raw_json = include_str!("../../../spec/vectors/obligation/vectors.json");
    let root: Value = serde_json::from_str(raw_json).expect("valid vectors JSON");
    let vectors = root["vectors"].as_array().expect("vectors array");

    assert_eq!(
        vectors.len(),
        20,
        "spec/vectors/obligation must have exactly 20 vectors"
    );

    let mut generated_obligations = Vec::new();

    for (vec_idx, vec_val) in vectors.iter().enumerate() {
        let id = vec_val["id"].as_str().expect("id string");
        let description = vec_val["description"].as_str().expect("description");
        let epoch = vec_val["epoch"].as_u64().expect("epoch u64");

        let target_v_hex = vec_val["target_vocabulary_hex"]
            .as_str()
            .expect("target_vocab_hex");
        let goal_hex = vec_val["goal_hex"].as_str().expect("goal_hex");

        let target_vocab_bytes = unhex(target_v_hex);
        let target_vocab =
            from_canon::<Vocabulary>(&target_vocab_bytes).expect("target vocabulary from_canon");

        let goal_bytes = unhex(goal_hex);
        let goal = from_canon::<Formula>(&goal_bytes).expect("goal formula from_canon");

        let raw_premises = vec_val["premises"].as_array().expect("premises array");

        // Decodificar vocabulários de origem e fórmulas das premissas
        let mut decoded_origin_vocabs = Vec::with_capacity(raw_premises.len());
        let mut decoded_formulas = Vec::with_capacity(raw_premises.len());
        let mut premise_specs = Vec::with_capacity(raw_premises.len());

        for p_val in raw_premises {
            let ref_hex = p_val["reference"].as_str().expect("ref_hex");
            let p_ref = parse_ref(ref_hex);
            let kind = parse_kind(p_val["kind"].as_str().expect("kind"));
            let p_epoch = p_val["epoch"].as_u64().expect("premise epoch");
            let retracted = p_val["retracted"].as_bool().expect("retracted bool");

            let orig_v_hex = p_val["origin_vocabulary_hex"].as_str();
            let opt_orig_v = orig_v_hex.map(|h| {
                from_canon::<Vocabulary>(&unhex(h)).expect("origin vocabulary from_canon")
            });
            decoded_origin_vocabs.push(opt_orig_v);

            let f_hex = p_val["formula_hex"].as_str().expect("formula_hex");
            let formula = from_canon::<Formula>(&unhex(f_hex)).expect("premise formula from_canon");
            decoded_formulas.push(formula);

            premise_specs.push((p_ref, kind, p_epoch, retracted));
        }

        // Construir entradas do InMemoryLedgerView
        let mut entries = HashMap::new();
        for (i, &(p_ref, kind, p_epoch, retracted)) in premise_specs.iter().enumerate() {
            let v_ref = match &decoded_origin_vocabs[i] {
                Some(v) => v,
                None => &target_vocab,
            };
            let f_ref = &decoded_formulas[i];

            entries.insert(
                p_ref,
                LedgerEntry {
                    kind,
                    epoch: p_epoch,
                    retracted,
                    expired: false,
                    untrusted: false,
                    vocabulary: v_ref,
                    formula: f_ref,
                },
            );
        }

        let view = InMemoryLedgerView {
            current_epoch: epoch,
            entries,
        };

        // Construir citações da proposta
        let raw_cites = vec_val["cites"].as_array().expect("cites array");
        let cites: Vec<PremiseRef> = raw_cites
            .iter()
            .map(|c| parse_ref(c.as_str().expect("cite hex")))
            .collect();

        let proposal = Proposal {
            vocabulary: &target_vocab,
            goal: &goal,
            cites: &cites,
        };

        // Gerar obrigação canônica
        let obligation = generate(proposal, &view).unwrap_or_else(|reject| {
            panic!("vector {id} ({description}) falhou inesperadamente: {reject:?}")
        });

        // Conferir accessors públicos
        assert_eq!(obligation.logic(), LogicId::FolMsEqV1, "{id}: logic_id");
        assert_eq!(obligation.encoder_version(), 1, "{id}: encoder_version");
        assert_eq!(obligation.goal(), &goal, "{id}: goal");

        let expected_val = &vec_val["expected"];
        let expected_v_hash = expected_val["vocab_hash"].as_str().expect("vocab_hash");
        assert_eq!(
            to_hex(obligation.vocab().as_bytes()),
            expected_v_hash,
            "{id}: vocab hash"
        );

        let expected_canonical_premises = expected_val["canonical_premises"]
            .as_array()
            .expect("canonical_premises");
        assert_eq!(
            obligation.premises().len(),
            expected_canonical_premises.len(),
            "{id}: premises count"
        );

        for (i, (actual_ref, actual_formula)) in obligation.premises().iter().enumerate() {
            let exp_premise = &expected_canonical_premises[i];
            let exp_ref_hex = exp_premise["reference"].as_str().expect("ref");
            let exp_f_hex = exp_premise["formula_hex"].as_str().expect("formula_hex");

            assert_eq!(
                to_hex(actual_ref.event_hash()),
                exp_ref_hex,
                "{id}: premise {i} reference"
            );
            assert_eq!(
                to_hex(&to_canon(actual_formula).expect("canon formula")),
                exp_f_hex,
                "{id}: premise {i} formula canon"
            );
        }

        // Conferir envelope canônico completo
        let actual_canon = to_canon(&obligation).expect("obligation to_canon");
        let expected_canon_hex = expected_val["canonical_hex"].as_str().expect("canon_hex");
        assert_eq!(
            to_hex(&actual_canon),
            expected_canon_hex,
            "{id}: canon bytes"
        );

        // Conferir hash da obrigação h(O)
        let actual_hash = logos_crypto::hash(&obligation).expect("canonical obligation hash");
        let expected_hash_hex = expected_val["obligation_hash"].as_str().expect("ob_hash");
        assert_eq!(
            to_hex(actual_hash.as_bytes()),
            expected_hash_hex,
            "{id}: obligation hash"
        );

        // 1. Invariância a permutações de citações
        let mut reversed_cites = cites.clone();
        reversed_cites.reverse();
        let prop_rev = Proposal {
            vocabulary: &target_vocab,
            goal: &goal,
            cites: &reversed_cites,
        };
        let ob_rev = generate(prop_rev, &view).expect("reversed cites generate");
        assert_eq!(
            to_canon(&ob_rev).unwrap(),
            actual_canon,
            "{id}: permutacao reversa deve produzir os mesmos bytes"
        );
        assert_eq!(
            logos_crypto::hash(&ob_rev)
                .expect("canonical reversed hash")
                .as_bytes(),
            actual_hash.as_bytes(),
            "{id}: permutacao reversa deve produzir o mesmo h(O)"
        );

        // 2. Invariância a duplicatas de citações
        let mut dup_cites = cites.clone();
        dup_cites.extend_from_slice(&cites);
        let prop_dup = Proposal {
            vocabulary: &target_vocab,
            goal: &goal,
            cites: &dup_cites,
        };
        let ob_dup = generate(prop_dup, &view).expect("duplicated cites generate");
        assert_eq!(
            to_canon(&ob_dup).unwrap(),
            actual_canon,
            "{id}: duplicatas devem produzir os mesmos bytes"
        );
        assert_eq!(
            logos_crypto::hash(&ob_dup)
                .expect("canonical duplicated hash")
                .as_bytes(),
            actual_hash.as_bytes(),
            "{id}: duplicatas devem produzir o mesmo h(O)"
        );

        generated_obligations.push((vec_idx, id.to_owned(), actual_canon, actual_hash));
    }

    // 3. Teste explícito de invariância temporal (vetores 10 e 11)
    // Mesma proposta lógica em épocas diferentes (1 vs 999) produz os mesmos bytes canônicos e hash
    let vec10 = &generated_obligations[9];
    let vec11 = &generated_obligations[10];
    assert_eq!(
        vec10.2, vec11.2,
        "vetores 10 e 11 devem produzir exatamente os mesmos bytes canônicos"
    );
    assert_eq!(
        vec10.3.as_bytes(),
        vec11.3.as_bytes(),
        "vetores 10 e 11 devem produzir exatamente o mesmo h(O)"
    );
}
