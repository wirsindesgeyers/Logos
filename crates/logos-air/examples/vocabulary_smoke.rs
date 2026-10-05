//! Exercício real da API pública de Vocabulary: `cargo run -p logos-air --example vocabulary_smoke`.

use std::error::Error;

use logos_air::{
    AttestorId, Compatibility, DomainSource, FactRef, Formula, FormulaNode, Grounding, Limits,
    Sort, SortDecl, SortDomain, Symbol, SymbolDecl, SymbolKind, SymbolResult, Term, TermNode,
    Vocabulary, VocabularyError,
};
use logos_canon::{Text, from_canon, to_canon};
use logos_crypto::hash;

fn require(condition: bool, message: &str) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // 1. Declarar sort aberto e sort enumerado com fonte de fato
    let sort_u = SortDecl {
        name: Text::new("User")?,
        domain: SortDomain::Open,
    };

    let c_active = Symbol {
        name: Text::new("Active")?,
        kind: SymbolKind::Const,
        arity: Vec::new(),
        result: SymbolResult::Sort(Sort {
            name: Text::new("Status")?,
        }),
    };
    let c_inactive = Symbol {
        name: Text::new("Inactive")?,
        kind: SymbolKind::Const,
        arity: Vec::new(),
        result: SymbolResult::Sort(Sort {
            name: Text::new("Status")?,
        }),
    };

    let sort_status = SortDecl {
        name: Text::new("Status")?,
        domain: SortDomain::Enumerated {
            constants: vec![c_active.clone(), c_inactive.clone()],
            domain_source: DomainSource::Fact(FactRef([0x42; 32])),
        },
    };

    // 2. Declarar símbolos com os três tipos de grounding:
    // a) Glossed (base linguística)
    let decl_active = SymbolDecl {
        signature: c_active,
        grounding: Grounding::Glossed(Text::new("Estado ativo da conta")?),
        gloss_pt: Text::new("Ativo")?,
    };
    let decl_inactive = SymbolDecl {
        signature: c_inactive,
        grounding: Grounding::Glossed(Text::new("Estado inativo da conta")?),
        gloss_pt: Text::new("Inativo")?,
    };

    // b) Anchored (base atestada externa)
    let p_verified = Symbol {
        name: Text::new("is_verified")?,
        kind: SymbolKind::Pred,
        arity: vec![Sort {
            name: Text::new("User")?,
        }],
        result: SymbolResult::Bool,
    };
    let decl_verified = SymbolDecl {
        signature: p_verified.clone(),
        grounding: Grounding::Anchored {
            attestor: AttestorId(Text::new("agent:did:logos:kyc-authority")?),
            procedure: Text::new("ed25519-v1-attest")?,
        },
        gloss_pt: Text::new("Usuário com identidade atestada")?,
    };

    // c) Defined (corpo definidor por fórmula FOL-ms-eq/v1)
    let p_can_access = Symbol {
        name: Text::new("can_access")?,
        kind: SymbolKind::Pred,
        arity: vec![Sort {
            name: Text::new("User")?,
        }],
        result: SymbolResult::Bool,
    };
    let term_user = Term::new(
        vec![TermNode::Var {
            index: 0,
            sort: Sort {
                name: Text::new("User")?,
            },
        }],
        Limits::default(),
    )?;
    let def_can_access = Formula::new(
        vec![FormulaNode::Pred {
            symbol: p_verified,
            args: vec![term_user],
        }],
        Limits::default(),
    )?;
    let decl_can_access = SymbolDecl {
        signature: p_can_access.clone(),
        grounding: Grounding::Defined(def_can_access),
        gloss_pt: Text::new("Usuário elegível para acesso")?,
    };

    // 3. Construção do vocabulário versionado
    let vocab_v1 = Vocabulary::new(
        1,
        vec![sort_u.clone(), sort_status],
        vec![
            decl_active,
            decl_inactive,
            decl_verified,
            decl_can_access.clone(),
        ],
    )?;

    require(vocab_v1.version() == 1, "versão inicial deve ser 1")?;
    require(vocab_v1.sorts().len() == 2, "deve conter 2 sorts")?;
    require(vocab_v1.symbols().len() == 4, "deve conter 4 símbolos")?;
    require(
        vocab_v1.definition_order() == [3],
        "apenas o símbolo Defined entra na ordem topológica",
    )?;

    // 4. Codificação e decodificação canônica
    let canonical_bytes = to_canon(&vocab_v1)?;
    let decoded_vocab: Vocabulary = from_canon(&canonical_bytes)?;
    require(
        decoded_vocab == vocab_v1,
        "roundtrip de Canon/CanonDecode divergiu",
    )?;

    let decoded_with_limits = Vocabulary::decode_with_limits(&canonical_bytes, Limits::default())?;
    require(
        decoded_with_limits == vocab_v1,
        "decode_with_limits divergiu do original",
    )?;

    // 5. Cálculo do hash de domínio h(V)
    let vocab_hash = hash(&vocab_v1)?;

    // 6. Extensão compatível (adiciona novo sort/símbolo)
    let sort_group = SortDecl {
        name: Text::new("Group")?,
        domain: SortDomain::Open,
    };
    let p_in_group = Symbol {
        name: Text::new("in_group")?,
        kind: SymbolKind::Pred,
        arity: vec![
            Sort {
                name: Text::new("User")?,
            },
            Sort {
                name: Text::new("Group")?,
            },
        ],
        result: SymbolResult::Bool,
    };
    let decl_in_group = SymbolDecl {
        signature: p_in_group,
        grounding: Grounding::Glossed(Text::new("Associação usuário-grupo")?),
        gloss_pt: Text::new("Membro do grupo")?,
    };

    let (vocab_v2, compat) = vocab_v1.extend(vec![sort_group], vec![decl_in_group])?;
    require(
        compat == Compatibility::Compatible,
        "adição deve ser marcada como compatível",
    )?;
    require(vocab_v2.version() == 2, "versão estendida deve incrementar")?;
    require(
        vocab_v2.compatibility_with(&vocab_v1) == Compatibility::Compatible,
        "compatibilidade direcional preservada",
    )?;

    // 7. Extensão incompatível (altera gloss de símbolo existente)
    let decl_can_access_modified_gloss = SymbolDecl {
        signature: p_can_access.clone(),
        grounding: decl_can_access.grounding,
        gloss_pt: Text::new("Gloss alterado indevidamente")?,
    };
    let (vocab_v2_incompat, compat_incompat) =
        vocab_v1.extend(Vec::new(), vec![decl_can_access_modified_gloss])?;
    require(
        compat_incompat == Compatibility::Incompatible,
        "alteração de gloss_pt deve ser Incompatible",
    )?;
    require(
        vocab_v2_incompat.compatibility_with(&vocab_v1) == Compatibility::Incompatible,
        "comparação direcional deve refletir incompatibilidade",
    )?;

    // 8. Rejeição de ciclos em definições
    let p_cycle = Symbol {
        name: Text::new("cycle_pred")?,
        kind: SymbolKind::Pred,
        arity: Vec::new(),
        result: SymbolResult::Bool,
    };
    let cycle_formula = Formula::new(
        vec![FormulaNode::Pred {
            symbol: p_cycle.clone(),
            args: Vec::new(),
        }],
        Limits::default(),
    )?;
    let decl_cycle = SymbolDecl {
        signature: p_cycle,
        grounding: Grounding::Defined(cycle_formula),
        gloss_pt: Text::new("Predicado com ciclo auto-referencial")?,
    };
    let cycle_result = Vocabulary::new(1, vec![sort_u], vec![decl_cycle]);
    require(
        cycle_result == Err(VocabularyError::CircularDefinition),
        "ciclo direto deve ser rejeitado com CircularDefinition",
    )?;

    println!(
        "VOCABULARY_SMOKE_OK: h(V)={vocab_hash}, compatible addition, incompatible gloss, cycle rejected ({})",
        std::env::consts::ARCH
    );
    Ok(())
}
