//! Provisionamento de fixture pública para testes10.1; não é ferramenta operacional.
#![forbid(unsafe_code)]

use logos_air::{Formula, FormulaNode, Limits, Vocabulary};
use logos_canon::{from_canon, to_canon};
use logos_crypto::FileSigner;
use logos_ground::{clausify, ground_for_check};
use logos_ledger::Ledger;
use logos_model::{FiniteModel, ModelData, ModelLimits};
use logos_obligation::{DecodeLimits, Proposal, generate};
use std::{fs, io::Write, os::unix::fs::OpenOptionsExt, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let (root, public_vocabulary) = match args.as_slice() {
        [root] => (root, None),
        [root, vocabulary] => (root, Some(vocabulary)),
        _ => return Err("usage: client_fixture DIRECTORY [PUBLIC_VOCABULARY]".into()),
    };
    let root = Path::new(root);
    // Semente pública de teste fica somente em Rust; Python recebe caminhos/objetos públicos.
    let signer = FileSigner::from_seed(&[0x76; 32])?;
    let mut key = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(root.join("operator.key"))?;
    key.write_all("76".repeat(32).as_bytes())?;
    key.sync_all()?;
    let ledger = Ledger::open(&root.join("ledger"), &signer)?;
    let vocabulary = match public_vocabulary {
        Some(path) => from_canon::<Vocabulary>(&fs::read(path)?)?,
        None => Vocabulary::new(1, vec![], vec![])?,
    };
    let vbytes = to_canon(&vocabulary)?;
    let hv = ledger.cas().put::<Vocabulary>(&vbytes)?;
    if public_vocabulary.is_some() {
        let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        println!(
            "{}",
            serde_json::json!({"vocabulary": hv.to_hex(), "vocabulary_bytes": hex(&vbytes)})
        );
        return Ok(());
    }
    let goal = Formula::new(vec![FormulaNode::True], Limits::default())?;
    let index = ledger.live_index()?;
    let view = index.snapshot(ledger.cas(), &[], "")?;
    let o = generate(
        Proposal {
            vocabulary: &vocabulary,
            goal: &goal,
            cites: &[],
        },
        &view,
    )?;
    let bytes = to_canon(&o)?;
    let ground = ground_for_check(
        &bytes,
        logos_crypto::hash(&o)?,
        &vocabulary,
        DecodeLimits::default(),
        Default::default(),
    )?;
    let (cnf, _) = clausify(&ground)?;
    let pair = cnf
        .clauses()
        .iter()
        .enumerate()
        .find_map(|(i, c)| {
            if c.len() != 1 {
                return None;
            }
            cnf.clauses()
                .iter()
                .position(|d| d == &[-c[0]])
                .map(|j| (i + 1, j + 1))
        })
        .ok_or("fixture requires opposite unit clauses")?;
    let proof = format!("{} 0 {} {} 0\n", cnf.clauses().len() + 1, pair.0, pair.1);
    let model = FiniteModel::new(
        ModelData {
            vocab: hv,
            domains: vec![],
            consts: vec![],
            funcs: vec![],
            preds: vec![],
        },
        &vocabulary,
        ModelLimits::default(),
    )?;
    let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    println!(
        "{}",
        serde_json::json!({"vocabulary": hv.to_hex(), "vocabulary_bytes": hex(&vbytes), "model": hex(&to_canon(&model)?), "proof": hex(proof.as_bytes())})
    );
    Ok(())
}
