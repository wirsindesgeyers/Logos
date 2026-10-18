//! Writer/CAS reais; fixtures de CheckRecord não simulam autorização CERTIFY5.4.

use super::*;
use logos_air::{Formula, FormulaNode, Limits, Vocabulary};
use logos_canon::to_canon;
use logos_core::{Budget, Exhausted, Reason};
use logos_crypto::{CryptoError, FileSigner, PublicKey, SigDomain, Signature};
use logos_ledger::{Certificate, Checker, LedgerHead, TaintedText};
use logos_obligation::{LedgerEntry, LedgerView, PremiseRef, Proposal, generate};
use std::cell::Cell;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        loop {
            let path = std::env::temp_dir().join(format!(
                "logos-kernel-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match std::fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("create fixture: {e}"),
            }
        }
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

struct Empty;
impl LedgerView for Empty {
    fn current_epoch(&self) -> u64 {
        0
    }
    fn lookup(
        &self,
        _: PremiseRef,
    ) -> Result<Option<LedgerEntry<'_>>, logos_obligation::LedgerError> {
        Ok(None)
    }
}

struct Fixture {
    proposal: EventProposal,
    check: CheckRecord,
    model: Hash<ModelObject>,
    author: KeyId,
}
impl Fixture {
    fn new<S: Signer>(ledger: &mut Ledger<'_, S>, signer: &S) -> Self {
        ledger
            .append(EventBody::Genesis {
                ledger_version: 1,
                kernel_binary: [0x51; 32],
                kernel_key: signer.key_id().unwrap(),
            })
            .unwrap();
        let v = Vocabulary::new(1, vec![], vec![]).unwrap();
        let goal = Formula::new(vec![FormulaNode::True], Limits::default()).unwrap();
        let obligation = generate(
            Proposal {
                vocabulary: &v,
                goal: &goal,
                cites: &[],
            },
            &Empty,
        )
        .unwrap();
        let cas = ledger.cas();
        let vocabulary = cas.put(&to_canon(&v).unwrap()).unwrap();
        let ho = cas.put(&to_canon(&obligation).unwrap()).unwrap();
        Self {
            proposal: EventProposal {
                text_pt: TaintedText::new("fixture de typestate".into()).unwrap(),
                goal,
                cites: vec![],
                speculative_on: vec![],
                vocabulary,
            },
            check: CheckRecord {
                obligation: ho,
                certificate: cas.put::<Certificate>(b"candidate fixture").unwrap(),
                checker: cas
                    .put::<Checker>(b"fixture; not active authority")
                    .unwrap(),
                budget: Budget::new(100, 10000, 10000, 10),
                outcome: CheckOutcome::Accepted,
            },
            model: cas.put::<ModelObject>(b"model fixture").unwrap(),
            author: signer.key_id().unwrap(),
        }
    }

    fn draft<S: Signer>(&self, t: &mut Transitions<'_, '_, S>) -> Claim<Draft> {
        t.propose(self.proposal.clone(), self.author).unwrap()
    }

    fn pending<S: Signer>(&self, t: &mut Transitions<'_, '_, S>) -> Claim<Pending> {
        let draft = self.draft(t);
        t.submit(draft, self.check.obligation, Route::Model, [0x42; 32], None)
            .unwrap()
    }

    fn verified<S: Signer>(&self, t: &mut Transitions<'_, '_, S>) -> Claim<Verified> {
        let pending = self.pending(t);
        t.accept(
            pending,
            self.check.clone(),
            vec![],
            vec![],
            Consistency::Unknown { check: None },
        )
        .unwrap()
    }
}

fn signer() -> FileSigner {
    FileSigner::from_seed(&[0x51; 32]).unwrap()
}

fn head<S: Signer>(t: &Transitions<'_, '_, S>) -> LedgerHead {
    t.ledger.head().unwrap().unwrap()
}

fn assert_step<T, S: Signer>(
    before: LedgerHead,
    t: &Transitions<'_, '_, S>,
    claim: &Claim<T>,
    tag: u64,
    key: &PublicKey,
) {
    let after = head(t);
    assert_eq!(after.seq, before.seq + 1, "one append per transition");
    let event = claim.receipt().event();
    assert_eq!(event.seq, after.seq);
    assert_eq!(event.prev, Some(before.hash));
    assert_eq!(event.body.tag(), tag, "independent transition table");
    assert_eq!(claim.receipt().hash(), hash(event).unwrap());
    assert_eq!(after.hash, claim.receipt().hash());
    event.verify(key).unwrap();
}

// Oráculo independente: tabela espec §1, sem consultar estado/corpo do recibo.
enum Run {
    Empty,
    Draft(Claim<Draft>),
    Pending(Claim<Pending>),
    Verified(Claim<Verified>),
    Terminal,
}

#[test]
fn random_interleaved_sequences_emit_exactly_one_signed_event_per_transition() {
    let mut coverage = [0usize; 6];
    for seed in 1..=8u64 {
        let temp = Temp::new();
        let signer = signer();
        let mut ledger = Ledger::open(&temp.0, &signer).unwrap();
        let f = Fixture::new(&mut ledger, &signer);
        let mut t = Transitions::new(&mut ledger);
        let mut slots = [Run::Empty, Run::Empty, Run::Empty, Run::Empty];
        let mut rng = seed;
        for _ in 0..128 {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let slot = ((rng >> 24) % 4) as usize;
            let before = head(&t);
            slots[slot] = match std::mem::replace(&mut slots[slot], Run::Empty) {
                Run::Empty | Run::Terminal => {
                    let c = f.draft(&mut t);
                    assert_eq!(c.proposal(), c.receipt().hash());
                    assert_eq!(c.obligation(), None);
                    assert_step(before, &t, &c, 2, signer.public_key());
                    coverage[0] += 1;
                    Run::Draft(c)
                }
                Run::Draft(c) => {
                    let proposal = c.proposal();
                    let c = t
                        .submit(c, f.check.obligation, Route::Model, [0; 32], None)
                        .unwrap();
                    assert_eq!(c.proposal(), proposal);
                    assert_eq!(c.obligation(), Some(f.check.obligation));
                    assert_step(before, &t, &c, 3, signer.public_key());
                    coverage[1] += 1;
                    Run::Pending(c)
                }
                Run::Pending(c) => match (rng >> 32) % 3 {
                    0 => {
                        let c = t
                            .accept(
                                c,
                                f.check.clone(),
                                vec![],
                                vec![],
                                Consistency::Unknown { check: None },
                            )
                            .unwrap();
                        assert_step(before, &t, &c, 4, signer.public_key());
                        coverage[2] += 1;
                        Run::Verified(c)
                    }
                    1 => {
                        let c = t.refute(c, f.check.clone(), f.model).unwrap();
                        assert_step(before, &t, &c, 5, signer.public_key());
                        coverage[3] += 1;
                        Run::Terminal
                    }
                    _ => {
                        let mut check = f.check.clone();
                        check.outcome = CheckOutcome::Unknown(Exhausted::Steps);
                        let c = t.unknown(c, check, Text::new("fixture").unwrap()).unwrap();
                        assert_step(before, &t, &c, 6, signer.public_key());
                        coverage[4] += 1;
                        Run::Terminal
                    }
                },
                Run::Verified(c) => {
                    let verified = c.receipt().hash();
                    let c = t.invalidate(c, vec![verified]).unwrap();
                    assert_step(before, &t, &c, 9, signer.public_key());
                    let EventBody::Invalidated { claim, .. } = c.receipt().event().body else {
                        panic!("wrong event")
                    };
                    assert_eq!(claim, verified);
                    coverage[5] += 1;
                    Run::Terminal
                }
            };
        }
        let expected = head(&t);
        drop(t);
        drop(ledger);
        let reopened = Ledger::open(&temp.0, &signer).unwrap();
        assert_eq!(reopened.head().unwrap(), Some(expected), "durable replay");
    }
    assert!(coverage.iter().all(|&count| count >= 30), "{coverage:?}");
}

#[test]
fn rejected_and_unknown_checks_cannot_emit_verified_or_refuted() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0, &signer).unwrap();
    let f = Fixture::new(&mut ledger, &signer);
    let mut t = Transitions::new(&mut ledger);
    let outcomes = [
        CheckOutcome::Rejected(Reason::new(Text::new("bad certificate").unwrap())),
        CheckOutcome::Unknown(Exhausted::Time),
        CheckOutcome::Unknown(Exhausted::Steps),
        CheckOutcome::Unknown(Exhausted::Memory),
        CheckOutcome::Unknown(Exhausted::Size),
    ];
    for outcome in outcomes {
        let mut check = f.check.clone();
        check.outcome = outcome;
        let c = f.pending(&mut t);
        let before = head(&t);
        assert!(
            t.accept(
                c,
                check.clone(),
                vec![],
                vec![],
                Consistency::Unknown { check: None }
            )
            .is_err()
        );
        assert_eq!(head(&t), before);
        let c = f.pending(&mut t);
        let before = head(&t);
        assert!(t.refute(c, check, f.model).is_err());
        assert_eq!(head(&t), before);
    }
    for outcome in [
        CheckOutcome::Accepted,
        CheckOutcome::Rejected(Reason::new(Text::new("rejected").unwrap())),
    ] {
        let c = f.pending(&mut t);
        let before = head(&t);
        let mut check = f.check.clone();
        check.outcome = outcome;
        assert!(t.unknown(c, check, Text::new("fixture").unwrap()).is_err());
        assert_eq!(head(&t), before);
    }
}

#[test]
fn obligation_mismatch_is_rejected_for_all_check_transitions() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0, &signer).unwrap();
    let f = Fixture::new(&mut ledger, &signer);
    let mut t = Transitions::new(&mut ledger);
    let mut check = f.check.clone();
    // Presente no CAS: o erro não pode ser detectado apenas pelo appender.
    check.obligation = t.ledger.cas().put(b"different obligation").unwrap();
    for action in 0..3 {
        let c = f.pending(&mut t);
        let before = head(&t);
        let err = match action {
            0 => t
                .accept(
                    c,
                    check.clone(),
                    vec![],
                    vec![],
                    Consistency::Unknown { check: None },
                )
                .unwrap_err(),
            1 => t.refute(c, check.clone(), f.model).unwrap_err(),
            _ => {
                let mut check = check.clone();
                check.outcome = CheckOutcome::Unknown(Exhausted::Time);
                t.unknown(c, check, Text::new("fixture").unwrap())
                    .unwrap_err()
            }
        };
        assert!(matches!(
            err,
            LedgerError::Invalid("check obligation differs from claim")
        ));
        assert_eq!(head(&t), before);
    }
}

#[test]
fn receipt_cannot_be_reused_in_another_session_even_with_identical_ledger_head() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0, &signer).unwrap();
    let f = Fixture::new(&mut ledger, &signer);
    let mut t = Transitions::new(&mut ledger);
    let draft = f.draft(&mut t);
    let pending = f.pending(&mut t);
    let verified = f.verified(&mut t);
    let before = head(&t);
    drop(t);
    let mut other = Transitions::new(&mut ledger);
    assert!(
        other
            .submit(draft, f.check.obligation, Route::Model, [0; 32], None)
            .is_err()
    );
    assert!(other.refute(pending, f.check.clone(), f.model).is_err());
    assert!(other.invalidate(verified, vec![before.hash]).is_err());
    assert_eq!(head(&other), before);
}

#[test]
fn epoch_changes_block_logical_transitions_but_allow_invalidation() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0, &signer).unwrap();
    let f = Fixture::new(&mut ledger, &signer);
    let mut t = Transitions::new(&mut ledger);
    let draft = f.draft(&mut t);
    let pending = f.pending(&mut t);
    let verified = f.verified(&mut t);
    let verified_hash = verified.receipt().hash();
    t.ledger
        .append(EventBody::EpochAdvanced {
            new_epoch: 1,
            reason: Text::new("new epoch").unwrap(),
            policy_root: None,
            vocab: None,
        })
        .unwrap();
    let before = head(&t);
    assert!(
        t.submit(draft, f.check.obligation, Route::Model, [0; 32], None)
            .is_err()
    );
    assert!(
        t.accept(
            pending,
            f.check.clone(),
            vec![],
            vec![],
            Consistency::Unknown { check: None }
        )
        .is_err()
    );
    assert_eq!(head(&t), before);
    let invalidated = t.invalidate(verified, vec![before.hash]).unwrap();
    assert_step(before, &t, &invalidated, 9, signer.public_key());
    assert_eq!(invalidated.epoch(), 0);
    assert_eq!(invalidated.receipt().event().epoch, 1);
    assert!(
        matches!(invalidated.receipt().event().body, EventBody::Invalidated { claim, .. } if claim == verified_hash)
    );
}

#[test]
fn verified_preserves_proposal_citations_and_catalogue_validation() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0, &signer).unwrap();
    let mut f = Fixture::new(&mut ledger, &signer);
    let a = Hash::<Event>::from_bytes([1; 32]);
    let b = Hash::<Event>::from_bytes([2; 32]);
    f.proposal.cites = vec![b, a, b];
    let mut t = Transitions::new(&mut ledger);
    for cited in [vec![], vec![a], vec![b, a], vec![a, a, b]] {
        let c = f.pending(&mut t);
        let before = head(&t);
        assert!(
            t.accept(
                c,
                f.check.clone(),
                cited,
                vec![],
                Consistency::Unknown { check: None }
            )
            .is_err()
        );
        assert_eq!(head(&t), before);
    }
    let c = f.pending(&mut t);
    let before = head(&t);
    assert!(
        t.accept(
            c,
            f.check.clone(),
            vec![a, b],
            vec![Hash::from_bytes([3; 32])],
            Consistency::Unknown { check: None }
        )
        .is_err()
    );
    assert_eq!(head(&t), before);
    let c = f.pending(&mut t);
    let c = t
        .accept(
            c,
            f.check.clone(),
            vec![a, b],
            vec![a],
            Consistency::Unknown { check: None },
        )
        .unwrap();
    let before = head(&t);
    assert!(
        matches!(c.receipt().event().body, EventBody::Verified { ref cited, ref used, .. } if cited == &[a,b] && used == &[a])
    );
    assert!(t.invalidate(c, vec![]).is_err());
    assert_eq!(head(&t), before);
}

struct FaultSigner {
    inner: FileSigner,
    fail: Cell<bool>,
    calls: Cell<usize>,
}
impl Signer for FaultSigner {
    fn public_key(&self) -> &PublicKey {
        self.inner.public_key()
    }
    fn sign<D: SigDomain>(&self, bytes: &[u8]) -> Result<Signature, CryptoError> {
        self.calls.set(self.calls.get() + 1);
        if self.fail.get() {
            return Err(CryptoError::Io(std::io::ErrorKind::Other));
        }
        self.inner.sign::<D>(bytes)
    }
}

#[test]
fn append_failure_never_returns_a_new_claim_or_retries() {
    let temp = Temp::new();
    let signer = FaultSigner {
        inner: signer(),
        fail: Cell::new(false),
        calls: Cell::new(0),
    };
    let mut ledger = Ledger::open(&temp.0, &signer).unwrap();
    let f = Fixture::new(&mut ledger, &signer);
    let mut t = Transitions::new(&mut ledger);
    let c = f.pending(&mut t);
    let before = head(&t);
    let calls = signer.calls.get();
    signer.fail.set(true);
    assert!(
        t.accept(
            c,
            f.check.clone(),
            vec![],
            vec![],
            Consistency::Unknown { check: None }
        )
        .is_err()
    );
    assert_eq!(signer.calls.get(), calls + 1, "no automatic retry");
    assert_eq!(head(&t), before);
    assert!(t.propose(f.proposal.clone(), f.author).is_err());
    assert_eq!(head(&t), before);
    signer.fail.set(false);
    let c = f.draft(&mut t);
    let before = head(&t);
    let calls = signer.calls.get();
    assert!(
        t.submit(c, Hash::from_bytes([0xff; 32]), Route::Model, [0; 32], None)
            .is_err()
    );
    assert_eq!(signer.calls.get(), calls, "CAS failure before signing");
    assert_eq!(head(&t), before);
    drop(t);
    drop(ledger);
    assert_eq!(
        Ledger::open(&temp.0, &signer).unwrap().head().unwrap(),
        Some(before)
    );
}

#[test]
fn every_normative_transition_is_exercised() {
    let spec = include_str!("../../../../spec/state-machine.md");
    let ids: Vec<_> = spec
        .lines()
        .filter_map(|line| {
            let cells: Vec<_> = line.split('|').map(str::trim).collect();
            (cells.len() == 8 && cells[1] != "ID" && cells[1] != "---").then(|| cells[1])
        })
        .collect();
    assert_eq!(
        ids,
        [
            "propose",
            "submit",
            "accept",
            "refute",
            "unknown",
            "invalidate"
        ]
    );
}
