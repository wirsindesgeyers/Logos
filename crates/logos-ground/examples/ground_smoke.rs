//! Smoke público de ground/v1 com O gerado e hipóteses explicitamente condicionais.
use logos_air::{Formula, Vocabulary};
use logos_canon::{from_canon, to_canon};
use logos_crypto::hash;
use logos_ground::ground;
use logos_obligation::{
    EntryKind, LedgerEntry, LedgerError, LedgerView, PremiseRef, Proposal, generate,
};
struct View<'a> {
    v: &'a Vocabulary,
    fs: &'a [Formula],
}
impl LedgerView for View<'_> {
    fn current_epoch(&self) -> u64 {
        0
    }
    fn lookup(&self, r: PremiseRef) -> Result<Option<LedgerEntry<'_>>, LedgerError> {
        let bytes = r.event_hash();
        let i = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        Ok(self.fs.get(i).map(|f| LedgerEntry {
            kind: EntryKind::Assumption,
            epoch: 0,
            retracted: false,
            vocabulary: self.v,
            formula: f,
        }))
    }
}
fn hex(s: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    s.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| Ok(u8::from_str_radix(std::str::from_utf8(b)?, 16)?))
        .collect()
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data: serde_json::Value =
        serde_json::from_str(include_str!("../../../spec/vectors/grounding/vectors.json"))?;
    let cases = data["vectors"].as_array().ok_or("vectors")?;
    for c in cases {
        let v: Vocabulary = from_canon(&hex(c["vocabulary"].as_str().ok_or("V")?)?)?;
        let goal: Formula = from_canon(&hex(c["goal"].as_str().ok_or("goal")?)?)?;
        let fs = c["premises"]
            .as_array()
            .ok_or("premises")?
            .iter()
            .map(|f| Ok(from_canon(&hex(f.as_str().ok_or("F")?)?)?))
            .collect::<Result<Vec<Formula>, Box<dyn std::error::Error>>>()?;
        let refs = (0..fs.len())
            .map(|i| {
                let mut r = [0; 32];
                r[..4].copy_from_slice(&(i as u32).to_be_bytes());
                PremiseRef::from_event_hash(r)
            })
            .collect::<Vec<_>>();
        let o = generate(
            Proposal {
                vocabulary: &v,
                goal: &goal,
                cites: &refs,
            },
            &View { v: &v, fs: &fs },
        )?;
        if to_canon(&o)? != hex(c["obligation"].as_str().ok_or("O")?)? {
            return Err("O vector".into());
        }
        let g = ground(&o, &v)?;
        if to_canon(&g)? != hex(c["ground"].as_str().ok_or("G")?)?
            || hash(&g)?.to_hex() != c["digest"].as_str().ok_or("hash")?
        {
            return Err("ground vector".into());
        }
    }
    println!(
        "GROUND_SMOKE_OK cases={} wire=ground/v1 claims=0",
        cases.len()
    );
    Ok(())
}
