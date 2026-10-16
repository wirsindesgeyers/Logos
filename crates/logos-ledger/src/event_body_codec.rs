//! Codificação e decodificação estritas de corpos de eventos e tipos associados conforme `canon/v1`.
//!
//! Todas as representações utilizam arrays de comprimento fixo e discriminadores
//! numéricos; opcionais estritos usam []/[bytes32], nunca null.

use logos_air::{Formula, Vocabulary};
use logos_canon::{Array, Canon, CanonDecode, CanonError, Decoder, Text, Value};
use logos_core::{Budget, CheckOutcome};
use logos_crypto::{Domain, Hash, Signature};
use logos_obligation::Obligation;

use crate::Event;
use crate::event_body::{
    Authority, Certificate, CheckRecord, Checker, Consistency, EventBody, EventProposal, MAX_REFS,
    MAX_TEXT_BYTES, ModelObject, Route, TaintedText,
};

#[inline]
fn array(items: Vec<Value>) -> Result<Value, CanonError> {
    Array::new(items).map(Value::Array)
}

#[inline]
fn items(value: &Value) -> Result<&[Value], CanonError> {
    match value {
        Value::Array(array) => Ok(array.items()),
        _ => Err(CanonError::WrongType),
    }
}

#[inline]
fn bytes32(value: &Value) -> Result<[u8; 32], CanonError> {
    match value {
        Value::Bytes(bytes) => {
            <[u8; 32]>::try_from(bytes.as_slice()).map_err(|_| CanonError::WrongType)
        }
        _ => Err(CanonError::WrongType),
    }
}

fn digest<D: Domain>(value: &Value) -> Result<Hash<D>, CanonError> {
    bytes32(value).map(Hash::from_bytes)
}

fn check_size(value: &Value) -> Result<(), CanonError> {
    if crate::event::encoded_size(value)? > crate::MAX_EVENT_BYTES {
        return Err(CanonError::WrongType);
    }
    Ok(())
}

fn bounded(value: Value) -> Result<Value, CanonError> {
    check_size(&value)?;
    Ok(value)
}

impl Canon for Route {
    fn to_value(&self) -> Result<Value, CanonError> {
        Ok(Value::Unsigned(*self as u64))
    }
}

impl CanonDecode for Route {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        match u64::from_value(value)? {
            0 => Ok(Self::Model),
            1 => Ok(Self::Lrat),
            2 => Ok(Self::Alethe),
            3 => Ok(Self::Lean),
            _ => Err(CanonError::WrongType),
        }
    }
}

impl Canon for Authority {
    fn to_value(&self) -> Result<Value, CanonError> {
        array(vec![self.key.to_value()?, self.sig.to_value()?])
    }
}

impl CanonDecode for Authority {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        match items(value)? {
            [key_val, sig_val] => {
                let key = digest::<logos_crypto::PublicKey>(key_val)?;
                let sig = Signature::from_value(sig_val)?;
                Ok(Self { key, sig })
            }
            _ => Err(CanonError::WrongType),
        }
    }
}

impl Canon for TaintedText {
    fn to_value(&self) -> Result<Value, CanonError> {
        if self.0.len() > MAX_TEXT_BYTES {
            return Err(CanonError::WrongType);
        }
        Ok(Value::Bytes(self.0.as_bytes().to_vec()))
    }
}

impl CanonDecode for TaintedText {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        match value {
            Value::Bytes(bytes) => {
                if bytes.len() > MAX_TEXT_BYTES {
                    return Err(CanonError::WrongType);
                }
                let s = std::str::from_utf8(bytes).map_err(|_| CanonError::WrongType)?;
                Ok(Self(s.to_owned()))
            }
            _ => Err(CanonError::WrongType),
        }
    }
}

impl Canon for EventProposal {
    fn to_value(&self) -> Result<Value, CanonError> {
        self.validate().map_err(|_| CanonError::WrongType)?;
        let cites_vals = self
            .cites
            .iter()
            .map(Canon::to_value)
            .collect::<Result<Vec<_>, _>>()?;
        let spec_vals = self
            .speculative_on
            .iter()
            .map(|b| Ok(Value::Bytes(b.to_vec())))
            .collect::<Result<Vec<_>, CanonError>>()?;

        bounded(array(vec![
            self.text_pt.to_value()?,
            self.goal.to_value()?,
            array(cites_vals)?,
            array(spec_vals)?,
            self.vocabulary.to_value()?,
        ])?)
    }
}

impl CanonDecode for EventProposal {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        check_size(value)?;
        match items(value)? {
            [text_pt_val, goal_val, cites_val, spec_val, vocab_val] => {
                let text_pt = TaintedText::from_value(text_pt_val)?;
                let goal = Formula::from_value(goal_val)?;
                let cites_items = items(cites_val)?;
                if cites_items.len() > MAX_REFS {
                    return Err(CanonError::WrongType);
                }
                let cites = cites_items
                    .iter()
                    .map(digest::<Event>)
                    .collect::<Result<Vec<_>, _>>()?;
                let spec_items = items(spec_val)?;
                if spec_items.len() > MAX_REFS {
                    return Err(CanonError::WrongType);
                }
                let speculative_on = spec_items
                    .iter()
                    .map(bytes32)
                    .collect::<Result<Vec<_>, _>>()?;
                let vocabulary = digest::<Vocabulary>(vocab_val)?;
                Ok(Self {
                    text_pt,
                    goal,
                    cites,
                    speculative_on,
                    vocabulary,
                })
            }
            _ => Err(CanonError::WrongType),
        }
    }
}

impl Canon for CheckRecord {
    fn to_value(&self) -> Result<Value, CanonError> {
        self.validate().map_err(|_| CanonError::WrongType)?;
        array(vec![
            self.obligation.to_value()?,
            self.certificate.to_value()?,
            self.checker.to_value()?,
            self.budget.to_value()?,
            self.outcome.to_value()?,
        ])
    }
}

impl CanonDecode for CheckRecord {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        match items(value)? {
            [ob_val, cert_val, chk_val, bud_val, out_val] => {
                let obligation = digest::<Obligation>(ob_val)?;
                let certificate = digest::<Certificate>(cert_val)?;
                let checker = digest::<Checker>(chk_val)?;
                let budget = Budget::from_value(bud_val)?;
                let outcome = CheckOutcome::from_value(out_val)?;
                let record = Self {
                    obligation,
                    certificate,
                    checker,
                    budget,
                    outcome,
                };
                record.validate().map_err(|_| CanonError::WrongType)?;
                Ok(record)
            }
            _ => Err(CanonError::WrongType),
        }
    }
}

impl Canon for Consistency {
    fn to_value(&self) -> Result<Value, CanonError> {
        self.validate().map_err(|_| CanonError::WrongType)?;
        match self {
            Self::Unknown { check } => {
                let opt_arr = match check {
                    None => array(Vec::new())?,
                    Some(c) => array(vec![c.to_value()?])?,
                };
                array(vec![Value::Unsigned(0), opt_arr])
            }
            Self::Witness { model, check } => array(vec![
                Value::Unsigned(1),
                model.to_value()?,
                check.to_value()?,
            ]),
        }
    }
}

impl CanonDecode for Consistency {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        let consistency = match items(value)? {
            [Value::Unsigned(0), opt_val] => {
                let check = match items(opt_val)? {
                    [] => None,
                    [c_val] => Some(CheckRecord::from_value(c_val)?),
                    _ => return Err(CanonError::WrongType),
                };
                Self::Unknown { check }
            }
            [Value::Unsigned(1), model_val, check_val] => {
                let model = digest::<ModelObject>(model_val)?;
                let check = CheckRecord::from_value(check_val)?;
                Self::Witness { model, check }
            }
            _ => return Err(CanonError::WrongType),
        };
        consistency.validate().map_err(|_| CanonError::WrongType)?;
        Ok(consistency)
    }
}

impl Canon for EventBody {
    fn to_value(&self) -> Result<Value, CanonError> {
        self.validate().map_err(|_| CanonError::WrongType)?;
        let value = match self {
            Self::Genesis {
                ledger_version,
                kernel_binary,
                kernel_key,
            } => array(vec![
                Value::Unsigned(1),
                Value::Unsigned(0),
                Value::Unsigned(*ledger_version),
                Value::Bytes(kernel_binary.to_vec()),
                kernel_key.to_value()?,
            ]),
            Self::VocabPublished {
                vocabulary,
                compatible,
                authority,
            } => array(vec![
                Value::Unsigned(1),
                Value::Unsigned(1),
                vocabulary.to_value()?,
                Value::Bool(*compatible),
                authority.to_value()?,
            ]),
            Self::Proposed { proposal, author } => array(vec![
                Value::Unsigned(1),
                Value::Unsigned(2),
                proposal.to_value()?,
                author.to_value()?,
            ]),
            Self::ObligationCreated {
                obligation,
                route,
                artifact,
                cnf,
            } => {
                if let Some(c) = cnf {
                    array(vec![
                        Value::Unsigned(2),
                        Value::Unsigned(3),
                        obligation.to_value()?,
                        route.to_value()?,
                        Value::Bytes(artifact.to_vec()),
                        array(vec![c.hash.to_value()?, c.encoder_version.to_value()?])?,
                    ])
                } else {
                    array(vec![
                        Value::Unsigned(1),
                        Value::Unsigned(3),
                        obligation.to_value()?,
                        route.to_value()?,
                        Value::Bytes(artifact.to_vec()),
                    ])
                }
            }
            Self::Verified {
                check,
                cited,
                used,
                consistency,
            } => {
                let cited_vals = cited
                    .iter()
                    .map(Canon::to_value)
                    .collect::<Result<Vec<_>, _>>()?;
                let used_vals = used
                    .iter()
                    .map(Canon::to_value)
                    .collect::<Result<Vec<_>, _>>()?;
                array(vec![
                    Value::Unsigned(1),
                    Value::Unsigned(4),
                    check.to_value()?,
                    array(cited_vals)?,
                    array(used_vals)?,
                    consistency.to_value()?,
                ])
            }
            Self::Refuted { check, model } => array(vec![
                Value::Unsigned(1),
                Value::Unsigned(5),
                check.to_value()?,
                model.to_value()?,
            ]),
            Self::UnknownRecorded { check, machine } => array(vec![
                Value::Unsigned(1),
                Value::Unsigned(6),
                check.to_value()?,
                machine.to_value()?,
            ]),
            Self::AssumptionDeclared {
                vocabulary,
                formula,
                scope,
                author,
            } => array(vec![
                Value::Unsigned(1),
                Value::Unsigned(7),
                vocabulary.to_value()?,
                formula.to_value()?,
                scope.to_value()?,
                author.to_value()?,
            ]),
            Self::Retracted {
                target,
                reason,
                authority,
            } => array(vec![
                Value::Unsigned(1),
                Value::Unsigned(8),
                target.to_value()?,
                reason.to_value()?,
                authority.to_value()?,
            ]),
            Self::Invalidated { claim, causes } => {
                let causes_vals = causes
                    .iter()
                    .map(Canon::to_value)
                    .collect::<Result<Vec<_>, _>>()?;
                array(vec![
                    Value::Unsigned(1),
                    Value::Unsigned(9),
                    claim.to_value()?,
                    array(causes_vals)?,
                ])
            }
            Self::EpochAdvanced {
                new_epoch,
                reason,
                policy_root,
                vocab,
            } => {
                if policy_root.is_none() && vocab.is_none() {
                    array(vec![
                        Value::Unsigned(1),
                        Value::Unsigned(10),
                        Value::Unsigned(*new_epoch),
                        reason.to_value()?,
                    ])
                } else {
                    array(vec![
                        Value::Unsigned(2),
                        Value::Unsigned(10),
                        Value::Unsigned(*new_epoch),
                        reason.to_value()?,
                        policy_root.to_value()?,
                        vocab.map(|h| *h.as_bytes()).to_value()?,
                    ])
                }
            }
            Self::CheckerRegistered {
                checker,
                format,
                authority,
            } => array(vec![
                Value::Unsigned(1),
                Value::Unsigned(11),
                checker.to_value()?,
                format.to_value()?,
                authority.to_value()?,
            ]),
            Self::CheckerRevoked {
                checker,
                reason,
                authority,
            } => array(vec![
                Value::Unsigned(1),
                Value::Unsigned(12),
                checker.to_value()?,
                reason.to_value()?,
                authority.to_value()?,
            ]),
        }?;
        bounded(value)
    }
}

impl CanonDecode for EventBody {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        check_size(value)?;
        let fields = items(value)?;
        if fields.len() < 2 {
            return Err(CanonError::WrongType);
        }
        if fields[0] == Value::Unsigned(2) {
            if let [_, Value::Unsigned(3), o, route, artifact, record] = fields {
                let [h, version] = items(record)? else {
                    return Err(CanonError::WrongType);
                };
                let body = Self::ObligationCreated {
                    obligation: digest::<Obligation>(o)?,
                    route: Route::from_value(route)?,
                    artifact: bytes32(artifact)?,
                    cnf: Some(crate::CnfRecord {
                        hash: digest::<crate::CnfObject>(h)?,
                        encoder_version: u64::from_value(version)?,
                    }),
                };
                body.validate().map_err(|_| CanonError::WrongType)?;
                return Ok(body);
            }
            let [_, Value::Unsigned(10), epoch, reason, root, vocab] = fields else {
                return Err(CanonError::WrongType);
            };
            let policy_root = Option::<[u8; 32]>::from_value(root)?;
            let vocab = Option::<[u8; 32]>::from_value(vocab)?.map(Hash::from_bytes);
            // O corpo v1 é a única codificação quando ambos estão ausentes.
            if policy_root.is_none() && vocab.is_none() {
                return Err(CanonError::WrongType);
            }
            let body = Self::EpochAdvanced {
                new_epoch: u64::from_value(epoch)?,
                reason: Text::from_value(reason)?,
                policy_root,
                vocab,
            };
            body.validate().map_err(|_| CanonError::WrongType)?;
            return Ok(body);
        }
        if fields[0] != Value::Unsigned(1) {
            return Err(CanonError::WrongType);
        }
        let tag = match fields[1] {
            Value::Unsigned(t) => t,
            _ => return Err(CanonError::WrongType),
        };
        let body = match (tag, &fields[2..]) {
            (0, [version_val, bin_val, key_val]) => {
                let ledger_version = u64::from_value(version_val)?;
                let kernel_binary = bytes32(bin_val)?;
                let kernel_key = digest::<logos_crypto::PublicKey>(key_val)?;
                Self::Genesis {
                    ledger_version,
                    kernel_binary,
                    kernel_key,
                }
            }
            (1, [vocab_val, comp_val, auth_val]) => {
                let vocabulary = digest::<Vocabulary>(vocab_val)?;
                let compatible = match comp_val {
                    Value::Bool(b) => *b,
                    _ => return Err(CanonError::WrongType),
                };
                let authority = Authority::from_value(auth_val)?;
                Self::VocabPublished {
                    vocabulary,
                    compatible,
                    authority,
                }
            }
            (2, [prop_val, author_val]) => {
                let proposal = EventProposal::from_value(prop_val)?;
                let author = digest::<logos_crypto::PublicKey>(author_val)?;
                Self::Proposed { proposal, author }
            }
            (3, [ob_val, route_val, art_val]) => {
                let obligation = digest::<Obligation>(ob_val)?;
                let route = Route::from_value(route_val)?;
                let artifact = bytes32(art_val)?;
                Self::ObligationCreated {
                    obligation,
                    route,
                    artifact,
                    cnf: None,
                }
            }
            (4, [chk_val, cited_val, used_val, cons_val]) => {
                let check = CheckRecord::from_value(chk_val)?;
                let cited_items = items(cited_val)?;
                if cited_items.len() > MAX_REFS {
                    return Err(CanonError::WrongType);
                }
                let cited = cited_items
                    .iter()
                    .map(digest::<Event>)
                    .collect::<Result<Vec<_>, _>>()?;
                let used_items = items(used_val)?;
                if used_items.len() > MAX_REFS {
                    return Err(CanonError::WrongType);
                }
                let used = used_items
                    .iter()
                    .map(digest::<Event>)
                    .collect::<Result<Vec<_>, _>>()?;
                let consistency = Consistency::from_value(cons_val)?;
                Self::Verified {
                    check,
                    cited,
                    used,
                    consistency,
                }
            }
            (5, [chk_val, model_val]) => {
                let check = CheckRecord::from_value(chk_val)?;
                let model = digest::<ModelObject>(model_val)?;
                Self::Refuted { check, model }
            }
            (6, [chk_val, machine_val]) => {
                let check = CheckRecord::from_value(chk_val)?;
                let machine = Text::from_value(machine_val)?;
                if machine.as_str().len() > MAX_TEXT_BYTES {
                    return Err(CanonError::WrongType);
                }
                Self::UnknownRecorded { check, machine }
            }
            (7, [vocab_val, form_val, scope_val, author_val]) => {
                let vocabulary = digest::<Vocabulary>(vocab_val)?;
                let formula = Formula::from_value(form_val)?;
                let scope = Text::from_value(scope_val)?;
                if scope.as_str().len() > MAX_TEXT_BYTES {
                    return Err(CanonError::WrongType);
                }
                let author = digest::<logos_crypto::PublicKey>(author_val)?;
                Self::AssumptionDeclared {
                    vocabulary,
                    formula,
                    scope,
                    author,
                }
            }
            (8, [target_val, reason_val, auth_val]) => {
                let target = digest::<Event>(target_val)?;
                let reason = Text::from_value(reason_val)?;
                if reason.as_str().len() > MAX_TEXT_BYTES {
                    return Err(CanonError::WrongType);
                }
                let authority = Authority::from_value(auth_val)?;
                Self::Retracted {
                    target,
                    reason,
                    authority,
                }
            }
            (9, [claim_val, causes_val]) => {
                let claim = digest::<Event>(claim_val)?;
                let causes_items = items(causes_val)?;
                if causes_items.len() > MAX_REFS {
                    return Err(CanonError::WrongType);
                }
                let causes = causes_items
                    .iter()
                    .map(digest::<Event>)
                    .collect::<Result<Vec<_>, _>>()?;
                Self::Invalidated { claim, causes }
            }
            (10, [epoch_val, reason_val]) => {
                let new_epoch = u64::from_value(epoch_val)?;
                let reason = Text::from_value(reason_val)?;
                if reason.as_str().len() > MAX_TEXT_BYTES {
                    return Err(CanonError::WrongType);
                }
                Self::EpochAdvanced {
                    new_epoch,
                    reason,
                    policy_root: None,
                    vocab: None,
                }
            }
            (11, [checker_val, format_val, auth_val]) => {
                let checker = digest::<Checker>(checker_val)?;
                let format = Text::from_value(format_val)?;
                if format.as_str().len() > MAX_TEXT_BYTES {
                    return Err(CanonError::WrongType);
                }
                let authority = Authority::from_value(auth_val)?;
                Self::CheckerRegistered {
                    checker,
                    format,
                    authority,
                }
            }
            (12, [checker_val, reason_val, auth_val]) => {
                let checker = digest::<Checker>(checker_val)?;
                let reason = Text::from_value(reason_val)?;
                if reason.as_str().len() > MAX_TEXT_BYTES {
                    return Err(CanonError::WrongType);
                }
                let authority = Authority::from_value(auth_val)?;
                Self::CheckerRevoked {
                    checker,
                    reason,
                    authority,
                }
            }
            _ => return Err(CanonError::WrongType),
        };
        body.validate().map_err(|_| CanonError::WrongType)?;
        Ok(body)
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, CanonError> {
        if decoder.remaining() > crate::MAX_EVENT_BYTES {
            return Err(CanonError::WrongType);
        }
        Self::from_value(&decoder.value()?)
    }
}
