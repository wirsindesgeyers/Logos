//! Fixture da 5.9: DAG abstrato em JSON → ledger real → retratações reais.
//!
//! `cargo run -p logos-kernel --example atms_fixture -- ENTRADA.json SAIDA.json`.
//! Entrada: `{"claims": [{"kind": "assumption"} | {"kind": "route", "cites": [i],
//! "used": [i]}], "retract": [i]}`; índices referem claims anteriores. Rotas com
//! o mesmo `cites` têm o mesmo h(O). Saída: estado final de cada claim e os
//! registros Invalidated como `[claim, retração_raiz, premissa]` (índices).
//! Só para o teste diferencial com a referência Python; não é API.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[cfg(target_os = "linux")]
fn main() {
    use std::collections::HashMap;

    use logos_air::{Formula, FormulaNode, Limits as AirLimits, Vocabulary};
    use logos_canon::{Text, to_canon};
    use logos_core::{Budget, CheckOutcome};
    use logos_crypto::{FileSigner, Hash, Signer, hash, hash_raw};
    use logos_kernel::retract::Retractions;
    use logos_ledger::{
        Cas, Certificate, CheckRecord, Checker, Consistency, Event, EventBody, EventProposal,
        Ledger, Limits, Route, TaintedText,
    };
    use logos_obligation::{EntryKind, Obligation};

    let args: Vec<String> = std::env::args().skip(1).collect();
    let input: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&args[0]).unwrap()).unwrap();
    let dir = std::env::temp_dir().join(format!("logos-atms-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let admin = FileSigner::from_seed(&[0x5A; 32]).unwrap();
    let cas = Cas::open(dir.join("cas"), Limits::default()).unwrap();
    let vocabulary = Vocabulary::new(1, vec![], vec![]).unwrap();
    let hv = cas.put(&to_canon(&vocabulary).unwrap()).unwrap();
    cas.put::<Certificate>(b"cert").unwrap();
    cas.put::<Checker>(b"checker").unwrap();
    let truth = || Formula::new(vec![FormulaNode::True], AirLimits::default()).unwrap();
    let mut ledger = Ledger::open(&dir, &admin).unwrap();
    ledger
        .append(EventBody::Genesis {
            ledger_version: 1,
            kernel_binary: [0x5A; 32],
            kernel_key: admin.key_id().unwrap(),
        })
        .unwrap();
    let indices = |v: &serde_json::Value| -> Vec<usize> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|i| usize::try_from(i.as_u64().unwrap()).unwrap())
            .collect()
    };
    let mut ids: Vec<Hash<Event>> = Vec::new();
    let mut obligations: HashMap<Vec<usize>, Hash<Obligation>> = HashMap::new();
    for claim in input["claims"].as_array().unwrap() {
        let id = if claim["kind"] == "assumption" {
            hash(
                &ledger
                    .append(EventBody::AssumptionDeclared {
                        vocabulary: hv,
                        formula: truth(),
                        scope: Text::new("").unwrap(),
                        author: admin.key_id().unwrap(),
                    })
                    .unwrap(),
            )
            .unwrap()
        } else {
            let mut cites_idx = indices(&claim["cites"]);
            cites_idx.sort_unstable();
            let mut cites: Vec<Hash<Event>> = cites_idx.iter().map(|&i| ids[i]).collect();
            cites.sort_unstable();
            let mut used: Vec<Hash<Event>> =
                indices(&claim["used"]).iter().map(|&i| ids[i]).collect();
            used.sort_unstable();
            // O ATMS só usa a identidade h(O); gerar O com snapshot custaria a
            // varredura de escopo dos ancestrais (3.3) a cada rota. Um objeto
            // distinto por conjunto de citações, publicado no CAS, basta aqui.
            let next = obligations.len();
            let obligation = *obligations.entry(cites_idx).or_insert_with(|| {
                cas.put::<Obligation>(format!("atms-node-{next}").as_bytes())
                    .unwrap()
            });
            ledger
                .append(EventBody::Proposed {
                    proposal: EventProposal {
                        text_pt: TaintedText::new("rota".to_owned()).unwrap(),
                        goal: truth(),
                        cites: cites.clone(),
                        speculative_on: vec![],
                        vocabulary: hv,
                    },
                    author: admin.key_id().unwrap(),
                })
                .unwrap();
            ledger
                .append(EventBody::ObligationCreated {
                    obligation,
                    route: Route::Model,
                    artifact: [0; 32],
                    cnf: None,
                })
                .unwrap();
            hash(
                &ledger
                    .append(EventBody::Verified {
                        check: CheckRecord {
                            obligation,
                            certificate: hash_raw::<Certificate>(b"cert"),
                            checker: hash_raw::<Checker>(b"checker"),
                            budget: Budget::new(100, 1024, 1024, 0),
                            outcome: CheckOutcome::Accepted,
                        },
                        cited: cites,
                        used,
                        consistency: Consistency::Unknown { check: None },
                    })
                    .unwrap(),
            )
            .unwrap()
        };
        ids.push(id);
    }
    let retractions = Retractions::new(admin.public_key().clone());
    let mut roots = Vec::new();
    for target in indices(&input["retract"]) {
        let result = retractions
            .retract(
                &mut ledger,
                &admin,
                ids[target],
                Text::new("diferencial").unwrap(),
            )
            .unwrap();
        roots.push(result.retracted);
    }
    let position: HashMap<Hash<Event>, usize> =
        ids.iter().enumerate().map(|(i, h)| (*h, i)).collect();
    let root_position: HashMap<Hash<Event>, usize> =
        roots.iter().enumerate().map(|(i, h)| (*h, i)).collect();
    let index = ledger.live_index().unwrap();
    let status: Vec<&str> = ids
        .iter()
        .map(|id| match index.status(*id).unwrap().unwrap().kind {
            EntryKind::Verified => "verified",
            EntryKind::Assumption => "assumption",
            EntryKind::Invalidated => "invalidated",
            other => panic!("estado inesperado {other:?}"),
        })
        .collect();
    drop(index);
    let head = ledger.head().unwrap().unwrap();
    let mut records = Vec::new();
    let mut next = 0;
    while next <= head.seq {
        let page = ledger.events_from(next, 256).unwrap();
        next += page.len() as u64;
        for event in page {
            if let EventBody::Invalidated { claim, causes } = event.body {
                records.push(serde_json::json!([
                    position[&claim],
                    root_position[&causes[0]],
                    position[&causes[1]],
                ]));
            }
        }
    }
    let hashes: Vec<String> = ids
        .iter()
        .map(|h| h.as_bytes().iter().map(|b| format!("{b:02x}")).collect())
        .collect();
    let output = serde_json::json!({"status": status, "records": records, "hashes": hashes});
    std::fs::write(&args[1], serde_json::to_string(&output).unwrap()).unwrap();
    drop(ledger);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("atms_fixture requer Linux");
    std::process::exit(1);
}
