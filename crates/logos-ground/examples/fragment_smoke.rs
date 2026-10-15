//! Smoke público de mecanismo genérico; nenhuma fonte é autenticada aqui.

use logos_air::{Formula, FormulaNode, Limits, Sort, Vocabulary};
use logos_canon::{Text, from_canon, to_canon};
use logos_core::Exhausted;
use logos_ground::{
    AxiomLimits, Fragment, GroundError, classify, domain_axioms, domain_axioms_with_limits,
};
use logos_obligation::{LedgerEntry, LedgerError, LedgerView, PremiseRef, Proposal, generate};

struct Empty;
impl LedgerView for Empty {
    fn current_epoch(&self) -> u64 {
        0
    }
    fn lookup(&self, _: PremiseRef) -> Result<Option<LedgerEntry<'_>>, LedgerError> {
        Ok(None)
    }
}
fn unhex(s: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    s.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| Ok(u8::from_str_radix(std::str::from_utf8(b)?, 16)?))
        .collect()
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data: serde_json::Value =
        serde_json::from_str(include_str!("../../../spec/vectors/fragment/vectors.json"))?;
    let cases = data["vectors"].as_array().ok_or("vectors")?;
    let sort = Sort {
        name: Text::new("E")?,
    };
    let mut total = 0;
    for case in cases {
        let v: Vocabulary = from_canon(&unhex(case["vocabulary"].as_str().ok_or("V")?)?)?;
        let goal = Formula::new(
            vec![FormulaNode::Forall(sort.clone()), FormulaNode::True],
            Limits::default(),
        )?;
        let o = generate(
            Proposal {
                vocabulary: &v,
                goal: &goal,
                cites: &[],
            },
            &Empty,
        )?;
        if classify(&o, &v)? != Fragment::FiniteDomain {
            return Err("classification".into());
        }
        let candidates = domain_axioms(&v, &sort)?;
        for (i, f) in std::iter::once(candidates.closure())
            .chain(candidates.distinct())
            .enumerate()
        {
            if to_canon(f)? != unhex(case["formulas"][i].as_str().ok_or("formula")?)? {
                return Err("independent vector".into());
            }
            total += 1;
        }
        if domain_axioms_with_limits(
            &v,
            &sort,
            AxiomLimits {
                max_nodes: 0,
                ..AxiomLimits::default()
            },
        ) != Err(GroundError::Unknown(Exhausted::Size))
        {
            return Err("size limit".into());
        }
    }
    println!(
        "FRAGMENT_SMOKE_OK cases={} candidates={total} source=opaque claims=0",
        cases.len()
    );
    Ok(())
}
