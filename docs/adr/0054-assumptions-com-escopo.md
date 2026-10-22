# ADR 0054 — Assumptions com escopo e claims condicionais

- Estado: aceito para implementação da Etapa 5.7 (2026-10-22), por delegação do
  mantenedor ("tome decisões competentes e deixe tudo documentado").
- Relacionadas: 0038 (PROPOSE), 0052 (ATMS/retratação), 0043 (cliente);
  `spec/kernel-protocol.md` §ASSUME, `spec/ledger-index.md`.

## Contexto

A 5.7 pede `Assume { formula, scope: Session | Claim(id) | Until(epoch), author }`,
claims dependentes carregando suas assumptions como premissas abertas (forma
exportável `⋀assumptions → φ`) e que assumption do agente nunca satisfaça requisito
de confiança para ações. O fio v1 já tinha `Assume [vocab, formula, scope_Text]`
(sem handler) e o PROPOSE usava escopo `""` fixo; o índice compara scope como
rótulo opaco exato.

## Decisão

1. **Formas normativas no Text existente**: `session:<rótulo>`, `until:<época>`,
   `claim:<h(Event)>`. Sem mudança de formato de evento; ASSUME recusa outras formas.
2. **Escopo derivado, não escolhido**: o escopo do PROPOSE é o único escopo das
   assumptions no fecho used das citações; misturas, `until` vencido e `claim:` sem
   o claim citado são recusados. A mesma função vale na regeneração de
   CERTIFY/WITNESS, que antes fixava `""`.
3. **Autor**: ASSUME pelo socket grava autor = chave do kernel (origem agente).
   Assumption de usuário só por caminho local; nada aqui a torna confiável para ação.
4. **Condicional no Query**: `Query [2, h]` → open-result/v1 com as assumptions do
   fecho used. A origem agente/usuário não entra no resultado nem no índice nesta
   etapa (exigiria nova versão do layout); a 8.2 resolve pela chave do autor.
5. **Retirada**: retratar a assumption usa a 5.9 e invalida os dependentes.

## Alternativas consideradas

1. **Enum estruturado de escopo no fio** (rejeitada): mudaria proposal/assume v1.
2. **Agente escolhe o escopo no PROPOSE** (rejeitada): não há campo e abriria
   mistura silenciosa de hipóteses de sessões diferentes.
3. **Estender Query status com a lista** (rejeitada): quebraria a aridade de status.

## Consequências

- `until:` só restringe de fato quando houver citação entre épocas (janelas da 6.1);
  hoje toda premissa já precisa ser da época corrente.
- O fecho de assumptions usa o mesmo CTE recursivo do snapshot (custo proporcional
  aos ancestrais; ver Armadilhas no STATUS).
