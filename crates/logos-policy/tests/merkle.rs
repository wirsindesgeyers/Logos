//! Etapa 7.2: árvore Merkle ordenada, provas de inclusão e de não-pertinência, `NoRule`.

use logos_air::Vocabulary;
use logos_canon::{Canon, Text, Value, from_canon, to_canon};
use logos_policy::{
    Class, InclusionProof, IndexKey, Leaf, NoRuleProof, NonMembership, Policy, PolicyTree, Rule,
    empty_root, verify_inclusion, verify_no_rule, verify_non_membership, verify_present,
};

const VOCABULARY: &str = r#"
vocab v1 {
  sort Action
  pred Allowed(Action)
  pred p(Action)
  pred q(Action)
  func drop(Action): Action
  func send(Action): Action
  func read(Action): Action
  func wipe(Action): Action
}
"#;

const SYMBOLS: [&str; 5] = ["drop", "send", "read", "wipe", ""];

fn vocabulary() -> Vocabulary {
    logos_air_text::parse_document(VOCABULARY, &Default::default())
        .unwrap()
        .vocabulary
}

fn rule(id: &str, class: Class, text: &str, comment: &str) -> Rule {
    Rule {
        id: Text::new(id).unwrap(),
        class,
        formula: logos_air_text::parse_formula(text, &vocabulary()).unwrap(),
        comment: Text::new(comment).unwrap(),
    }
}

fn head(symbol: &str) -> String {
    if symbol.is_empty() {
        "x".to_owned()
    } else {
        format!("{symbol}(x)")
    }
}

fn allow(id: &str, symbol: &str) -> Rule {
    rule(
        id,
        Class::Allow,
        &format!("forall x:Action. p(x) -> Allowed({})", head(symbol)),
        "",
    )
}

fn deny(id: &str, symbol: &str) -> Rule {
    rule(
        id,
        Class::Deny,
        &format!("forall x:Action. q(x) -> not Allowed({})", head(symbol)),
        "",
    )
}

fn policy(version: u64, rules: Vec<Rule>) -> Policy {
    Policy::new(version, &vocabulary(), rules).unwrap()
}

fn tree(rules: Vec<Rule>) -> PolicyTree {
    PolicyTree::new(&policy(1, rules)).unwrap()
}

/// Gerador xorshift determinístico para as políticas aleatórias.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn random_rules(rng: &mut Rng, count: usize) -> Vec<Rule> {
    (0..count)
        .map(|i| {
            let id = format!("r{i:03}");
            let symbol = SYMBOLS[rng.below(SYMBOLS.len())];
            match rng.below(4) {
                0 => allow(&id, symbol),
                1 => deny(&id, symbol),
                2 => rule(
                    &id,
                    Class::Derive,
                    if rng.below(2) == 0 {
                        "forall x:Action. q(x) -> p(x)"
                    } else {
                        "forall x:Action. p(x) -> q(x)"
                    },
                    "",
                ),
                _ => rule(
                    &id,
                    Class::Require,
                    "forall x:Action. p(x) -> Allowed(x)",
                    "",
                ),
            }
        })
        .collect()
}

fn universe() -> Vec<IndexKey> {
    let mut keys = Vec::new();
    for class in [Class::Allow, Class::Deny, Class::Require, Class::Derive] {
        for symbol in [
            "", "drop", "send", "read", "wipe", "zzz", "aaa", "p", "q", "~",
        ] {
            keys.push(IndexKey::new(class, symbol));
        }
    }
    keys
}

/// Toda prova de não-pertinência que um adversário poderia montar com folhas reais: qualquer par
/// de folhas (com caminhos corretos) ou uma folha só, nas duas bordas.
fn every_non_membership(tree: &PolicyTree) -> Vec<NonMembership> {
    let size = tree.leaves().len() as u64;
    let inclusions: Vec<_> = tree
        .leaves()
        .iter()
        .map(|leaf| tree.prove_inclusion(leaf.rule_hash()).unwrap())
        .collect();
    let mut all = vec![NonMembership {
        size,
        left: None,
        right: None,
    }];
    for a in &inclusions {
        all.push(NonMembership {
            size,
            left: Some(a.clone()),
            right: None,
        });
        all.push(NonMembership {
            size,
            left: None,
            right: Some(a.clone()),
        });
        for b in &inclusions {
            all.push(NonMembership {
                size,
                left: Some(a.clone()),
                right: Some(b.clone()),
            });
        }
    }
    all
}

#[test]
fn for_every_key_exactly_one_kind_of_proof_verifies() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    // Escala reduzida no nível rápido (mesma lógica); critério completo sem LOGOS_TIER.
    let rounds = if std::env::var("LOGOS_TIER").is_ok_and(|t| t == "fast") {
        9
    } else {
        40
    };
    for round in 0..rounds {
        let count = [0usize, 1, 2, 3, 5, 8, 13, 21, 34][round % 9];
        let tree = tree(random_rules(&mut rng, count));
        let root = *tree.root();
        let candidates = every_non_membership(&tree);
        for key in universe() {
            let present_proofs = tree
                .leaves()
                .iter()
                .filter(|leaf| {
                    let (l, p) = tree.prove_inclusion(leaf.rule_hash()).unwrap();
                    verify_present(&root, &key, &l, &p)
                })
                .count();
            let absent_proofs = candidates
                .iter()
                .filter(|proof| verify_non_membership(&root, &key, proof))
                .count();
            let exists = tree.leaves().iter().any(|l| l.key() == &key);
            assert_eq!(exists, present_proofs > 0, "round {round} {key:?}");
            assert_eq!(!exists, absent_proofs > 0, "round {round} {key:?}");
            // O provador honesto produz exatamente a prova do lado certo.
            assert_eq!(tree.prove_present(&key).is_some(), exists);
            match tree.prove_non_membership(&key) {
                Some(proof) => {
                    assert!(!exists && verify_non_membership(&root, &key, &proof));
                    // Mesmo bytes após codec.
                    let wire = to_canon(&proof).unwrap();
                    assert_eq!(from_canon::<NonMembership>(&wire).unwrap(), proof);
                }
                None => assert!(exists),
            }
            if let Some((leaf, proof)) = tree.prove_present(&key) {
                assert!(verify_present(&root, &key, &leaf, &proof));
            }
        }
    }
}

#[test]
fn tampering_with_a_rule_changes_the_root_and_old_proofs_stop_verifying() {
    let base = vec![allow("a", "drop"), deny("b", "send"), allow("c", "")];
    let original = tree(base.clone());
    let root = *original.root();
    let (leaf, inclusion) = original
        .prove_inclusion(Leaf::of_rule(&base[1]).unwrap().rule_hash())
        .unwrap();
    assert!(verify_inclusion(&root, &leaf, &inclusion));
    let absent = IndexKey::new(Class::Deny, "read");
    let non_member = original.prove_non_membership(&absent).unwrap();
    assert!(verify_non_membership(&root, &absent, &non_member));
    // Troca de comentário, de fórmula, de classe, de id, remoção e acréscimo: raiz nova.
    let variants: Vec<Vec<Rule>> = vec![
        vec![
            allow("a", "drop"),
            Rule {
                comment: Text::new("glosa nova").unwrap(),
                ..deny("b", "send")
            },
            allow("c", ""),
        ],
        vec![allow("a", "drop"), deny("b", "read"), allow("c", "")],
        vec![
            allow("a", "drop"),
            rule("b", Class::Derive, "forall x:Action. q(x) -> p(x)", ""),
            allow("c", ""),
        ],
        vec![allow("a", "drop"), deny("bb", "send"), allow("c", "")],
        vec![allow("a", "drop"), allow("c", "")],
        vec![
            allow("a", "drop"),
            deny("b", "send"),
            allow("c", ""),
            allow("d", "read"),
        ],
    ];
    for (i, rules) in variants.into_iter().enumerate() {
        let changed = tree(rules);
        assert_ne!(changed.root(), &root, "variante {i}");
        // A prova antiga não verifica contra a raiz nova, e a de não-pertinência também não.
        assert!(
            !verify_inclusion(changed.root(), &leaf, &inclusion),
            "variante {i}"
        );
        assert!(
            !verify_non_membership(changed.root(), &absent, &non_member),
            "variante {i}"
        );
    }
    // A ordem de entrada e a versão não mudam a raiz; o hash da política muda com a versão.
    let reordered = tree(vec![allow("c", ""), allow("a", "drop"), deny("b", "send")]);
    assert_eq!(reordered.root(), &root);
    assert_ne!(
        policy(1, base.clone()).hash().unwrap(),
        policy(2, base).hash().unwrap()
    );
}

#[test]
fn tree_shapes_from_zero_to_many_leaves_verify_every_leaf() {
    let mut rng = Rng(7);
    assert_eq!(*tree(vec![]).root(), empty_root());
    for count in 1..=40 {
        let tree = tree(random_rules(&mut rng, count));
        assert_eq!(tree.leaves().len(), count);
        assert!(tree.leaves().windows(2).all(|w| w[0] < w[1]));
        for leaf in tree.leaves() {
            let (l, p) = tree.prove_inclusion(leaf.rule_hash()).unwrap();
            assert_eq!(p.size, count as u64);
            assert!(verify_inclusion(tree.root(), &l, &p), "n={count}");
        }
    }
    let t = tree(random_rules(&mut Rng(1), 3));
    assert!(t.prove_inclusion(&[0xab; 32]).is_none());
}

#[test]
fn malformed_proofs_are_rejected() {
    let t = tree(vec![
        allow("a", "drop"),
        deny("b", "send"),
        allow("c", ""),
        deny("d", "read"),
        allow("e", "wipe"),
    ]);
    let root = *t.root();
    let (leaf, proof) = t.prove_inclusion(t.leaves()[2].rule_hash()).unwrap();
    assert!(verify_inclusion(&root, &leaf, &proof));
    let bad = |change: &dyn Fn(&mut InclusionProof)| {
        let mut p = proof.clone();
        change(&mut p);
        !verify_inclusion(&root, &leaf, &p)
    };
    assert!(bad(&|p| p.index = p.size));
    // Sem a guarda `index < size` uma posição fora da árvore poderia imitar uma folha real: em
    // 2 folhas, o índice 2 toma as mesmas decisões de ordem que o índice 0.
    let pair = tree(vec![allow("a", "drop"), allow("b", "send")]);
    let (l0, p0) = pair.prove_inclusion(pair.leaves()[0].rule_hash()).unwrap();
    assert!(verify_inclusion(pair.root(), &l0, &p0));
    let alias = InclusionProof {
        index: 2,
        ..p0.clone()
    };
    assert!(!verify_inclusion(pair.root(), &l0, &alias));
    assert!(bad(&|p| p.index += 1));
    assert!(bad(&|p| p.size = 0));
    assert!(bad(&|p| p.size = 1 << 20));
    // O tamanho não é autenticado sozinho: 5→6 deixa a mesma cadeia de hashes (a 6ª folha estaria
    // dentro do irmão da direita). Isso não esconde folhas (ver o teste de propriedade, que tenta
    // todos os pares); um tamanho que muda a forma da árvore no caminho é recusado.
    assert!(bad(&|p| p.size += 8));
    assert!(bad(&|p| p.path.pop().map(|_| ()).unwrap_or(())));
    assert!(bad(&|p| p.path.push([0; 32])));
    assert!(bad(&|p| p.path[0][0] ^= 1));
    assert!(bad(&|p| p.path.swap(0, 1)));
    assert!(!verify_inclusion(&[0; 32], &leaf, &proof));
    let (other, _) = t.prove_inclusion(t.leaves()[1].rule_hash()).unwrap();
    assert!(!verify_inclusion(&root, &other, &proof));
    // Caminho curto demais para o tamanho: `last` ainda não chegou a zero.
    let single = tree(vec![allow("a", "drop")]);
    let (l, p) = single
        .prove_inclusion(single.leaves()[0].rule_hash())
        .unwrap();
    assert!(p.path.is_empty() && verify_inclusion(single.root(), &l, &p));
    let mut long = p.clone();
    long.path.push([1; 32]);
    assert!(!verify_inclusion(single.root(), &l, &long));
}

#[test]
fn non_membership_rejects_gaps_wrong_sides_and_wrong_sizes() {
    let t = tree(vec![
        allow("a", "drop"),
        deny("b", "read"),
        deny("c", "wipe"),
        allow("d", "send"),
    ]);
    let root = *t.root();
    let key = IndexKey::new(Class::Deny, "send");
    let honest = t.prove_non_membership(&key).unwrap();
    assert!(verify_non_membership(&root, &key, &honest));
    // Vizinhas que não são adjacentes (pula uma folha).
    let leaves: Vec<_> = t
        .leaves()
        .iter()
        .map(|l| t.prove_inclusion(l.rule_hash()).unwrap())
        .collect();
    let gap = NonMembership {
        size: 4,
        left: Some(leaves[0].clone()),
        right: Some(leaves[2].clone()),
    };
    assert!(!verify_non_membership(
        &root,
        &IndexKey::new(Class::Allow, "read"),
        &gap
    ));
    // Lados trocados e chave que coincide com uma vizinha.
    let swapped = NonMembership {
        size: honest.size,
        left: honest.right.clone(),
        right: honest.left.clone(),
    };
    assert!(!verify_non_membership(&root, &key, &swapped));
    let on_left = honest.left.clone().unwrap().0.key().clone();
    assert!(!verify_non_membership(&root, &on_left, &honest));
    // Tamanho declarado diferente do das provas.
    let wrong_size = NonMembership {
        size: honest.size + 1,
        ..honest.clone()
    };
    assert!(!verify_non_membership(&root, &key, &wrong_size));
    // Bordas: chave antes de tudo e depois de tudo, e bordas forjadas.
    let first = IndexKey::new(Class::Allow, "");
    let before = t.prove_non_membership(&first).unwrap();
    assert!(before.left.is_none() && verify_non_membership(&root, &first, &before));
    let last = IndexKey::new(Class::Derive, "zz");
    let after = t.prove_non_membership(&last).unwrap();
    assert!(after.right.is_none() && verify_non_membership(&root, &last, &after));
    let fake_left_edge = NonMembership {
        size: 4,
        left: None,
        right: Some(leaves[1].clone()),
    };
    assert!(!verify_non_membership(
        &root,
        &IndexKey::new(Class::Allow, "read"),
        &fake_left_edge
    ));
    let fake_right_edge = NonMembership {
        size: 4,
        left: Some(leaves[1].clone()),
        right: None,
    };
    assert!(!verify_non_membership(
        &root,
        &IndexKey::new(Class::Deny, "zz"),
        &fake_right_edge
    ));
    // Cada lado só vale para chaves entre as vizinhas: a mesma prova, com chaves de fora, falha.
    let below_all = IndexKey::new(Class::Allow, "");
    let above_all = IndexKey::new(Class::Derive, "zzzz");
    assert!(!verify_non_membership(&root, &below_all, &after));
    assert!(!verify_non_membership(&root, &key, &before));
    assert!(!verify_non_membership(&root, &below_all, &honest));
    assert!(!verify_non_membership(&root, &above_all, &honest));
    // Caminho adulterado em cada vizinha (em cada forma de prova): a inclusão falha.
    let corrupt = |proof: &NonMembership, left: bool| {
        let mut forged = proof.clone();
        let side = if left {
            &mut forged.left
        } else {
            &mut forged.right
        };
        if let Some((_, inclusion)) = side {
            inclusion.path[0][0] ^= 1;
        }
        forged
    };
    assert!(!verify_non_membership(&root, &key, &corrupt(&honest, true)));
    assert!(!verify_non_membership(
        &root,
        &key,
        &corrupt(&honest, false)
    ));
    assert!(!verify_non_membership(&root, &last, &corrupt(&after, true)));
    assert!(!verify_non_membership(
        &root,
        &first,
        &corrupt(&before, false)
    ));
    // Árvore vazia: só a prova vazia, e só contra a raiz vazia.
    let nothing = NonMembership {
        size: 0,
        left: None,
        right: None,
    };
    assert!(verify_non_membership(&empty_root(), &key, &nothing));
    assert!(!verify_non_membership(&root, &key, &nothing));
    let sized = NonMembership {
        size: 4,
        left: None,
        right: None,
    };
    assert!(!verify_non_membership(&empty_root(), &key, &sized));
}

#[test]
fn no_rule_covers_the_specific_symbol_and_the_wildcard() {
    let t = tree(vec![
        allow("a", "drop"),
        deny("b", "send"),
        deny("c", "wipe"),
    ]);
    let root = *t.root();
    // Nenhuma regra deny casa com `read`: nem a específica nem o curinga existem.
    let proof = t.prove_no_rule(Class::Deny, "read").unwrap();
    assert!(verify_no_rule(&root, Class::Deny, "read", &proof));
    // Com ação diferente, ou classe diferente, a mesma prova não vale.
    assert!(!verify_no_rule(&root, Class::Deny, "wipe", &proof));
    assert!(!verify_no_rule(&root, Class::Allow, "read", &proof));
    // `send` tem regra deny: não existe prova.
    assert!(t.prove_no_rule(Class::Deny, "send").is_none());
    // Com um deny curinga, nenhuma ação tem NoRule.
    let wild = tree(vec![deny("w", "")]);
    assert!(wild.prove_no_rule(Class::Deny, "read").is_none());
    assert!(wild.prove_no_rule(Class::Deny, "").is_none());
    // Símbolo curinga: só a prova do curinga.
    let any = t.prove_no_rule(Class::Deny, "").unwrap();
    assert!(any.specific.is_none() && verify_no_rule(&root, Class::Deny, "", &any));
    // A forma da prova precisa casar com o símbolo.
    assert!(!verify_no_rule(&root, Class::Deny, "read", &any));
    let with_extra = NoRuleProof {
        specific: proof.specific.clone(),
        wildcard: proof.wildcard.clone(),
    };
    assert!(!verify_no_rule(&root, Class::Deny, "", &with_extra));
    // Codec estrito de ida e volta.
    let wire = to_canon(&proof).unwrap();
    assert_eq!(from_canon::<NoRuleProof>(&wire).unwrap(), proof);
    let wire = to_canon(&any).unwrap();
    assert_eq!(from_canon::<NoRuleProof>(&wire).unwrap(), any);
}

#[test]
fn index_keys_follow_the_rule_head() {
    let keys = |rules: Vec<Rule>| -> Vec<(Class, String)> {
        rules
            .iter()
            .map(|r| {
                let k = IndexKey::of_rule(r).unwrap();
                (k.class().unwrap(), k.symbol().to_owned())
            })
            .collect()
    };
    assert_eq!(
        keys(vec![
            allow("a", "drop"),
            allow("b", ""),
            deny("c", "send"),
            rule("d", Class::Derive, "forall x:Action. p(x) -> q(x)", ""),
            rule(
                "e",
                Class::Require,
                "forall x:Action. p(x) -> Allowed(x)",
                ""
            ),
        ]),
        [
            (Class::Allow, "drop".to_owned()),
            (Class::Allow, String::new()),
            (Class::Deny, "send".to_owned()),
            (Class::Derive, "q".to_owned()),
            (Class::Require, String::new()),
        ]
    );
    let leaf = Leaf::of_rule(&allow("a", "drop")).unwrap();
    assert_eq!(leaf.key(), &IndexKey::new(Class::Allow, "drop"));
    // `h(regra)` vem do domínio `rule`: muda com qualquer campo.
    assert_ne!(
        leaf.rule_hash(),
        Leaf::of_rule(&allow("b", "drop")).unwrap().rule_hash()
    );
    // Regra fora da forma da classe não tem chave.
    let not_horn = rule(
        "n",
        Class::Allow,
        "forall x:Action. (p(x) or q(x)) -> Allowed(x)",
        "",
    );
    assert!(IndexKey::of_rule(&not_horn).is_err());
}

#[test]
fn proof_codecs_are_strict() {
    let t = tree(vec![allow("a", "drop"), deny("b", "send"), allow("c", "")]);
    let (leaf, proof) = t.prove_inclusion(t.leaves()[1].rule_hash()).unwrap();
    for (name, wire) in [
        ("leaf", to_canon(&leaf).unwrap()),
        ("inclusion", to_canon(&proof).unwrap()),
    ] {
        let mut extra = wire.clone();
        extra.push(0);
        match name {
            "leaf" => {
                assert!(from_canon::<Leaf>(&extra).is_err());
                assert_eq!(from_canon::<Leaf>(&wire).unwrap(), leaf);
            }
            _ => {
                assert!(from_canon::<InclusionProof>(&extra).is_err());
                assert_eq!(from_canon::<InclusionProof>(&wire).unwrap(), proof);
            }
        }
    }
    let array = |items: Vec<Value>| Value::Array(logos_canon::Array::new(items).unwrap());
    let decode_leaf = |v: Value| from_canon::<Leaf>(&to_canon(&v).unwrap());
    // Classe desconhecida, hash curto, aridade, tipos.
    assert!(
        decode_leaf(array(vec![
            Value::Unsigned(9),
            Text::new("x").unwrap().to_value().unwrap(),
            Value::Bytes(vec![0; 32])
        ]))
        .is_err()
    );
    assert!(
        decode_leaf(array(vec![
            Value::Unsigned(0),
            Text::new("x").unwrap().to_value().unwrap(),
            Value::Bytes(vec![0; 31])
        ]))
        .is_err()
    );
    assert!(decode_leaf(array(vec![Value::Unsigned(0)])).is_err());
    assert!(decode_leaf(Value::Unsigned(0)).is_err());
    let decode_proof = |v: Value| from_canon::<InclusionProof>(&to_canon(&v).unwrap());
    let path = |n: usize| array((0..n).map(|_| Value::Bytes(vec![0; 32])).collect());
    assert!(
        decode_proof(array(vec![
            Value::Unsigned(0),
            Value::Unsigned(1),
            path(65)
        ]))
        .is_err()
    );
    assert!(
        decode_proof(array(vec![
            Value::Unsigned(0),
            Value::Unsigned(1),
            path(64)
        ]))
        .is_ok()
    );
    assert!(decode_proof(array(vec![Value::Unsigned(0), Value::Unsigned(1)])).is_err());
    assert!(
        decode_proof(array(vec![
            Value::Unsigned(0),
            Value::Unsigned(1),
            array(vec![Value::Bytes(vec![0; 3])])
        ]))
        .is_err()
    );
    let decode_non = |v: Value| from_canon::<NonMembership>(&to_canon(&v).unwrap());
    assert!(
        decode_non(array(vec![
            Value::Unsigned(0),
            array(vec![]),
            array(vec![])
        ]))
        .is_ok()
    );
    assert!(
        decode_non(array(vec![
            Value::Unsigned(0),
            array(vec![]),
            Value::Unsigned(1)
        ]))
        .is_err()
    );
    assert!(
        decode_non(array(vec![
            Value::Unsigned(0),
            array(vec![Value::Unsigned(1), Value::Unsigned(2)]),
            array(vec![])
        ]))
        .is_err()
    );
    assert!(
        decode_non(array(vec![
            Value::Unsigned(0),
            array(vec![Value::Unsigned(1)]),
            array(vec![])
        ]))
        .is_err()
    );
    assert!(decode_non(array(vec![Value::Unsigned(0)])).is_err());
    let decode_no_rule = |v: Value| from_canon::<NoRuleProof>(&to_canon(&v).unwrap());
    let empty_non = || array(vec![Value::Unsigned(0), array(vec![]), array(vec![])]);
    assert!(decode_no_rule(array(vec![array(vec![]), empty_non()])).is_ok());
    assert!(decode_no_rule(array(vec![array(vec![empty_non()]), empty_non()])).is_ok());
    assert!(
        decode_no_rule(array(vec![
            array(vec![empty_non(), empty_non()]),
            empty_non()
        ]))
        .is_err()
    );
    assert!(decode_no_rule(array(vec![array(vec![])])).is_err());
}
