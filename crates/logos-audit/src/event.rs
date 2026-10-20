//! Decoder independente derivado de spec/events.md, sem tipos do appender.

use logos_air::Formula;
use logos_canon::{Array, CanonDecode, Text, Value, from_canon, to_canon};
use logos_core::{Budget, CheckOutcome};
use logos_crypto::{Domain, Hash, PublicKey, SigDomain, Signature, hash_raw, sig_tags, tags};
use logos_obligation::Obligation;

use crate::{Result, fail};

pub(crate) const MAX_WIRE: usize = 16_777_216;

pub(crate) struct EventDomain;
impl Domain for EventDomain {
    const TAG: &'static [u8] = tags::EVENT;
}
impl SigDomain for EventDomain {
    const TAG: &'static [u8] = sig_tags::EVENT;
}
pub(crate) struct Certificate;
impl Domain for Certificate {
    const TAG: &'static [u8] = tags::CERTIFICATE;
}
pub(crate) struct Checker;
impl Domain for Checker {
    const TAG: &'static [u8] = tags::CHECKER;
}

#[derive(Clone)]
pub(crate) struct Check {
    pub obligation: Hash<Obligation>,
    pub certificate: Hash<Certificate>,
    pub checker: [u8; 32],
    pub budget: Budget,
    pub outcome: CheckOutcome,
}

pub(crate) struct Recheck {
    pub check: Check,
    pub model: [u8; 32],
    pub consistency: bool,
}

pub(crate) struct Derivation {
    pub obligation: Hash<Obligation>,
    pub artifact: [u8; 32],
    pub cnf: Option<[u8; 32]>,
}

pub(crate) struct Parsed {
    pub seq: u64,
    pub epoch: u64,
    pub prev: Option<[u8; 32]>,
    pub digest: [u8; 32],
    pub advance: bool,
    pub proof: bool,
    pub proof_check: Option<Check>,
    pub refs: Vec<(&'static str, [u8; 32])>,
    pub rechecks: Vec<Recheck>,
    pub derivation: Option<Derivation>,
}

pub(crate) fn arr(v: &Value) -> Result<&[Value]> {
    match v {
        Value::Array(a) => Ok(a.items()),
        _ => Err(fail("expected array")),
    }
}
pub(crate) fn number(v: &Value) -> Result<u64> {
    Ok(u64::from_value(v)?)
}
pub(crate) fn digest(v: &Value) -> Result<[u8; 32]> {
    match v {
        Value::Bytes(b) => b
            .as_slice()
            .try_into()
            .map_err(|_| fail("expected bytes32")),
        _ => Err(fail("expected bytes32")),
    }
}
fn text(v: &Value) -> Result<()> {
    let t = Text::from_value(v)?;
    if t.as_str().len() > 65_536 {
        return Err(fail("text exceeds 64 KiB"));
    }
    Ok(())
}
fn authority(v: &Value) -> Result<()> {
    let [key, sig] = arr(v)? else {
        return Err(fail("invalid authority"));
    };
    digest(key)?;
    Signature::from_value(sig)?;
    Ok(())
}
fn refs(v: &Value, sorted: bool) -> Result<Vec<[u8; 32]>> {
    let a = arr(v)?;
    if a.len() > 1024 {
        return Err(fail("too many references"));
    }
    let values = a.iter().map(digest).collect::<Result<Vec<_>>>()?;
    if sorted && !values.windows(2).all(|p| p[0] < p[1]) {
        return Err(fail("references not strictly ordered"));
    }
    Ok(values)
}
fn optional(v: &Value) -> Result<Option<[u8; 32]>> {
    match arr(v)? {
        [] => Ok(None),
        [v] => Ok(Some(digest(v)?)),
        _ => Err(fail("invalid optional digest")),
    }
}
fn check(v: &Value, parsed: &mut Parsed) -> Result<Check> {
    let [o, cert, checker, budget, outcome] = arr(v)? else {
        return Err(fail("invalid check"));
    };
    let obligation = digest(o)?;
    let certificate = digest(cert)?;
    parsed.refs.extend([
        ("obligation", obligation),
        ("certificate", certificate),
        ("checker", digest(checker)?),
    ]);
    let outcome = CheckOutcome::from_value(outcome)?;
    if let CheckOutcome::Rejected(reason) = &outcome
        && reason.as_str().len() > 65_536
    {
        return Err(fail("check reason exceeds 64 KiB"));
    }
    Ok(Check {
        obligation: Hash::from_bytes(obligation),
        certificate: Hash::from_bytes(certificate),
        checker: digest(checker)?,
        budget: Budget::from_value(budget)?,
        outcome,
    })
}
fn accepted(c: &Check) -> Result<()> {
    if c.outcome != CheckOutcome::Accepted {
        return Err(fail("event requires Accepted"));
    }
    Ok(())
}
fn model(c: Check, m: &Value, consistency: bool, parsed: &mut Parsed) -> Result<()> {
    accepted(&c)?;
    let m = digest(m)?;
    parsed.refs.push(("model", m));
    parsed.rechecks.push(Recheck {
        check: c,
        model: m,
        consistency,
    });
    Ok(())
}

pub(crate) fn parse(wire: &[u8], key: &PublicKey) -> Result<Parsed> {
    if wire.len() > MAX_WIRE {
        return Err(fail("event exceeds 16 MiB"));
    }
    let value = from_canon::<Value>(wire)?;
    let [
        Value::Unsigned(1),
        Value::Unsigned(0),
        seq,
        epoch,
        prev,
        body,
        key_id,
        sig,
    ] = arr(&value)?
    else {
        return Err(fail("invalid event envelope"));
    };
    let kernel = digest(key_id)?;
    if key.key_id().map_err(|e| fail(e.to_string()))?.as_bytes() != &kernel {
        return Err(fail("kernel KeyId mismatch"));
    }
    let fields = arr(&value)?;
    let unsigned = Value::Array(Array::new(fields[..7].to_vec())?);
    logos_crypto::verify::<EventDomain>(&to_canon(&unsigned)?, &Signature::from_value(sig)?, key)
        .map_err(|e| fail(format!("signature: {e}")))?;
    let mut p = Parsed {
        seq: number(seq)?,
        epoch: number(epoch)?,
        prev: optional(prev)?,
        digest: *hash_raw::<EventDomain>(wire).as_bytes(),
        advance: false,
        proof: false,
        proof_check: None,
        refs: Vec::new(),
        rechecks: Vec::new(),
        derivation: None,
    };
    let b = arr(body)?;
    match b {
        [
            Value::Unsigned(1),
            Value::Unsigned(0),
            version,
            binary,
            genesis_key,
        ] => {
            if number(version)? != 1
                || p.seq != 0
                || p.epoch != 0
                || p.prev.is_some()
                || digest(genesis_key)? != kernel
            {
                return Err(fail("invalid Genesis"));
            }
            digest(binary)?;
        }
        [Value::Unsigned(1), Value::Unsigned(1), v, compatible, auth] => {
            bool::from_value(compatible)?;
            authority(auth)?;
            p.refs.push(("vocab", digest(v)?));
        }
        [Value::Unsigned(1), Value::Unsigned(2), proposal, author] => {
            let [tainted, formula, cites, spec, vocab] = arr(proposal)? else {
                return Err(fail("invalid proposal"));
            };
            let Value::Bytes(bytes) = tainted else {
                return Err(fail("invalid tainted text"));
            };
            if bytes.len() > 65_536 || std::str::from_utf8(bytes).is_err() {
                return Err(fail("invalid tainted UTF-8"));
            }
            Formula::from_value(formula)?;
            refs(cites, false)?;
            refs(spec, false)?;
            digest(author)?;
            p.refs.push(("vocab", digest(vocab)?));
        }
        [Value::Unsigned(1), Value::Unsigned(3), o, route, artifact] => {
            if number(route)? > 3 {
                return Err(fail("unknown route"));
            }
            digest(artifact)?;
            p.refs.push(("obligation", digest(o)?));
            if number(route)? == 1 {
                p.derivation = Some(Derivation {
                    obligation: Hash::from_bytes(digest(o)?),
                    artifact: digest(artifact)?,
                    cnf: None,
                });
            }
        }
        [
            Value::Unsigned(2),
            Value::Unsigned(3),
            o,
            Value::Unsigned(1),
            artifact,
            record,
        ] => {
            let [h, Value::Unsigned(1)] = arr(record)? else {
                return Err(fail("CNF encoder version/record"));
            };
            let cnf = digest(h)?;
            p.refs.extend([("obligation", digest(o)?), ("cnf", cnf)]);
            p.derivation = Some(Derivation {
                obligation: Hash::from_bytes(digest(o)?),
                artifact: digest(artifact)?,
                cnf: Some(cnf),
            });
        }
        [
            Value::Unsigned(1),
            Value::Unsigned(4),
            record,
            cited,
            used,
            consistency,
        ] => {
            let c = check(record, &mut p)?;
            accepted(&c)?;
            let cited = refs(cited, true)?;
            let used = refs(used, true)?;
            if !used.iter().all(|r| cited.binary_search(r).is_ok()) {
                return Err(fail("used not subset of cited"));
            }
            match arr(consistency)? {
                [Value::Unsigned(0), opt] => match arr(opt)? {
                    [] => {}
                    [v] => {
                        let other = check(v, &mut p)?;
                        if other.obligation != c.obligation
                            || other.outcome == CheckOutcome::Accepted
                        {
                            return Err(fail("invalid Unknown consistency"));
                        }
                    }
                    _ => return Err(fail("invalid consistency optional")),
                },
                [Value::Unsigned(1), m, record] => {
                    let other = check(record, &mut p)?;
                    if other.obligation != c.obligation {
                        return Err(fail("consistency obligation mismatch"));
                    }
                    model(other, m, true, &mut p)?;
                }
                _ => return Err(fail("invalid consistency")),
            }
            p.proof = true;
            p.proof_check = Some(c);
        }
        [Value::Unsigned(1), Value::Unsigned(5), record, m] => {
            let c = check(record, &mut p)?;
            model(c, m, false, &mut p)?;
        }
        [Value::Unsigned(2), Value::Unsigned(13), record] => {
            let c = check(record, &mut p)?;
            if !matches!(c.outcome, CheckOutcome::Rejected(_)) {
                return Err(fail("CertificateRejected requires Rejected"));
            }
        }
        [Value::Unsigned(1), Value::Unsigned(6), record, machine] => {
            let c = check(record, &mut p)?;
            if !matches!(c.outcome, CheckOutcome::Unknown(_)) {
                return Err(fail("UnknownRecorded requires Unknown"));
            }
            text(machine)?;
        }
        [
            Value::Unsigned(1),
            Value::Unsigned(7),
            vocab,
            formula,
            scope,
            author,
        ] => {
            p.refs.push(("vocab", digest(vocab)?));
            Formula::from_value(formula)?;
            text(scope)?;
            digest(author)?;
        }
        [Value::Unsigned(1), Value::Unsigned(8), target, reason, auth] => {
            digest(target)?;
            text(reason)?;
            authority(auth)?;
        }
        [Value::Unsigned(1), Value::Unsigned(9), claim, causes] => {
            digest(claim)?;
            if refs(causes, false)?.is_empty() {
                return Err(fail("empty causes"));
            }
        }
        [Value::Unsigned(1), Value::Unsigned(10), new_epoch, reason] => {
            advance(new_epoch, reason, &mut p)?;
        }
        [
            Value::Unsigned(2),
            Value::Unsigned(10),
            new_epoch,
            reason,
            policy,
            vocab,
        ] => {
            let policy = optional(policy)?;
            let vocab = optional(vocab)?;
            if policy.is_none() && vocab.is_none() {
                return Err(fail("empty epoch-body/v2"));
            }
            if let Some(v) = vocab {
                p.refs.push(("vocab", v));
            }
            advance(new_epoch, reason, &mut p)?;
        }
        [
            Value::Unsigned(1),
            Value::Unsigned(11),
            checker,
            format,
            auth,
        ]
        | [
            Value::Unsigned(1),
            Value::Unsigned(12),
            checker,
            format,
            auth,
        ] => {
            p.refs.push(("checker", digest(checker)?));
            text(format)?;
            authority(auth)?;
        }
        _ => return Err(fail("unknown event body/version/arity")),
    }
    let genesis = matches!(b.get(1), Some(Value::Unsigned(0)));
    if !genesis && (p.seq == 0 || p.prev.is_none()) {
        return Err(fail("non-Genesis header"));
    }
    Ok(p)
}

fn advance(epoch: &Value, reason: &Value, p: &mut Parsed) -> Result<()> {
    if number(epoch)? != p.epoch {
        return Err(fail("epoch body/envelope mismatch"));
    }
    text(reason)?;
    p.advance = true;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use logos_crypto::SigAlg;

    fn unhex(s: &str) -> Vec<u8> {
        s.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|v| u8::from_str_radix(std::str::from_utf8(v).unwrap(), 16).unwrap())
            .collect()
    }

    #[test]
    fn independently_decodes_all_event_vectors_byte_for_byte() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../../spec/vectors/events/vectors.json"))
                .unwrap();
        let vectors = fixture["vectors"].as_array().unwrap();
        assert_eq!(vectors.len(), 24);
        for v in vectors {
            let key = PublicKey::new(
                SigAlg::Ed25519,
                &unhex(v["kernel_public_key_hex"].as_str().unwrap()),
            )
            .unwrap();
            let wire = unhex(v["event_wire_hex"].as_str().unwrap());
            let parsed = parse(&wire, &key).unwrap();
            assert_eq!(
                parsed.digest.as_slice(),
                unhex(v["event_hash_hex"].as_str().unwrap())
            );
            assert_eq!(
                to_canon(&from_canon::<Value>(&wire).unwrap()).unwrap(),
                wire
            );
            let mut extra = wire.clone();
            extra.push(0);
            assert!(parse(&extra, &key).is_err());
        }
    }
}
