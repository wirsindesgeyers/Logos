# Etapa7.2 — árvore Merkle ordenada e raiz assinada

Estado: implementada localmente (2026-10-24). Decisão em [ADR 0070](../adr/0070-merkle-de-politica.md);
especificação em [spec/policy-merkle.md](../../spec/policy-merkle.md).

1. **Entradas:** `policy/v1` (7.1), `Authority` dos eventos administrativos, épocas (3.4).
2. **Saídas:** `logos_policy::{PolicyTree, IndexKey, Leaf, InclusionProof, NonMembership, NoRuleProof,
   verify_inclusion, verify_non_membership, verify_present, verify_no_rule, empty_root}`;
   `EventBody::PolicyPublished` (tag 20), `CasReference::Policy`, ledger-index/v7 (`policies`,
   `current_policy`); `logos_kernel::policy::publish`; auditor com rederivação da raiz;
   domínios `rule`/`merkle-*`; fuzz `policy_proof_decode`; vetores Python independentes.
3. **Critério → evidência:**
   - *Para toda chave, exatamente uma das provas (inclusão ou não-pertinência) verifica*:
     `for_every_key_exactly_one_kind_of_proof_verifies` (40 políticas aleatórias de 0 a 34 regras,
     40 chaves, todas as provas que um adversário monta com folhas reais — todos os pares e bordas —
     contra a existência da chave).
   - *Adulterar uma regra muda `r_v`; prova antiga deixa de verificar*:
     `tampering_with_a_rule_changes_the_root_and_old_proofs_stop_verifying` (comentário, fórmula,
     classe, id, remoção, acréscimo) e, no kernel, a segunda versão publicada invalida a prova da
     primeira (`publish_records_event_epoch_and_root_and_proofs_follow_the_current_root`).
   - Independência: o Python reimplementa chave, ordem, árvore, caminhos e provas canônicas; o Rust
     bate byte a byte (`rust_tree_matches_the_independent_python_merkle_implementation`).
   - Kernel/ledger/auditor: `logos-kernel/tests/policy.rs` (publicação, recusas sem evento, projeção
     recusando versão velha, auditor recusando raiz/versão forjadas).
   - Mutações 20 mortas, 1 equivalente (Q19: a chave primária de `policies` também recusa a versão
     repetida) — `etapa-7.2-mutacoes.json`. Fuzz `policy_proof_decode` 45 s, 221 mil execuções, sem
     crash. `ci/check-policy-coverage.sh`: 100% dos ramos de `logos-policy/src` (merkle 58/58).
4. **Riscos:** o tamanho de uma prova não é autenticado sozinho (inofensivo, ver spec); a chave do
   `Allowed` aninhado usa só o símbolo externo; sem atualização incremental.
5. **Tamanho:** `M`, TCB.
