# ADR 0070 — Árvore Merkle de política e publicação

- Estado: aceito para a Etapa 7.2 (2026-10-24), por delegação do mantenedor. Revisável.
- Relacionadas: 0069 (policy/v1), 0063/0064 (eventos com `Authority`), `spec/policy-merkle.md`.

## Contexto

A 7.2 pede árvore Merkle sobre as regras ordenadas por `(chave de indexação, h(regra))`,
`PolicyPublished { version, root, admin_sig }` com `EpochAdvanced` associado, `prove_inclusion` /
`prove_non_membership` com verificadores na TCB e a premissa `NoRule(deny, key(a), v)`.

## Decisão

1. **Chave = `(classe, símbolo-cabeça da ação)`**, vazio = qualquer ação. `NoRule` exige duas
   não-pertinências (a específica e o curinga), porque uma regra com ação variável casa com tudo.
2. **Árvore de RFC 9162** com hashes separados por domínio. Quatro domínios novos reservados:
   `rule`, `merkle-leaf`, `merkle-node`, `merkle-empty` (tabela de `spec/domains.md`).
3. **Não-pertinência por vizinhas adjacentes**, não por árvore esparsa: a ordenação por chave é o que
   dá exatamente-uma-das-duas provas; sem atualização incremental (fora do escopo).
4. **Evento `PolicyPublished`** (tag 20): `[1,20,h(Policy),version,root,Authority]`. O kernel
   recomputa a raiz e revalida a política antes do append; o `EpochAdvanced` seguinte carrega
   `policy_root` e `vocab`. O índice v7 guarda `policies(version, policy, root, event)`; o auditor
   rederiva a raiz do objeto no CAS e recusa raiz/versão forjadas. O auditor passa a depender de
   `logos-policy` (não de kernel nem de ledger; `ci/check-audit-independence.sh` segue verde): a
   conferência é uma rederivação a partir dos objetos, mas o código da árvore é compartilhado.
5. **Verificação sem tamanho autenticado.** Mantida a forma do RFC; a guarda `índice < tamanho` e as
   condições de adjacência/borda fecham o que o tamanho solto abriria (teste do alias 2→0).

## Alternativas consideradas

1. **Árvore Merkle esparsa/trie** para não-pertinência direta: mais código na TCB e raiz que muda
   a cada inserção; a árvore ordenada já satisfaz o critério.
2. **Raiz dentro do objeto policy/v1**: circular (a identidade do objeto incluiria a raiz das
   próprias regras); a raiz fica no evento e é recomputável.
3. **Auditor reimplementar a árvore**: duplica TCB; o vetor Python independente cobre a
   independência de implementação, o auditor cobre a consistência evento↔objeto.

## Consequências

- Formato novo (evento 20, índice v7, quatro domínios de hash) com vetores Python independentes
  (raízes, caminhos, provas canônicas), fuzz `policy_proof_decode` e cobertura 100% de ramos em
  `logos-policy/src`.
- Risco: `require` e `derive` entram na árvore sem semântica de avaliação (7.3); a chave de um
  `Allowed(f(g(x)))` usa só o símbolo externo `f`.
