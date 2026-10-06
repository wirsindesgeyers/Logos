# Plano — Etapa 3.1: catálogo assinado de eventos

## 1. Entradas

Canon/CanonDecode/Value/Text; Domain/Hash/Signer/Signature/KeyId/PublicKey;
Budget/CheckOutcome; AIR Formula/Vocabulary; Obligation e h(O) existentes.
Ledger já possui CAS. Nenhuma dependência ausente; não criar kernel/appender.
Spec conceitual §3–5 e ROADMAP 3.1/Apêndice C; ADR 0020/spec/events.md.

## 2. Saídas

`EventBody` (13 variantes), `Event`, `CheckRecord`, `Consistency`, `Authority`,
`EventProposal`/`TaintedText` e `Route`. APIs de bytes/Canon com erro explícito;
`Event::sign(seq,epoch,prev,body,&signer) -> Result<Event,EventError>`;
`Event::verify(&PublicKey) -> Result<(),EventError>` verifica somente assinatura
externa. Payload/verify de autoridade separados, sem política de trust embutida.
Hash<Event> usa tag existente e inclui assinatura. Schema normativo event/v1.

## 3. Ordem e arquivos

1. Fixar spec/events.md/ADR 0020 e contratos. Sem alteração de formatos antigos.
2. `crates/logos-ledger/src/event_body.rs`, `event_body_codec.rs`: catálogo,
   campos mínimos, invariantes de checks/consistência/refs, tainted texto.
3. `src/event.rs`, lib/Cargo/manifesto/deny: envelope, assinatura, decode/caps.
4. `tools/gen_event_vectors.py`, `spec/vectors/events/vectors.json`,
   `tests/events.rs`: oráculo Python canon/hashlib/cryptography e vetores de
   cada variante, alteração de campos, erro estrutural e invariantes.
5. `examples/events_smoke.rs`, `fuzz/fuzz_targets/event_decode.rs`, integração
   verify/workflows: execução pública com chave/assinatura real, todos parsers
   e decoder de body exercitados por fuzz. Scripts existentes reutilizados.
6. Mutações automáticas capturadas, fuzz final 60s via nightly datado;
   ci/verify.sh --selftest, medição da TCB; docs/estado e commit sem push.

## 4. Critérios → evidência

- Vetores canônicos de cada tipo: gerador independente --check e teste Rust
  decodifica/recodifica, compara hash/assinatura/chave conhecidos.
- Alterar qualquer campo invalida assinatura: regressão de adulteração do
  envelope e folhas de todos os bodies, inclusive metadados/budgets/refs,
  assinatura de autoridade e assinatura externa. Controles positivos reais.
- Regras TCB: manifesto/lints/deny, fuzz do decoder, mutantes de vínculo de
  assinatura/KeyId/estado rejeitados; smoke API pública confirma cadeia de
  identidade/assinatura sem alegar persistência ou validade lógica.

## 5. Riscos/decisões

ADR 0020 resolve wire/payloads/signature/hash/consistência/proposta. Autoridade
externa é fornecida pelo consumidor; chave assinante não equivale a admin.
Rota/artifact não demonstra tradução correta nem executa rotas futuras.
Caps de wire/Text/refs limitam entradas, não hard RSS. Generic CanonDecode
não é wrapper de rede; Event::decode confere bytes antes de materializar Value.
Assinatura válida não prova cadeia/logic/causas; estas etapas continuam ausentes.

## 6. Tamanho

M, conforme roadmap, sem mudança de escopo ou ordem.

## 7. Evidência local

- 18 vetores independentes: 13 tags e variantes de consistência/Unknown/
  proposta não NFC; Python --check, Rust decode/encode/hash/verify e assinatura
  reproduzida com Signer real coincidem byte a byte.
- 15 regressões: adulterações de todas as folhas/formatos de arrays,
  identidade/chave/assinatura, recibos administrativos, outcome, consistência,
  subconjunto/ordenação de refs, limites inclusivos e versões/tags estritos.
  `EVENT_FIELDS_TAMPER_OK vectors=18 mutations=599`.
- `EVENTS_SMOKE_OK`: Genesis/proposta/admin/Verified reais; assinatura
  adulterada e chave errada rejeitadas. Consistência Unknown permanece Unknown.
  Fixtures não representam checagem lógica nem raiz de confiança de produção.
- Fuzz `FUZZ_TOOLCHAIN=nightly-2026-09-28 ci/fuzz.sh 60 event_decode`:
  55 seeds iniciais/98154 execuções; runner permanente semeia 18 wires
  válidos da spec na CI limpa. Execução final: 123392/61 s, sem crash.
  libFuzzer limita entradas geradas a 4096 bytes neste smoke; limites maiores
  têm regressões determinísticas, não prova de fuzz longo.
- Medição: correção 7892 próprios + 78287 runtime = 86179 LOC;
  só compilação 51581, ledger 1877. +1215 próprios; nenhuma dependência
  runtime externa nova. Testes/CAS anteriores preservados.
- 18 mutantes viáveis capturados, zero sobreviventes; baseline já exercitado
  pelos 15 testes. Remoção das validações, vínculo KeyId, outcomes e h(O)
  de consistência detectados. Filtro exclui mutações dos limiares de tamanho;
  não é campanha completa de todos os operadores/arquivos.

```sh
cargo mutants -p logos-ledger -f '*event*rs' \
  -F 'Event::verify|EventBody::verify_authority|EventBody::validate|Consistency::validate' \
  -E 'replace >' --baseline skip --jobs 2 --timeout 120 --caught -- --test events
```

- Full `ci/verify.sh --selftest`: `VERIFY_OK`, exit 0, 249,58 s;
  227 Python + 25 TCB, todos smokes/vetores/controles passaram.
  Autoteste concorrente avisou tar por corpus mudando; `ci/selftest.sh`
  isolado passou sem avisos (14,26 s). Avisos deny preexistentes.
- Erros iniciais de compilação (Reason privado/helper sombreado),
  Ruff/mypy e clippy corrigidos antes das execuções finais.
- CI/ARM/fuzz ≥ 1 h ainda não observados. Nenhuma persistência, índice
  ou validade lógica implementada; formatos anteriores inalterados.


