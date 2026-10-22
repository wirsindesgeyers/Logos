//! Planilha de revisão humana da Etapa 9.1 a partir do corpus de políticas.
//!
//! `cargo run -p logos-seal --example review_sheet -- [--check]` lê
//! `docs/plans/etapa-9.1-revisao/{politicas.air,intencoes.tsv}` e grava (ou
//! confere) `planilha.md` com a paráfrase exata de `render_pt` e seu `h(P)`.
//! Não marca nenhuma leitura como correta: isso é trabalho do revisor.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::{collections::BTreeMap, fmt::Write as _, fs, path::Path};

use logos_air_text::{References, parse_document};
use logos_seal::{GRAMMAR_ID, Paraphraser, RENDERER_VERSION};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/plans/etapa-9.1-revisao");
    let source = fs::read_to_string(dir.join("politicas.air")).unwrap();
    let document = parse_document(&source, &References::new()).unwrap();
    let intents: BTreeMap<String, String> = fs::read_to_string(dir.join("intencoes.tsv"))
        .unwrap()
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| {
            let (claim, intent) = line.split_once('\t').unwrap();
            (claim.to_owned(), intent.to_owned())
        })
        .collect();
    assert_eq!(
        intents.len(),
        document.claims.len(),
        "uma intenção por claim"
    );
    let paraphraser = Paraphraser::new(&document.vocabulary).unwrap();
    let h_v = hex(logos_crypto::hash(&document.vocabulary).unwrap().as_bytes());
    let mut out = String::new();
    writeln!(out, "# Planilha de revisão humana — Etapa 9.1\n").unwrap();
    writeln!(
        out,
        "Gerada por `cargo run -p logos-seal --example review_sheet`; não editar à mão\n\
         as paráfrases. Gramática `{GRAMMAR_ID}`, renderizador `{RENDERER_VERSION}`,\n\
         `h(V)` = `{h_v}`. Leia a paráfrase **como está escrita** e compare com a\n\
         intenção: a leitura só é correta se as duas dizem a mesma coisa, inclusive\n\
         escopo dos quantificadores, negações e condições. Registre o resultado no\n\
         [README](README.md#registro-da-revisão).\n"
    )
    .unwrap();
    for claim in &document.claims {
        let name = claim.name.as_str();
        let paraphrase = paraphraser.render(&claim.formula).unwrap();
        assert_eq!(paraphraser.parse(paraphrase.text()).unwrap(), claim.formula);
        writeln!(out, "## {name}\n").unwrap();
        writeln!(out, "**Intenção:** {}\n", intents[name]).unwrap();
        writeln!(out, "```text\n{}\n```\n", paraphrase.text()).unwrap();
        writeln!(
            out,
            "`h(P)` = `{}`\n",
            hex(paraphrase.digest().unwrap().as_bytes())
        )
        .unwrap();
        writeln!(out, "Leitura correta: ☐ sim ☐ não — observações:\n").unwrap();
    }
    let path = dir.join("planilha.md");
    if std::env::args().any(|arg| arg == "--check") {
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            out,
            "planilha.md desatualizada"
        );
    } else {
        fs::write(&path, &out).unwrap();
    }
    println!("REVIEW_SHEET_OK claims={}", document.claims.len());
}
