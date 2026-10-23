//! Exercício reproduzível da API pública de `logos-model` (Etapa 2.3).
//!
//! Comando: `cargo run -p logos-model --example checker_smoke`.
//!
//! Demonstra:
//! 1. Geração de obrigação real `O` via `logos_obligation::generate` com `MemoryLedgerView`;
//! 2. Codificação canônica de modelo finito `M` e persistência/reabertura em `Cas` real;
//! 3. Checagem em memória (`check_refutation` e `check_consistency`);
//! 4. Checagem com modelo no CAS (`check_refutation_from_source` e `check_consistency_from_source`);
//! 5. Rejeição determinística por premissa falsa;
//! 6. Rejeição determinística por violação de P2 (hash incorreto);
//! 7. Detecção de corrupção em disco com erro de integridade (`CasError::Integrity`);
//! 8. Esgotamento determinístico sob orçamento compartilhado (`Unknown(Exhausted::Steps)`);
//! 9. Saída de terminal normatizada `CHECKER_SMOKE_OK`.

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use logos_air::{
    Formula, FormulaNode, Grounding, Limits, Sort, SortDecl, SortDomain, Symbol, SymbolDecl,
    SymbolKind, SymbolResult, Term, TermNode, Vocabulary,
};
use logos_canon::{Text, to_canon};
use logos_core::{Budget, CheckOutcome, Exhausted};
use logos_crypto::{Hash, hash};
use logos_ledger::{Cas, CasError, Limits as CasLimits};
use logos_model::{FiniteModel, ModelChecker, ModelData, ModelLimits};
use logos_obligation::{
    EntryKind, LedgerEntry, LedgerError, LedgerView, Obligation, PremiseRef, Proposal, generate,
};

struct MemoryLedgerView<'a> {
    epoch: u64,
    entries: HashMap<PremiseRef, LedgerEntry<'a>>,
}

impl<'a> LedgerView for MemoryLedgerView<'a> {
    fn current_epoch(&self) -> u64 {
        self.epoch
    }

    fn lookup(&self, reference: PremiseRef) -> Result<Option<LedgerEntry<'a>>, LedgerError> {
        Ok(self.entries.get(&reference).cloned())
    }
}

fn require(cond: bool, msg: &str) -> Result<(), Box<dyn Error>> {
    if cond { Ok(()) } else { Err(msg.into()) }
}

fn walk_files(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                files.push(path);
            } else if path.is_dir() {
                files.extend(walk_files(&path)?);
            }
        }
    }
    Ok(files)
}

fn main() -> Result<(), Box<dyn Error>> {
    // 1. Construir vocabulário com sort "S", constante "c: S" e predicado unário "P(S)"
    let s_sort = Sort {
        name: Text::new("S")?,
    };
    let sd_sort = SortDecl {
        name: Text::new("S")?,
        domain: SortDomain::Open,
    };
    let sym_c = Symbol {
        name: Text::new("c")?,
        kind: SymbolKind::Const,
        arity: Vec::new(),
        result: SymbolResult::Sort(s_sort.clone()),
    };
    let sym_p = Symbol {
        name: Text::new("P")?,
        kind: SymbolKind::Pred,
        arity: vec![s_sort],
        result: SymbolResult::Bool,
    };
    let syd_c = SymbolDecl {
        signature: sym_c.clone(),
        grounding: Grounding::Glossed(Text::new("c const")?),
        gloss_pt: Text::new("c const pt")?,
    };
    let syd_p = SymbolDecl {
        signature: sym_p.clone(),
        grounding: Grounding::Glossed(Text::new("p pred")?),
        gloss_pt: Text::new("p pred pt")?,
    };
    let vocab = Vocabulary::new(1, vec![sd_sort], vec![syd_c, syd_p])?;

    // 2. Construir modelo finito onde P(c) é verdadeiro e ~P(c) é falso
    // Cardinalidade do domínio S = 2 (elementos 0 e 1); c mapeado para 0; P é verdadeiro em {0}
    let model_data = ModelData {
        vocab: hash(&vocab)?,
        domains: vec![2],
        consts: vec![(0, 0)],
        funcs: Vec::new(),
        preds: vec![(1, vec![vec![0]])],
    };
    let model = FiniteModel::new(model_data, &vocab, ModelLimits::default())?;

    // 3. Fórmulas: premissa P(c) e meta ~P(c)
    let term_c = Term::new(
        vec![TermNode::App {
            symbol: sym_c,
            args: 0,
        }],
        Limits::default(),
    )?;
    let f_pc = Formula::new(
        vec![FormulaNode::Pred {
            symbol: sym_p.clone(),
            args: vec![term_c.clone()],
        }],
        Limits::default(),
    )?;
    let f_not_pc = Formula::new(
        vec![
            FormulaNode::Not,
            FormulaNode::Pred {
                symbol: sym_p,
                args: vec![term_c],
            },
        ],
        Limits::default(),
    )?;

    // 4. Gerar obrigação canônica O via MemoryLedgerView: [P(c)] |- ~P(c)
    let ref_1 = PremiseRef::from_event_hash([0x01; 32]);
    let mut ledger_entries = HashMap::new();
    ledger_entries.insert(
        ref_1,
        LedgerEntry {
            kind: EntryKind::Fact,
            epoch: 1,
            retracted: false,
            expired: false,
            vocabulary: &vocab,
            formula: &f_pc,
        },
    );
    let ledger_view = MemoryLedgerView {
        epoch: 1,
        entries: ledger_entries,
    };
    let proposal = Proposal {
        vocabulary: &vocab,
        goal: &f_not_pc,
        cites: &[ref_1],
    };
    let obligation = generate(proposal, &ledger_view)?;
    let o_bytes = to_canon(&obligation)?;
    let o_hash = obligation.hash()?;

    // 5. Armazenar modelo finito M em CAS real e reabrir
    let temp_dir = std::env::temp_dir().join(format!(
        "logos-checker-smoke-cas-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    fs::create_dir_all(&temp_dir)?;

    let cas = Cas::open(&temp_dir, CasLimits::default())?;
    let m_bytes = model.encode_with_limits(ModelLimits::default())?;
    let model_hash = cas.put::<FiniteModel>(&m_bytes)?;
    drop(cas);

    // Reabrir CAS para garantir persistência em disco
    let cas = Cas::open(&temp_dir, CasLimits::default())?;

    // 6. Instanciar checker e orçamentos
    let checker = ModelChecker::new(&vocab)?;
    let generous = Budget::new(10_000_000, 10_000_000, 10_000_000, 0);

    // 7. Ambas as APIs em memória
    // Refutação: premissa P(c) é verdadeira, meta ~P(c) é falsa => contramodelo aceito!
    let outcome_ref = checker.check_refutation(&o_bytes, o_hash, &model, &generous);
    require(
        outcome_ref == CheckOutcome::Accepted,
        "refutação em memória deve ser aceita",
    )?;

    // Consistência: premissa P(c) é verdadeira => aceita!
    let outcome_cons = checker.check_consistency(&o_bytes, o_hash, &model, &generous);
    require(
        outcome_cons == CheckOutcome::Accepted,
        "consistência em memória deve ser aceita",
    )?;

    // 8. Ambas as APIs a partir do CAS
    let outcome_cas_ref = checker.check_refutation_from_source(
        &o_bytes,
        o_hash,
        |h| cas.get(h),
        model_hash,
        &generous,
    )?;
    require(
        outcome_cas_ref == CheckOutcome::Accepted,
        "refutação a partir do CAS deve ser aceita",
    )?;

    let outcome_cas_cons = checker.check_consistency_from_source(
        &o_bytes,
        o_hash,
        |h| cas.get(h),
        model_hash,
        &generous,
    )?;
    require(
        outcome_cas_cons == CheckOutcome::Accepted,
        "consistência a partir do CAS deve ser aceita",
    )?;

    // 9. Rejeição por premissa falsa: [~P(c)] |- ~P(c)
    let ref_2 = PremiseRef::from_event_hash([0x02; 32]);
    let mut bad_entries = HashMap::new();
    bad_entries.insert(
        ref_2,
        LedgerEntry {
            kind: EntryKind::Fact,
            epoch: 1,
            retracted: false,
            expired: false,
            vocabulary: &vocab,
            formula: &f_not_pc,
        },
    );
    let bad_view = MemoryLedgerView {
        epoch: 1,
        entries: bad_entries,
    };
    let bad_proposal = Proposal {
        vocabulary: &vocab,
        goal: &f_not_pc,
        cites: &[ref_2],
    };
    let bad_obligation = generate(bad_proposal, &bad_view)?;
    let bad_o_bytes = to_canon(&bad_obligation)?;
    let bad_o_hash = bad_obligation.hash()?;

    require(
        matches!(
            checker.check_refutation(&bad_o_bytes, bad_o_hash, &model, &generous),
            CheckOutcome::Rejected(_)
        ),
        "premissa falsa deve rejeitar refutação",
    )?;
    require(
        matches!(
            checker.check_consistency(&bad_o_bytes, bad_o_hash, &model, &generous),
            CheckOutcome::Rejected(_)
        ),
        "premissa falsa deve rejeitar consistência",
    )?;

    // 10. Rejeição por hash incorreto de O (P2)
    let wrong_o_hash = Hash::<Obligation>::from_bytes([0xEE; 32]);
    require(
        matches!(
            checker.check_refutation(&o_bytes, wrong_o_hash, &model, &generous),
            CheckOutcome::Rejected(_)
        ),
        "hash divergente deve rejeitar refutação",
    )?;
    require(
        matches!(
            checker.check_consistency(&o_bytes, wrong_o_hash, &model, &generous),
            CheckOutcome::Rejected(_)
        ),
        "hash divergente deve rejeitar consistência",
    )?;

    // 11. Esgotamento de passos no orçamento compartilhado
    let tiny_budget = Budget::new(1, 10_000_000, 10_000_000, 0);
    require(
        checker.check_refutation(&o_bytes, o_hash, &model, &tiny_budget)
            == CheckOutcome::Unknown(Exhausted::Steps),
        "orçamento exíguo deve retornar Unknown(Steps)",
    )?;

    // 12. Corrupção no CAS deve retornar CasError::Integrity
    let files = walk_files(&temp_dir)?;
    require(!files.is_empty(), "arquivo de objeto do CAS deve existir")?;
    let mut data = fs::read(&files[0])?;
    if !data.is_empty() {
        data[0] ^= 0xFF;
        fs::write(&files[0], &data)?;
    }
    let corrupt_res = checker.check_refutation_from_source(
        &o_bytes,
        o_hash,
        |h| cas.get(h),
        model_hash,
        &generous,
    );
    require(
        matches!(corrupt_res, Err(CasError::Integrity)),
        "objeto corrompido no CAS deve retornar CasError::Integrity",
    )?;

    // Limpeza
    let _ = fs::remove_dir_all(&temp_dir);

    println!("CHECKER_SMOKE_OK");
    Ok(())
}
