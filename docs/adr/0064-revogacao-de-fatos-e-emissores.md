# ADR 0064 — Revogação de fatos e de emissores

- Estado: aceito para implementação da Etapa 6.4 (2026-10-24), por delegação do
  mantenedor. Revisável.
- Relacionadas: 0052 (ATMS/cascata), 0056 (revogação de checker suspende), 0061,
  0063; `spec/fact.md` §5, `spec/events.md` §13, `spec/ledger-index.md`.

## Contexto

A 6.4 pede `Retract` (5.9) para fatos e `IssuerRevoked { KeyId, since }` para
atestador comprometido, que retrata em massa os fatos dele após `since`, com
cascata. Critério: revogar `backup-svc` invalida todos os claims que dependiam de
fatos dele, e só esses. Fora do escopo: rotação de chaves.

## Decisão

1. **Retratar fato** usa o `Retracted` existente; só o admin pode (o emissor não
   assina eventos do kernel). A projeção já matava qualquer nó Single; o kernel
   passa a aceitar `EntryKind::Fact` como alvo, e o `reconcile` acompanha fatos.
2. **`IssuerRevoked { issuer: KeyId, since: u64, authority }`**, tag 18, assinado
   pelo admin (mesma `Authority` de 0063). `since` é a **época de registro** do
   fato, gravada pelo kernel, e não o `observed_at`: o emissor comprometido assina
   `observed_at` e poderia datar fatos falsos para antes de `since`.
3. **Efeito**: todo fato vivo do emissor com época de registro ≥ `since` morre
   (status Invalidated, como numa retratação); a cascata ATMS derruba os claims
   sem rota alternativa viva, e o `reconcile` grava `Invalidated` com
   `causes = [h(IssuerRevoked), premissa morta]`. Fatos anteriores a `since`
   continuam válidos.
4. **Revogação é terminal**: emissor revogado não registra mais fatos (kernel,
   projeção e auditor recusam), independentemente da configuração de confiança.
   Uma revogação por emissor; a segunda é recusada.
5. Morte, não suspensão: diferente do checker (0056), fato de emissor comprometido
   não tem recheck que o reabilite.

## Alternativas consideradas

1. **`since` por `observed_at`** (rejeitada): controlado pelo atacante.
2. **Suspender em vez de matar** (rejeitada): não há o que rechecar num fato.
3. **Remover o emissor da configuração de confiança** (insuficiente): na 6.2 isso
   só barra citações novas, sem derrubar claims.

## Consequências

- Formatos novos: corpo de evento tag 18; ledger-index/v6 (tabela
  `revoked_issuers`; caches v5 recusados e reconstruídos).
- API: `facts::revoke_issuer`, `LedgerLiveIndex::issuer_revoked`; `Retractions`
  aceita fato.
- Como em 0063, a chave do admin está fora do ledger; o auditor confere a regra
  (sem fato de emissor revogado, sem revogação dupla), não a identidade do admin.
