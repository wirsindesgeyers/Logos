# ADR 0034 — Typestate e recibos de transição duráveis

- Estado: aceito para implementação da Etapa5.1
- Data: 2026-10-18
- Relacionadas: 0006, 0020, 0021, 0030, 0033

## Contexto

O catálogo tem Event, não SignedEvent<Verified>. Um Event decodificado ou
assinado por um caller não comprova append e CheckOutcome::Accepted é um
enum público, não uma credencial de checker ativo. A5.1 precisa entregar
tipos/transições sem antecipar CERTIFY5.4. O appender3.2 já está pronto.

## Decisão

1. Claim<S> e SignedEvent<S> têm campos privados no módulo de transições;
   não implementam Clone, Copy, Default, CanonDecode nem conversões de dados
   externos. O recibo tipado contém o Event imutável retornado pelo append
   confirmado e h(Event); não muda event/v1 nem a assinatura existente.
2. Transitions interno empresta exclusivamente um Ledger e cria identidade
   de sessão Arc privada. Claims da sessão podem intercalar eventos; claims
   de outra sessão são recusados antes do append, mesmo sob a mesma chave.
   Nenhuma trait pública de sink permite forjar persistência.
3. Entrada Proposed cria Draft já registrado; Draft→Pending registra
   ObligationCreated. Logo PROPOSE5.3 terá dois eventos, um por operação;
   não prometer atomicidade conjunta que o appender não oferece.
4. Pending→Verified/Refuted exige Accepted no contexto respectivo, h(O)
   exato e época corrente. Verified conserva o conjunto de citações da
   proposta; used⊆cited é validado pelo catálogo. Pending→Unknown exige
   Unknown, sem promoção posterior. Verified→Invalidated referencia seu
   próprio evento Verified e permite época posterior. A cascata é da5.9.
5. Somente o módulo de transições constrói Claim<Verified>; sua API é
   pub(crate) e não constitui handler público. Até5.4, accept/refute têm
   precondição interna de evidência autenticada por checker ativo. Nenhuma
   função pública aceita Event, Verdict ou CheckOutcome para emitir claims.
   Rejected não muda typestate; registrar tentativas e política é da5.4.
6. Erro antes do append não escreve evento. Erro no append não publica novo
   Claim; o anterior já foi consumido. Persistência ambígua exige replay,
   sem retry automático ou afirmação de exactly-once após crash. O hash do
   recibo também é falível: eventual erro após commit não publica estado.
7. Novas arestas runtime kernel→canon/core/crypto/ledger/obligation já permitidos;
   air/trybuild somente dev. Nenhum pacote externo novo, parser ou
   formato serializado. Registrar fontes no manifesto e provar guardas com
   mutações. Auditor continua sem dependência kernel/ledger.

## Consequências

O tipo expressa uma transição local registrada, não citabilidade eterna ou
autorização de ação. Reabertura/reconstrução dos claims e handlers ficam nas
etapas donas; não há construtor público para importar eventos assinados.
spec/state-machine.md limita a tabela da5.1 e distingue tentativas futuras.
