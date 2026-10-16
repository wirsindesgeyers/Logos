//! Worker do diferencial SAT: 5000 O distintas e tabela verdade independente.
#[path = "../tests/support/mod.rs"]
mod support;
use logos_air::{FormulaNode as F, Vocabulary};
use logos_canon::from_canon;
use logos_crypto::hash;
use logos_ground::{clausify, ground};
use std::collections::BTreeSet;
use std::io::{self, Write};
use support::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data: serde_json::Value =
        serde_json::from_str(include_str!("../../../spec/vectors/grounding/vectors.json"))?;
    let raw = data["vectors"][0]["vocabulary"].as_str().ok_or("V")?;
    let bytes = raw
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|s| u8::from_str_radix(std::str::from_utf8(s).unwrap(), 16))
        .collect::<Result<Vec<_>, _>>()?;
    let v: Vocabulary = from_canon(&bytes)?;
    let p = pred(v.symbols()[0].signature.clone(), vec![]);
    let q = pred(v.symbols()[1].signature.clone(), vec![]);
    let bases = vec![
        f(vec![F::True]),
        f(vec![F::False]),
        p.clone(),
        q.clone(),
        combine(F::Not, std::slice::from_ref(&p)),
        combine(F::Not, std::slice::from_ref(&q)),
        combine(F::And(2), &[p.clone(), q.clone()]),
        combine(F::Or(2), &[p.clone(), q.clone()]),
        combine(F::Implies, &[p.clone(), q.clone()]),
        combine(F::Iff, &[p.clone(), q]),
        combine(
            F::Not,
            &[combine(F::And(2), &[p.clone(), f(vec![F::False])])],
        ),
        combine(F::Not, &[combine(F::Or(2), &[p.clone(), f(vec![F::True])])]),
    ];
    let gammas = [
        vec![],
        vec![p.clone()],
        vec![combine(F::Not, &[p])],
        vec![f(vec![F::False])],
    ];
    let ops = [F::And(2), F::Or(2), F::Implies, F::Iff];
    let mut seen = BTreeSet::new();
    let mut out = io::BufWriter::new(io::stdout().lock());
    for i in 0..5000 {
        let mut code = i;
        let a = code % 12;
        code /= 12;
        let b = code % 12;
        code /= 12;
        let c = code % 12;
        code /= 12;
        let x = code % 4;
        code /= 4;
        let y = code % 4;
        let goal = combine(
            ops[x].clone(),
            &[
                bases[a].clone(),
                combine(ops[y].clone(), &[bases[b].clone(), bases[c].clone()]),
            ],
        );
        let o = closed(&v, &goal, &gammas[i % 4]);
        let ho = hash(&o)?;
        if !seen.insert(ho) {
            return Err("duplicate obligation".into());
        }
        let g = ground(&o, &v)?;
        let (cnf, map) = clausify(&g)?;
        if map.origins().len() != cnf.clauses().len() {
            return Err("provenance".into());
        }
        writeln!(
            out,
            "{}",
            serde_json::json!({"case":i,"obligation":ho.to_hex(),"dimacs":std::str::from_utf8(cnf.dimacs())?,"valid":tautology(&g)})
        )?;
    }
    out.flush()?;
    Ok(())
}
