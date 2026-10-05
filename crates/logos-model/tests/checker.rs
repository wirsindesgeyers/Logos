//! Testes de integração e comportamento determinístico para o ModelChecker (Etapa 2.3).
//!
//! Cobre:
//! - Tabelas de verdade completas para refutação e consistência (premissas vazias,
//!   unitárias e múltiplas com todas as combinações de true/false)
//! - Posição de falsidade em premissas múltiplas e deduplicação de citações
//!   com fórmulas duplicadas em referências distintas
//! - Invariante P2: troca de bytes/hash e hashes recalculados sobre wires malformados
//! - Incompatibilidade de vocabulário e modelo (divergência de h(V))
//! - Validação obrigatória de Defined (mesmo não utilizados e mesmo com premissas vazias)
//! - Rejeição de meta mal tipada pela consistência (embora ignore a verdade do goal)
//! - Orçamento compartilhado: cobrança cumulativa (bytes.len() + contexto + evals),
//!   esgotamento exato (passos - 1), orçamento zero e ausência de reset por premissa
//! - Fronteira exata de size_bytes e teto padrão de 1 MiB
//! - 1000 repetições determinísticas locais sem dependência de relógio
//! - Integração real com CAS isolado por PID: put/get/reopen de modelo tipado,
//!   modelo ausente (Rejected), corrupção (CasError::Integrity), teto CAS (TooLarge)
//!   versus teto de orçamento (Unknown(Size)), e cobrança de bytes de modelo.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use logos_air::{
    Formula, FormulaNode, Grounding, Limits, Sort, SortDecl, SortDomain, Symbol, SymbolDecl,
    SymbolKind, SymbolResult, Term, TermNode, Vocabulary,
};
use logos_canon::{Array, Canon, Text, Value, from_canon, to_canon};
use logos_core::{Budget, CheckOutcome, Exhausted};
use logos_crypto::{Hash, hash, hash_raw};
use logos_ledger::{Cas, CasError, Limits as CasLimits};
use logos_model::{FiniteModel, ModelChecker, ModelData, ModelLimits};
use logos_obligation::{
    EntryKind, LedgerEntry, LedgerError, LedgerView, Obligation, PremiseRef, Proposal, generate,
};

// ============================================================================
// Isolamento Atômico de CAS Temporário
// ============================================================================

static CAS_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempCas {
    path: PathBuf,
    cas: Cas,
}

impl TempCas {
    fn new() -> Self {
        let counter = CAS_COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "logos-model-checker-test-{}-{}-{}",
            std::process::id(),
            counter,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        fs::create_dir_all(&path).expect("create dir");
        let cas = Cas::open(&path, CasLimits::default()).expect("open cas");
        Self { path, cas }
    }

    fn reopen(&self) -> Cas {
        Cas::open(&self.path, CasLimits::default()).expect("reopen cas")
    }
}

impl Drop for TempCas {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

// ============================================================================
// Snapshot de Ledger em Memória para Geração de Obrigação
// ============================================================================

struct TestLedgerView<'a> {
    epoch: u64,
    entries: HashMap<PremiseRef, LedgerEntry<'a>>,
}

impl<'a> LedgerView for TestLedgerView<'a> {
    fn current_epoch(&self) -> u64 {
        self.epoch
    }

    fn lookup(&self, reference: PremiseRef) -> Result<Option<LedgerEntry<'a>>, LedgerError> {
        Ok(self.entries.get(&reference).cloned())
    }
}

fn make_ref(byte: u8) -> PremiseRef {
    PremiseRef::from_event_hash([byte; 32])
}

// ============================================================================
// Helpers de Vocabulário, Fórmulas e Modelo
// ============================================================================

fn sort(name: &str) -> Sort {
    Sort {
        name: Text::new(name).expect("sort text"),
    }
}

fn sort_decl_open(name: &str) -> SortDecl {
    SortDecl {
        name: Text::new(name).expect("sort decl text"),
        domain: SortDomain::Open,
    }
}

fn const_sym(name: &str, sort_name: &str) -> Symbol {
    Symbol {
        name: Text::new(name).expect("symbol text"),
        kind: SymbolKind::Const,
        arity: Vec::new(),
        result: SymbolResult::Sort(sort(sort_name)),
    }
}

fn pred_sym(name: &str, arity: Vec<Sort>) -> Symbol {
    Symbol {
        name: Text::new(name).expect("symbol text"),
        kind: SymbolKind::Pred,
        arity,
        result: SymbolResult::Bool,
    }
}

fn primitive(sig: Symbol) -> SymbolDecl {
    SymbolDecl {
        signature: sig,
        grounding: Grounding::Glossed(Text::new("primitivo").expect("gloss")),
        gloss_pt: Text::new("glosa").expect("gloss_pt"),
    }
}

fn defined(sig: Symbol, body: Formula) -> SymbolDecl {
    SymbolDecl {
        signature: sig,
        grounding: Grounding::Defined(body),
        gloss_pt: Text::new("definido").expect("gloss_pt"),
    }
}

fn generous_budget() -> Budget {
    Budget::new(10_000_000, 10_000_000, 10_000_000, 0)
}

fn form(nodes: Vec<FormulaNode>) -> Formula {
    Formula::new(nodes, Limits::default()).expect("valid formula")
}

fn term_const(sym: Symbol) -> Term {
    Term::new(
        vec![TermNode::App {
            symbol: sym,
            args: 0,
        }],
        Limits::default(),
    )
    .expect("valid const term")
}

/// Vocabulário padrão com um sort aberto "S" e sem símbolos adicionais.
fn standard_setup() -> (Vocabulary, FiniteModel) {
    let vocab = Vocabulary::new(1, vec![sort_decl_open("S")], Vec::new()).expect("vocab");
    let model_data = ModelData {
        vocab: hash(&vocab).expect("hash"),
        domains: vec![2],
        consts: Vec::new(),
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    let model = FiniteModel::new(model_data, &vocab, ModelLimits::default()).expect("model");
    (vocab, model)
}

/// Gera uma obrigação determinística com as premissas e a meta especificadas.
fn generate_obligation(
    vocab: &Vocabulary,
    premises: &[Formula],
    goal: &Formula,
) -> (Obligation, Vec<u8>, Hash<Obligation>) {
    let mut entries = HashMap::new();
    let mut cites = Vec::new();
    for (i, p) in premises.iter().enumerate() {
        let r = make_ref((i + 1) as u8);
        cites.push(r);
        entries.insert(
            r,
            LedgerEntry {
                kind: EntryKind::Fact,
                epoch: 1,
                retracted: false,
                vocabulary: vocab,
                formula: p,
            },
        );
    }
    let view = TestLedgerView { epoch: 1, entries };
    let proposal = Proposal {
        vocabulary: vocab,
        goal,
        cites: &cites,
    };
    let obligation = generate(proposal, &view).expect("generate obligation");
    let wire = to_canon(&obligation).expect("canon encode");
    let h = obligation.hash().expect("obligation hash");
    (obligation, wire, h)
}

/// Gera uma obrigação determinística com referências explícitas para premissas.
fn generate_obligation_with_refs(
    vocab: &Vocabulary,
    premises: &[(u8, &Formula)],
    goal: &Formula,
) -> (Obligation, Vec<u8>, Hash<Obligation>) {
    let mut entries = HashMap::new();
    let mut cites = Vec::new();
    for &(ref_byte, formula) in premises {
        let r = make_ref(ref_byte);
        cites.push(r);
        entries.insert(
            r,
            LedgerEntry {
                kind: EntryKind::Fact,
                epoch: 1,
                retracted: false,
                vocabulary: vocab,
                formula,
            },
        );
    }
    let view = TestLedgerView { epoch: 1, entries };
    let proposal = Proposal {
        vocabulary: vocab,
        goal,
        cites: &cites,
    };
    let obligation = generate(proposal, &view).expect("generate obligation");
    let wire = to_canon(&obligation).expect("canon encode");
    let h = obligation.hash().expect("obligation hash");
    (obligation, wire, h)
}

// ============================================================================
// 1. Tabelas de Verdade: Refutação e Consistência
// ============================================================================

#[test]
fn test_truth_table_refutation_all_combinations() {
    let (vocab, model) = standard_setup();
    let checker = ModelChecker::new(&vocab).expect("checker");
    let budget = generous_budget();

    let f_true = form(vec![FormulaNode::True]);
    let f_false = form(vec![FormulaNode::False]);

    // Premissas vazias
    // [] |- False : refutação aceita (0 premissas verdadeiras por vacuidade, meta falsa)
    let (_, wire, h) = generate_obligation(&vocab, &[], &f_false);
    assert_eq!(
        checker.check_refutation(&wire, h, &model, &budget),
        CheckOutcome::Accepted
    );

    // [] |- True : refutação rejeita (meta satisfeita no modelo)
    let (_, wire, h) = generate_obligation(&vocab, &[], &f_true);
    assert!(matches!(
        checker.check_refutation(&wire, h, &model, &budget),
        CheckOutcome::Rejected(_)
    ));

    // Premissa unitária
    // [True] |- False : aceita
    let (_, wire, h) = generate_obligation(&vocab, std::slice::from_ref(&f_true), &f_false);
    assert_eq!(
        checker.check_refutation(&wire, h, &model, &budget),
        CheckOutcome::Accepted
    );

    // [True] |- True : rejeita (meta verdadeira)
    let (_, wire, h) = generate_obligation(&vocab, std::slice::from_ref(&f_true), &f_true);
    assert!(matches!(
        checker.check_refutation(&wire, h, &model, &budget),
        CheckOutcome::Rejected(_)
    ));

    // [False] |- False : rejeita (premissa falsa)
    let (_, wire, h) = generate_obligation(&vocab, std::slice::from_ref(&f_false), &f_false);
    assert!(matches!(
        checker.check_refutation(&wire, h, &model, &budget),
        CheckOutcome::Rejected(_)
    ));

    // [False] |- True : rejeita (premissa falsa e meta verdadeira)
    let (_, wire, h) = generate_obligation(&vocab, std::slice::from_ref(&f_false), &f_true);
    assert!(matches!(
        checker.check_refutation(&wire, h, &model, &budget),
        CheckOutcome::Rejected(_)
    ));

    // Premissas múltiplas (2 premissas)
    // [True, True] |- False : aceita
    let (_, wire, h) = generate_obligation(&vocab, &[f_true.clone(), f_true.clone()], &f_false);
    assert_eq!(
        checker.check_refutation(&wire, h, &model, &budget),
        CheckOutcome::Accepted
    );

    // [True, True] |- True : rejeita (meta verdadeira)
    let (_, wire, h) = generate_obligation(&vocab, &[f_true.clone(), f_true.clone()], &f_true);
    assert!(matches!(
        checker.check_refutation(&wire, h, &model, &budget),
        CheckOutcome::Rejected(_)
    ));

    // [False, True] |- False : rejeita (primeira premissa falsa)
    let (_, wire, h) = generate_obligation(&vocab, &[f_false.clone(), f_true.clone()], &f_false);
    assert!(matches!(
        checker.check_refutation(&wire, h, &model, &budget),
        CheckOutcome::Rejected(_)
    ));

    // [True, False] |- False : rejeita (segunda premissa falsa)
    let (_, wire, h) = generate_obligation(&vocab, &[f_true.clone(), f_false.clone()], &f_false);
    assert!(matches!(
        checker.check_refutation(&wire, h, &model, &budget),
        CheckOutcome::Rejected(_)
    ));

    // [False, False] |- False : rejeita (ambas falsas)
    let (_, wire, h) = generate_obligation(&vocab, &[f_false.clone(), f_false.clone()], &f_false);
    assert!(matches!(
        checker.check_refutation(&wire, h, &model, &budget),
        CheckOutcome::Rejected(_)
    ));

    // [True, False] |- True : rejeita
    let (_, wire, h) = generate_obligation(&vocab, &[f_true.clone(), f_false.clone()], &f_true);
    assert!(matches!(
        checker.check_refutation(&wire, h, &model, &budget),
        CheckOutcome::Rejected(_)
    ));
}

#[test]
fn test_truth_table_consistency_all_combinations() {
    let (vocab, model) = standard_setup();
    let checker = ModelChecker::new(&vocab).expect("checker");
    let budget = generous_budget();

    let f_true = form(vec![FormulaNode::True]);
    let f_false = form(vec![FormulaNode::False]);

    // Premissas vazias: consistência aceita independente da verdade da meta!
    // [] |- False : aceita (0 premissas são vacuamente verdadeiras)
    let (_, wire, h) = generate_obligation(&vocab, &[], &f_false);
    assert_eq!(
        checker.check_consistency(&wire, h, &model, &budget),
        CheckOutcome::Accepted
    );

    // [] |- True : aceita
    let (_, wire, h) = generate_obligation(&vocab, &[], &f_true);
    assert_eq!(
        checker.check_consistency(&wire, h, &model, &budget),
        CheckOutcome::Accepted
    );

    // Premissa unitária: consistência ignora a verdade da meta
    // [True] |- False : aceita
    let (_, wire, h) = generate_obligation(&vocab, std::slice::from_ref(&f_true), &f_false);
    assert_eq!(
        checker.check_consistency(&wire, h, &model, &budget),
        CheckOutcome::Accepted
    );

    // [True] |- True : aceita (diferente da refutação que rejeitaria!)
    let (_, wire, h) = generate_obligation(&vocab, std::slice::from_ref(&f_true), &f_true);
    assert_eq!(
        checker.check_consistency(&wire, h, &model, &budget),
        CheckOutcome::Accepted
    );

    // [False] |- False : rejeita (premissa falsificada)
    let (_, wire, h) = generate_obligation(&vocab, std::slice::from_ref(&f_false), &f_false);
    assert!(matches!(
        checker.check_consistency(&wire, h, &model, &budget),
        CheckOutcome::Rejected(_)
    ));

    // [False] |- True : rejeita (premissa falsificada)
    let (_, wire, h) = generate_obligation(&vocab, std::slice::from_ref(&f_false), &f_true);
    assert!(matches!(
        checker.check_consistency(&wire, h, &model, &budget),
        CheckOutcome::Rejected(_)
    ));

    // Múltiplas premissas
    // [True, True] |- True : aceita
    let (_, wire, h) = generate_obligation(&vocab, &[f_true.clone(), f_true.clone()], &f_true);
    assert_eq!(
        checker.check_consistency(&wire, h, &model, &budget),
        CheckOutcome::Accepted
    );

    // [True, True] |- False : aceita
    let (_, wire, h) = generate_obligation(&vocab, &[f_true.clone(), f_true.clone()], &f_false);
    assert_eq!(
        checker.check_consistency(&wire, h, &model, &budget),
        CheckOutcome::Accepted
    );

    // [False, True] |- True : rejeita
    let (_, wire, h) = generate_obligation(&vocab, &[f_false.clone(), f_true.clone()], &f_true);
    assert!(matches!(
        checker.check_consistency(&wire, h, &model, &budget),
        CheckOutcome::Rejected(_)
    ));

    // [True, False] |- True : rejeita
    let (_, wire, h) = generate_obligation(&vocab, &[f_true.clone(), f_false.clone()], &f_true);
    assert!(matches!(
        checker.check_consistency(&wire, h, &model, &budget),
        CheckOutcome::Rejected(_)
    ));
}

#[test]
fn test_multiple_premises_false_position_sensitivity() {
    let (vocab, model) = standard_setup();
    let checker = ModelChecker::new(&vocab).expect("checker");
    let budget = generous_budget();

    let f_true = form(vec![FormulaNode::True]);
    let f_false = form(vec![FormulaNode::False]);

    // 1. Falsa na posição 0: [False, True, True]
    let (_, wire_0, h_0) = generate_obligation(
        &vocab,
        &[f_false.clone(), f_true.clone(), f_true.clone()],
        &f_false,
    );
    assert!(matches!(
        checker.check_refutation(&wire_0, h_0, &model, &budget),
        CheckOutcome::Rejected(_)
    ));
    assert!(matches!(
        checker.check_consistency(&wire_0, h_0, &model, &budget),
        CheckOutcome::Rejected(_)
    ));

    // 2. Falsa na posição 1: [True, False, True]
    let (_, wire_1, h_1) = generate_obligation(
        &vocab,
        &[f_true.clone(), f_false.clone(), f_true.clone()],
        &f_false,
    );
    assert!(matches!(
        checker.check_refutation(&wire_1, h_1, &model, &budget),
        CheckOutcome::Rejected(_)
    ));
    assert!(matches!(
        checker.check_consistency(&wire_1, h_1, &model, &budget),
        CheckOutcome::Rejected(_)
    ));

    // 3. Falsa na posição 2: [True, True, False]
    let (_, wire_2, h_2) = generate_obligation(
        &vocab,
        &[f_true.clone(), f_true.clone(), f_false.clone()],
        &f_false,
    );
    assert!(matches!(
        checker.check_refutation(&wire_2, h_2, &model, &budget),
        CheckOutcome::Rejected(_)
    ));
    assert!(matches!(
        checker.check_consistency(&wire_2, h_2, &model, &budget),
        CheckOutcome::Rejected(_)
    ));
}

#[test]
fn test_duplicate_formula_distinct_references() {
    let (vocab, model) = standard_setup();
    let checker = ModelChecker::new(&vocab).expect("checker");
    let budget = generous_budget();

    let f_true = form(vec![FormulaNode::True]);
    let f_false = form(vec![FormulaNode::False]);

    // Fórmulas idênticas (True) com referências de eventos estritamente crescentes distintos
    let (_, wire, h) =
        generate_obligation_with_refs(&vocab, &[(0x10, &f_true), (0x20, &f_true)], &f_false);
    assert_eq!(
        checker.check_refutation(&wire, h, &model, &budget),
        CheckOutcome::Accepted
    );
    assert_eq!(
        checker.check_consistency(&wire, h, &model, &budget),
        CheckOutcome::Accepted
    );

    // Duplicatas de False: ambas as checagens rejeitam
    let (_, wire_f, h_f) =
        generate_obligation_with_refs(&vocab, &[(0x10, &f_false), (0x20, &f_false)], &f_false);
    assert!(matches!(
        checker.check_refutation(&wire_f, h_f, &model, &budget),
        CheckOutcome::Rejected(_)
    ));
    assert!(matches!(
        checker.check_consistency(&wire_f, h_f, &model, &budget),
        CheckOutcome::Rejected(_)
    ));
}

// ============================================================================
// 2. Invariante P2 e Resistência a Malformações
// ============================================================================

#[test]
fn test_p2_swapped_bytes_and_hash() {
    let (vocab, model) = standard_setup();
    let checker = ModelChecker::new(&vocab).expect("checker");
    let budget = generous_budget();

    let f_true = form(vec![FormulaNode::True]);
    let f_false = form(vec![FormulaNode::False]);

    let (_, wire_a, h_a) = generate_obligation(&vocab, std::slice::from_ref(&f_true), &f_false);
    let (_, wire_b, h_b) = generate_obligation(&vocab, &[], &f_false);

    assert_ne!(wire_a, wire_b);
    assert_ne!(h_a, h_b);

    // Apresentar wire_a com hash de b: violação imediata de P2
    assert!(matches!(
        checker.check_refutation(&wire_a, h_b, &model, &budget),
        CheckOutcome::Rejected(_)
    ));
    assert!(matches!(
        checker.check_consistency(&wire_a, h_b, &model, &budget),
        CheckOutcome::Rejected(_)
    ));

    // Apresentar wire_b com hash de a
    assert!(matches!(
        checker.check_refutation(&wire_b, h_a, &model, &budget),
        CheckOutcome::Rejected(_)
    ));
    assert!(matches!(
        checker.check_consistency(&wire_b, h_a, &model, &budget),
        CheckOutcome::Rejected(_)
    ));
}

#[test]
fn test_p2_malformed_wire_with_recomputed_hash() {
    let (vocab, model) = standard_setup();
    let checker = ModelChecker::new(&vocab).expect("checker");
    let budget = generous_budget();

    let f_true = form(vec![FormulaNode::True]);
    let f_false = form(vec![FormulaNode::False]);
    let (_, wire, _) = generate_obligation(&vocab, &[f_true], &f_false);

    // 1. Bytes adicionais ao final (trailing bytes / CBOR não canônico)
    let mut trailing = wire.clone();
    trailing.push(0x00);
    let h_trailing = hash_raw::<Obligation>(&trailing);
    assert!(matches!(
        checker.check_refutation(&trailing, h_trailing, &model, &budget),
        CheckOutcome::Rejected(_)
    ));
    assert!(matches!(
        checker.check_consistency(&trailing, h_trailing, &model, &budget),
        CheckOutcome::Rejected(_)
    ));

    // 2. Wire truncado
    let truncated = &wire[..wire.len() - 1];
    let h_truncated = hash_raw::<Obligation>(truncated);
    assert!(matches!(
        checker.check_refutation(truncated, h_truncated, &model, &budget),
        CheckOutcome::Rejected(_)
    ));
    assert!(matches!(
        checker.check_consistency(truncated, h_truncated, &model, &budget),
        CheckOutcome::Rejected(_)
    ));

    // 3. Envelope CBOR inválido (cabeçalho corrompido)
    let mut bad_header = wire.clone();
    bad_header[0] = 0xFF;
    let h_bad_header = hash_raw::<Obligation>(&bad_header);
    assert!(matches!(
        checker.check_refutation(&bad_header, h_bad_header, &model, &budget),
        CheckOutcome::Rejected(_)
    ));
    assert!(matches!(
        checker.check_consistency(&bad_header, h_bad_header, &model, &budget),
        CheckOutcome::Rejected(_)
    ));
}

#[test]
fn test_p2_vocabulary_mismatch_and_model_mismatch() {
    let (vocab_1, _) = standard_setup();

    // Segundo vocabulário com sort diferente ("T")
    let vocab_2 = Vocabulary::new(1, vec![sort_decl_open("T")], Vec::new()).expect("vocab_2");
    let model_data_2 = ModelData {
        vocab: hash(&vocab_2).expect("hash"),
        domains: vec![2],
        consts: Vec::new(),
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    let model_2 =
        FiniteModel::new(model_data_2, &vocab_2, ModelLimits::default()).expect("model_2");

    let checker_1 = ModelChecker::new(&vocab_1).expect("checker_1");
    let checker_2 = ModelChecker::new(&vocab_2).expect("checker_2");
    let budget = generous_budget();

    let f_true = form(vec![FormulaNode::True]);
    let f_false = form(vec![FormulaNode::False]);

    // Obrigação gerada no Vocabulário 1
    let (_, wire_1, h_1) = generate_obligation(&vocab_1, &[f_true], &f_false);

    // 1. Checker com Vocabulário 2 tentando verificar obrigação do Vocabulário 1
    assert!(matches!(
        checker_2.check_refutation(&wire_1, h_1, &model_2, &budget),
        CheckOutcome::Rejected(_)
    ));
    assert!(matches!(
        checker_2.check_consistency(&wire_1, h_1, &model_2, &budget),
        CheckOutcome::Rejected(_)
    ));

    // 2. Modelo 2 (vinculado a Vocabulário 2) passado para Checker 1
    assert!(matches!(
        checker_1.check_refutation(&wire_1, h_1, &model_2, &budget),
        CheckOutcome::Rejected(_)
    ));
    assert!(matches!(
        checker_1.check_consistency(&wire_1, h_1, &model_2, &budget),
        CheckOutcome::Rejected(_)
    ));
}

// ============================================================================
// 3. Validação de Contexto: Defined Inválido e Meta Mal Tipada
// ============================================================================

#[test]
fn test_invalid_defined_symbol_rejects_even_when_unused() {
    let c0 = const_sym("c0", "S");
    // Constante definida com corpo False (zero resultados possíveis -> inválida)
    let c_bad = const_sym("c_bad", "S");
    let c_bad_body = form(vec![FormulaNode::False]);

    let vocab = Vocabulary::new(
        1,
        vec![sort_decl_open("S")],
        vec![primitive(c0), defined(c_bad, c_bad_body)],
    )
    .expect("vocab");

    let model_data = ModelData {
        vocab: hash(&vocab).expect("hash"),
        domains: vec![2],
        consts: vec![(0, 0)],
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    let model = FiniteModel::new(model_data, &vocab, ModelLimits::default()).expect("model");
    let checker = ModelChecker::new(&vocab).expect("checker");
    let budget = generous_budget();

    let f_true = form(vec![FormulaNode::True]);
    let f_false = form(vec![FormulaNode::False]);

    // As fórmulas NÃO mencionam c_bad!
    let (_, wire, h) = generate_obligation(&vocab, &[f_true], &f_false);

    // Refutação deve rejeitar devido ao contexto inválido
    assert!(matches!(
        checker.check_refutation(&wire, h, &model, &budget),
        CheckOutcome::Rejected(_)
    ));

    // Consistência sem premissas também DEVE rejeitar!
    let (_, wire_empty, h_empty) = generate_obligation(&vocab, &[], &f_false);
    assert!(matches!(
        checker.check_consistency(&wire_empty, h_empty, &model, &budget),
        CheckOutcome::Rejected(_)
    ));
}

#[test]
fn test_consistency_rejects_maltyped_goal_despite_ignoring_truth() {
    // Vocabulário com sorts "S" e "T"
    let sym_c = const_sym("c", "S");
    let sym_p = pred_sym("P", vec![sort("T")]); // P espera sort T, mas passaremos c: S

    let vocab = Vocabulary::new(
        1,
        vec![sort_decl_open("S"), sort_decl_open("T")],
        vec![primitive(sym_c.clone()), primitive(sym_p.clone())],
    )
    .expect("vocab");

    let model_data = ModelData {
        vocab: hash(&vocab).expect("hash"),
        domains: vec![2, 2],
        consts: vec![(0, 0)],
        funcs: Vec::new(),
        preds: vec![(1, Vec::new())],
    };
    let model = FiniteModel::new(model_data, &vocab, ModelLimits::default()).expect("model");
    let checker = ModelChecker::new(&vocab).expect("checker");
    let budget = generous_budget();

    // Sintaticamente válida na AST, mas mal tipada (SortError) contra o vocabulário:
    let maltyped_goal = Formula::new(
        vec![FormulaNode::Pred {
            symbol: sym_p,
            args: vec![term_const(sym_c)],
        }],
        Limits::default(),
    )
    .expect("valid syntax");

    // Construir manualmente envelope CBOR com a meta mal tipada
    let envelope = Array::new(vec![
        Value::Unsigned(1),
        Value::Unsigned(0),
        Value::Text(Text::new("FOL-ms-eq/v1").expect("text")),
        Value::Unsigned(1),
        Value::Bytes(hash(&vocab).expect("h").as_bytes().to_vec()),
        Value::Array(Array::new(Vec::new()).expect("empty premises")),
        maltyped_goal.to_value().expect("goal value"),
    ])
    .expect("envelope");
    let maltyped_wire = to_canon(&Value::Array(envelope)).expect("wire");
    let h = hash_raw::<Obligation>(&maltyped_wire);

    // Consistência DEVE rejeitar a obrigação mal tipada
    assert!(matches!(
        checker.check_consistency(&maltyped_wire, h, &model, &budget),
        CheckOutcome::Rejected(_)
    ));
}

// ============================================================================
// 4. Orçamento de Passos e Cobrança Cumulativa (S2 / Shared Meter)
// ============================================================================

#[test]
fn test_budget_steps_zero_exhaustion() {
    let (vocab, model) = standard_setup();
    let checker = ModelChecker::new(&vocab).expect("checker");

    let f_true = form(vec![FormulaNode::True]);
    let f_false = form(vec![FormulaNode::False]);
    let (_, wire, h) = generate_obligation(&vocab, &[f_true], &f_false);

    let budget = Budget::new(0, 10_000_000, 10_000_000, 0);

    assert_eq!(
        checker.check_refutation(&wire, h, &model, &budget),
        CheckOutcome::Unknown(Exhausted::Steps)
    );
    assert_eq!(
        checker.check_consistency(&wire, h, &model, &budget),
        CheckOutcome::Unknown(Exhausted::Steps)
    );
}

#[test]
fn test_shared_budget_cumulative_exact_step_accounting() {
    let (vocab, model) = standard_setup();
    let checker = ModelChecker::new(&vocab).expect("checker");

    let f_true = form(vec![FormulaNode::True]);
    let f_false = form(vec![FormulaNode::False]);

    // 3 premissas: [True, True, True] e meta False
    let (_, wire, h) = generate_obligation(
        &vocab,
        &[f_true.clone(), f_true.clone(), f_true.clone()],
        &f_false,
    );

    // Custo exato conforme especificação normativa do ADR 0016 / etapa 2.3:
    // Custo total = bytes.len() + 1 (contexto M/V) + 2 * k fórmulas avaliadas
    // Refutação avalia 3 premissas + 1 meta = 4 fórmulas => 2 * 4 = 8 passos
    let o_bytes_len = wire.len() as u64;
    let refutation_exact_steps = o_bytes_len + 1 + 2 * 4;

    // Orçamento exato aceita
    let budget_exact = Budget::new(refutation_exact_steps, 10_000_000, 10_000_000, 0);
    assert_eq!(
        checker.check_refutation(&wire, h, &model, &budget_exact),
        CheckOutcome::Accepted
    );

    // Orçamento com 1 passo a menos esgota cumulativamente
    let budget_short = Budget::new(refutation_exact_steps - 1, 10_000_000, 10_000_000, 0);
    assert_eq!(
        checker.check_refutation(&wire, h, &model, &budget_short),
        CheckOutcome::Unknown(Exhausted::Steps)
    );

    // Consistência NÃO avalia a meta: 3 premissas => 2 * 3 = 6 passos
    let consistency_exact_steps = o_bytes_len + 1 + 2 * 3;

    let budget_cons_exact = Budget::new(consistency_exact_steps, 10_000_000, 10_000_000, 0);
    assert_eq!(
        checker.check_consistency(&wire, h, &model, &budget_cons_exact),
        CheckOutcome::Accepted
    );

    let budget_cons_short = Budget::new(consistency_exact_steps - 1, 10_000_000, 10_000_000, 0);
    assert_eq!(
        checker.check_consistency(&wire, h, &model, &budget_cons_short),
        CheckOutcome::Unknown(Exhausted::Steps)
    );
}

#[test]
fn test_shared_meter_does_not_reset_between_premises() {
    let (vocab, model) = standard_setup();
    let checker = ModelChecker::new(&vocab).expect("checker");

    let f_true = form(vec![FormulaNode::True]);
    let f_false = form(vec![FormulaNode::False]);

    // 5 premissas True
    let (_, wire, h) = generate_obligation(
        &vocab,
        &[
            f_true.clone(),
            f_true.clone(),
            f_true.clone(),
            f_true.clone(),
            f_true.clone(),
        ],
        &f_false,
    );

    let o_bytes_len = wire.len() as u64;
    // Se o medidor fosse reiniciado a cada premissa, caberia com bytes.len() + 1 + 2 passos.
    // Como o medidor é estritamente acumulado e compartilhado, isso deve esgotar.
    let budget_reset_hypothetical = Budget::new(o_bytes_len + 1 + 2, 10_000_000, 10_000_000, 0);
    assert_eq!(
        checker.check_consistency(&wire, h, &model, &budget_reset_hypothetical),
        CheckOutcome::Unknown(Exhausted::Steps)
    );
}

// ============================================================================
// 5. Limites de Tamanho de Certificado (size_bytes)
// ============================================================================

#[test]
fn test_cert_size_boundary_exact_and_default_limits() {
    let (vocab, model) = standard_setup();
    let checker = ModelChecker::new(&vocab).expect("checker");

    let f_true = form(vec![FormulaNode::True]);
    let f_false = form(vec![FormulaNode::False]);
    let (_, wire, h) = generate_obligation(&vocab, &[f_true], &f_false);

    let wire_len = wire.len() as u64;

    // 1. Limite size_bytes exatamente igual ao tamanho dos bytes: aceito para processamento
    let budget_exact_size = Budget::new(10_000_000, 10_000_000, wire_len, 0);
    assert_eq!(
        checker.check_refutation(&wire, h, &model, &budget_exact_size),
        CheckOutcome::Accepted
    );

    // 2. Limite size_bytes menor por 1 byte: esgotamento imediato com Unknown(Size)
    let budget_short_size = Budget::new(10_000_000, 10_000_000, wire_len - 1, 0);
    assert_eq!(
        checker.check_refutation(&wire, h, &model, &budget_short_size),
        CheckOutcome::Unknown(Exhausted::Size)
    );
    assert_eq!(
        checker.check_consistency(&wire, h, &model, &budget_short_size),
        CheckOutcome::Unknown(Exhausted::Size)
    );
}

// ============================================================================
// 6. 1000 Repetições Determinísticas sem Dependência de Relógio
// ============================================================================

#[test]
fn test_1000_deterministic_repeated_executions() {
    let (vocab, model) = standard_setup();
    let checker = ModelChecker::new(&vocab).expect("checker");

    let f_true = form(vec![FormulaNode::True]);
    let f_false = form(vec![FormulaNode::False]);

    let (_, wire_acc, h_acc) = generate_obligation(&vocab, std::slice::from_ref(&f_true), &f_false);
    let (_, wire_rej, h_rej) =
        generate_obligation(&vocab, std::slice::from_ref(&f_false), &f_false);

    let generous = generous_budget();
    let budget_zero = Budget::new(0, 10_000_000, 10_000_000, 0);

    for _ in 0..1000 {
        // Accepted
        let outcome = checker.check_refutation(&wire_acc, h_acc, &model, &generous);
        assert_eq!(outcome, CheckOutcome::Accepted);

        // Rejected
        let outcome = checker.check_refutation(&wire_rej, h_rej, &model, &generous);
        assert!(matches!(outcome, CheckOutcome::Rejected(_)));

        // Unknown(Steps)
        let outcome = checker.check_refutation(&wire_acc, h_acc, &model, &budget_zero);
        assert_eq!(outcome, CheckOutcome::Unknown(Exhausted::Steps));

        // Consistency Accepted
        let outcome = checker.check_consistency(&wire_acc, h_acc, &model, &generous);
        assert_eq!(outcome, CheckOutcome::Accepted);
    }
}

// ============================================================================
// 7. Integração Real com CAS
// ============================================================================

#[test]
fn test_cas_integration_put_reopen_and_checks() {
    let temp_cas = TempCas::new();
    let (vocab, model) = standard_setup();
    let checker = ModelChecker::new(&vocab).expect("checker");
    let budget = generous_budget();

    let model_wire = model
        .encode_with_limits(ModelLimits::default())
        .expect("encode model");
    let model_hash = temp_cas
        .cas
        .put::<FiniteModel>(&model_wire)
        .expect("put model");

    // Reabertura do CAS no mesmo diretório em disco
    let cas_reopened = temp_cas.reopen();

    let f_true = form(vec![FormulaNode::True]);
    let f_false = form(vec![FormulaNode::False]);
    let (_, wire, h) = generate_obligation(&vocab, &[f_true], &f_false);

    // Refutação e consistência a partir do CAS
    let res_ref = checker.check_refutation_from_cas(&wire, h, &cas_reopened, model_hash, &budget);
    assert_eq!(res_ref.expect("success"), CheckOutcome::Accepted);

    let res_cons = checker.check_consistency_from_cas(&wire, h, &cas_reopened, model_hash, &budget);
    assert_eq!(res_cons.expect("success"), CheckOutcome::Accepted);

    let truth = form(vec![FormulaNode::True]);
    let (_, true_wire, true_h) = generate_obligation(&vocab, std::slice::from_ref(&truth), &truth);
    assert!(matches!(
        checker
            .check_refutation_from_cas(&true_wire, true_h, &cas_reopened, model_hash, &budget)
            .unwrap(),
        CheckOutcome::Rejected(_)
    ));
    assert_eq!(
        checker
            .check_consistency_from_cas(&true_wire, true_h, &cas_reopened, model_hash, &budget)
            .unwrap(),
        CheckOutcome::Accepted
    );
}

#[test]
fn test_cas_missing_model_hash_is_rejected() {
    let temp_cas = TempCas::new();
    let (vocab, _) = standard_setup();
    let checker = ModelChecker::new(&vocab).expect("checker");
    let budget = generous_budget();

    let f_true = form(vec![FormulaNode::True]);
    let f_false = form(vec![FormulaNode::False]);
    let (_, wire, h) = generate_obligation(&vocab, &[f_true], &f_false);

    // Hash inexistente no CAS
    let missing_hash = Hash::<FiniteModel>::from_bytes([0xAA; 32]);

    let res = checker.check_refutation_from_cas(&wire, h, &temp_cas.cas, missing_hash, &budget);
    assert!(matches!(
        res.expect("check outcome"),
        CheckOutcome::Rejected(_)
    ));

    let res_cons =
        checker.check_consistency_from_cas(&wire, h, &temp_cas.cas, missing_hash, &budget);
    assert!(matches!(
        res_cons.expect("check outcome"),
        CheckOutcome::Rejected(_)
    ));
}

#[test]
fn test_cas_tampered_model_object_returns_integrity_error() {
    let temp_cas = TempCas::new();
    let (vocab, model) = standard_setup();
    let checker = ModelChecker::new(&vocab).expect("checker");
    let budget = generous_budget();

    let model_wire = model
        .encode_with_limits(ModelLimits::default())
        .expect("encode model");
    let model_hash = temp_cas
        .cas
        .put::<FiniteModel>(&model_wire)
        .expect("put model");

    // Localizar o arquivo do objeto em disco no CAS e corromper seu conteúdo
    let mut found = false;
    for entry in fs::read_dir(&temp_cas.path).expect("read dir") {
        let entry = entry.expect("entry");
        if entry.file_type().expect("type").is_dir() {
            // cas/<domain>/<shard>/<hex>
            for sub in walkdir_files(&entry.path()) {
                let mut data = fs::read(&sub).expect("read file");
                if !data.is_empty() {
                    data[0] ^= 0xFF; // Inverter primeiro byte
                    fs::write(&sub, &data).expect("write tampered");
                    found = true;
                    break;
                }
            }
        }
    }
    assert!(found, "objeto do CAS deve ser localizado em disco");

    let cas_reopened = temp_cas.reopen();
    let f_true = form(vec![FormulaNode::True]);
    let f_false = form(vec![FormulaNode::False]);
    let (_, wire, h) = generate_obligation(&vocab, &[f_true], &f_false);

    let res = checker.check_refutation_from_cas(&wire, h, &cas_reopened, model_hash, &budget);
    assert!(matches!(res, Err(CasError::Integrity)));

    let res_cons = checker.check_consistency_from_cas(&wire, h, &cas_reopened, model_hash, &budget);
    assert!(matches!(res_cons, Err(CasError::Integrity)));
}

fn walkdir_files(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                files.push(path);
            } else if path.is_dir() {
                files.extend(walkdir_files(&path));
            }
        }
    }
    files
}

#[test]
fn test_cas_too_large_error_vs_budget_unknown_size() {
    let (vocab, model) = standard_setup();
    let checker = ModelChecker::new(&vocab).expect("checker");

    let model_wire = model
        .encode_with_limits(ModelLimits::default())
        .expect("encode model");

    let f_true = form(vec![FormulaNode::True]);
    let f_false = form(vec![FormulaNode::False]);
    let (_, wire, h) = generate_obligation(&vocab, &[f_true], &f_false);

    let temp_cas = TempCas::new();
    let model_hash = temp_cas
        .cas
        .put::<FiniteModel>(&model_wire)
        .expect("put model");
    let small_limits = CasLimits::default()
        .with_domain_limit::<FiniteModel>(model_wire.len() as u64 - 1)
        .expect("domain limit");
    let small_cas = Cas::open(&temp_cas.path, small_limits).expect("open restricted cas");
    assert!(matches!(
        checker.check_refutation_from_cas(&wire, h, &small_cas, model_hash, &generous_budget()),
        Err(CasError::TooLarge { .. })
    ));

    let total_bytes = wire.len() as u64 + model_wire.len() as u64;
    for size in [total_bytes, total_bytes + 1] {
        let budget = Budget::new(10_000_000, 10_000_000, size, 0);
        assert_eq!(
            checker
                .check_refutation_from_cas(&wire, h, &temp_cas.cas, model_hash, &budget)
                .unwrap(),
            CheckOutcome::Accepted
        );
    }
    let budget_insufficient_size = Budget::new(10_000_000, 10_000_000, total_bytes - 1, 0);

    let res = checker.check_refutation_from_cas(
        &wire,
        h,
        &temp_cas.cas,
        model_hash,
        &budget_insufficient_size,
    );
    assert_eq!(
        res.expect("outcome"),
        CheckOutcome::Unknown(Exhausted::Size)
    );
}

#[test]
fn test_cas_shared_budget_model_bytes_step_accounting() {
    let temp_cas = TempCas::new();
    let (vocab, model) = standard_setup();
    let checker = ModelChecker::new(&vocab).expect("checker");

    let model_wire = model
        .encode_with_limits(ModelLimits::default())
        .expect("encode model");
    let model_hash = temp_cas
        .cas
        .put::<FiniteModel>(&model_wire)
        .expect("put model");

    let f_true = form(vec![FormulaNode::True]);
    let f_false = form(vec![FormulaNode::False]);
    let (_, wire, h) = generate_obligation(&vocab, &[f_true], &f_false);

    // No caminho do CAS, o orçamento de passos cobra:
    // bytes_O + bytes_M + 1 (contexto) + 2 * (1 premissa + 1 meta) = bytes_O + bytes_M + 5
    let exact_cas_steps = wire.len() as u64 + model_wire.len() as u64 + 1 + 2 * 2;

    let budget_exact = Budget::new(exact_cas_steps, 10_000_000, 10_000_000, 0);
    let res_ok =
        checker.check_refutation_from_cas(&wire, h, &temp_cas.cas, model_hash, &budget_exact);
    assert_eq!(res_ok.expect("outcome"), CheckOutcome::Accepted);

    let budget_short = Budget::new(exact_cas_steps - 1, 10_000_000, 10_000_000, 0);
    let res_exhausted =
        checker.check_refutation_from_cas(&wire, h, &temp_cas.cas, model_hash, &budget_short);
    assert_eq!(
        res_exhausted.expect("outcome"),
        CheckOutcome::Unknown(Exhausted::Steps)
    );
}

#[test]
fn cas_contextual_decode_preserves_invalid_and_size_outcomes() {
    let temp_cas = TempCas::new();
    let (vocab, model) = standard_setup();
    let checker = ModelChecker::new(&vocab).unwrap();
    let model_wire = to_canon(&model).unwrap();
    let (_, wire, h) = generate_obligation(&vocab, &[], &form(vec![FormulaNode::False]));
    let budget = generous_budget();

    let mut trailing = model_wire.clone();
    trailing.push(0);
    let trailing_h = temp_cas.cas.put::<FiniteModel>(&trailing).unwrap();
    assert!(matches!(
        checker
            .check_refutation_from_cas(&wire, h, &temp_cas.cas, trailing_h, &budget)
            .unwrap(),
        CheckOutcome::Rejected(_)
    ));

    let Value::Array(envelope) = from_canon::<Value>(&model_wire).unwrap() else {
        panic!("model envelope must be an array");
    };
    let mut fields = envelope.items().to_vec();
    fields[2] = Value::Array(Array::new(vec![Value::Unsigned(65_537)]).unwrap());
    let oversized = to_canon(&Value::Array(Array::new(fields).unwrap())).unwrap();
    let oversized_h = temp_cas.cas.put::<FiniteModel>(&oversized).unwrap();
    assert_eq!(
        checker
            .check_consistency_from_cas(&wire, h, &temp_cas.cas, oversized_h, &budget)
            .unwrap(),
        CheckOutcome::Unknown(Exhausted::Size)
    );
}

#[test]
fn quantified_defined_predicate_reuses_context_across_premises_and_goal() {
    let c = const_sym("c", "S");
    let d = pred_sym("D", vec![sort("S")]);
    let variable = Term::new(
        vec![TermNode::Var {
            index: 0,
            sort: sort("S"),
        }],
        Limits::default(),
    )
    .unwrap();
    let definition = form(vec![FormulaNode::Eq(
        variable.clone(),
        term_const(c.clone()),
    )]);
    let vocab = Vocabulary::new(
        1,
        vec![sort_decl_open("S")],
        vec![primitive(c.clone()), defined(d.clone(), definition)],
    )
    .unwrap();
    let exists = form(vec![
        FormulaNode::Exists(sort("S")),
        FormulaNode::Pred {
            symbol: d.clone(),
            args: vec![variable.clone()],
        },
    ]);
    let equality = form(vec![FormulaNode::Eq(term_const(c.clone()), term_const(c))]);
    let goal = form(vec![
        FormulaNode::Forall(sort("S")),
        FormulaNode::Pred {
            symbol: d,
            args: vec![variable],
        },
    ]);
    let (_, wire, h) = generate_obligation(&vocab, &[exists, equality], &goal);
    let checker = ModelChecker::new(&vocab).unwrap();
    for cardinality in [1, 2] {
        let model = FiniteModel::new(
            ModelData {
                vocab: hash(&vocab).unwrap(),
                domains: vec![cardinality],
                consts: vec![(0, 0)],
                funcs: Vec::new(),
                preds: Vec::new(),
            },
            &vocab,
            ModelLimits::default(),
        )
        .unwrap();
        assert_eq!(
            checker.check_consistency(&wire, h, &model, &generous_budget()),
            CheckOutcome::Accepted
        );
        let refutation = checker.check_refutation(&wire, h, &model, &generous_budget());
        if cardinality == 1 {
            assert!(matches!(refutation, CheckOutcome::Rejected(_)));
        } else {
            assert_eq!(refutation, CheckOutcome::Accepted);
        }
    }
}
