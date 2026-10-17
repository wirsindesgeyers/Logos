//! Apoio não autoritativo da4.6: compõe APIs existentes; não emite claims.
use logos_air::Vocabulary;
use logos_canon::{from_canon, to_canon};
use logos_core::{Budget, CheckOutcome};
use logos_crypto::hash_raw;
use logos_ground::{Cnf, GroundLimits, clausify, ground_for_check};
use logos_lrat::{CakeRegistry, LratChecker};
use logos_model::{FiniteModel, ModelChecker, ModelLimits};
use logos_obligation::{DecodeLimits, Obligation};
use serde_json::{Value, json};
use std::io::{self, BufRead, Read, Write};
use std::path::Path;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn bytes(v: &Value, key: &str) -> Result<Vec<u8>> {
    let s = v[key].as_str().ok_or("missing hex field")?;
    if !s.len().is_multiple_of(2) || s.len() > 33_554_432 {
        return Err("hex size".into());
    }
    s.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| Ok(u8::from_str_radix(std::str::from_utf8(b)?, 16)?))
        .collect()
}
fn process(request: &Value, cake: Option<&CakeRegistry>) -> Result<Value> {
    if request["version"] != 1 {
        return Err("transport version".into());
    }
    let vocabulary: Vocabulary = from_canon(&bytes(request, "vocabulary")?)?;
    let obligation = bytes(request, "obligation")?;
    let h_o = hash_raw::<Obligation>(&obligation);
    let g = ground_for_check(
        &obligation,
        h_o,
        &vocabulary,
        DecodeLimits::default(),
        GroundLimits::default(),
    )?;
    let (cnf, _) = clausify(&g)?;
    let h_cnf = hash_raw::<Cnf>(cnf.dimacs());
    let budget = Budget::new(100_000_000, 268_435_456, 16_777_216, 10_000);
    match request["mode"].as_str().ok_or("mode")? {
        "encode" => Ok(json!({"ground":hex(&to_canon(&g)?),"cnf":hex(cnf.dimacs())})),
        "proof" => {
            if bytes(request, "cnf")? != cnf.dimacs() {
                return Err("CNF differs from obligation".into());
            }
            let proof = bytes(request, "proof")?;
            let rust = LratChecker::new()?.check_lrat(cnf.dimacs(), h_cnf, &proof, &budget);
            let external = cake.ok_or("cake_lpr required")?.check_bytes(
                cnf.dimacs(),
                h_cnf,
                &proof,
                hash_raw(&proof),
                &budget,
            )?;
            Ok(json!({"rust":format!("{rust:?}"),"cake":format!("{external:?}")}))
        }
        "model" | "witness" => {
            let model = FiniteModel::decode_with_limits(
                &bytes(request, "model")?,
                &vocabulary,
                ModelLimits::default(),
            )?;
            let checker = ModelChecker::new(&vocabulary)?;
            let result = if request["mode"] == "witness" {
                checker.check_consistency(&obligation, h_o, &model, &budget)
            } else {
                checker.check_refutation(&obligation, h_o, &model, &budget)
            };
            Ok(json!({"model":format!("{result:?}"),"accepted":result==CheckOutcome::Accepted}))
        }
        _ => Err("unknown mode".into()),
    }
}
fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let cake = match args.as_slice() {
        [] => None,
        [flag, path] if flag == "--cake-lpr" => Some(CakeRegistry::register(Path::new(path))?),
        _ => return Err("usage: prover_bridge [--cake-lpr BIN]".into()),
    };
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    loop {
        let mut line = String::new();
        let n = std::io::Read::by_ref(&mut input)
            .take(100_663_297)
            .read_line(&mut line)?;
        if n == 0 {
            break;
        }
        if n > 100_663_296 {
            return Err("request size".into());
        }
        let request: Value = serde_json::from_str(&line)?;
        let response = match process(&request, cake.as_ref()) {
            Ok(response) => json!({"version":1,"ok":true,"result":response}),
            Err(error) => json!({"version":1,"ok":false,"error":error.to_string()}),
        };
        writeln!(output, "{response}")?;
        output.flush()?;
    }
    Ok(())
}
