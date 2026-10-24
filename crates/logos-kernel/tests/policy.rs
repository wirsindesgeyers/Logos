//! Etapa 7.2 (ADR0070): publicação de política pelo admin, raiz Merkle recomputada pelo kernel,
//! `EpochAdvanced` associado, provas contra a raiz vigente e rejeição de raiz forjada pelo auditor.
#![cfg(target_os = "linux")]
mod support;

use logos_air::Vocabulary;
use logos_canon::{Text, to_canon};
use logos_crypto::{FileSigner, Hash, Signer, hash};
use logos_kernel::{TransportError, policy::publish};
use logos_ledger::{Authority, Event, EventBody, Ledger, LedgerIndex, PolicyObject};
use logos_policy::{Class, Policy, PolicyTree, verify_no_rule};
use support::*;

const VOCABULARY: &str = r#"
vocab v1 {
  sort Action
  pred Allowed(Action)
  pred p(Action)
  pred q(Action)
  func drop(Action): Action
  func send(Action): Action
  func read(Action): Action
}
"#;

type L<'a> = Ledger<'a, FileSigner>;

fn vocabulary() -> Vocabulary {
    logos_air_text::parse_document(VOCABULARY, &Default::default())
        .unwrap()
        .vocabulary
}

fn policy_text(version: u64, comment: &str) -> String {
    format!(
        r#"policy v{version} {{
          rule a1 allow: forall x:Action. p(x) -> Allowed(read(x)) comment "{comment}"
          rule d1 deny: forall x:Action. q(x) -> not Allowed(drop(x))
          rule d2 deny: forall x:Action. p(x) -> not Allowed(send(x))
        }}"#
    )
}

fn admin() -> FileSigner {
    FileSigner::from_seed(&[0xad; 32]).unwrap()
}

fn world(temp: &Temp) -> (FileSigner, std::path::PathBuf) {
    let kernel = signer();
    let path = temp.0.join("ledger");
    let mut ledger = Ledger::open(&path, &kernel).unwrap();
    genesis(&mut ledger, &kernel);
    ledger
        .cas()
        .put::<Vocabulary>(&to_canon(&vocabulary()).unwrap())
        .unwrap();
    (kernel, path)
}

fn put_policy(ledger: &L<'_>, text: &str) -> (Policy, Hash<PolicyObject>) {
    let policy = logos_policy_text::parse_policy(text, &vocabulary()).unwrap();
    let h = ledger
        .cas()
        .put::<PolicyObject>(&to_canon(&policy).unwrap())
        .unwrap();
    (policy, h)
}

fn reason() -> Text {
    Text::new("política publicada").unwrap()
}

#[test]
fn publish_records_event_epoch_and_root_and_proofs_follow_the_current_root() {
    let temp = Temp::new();
    let (kernel, path) = world(&temp);
    let admin = admin();
    let mut ledger = Ledger::open(&path, &kernel).unwrap();
    let (policy, h) = put_policy(&ledger, &policy_text(1, "ler quando p"));
    let published = publish(&mut ledger, admin.public_key(), &admin, h, reason()).unwrap();
    let tree = PolicyTree::new(&policy).unwrap();
    assert_eq!(&published.root, tree.root());
    assert_eq!(published.version, 1);
    // PolicyPublished seguido do EpochAdvanced que carrega a raiz.
    let head = ledger.head().unwrap().unwrap();
    let events = ledger.events_from(head.seq - 1, 2).unwrap();
    match (&events[0].body, &events[1].body) {
        (
            EventBody::PolicyPublished {
                policy: p,
                version,
                root,
                authority,
            },
            EventBody::EpochAdvanced {
                new_epoch,
                policy_root,
                vocab,
                ..
            },
        ) => {
            assert_eq!((p, *version, root), (&h, 1, tree.root()));
            assert_eq!(authority.key, admin.key_id().unwrap());
            assert_eq!(*new_epoch, events[0].epoch + 1);
            assert_eq!(policy_root, &Some(*tree.root()));
            assert_eq!(*vocab, Some(policy.vocabulary()));
        }
        other => panic!("unexpected events {other:?}"),
    }
    assert_eq!(hash(&events[0]).unwrap(), published.published);
    assert_eq!(hash(&events[1]).unwrap(), published.epoch);
    // O índice entrega a política vigente; as provas valem contra a raiz publicada.
    let current = ledger
        .live_index()
        .unwrap()
        .current_policy()
        .unwrap()
        .unwrap();
    assert_eq!((current.version, current.root), (1, *tree.root()));
    assert_eq!(current.policy, h);
    assert_eq!(current.event, published.published);
    let none_for_read = tree.prove_no_rule(Class::Deny, "read").unwrap();
    assert!(verify_no_rule(
        &current.root,
        Class::Deny,
        "read",
        &none_for_read
    ));
    assert!(tree.prove_no_rule(Class::Deny, "drop").is_none());
    // Uma segunda versão com um comentário diferente: raiz nova, a prova antiga deixa de valer.
    let (policy2, h2) = put_policy(&ledger, &policy_text(2, "ler quando p (revisado)"));
    let second = publish(&mut ledger, admin.public_key(), &admin, h2, reason()).unwrap();
    assert_ne!(second.root, published.root);
    assert_eq!(PolicyTree::new(&policy2).unwrap().root(), &second.root);
    let current = ledger
        .live_index()
        .unwrap()
        .current_policy()
        .unwrap()
        .unwrap();
    assert_eq!((current.version, current.root), (2, second.root));
    assert!(!verify_no_rule(
        &current.root,
        Class::Deny,
        "read",
        &none_for_read
    ));
    // Auditor independente (rederiva a raiz do CAS) e índice reconstruído concordam.
    drop(ledger);
    logos_audit::verify(&path, &path.join("cas"), kernel.public_key()).unwrap();
    let rebuilt = LedgerIndex::open(&path, kernel.public_key()).unwrap();
    assert_eq!(rebuilt.current_policy().unwrap().unwrap().root, second.root);
}

#[test]
fn only_the_admin_publishes_valid_increasing_policies_and_refusals_write_nothing() {
    let temp = Temp::new();
    let (kernel, path) = world(&temp);
    let admin = admin();
    let mut ledger = Ledger::open(&path, &kernel).unwrap();
    let (_, h) = put_policy(&ledger, &policy_text(3, "v3"));
    let head = ledger.head().unwrap();
    let stranger = FileSigner::from_seed(&[0x13; 32]).unwrap();
    assert!(matches!(
        publish(&mut ledger, admin.public_key(), &stranger, h, reason()),
        Err(TransportError::Unauthorized)
    ));
    // Objeto ausente, bytes que não são política, vocabulário ausente.
    let missing = Hash::<PolicyObject>::from_bytes([0x44; 32]);
    assert!(publish(&mut ledger, admin.public_key(), &admin, missing, reason()).is_err());
    let garbage = ledger.cas().put::<PolicyObject>(b"not a policy").unwrap();
    assert!(publish(&mut ledger, admin.public_key(), &admin, garbage, reason()).is_err());
    let lonely = Vocabulary::new(2, vec![], vec![]).unwrap();
    let other = logos_policy::Policy::new(1, &lonely, vec![]).unwrap();
    let no_vocabulary = ledger
        .cas()
        .put::<PolicyObject>(&to_canon(&other).unwrap())
        .unwrap();
    assert!(
        publish(
            &mut ledger,
            admin.public_key(),
            &admin,
            no_vocabulary,
            reason()
        )
        .is_err()
    );
    // Política que a TCB recusa (ciclo pela negação), gravada crua no CAS por alguém sem validar.
    let valid = logos_policy_text::parse_policy(&policy_text(5, "x"), &vocabulary()).unwrap();
    let mut value: logos_canon::Value =
        logos_canon::from_canon(&to_canon(&valid).unwrap()).unwrap();
    // Troca a classe da primeira regra (allow→derive): o cabeçalho decodifica, a validação recusa.
    if let logos_canon::Value::Array(top) = &mut value {
        let mut items = top.items().to_vec();
        if let logos_canon::Value::Array(rules) = &items[3] {
            let mut rules = rules.items().to_vec();
            if let logos_canon::Value::Array(rule) = &rules[0] {
                let mut fields = rule.items().to_vec();
                fields[1] = logos_canon::Value::Unsigned(3);
                rules[0] = logos_canon::Value::Array(logos_canon::Array::new(fields).unwrap());
            }
            items[3] = logos_canon::Value::Array(logos_canon::Array::new(rules).unwrap());
        }
        *top = logos_canon::Array::new(items).unwrap();
    }
    let bad = ledger
        .cas()
        .put::<PolicyObject>(&to_canon(&value).unwrap())
        .unwrap();
    assert!(publish(&mut ledger, admin.public_key(), &admin, bad, reason()).is_err());
    // Ciclo pela negação costurado nos bytes: o cabeçalho e a árvore ainda montam, só a
    // revalidação completa (estratificação) recusa.
    let parse = |text: &str| logos_policy_text::parse_policy(text, &vocabulary()).unwrap();
    let target = parse(
        "policy v6 { rule d1 derive: forall x:Action. p(x) -> q(x)  rule d2 derive: forall x:Action. q(x) -> p(x) }",
    );
    let donor = parse("policy v6 { rule d1 derive: forall x:Action. (not q(x)) -> p(x) }");
    let rules_of = |policy: &Policy| -> Vec<logos_canon::Value> {
        let value: logos_canon::Value =
            logos_canon::from_canon(&to_canon(policy).unwrap()).unwrap();
        let logos_canon::Value::Array(top) = value else {
            unreachable!()
        };
        let logos_canon::Value::Array(rules) = &top.items()[3] else {
            unreachable!()
        };
        rules.items().to_vec()
    };
    let (donor_rules, mut rules) = (rules_of(&donor), rules_of(&target));
    let (logos_canon::Value::Array(from), logos_canon::Value::Array(into)) =
        (&donor_rules[0], &rules[1])
    else {
        unreachable!()
    };
    let mut fields = into.items().to_vec();
    fields[2] = from.items()[2].clone();
    rules[1] = logos_canon::Value::Array(logos_canon::Array::new(fields).unwrap());
    let logos_canon::Value::Array(top) =
        logos_canon::from_canon(&to_canon(&target).unwrap()).unwrap()
    else {
        unreachable!()
    };
    let mut items = top.items().to_vec();
    items[3] = logos_canon::Value::Array(logos_canon::Array::new(rules).unwrap());
    let stitched = ledger
        .cas()
        .put::<PolicyObject>(
            &to_canon(&logos_canon::Value::Array(
                logos_canon::Array::new(items).unwrap(),
            ))
            .unwrap(),
        )
        .unwrap();
    assert!(Policy::decode(&to_canon(&target).unwrap(), &vocabulary()).is_ok());
    assert!(publish(&mut ledger, admin.public_key(), &admin, stitched, reason()).is_err());
    assert_eq!(ledger.head().unwrap(), head);
    // Publica a v3; v3 de novo e versão menor são recusadas.
    publish(&mut ledger, admin.public_key(), &admin, h, reason()).unwrap();
    let head = ledger.head().unwrap();
    assert!(publish(&mut ledger, admin.public_key(), &admin, h, reason()).is_err());
    let (_, lower) = put_policy(&ledger, &policy_text(2, "v2"));
    assert!(publish(&mut ledger, admin.public_key(), &admin, lower, reason()).is_err());
    assert_eq!(ledger.head().unwrap(), head);
}

#[test]
fn a_forged_root_or_version_is_caught_by_the_independent_auditor() {
    for forged in ["root", "version"] {
        let temp = Temp::new();
        let (kernel, path) = world(&temp);
        let admin = admin();
        let mut ledger = Ledger::open(&path, &kernel).unwrap();
        let (_, h) = put_policy(&ledger, &policy_text(1, "ok"));
        // Evento construído à mão, com a autoridade do admin, mas sem a recomputação do kernel.
        let mut body = EventBody::PolicyPublished {
            policy: h,
            version: if forged == "version" { 9 } else { 1 },
            root: if forged == "root" {
                [0xee; 32]
            } else {
                [0; 32]
            },
            authority: Authority {
                key: admin.key_id().unwrap(),
                sig: admin.sign::<Event>(b"").unwrap(),
            },
        };
        if forged == "version" {
            // Versão do evento diferente da do objeto, mas com a raiz certa.
            let valid =
                logos_policy_text::parse_policy(&policy_text(1, "ok"), &vocabulary()).unwrap();
            let EventBody::PolicyPublished { root, .. } = &mut body else {
                unreachable!()
            };
            *root = *PolicyTree::new(&valid).unwrap().root();
        }
        let message = body.authority_message().unwrap().unwrap();
        let EventBody::PolicyPublished { authority, .. } = &mut body else {
            unreachable!()
        };
        authority.sig = admin.sign::<Event>(&message).unwrap();
        ledger.append(body).unwrap();
        drop(ledger);
        let error = logos_audit::verify(&path, &path.join("cas"), kernel.public_key()).unwrap_err();
        let text = error.to_string();
        assert!(
            text.contains("does not match") || text.contains("differs from the object"),
            "{forged}: {text}"
        );
    }
}

#[test]
fn the_projection_refuses_a_policy_event_with_a_stale_version_or_a_missing_object() {
    // O kernel nunca grava esses eventos; se alguém os grava direto, a projeção recusa.
    for (case, version, missing) in [
        ("same version", 4u64, false),
        ("lower version", 2, false),
        ("missing object", 9, true),
    ] {
        let temp = Temp::new();
        let (kernel, path) = world(&temp);
        let admin = admin();
        let mut ledger = Ledger::open(&path, &kernel).unwrap();
        let (policy, h) = put_policy(&ledger, &policy_text(4, "v4"));
        publish(&mut ledger, admin.public_key(), &admin, h, reason()).unwrap();
        assert!(ledger.live_index().is_ok());
        let root = *PolicyTree::new(&policy).unwrap().root();
        let mut body = EventBody::PolicyPublished {
            policy: if missing {
                Hash::from_bytes([0x33; 32])
            } else {
                h
            },
            version,
            root,
            authority: Authority {
                key: admin.key_id().unwrap(),
                sig: admin.sign::<Event>(b"").unwrap(),
            },
        };
        let message = body.authority_message().unwrap().unwrap();
        let EventBody::PolicyPublished { authority, .. } = &mut body else {
            unreachable!()
        };
        authority.sig = admin.sign::<Event>(&message).unwrap();
        if missing {
            // Objeto ausente do CAS: o próprio append recusa (referências de CAS conferidas).
            assert!(ledger.append(body).is_err(), "{case}");
        } else {
            ledger.append(body).unwrap();
            assert!(ledger.live_index().is_err(), "{case}");
        }
    }
    // Versão zero nem chega a ser um evento válido.
    let temp = Temp::new();
    let (kernel, path) = world(&temp);
    let mut ledger = Ledger::open(&path, &kernel).unwrap();
    let (_, h) = put_policy(&ledger, &policy_text(1, "v1"));
    let zero = EventBody::PolicyPublished {
        policy: h,
        version: 0,
        root: [0; 32],
        authority: Authority {
            key: admin().key_id().unwrap(),
            sig: admin().sign::<Event>(b"").unwrap(),
        },
    };
    assert!(ledger.append(zero).is_err());
}
