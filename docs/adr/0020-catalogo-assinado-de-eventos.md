# ADR 0020 — Catálogo assinado event/v1

- Estado: aceito
- Data: 2026-10-05
- Escopo: ROADMAP 3.1 (TCB), sem persistência da 3.2

## Contexto

Ledger contém apenas CAS. Canon, assinatura agnóstica por algoritmo, Budget,
CheckOutcome, AIR e h(O) existem; catálogo/assinatura de eventos não existem.
Apêndice C define 13 eventos iniciais e §4 exige consistência explícita e
separação de Rejected/Refuted. Assinatura externa não autentica política/admin.

## Decisão

Novo event/v1 de arrays fixos versionados, discriminadores 0 (evento) e 1
(payload de autoridade). Tags de hash/assinatura já reservadas em crypto,
sem domínio novo. Hash inclui assinatura; assinatura cobre todos os outros
campos, inclusive seq/epoch/prev/kernel_key. KeyId é conferido contra chave
pública externa. Genesis único formato seq0/epoch0/prevNone/chave coincidente;
esta relação não implementa histórico/appender.

Catálogo e invariantes exatos em spec/events.md: 13 variantes fechadas,
CheckRecord em toda tentativa registrada, consistência Witness/Unknown,
used ⊆ cited, refs conjuntos estritamente ordenados. Unknown/Rejected nunca
produzem eventos Accepted. Decoder/assinatura não provam lógica, DAG ou
confiança no emissor; kernel futuro continua único proprietário da chave.
Assinaturas de autoridade são recibos verificáveis em payload com discriminador
próprio, sem assumir que uma chave embutida seja admin. API de leitura pública
não é typestate de Claim nem construtor de Verdict.

Snapshot de proposta conserva os cinco campos de proposal/v1, mas tainted
texto é UTF-8 em bytes para preservar não NFC e caracteres sem conversão
canônica. Sem depender de logos-protocol (fora da TCB, serde runtime) ou
reinterpretar texto. Não altera proposal/v1. Artefatos/binário kernel usam
SHA-256 cru bytes32; cert/checker/model usam domínios reservados distintos.
Hashes tipados de objetos ainda externos usam marcadores locais, evitando
ciclo logos-model → logos-ledger → logos-model.

Dependências novas diretas do ledger somente crates TCB já auditadas no workspace:
logos-canon, logos-core, logos-air e logos-obligation. serde_json somente dev
para consumir vetores independentes, conforme padrão de testes existentes;
allowlist deny.toml anota esta decisão. Nenhuma dependência runtime externa nova.
Caps por campo/lista e byte wire em spec; falhas explícitas sem truncamento.

## Alternativas e consequências

- JSON/protocol runtime no ledger ampliaria TCB desnecessariamente.
- Text para proposal normalizaria/rejeitaria dados que protocolo preserva.
- Uma assinatura com payload ambíguo permitiria reuso entre papéis/eventos.
- Marcar Refuted a partir de Rejected contrariaria core/spec.

Arquivos TCB novos entram no manifesto; decoder/body/autorizações recebem
fuzz e vetores independentes, adulteração de todo campo e mutações automáticas.
Sem eventos de fases posteriores, autorização de ações, índices ou persistência.
