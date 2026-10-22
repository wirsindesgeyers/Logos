# ADR 0053 — S4: bloqueio de re-proposta refutada

- Estado: aceito para implementação da Etapa 5.6 (2026-10-22), por delegação do
  mantenedor ("tome decisões competentes e deixe tudo documentado").
- Base: 48f5cec (worktree paralelo; rebaseado com 9.1 e 5.9)
- Relacionadas: 0022 (índices), 0035 (transporte), 0038 (PROPOSE), 0043 (cliente);
  invariante S4 do roadmap; `spec/kernel-protocol.md`, `spec/ledger-index.md`.

## Contexto

S4: uma obrigação refutada não pode ser re-proposta (mesmo h(O)). O roadmap pede
`refuted_obligations` no índice e `Reject::AlreadyRefuted(h(M))`. Reordenar citações
já colapsa no mesmo h(O) pela geração canônica. O protocolo v1 só devolvia código
numérico de erro; a tabela `claims` já guarda `h_o` das linhas Refuted, mas sem índice.

## Decisão

1. **Conjunto = consulta.** `LedgerIndex::refuted(h(O))` devolve o primeiro evento
   Refuted da obrigação a partir de `claims` (status Refuted). Novo índice SQL
   `claims_h_o` (ledger-index/v2, `user_version` 2): o cache v1 é recusado e
   reconstruído, como qualquer cache derivado. O índice também serve às consultas
   por nó h(O) da 5.9.
2. **Checagem antes de publicar.** PROPOSE gera O, calcula h(O) e, se já refutado,
   recusa **antes** de gravar O no CAS e antes de qualquer evento.
3. **Erro com detalhe.** Código 5 `AlreadyRefuted` com valor `[5, h_O, h(Refuted),
   h(M)]` no lugar do código numérico; o envelope continua com 4 campos. `h(M)` sai
   do corpo do Refuted (replay só no caminho de recusa). Cliente Python expõe
   `AlreadyRefuted(KernelError)` com os três hashes; detalhe malformado é
   `ProtocolError`.
4. **Sem conversão para Unknown.** CERTIFY/REFUTE sobre O refutado já não têm
   instância Pending; com a re-proposta bloqueada, nenhuma tentativa posterior grava
   Unknown ou Pending para esse h(O).

## Alternativas consideradas

1. **Tabela `refuted_obligations` própria** (rejeitada): duplicaria `claims`.
2. **Erro sem detalhe** (rejeitada): o agente perderia o contramodelo para explicar.
3. **Novo envelope de 5 campos** (rejeitada): quebraria todos os parsers; o detalhe
   cabe no valor de erro.
4. **Equivalência semântica** (fora do escopo): só identidade de h(O).

## Consequências

- Formatos: ledger-index/v2 (cache) e erro already-refuted/v1, com vetores.
- O agente recebe a refutação como dado; consultar o ledger antes de usá-la.
