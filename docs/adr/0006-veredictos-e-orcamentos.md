# ADR 0006 — Veredictos e orçamentos

- **Status:** aceito
- **Data:** 2026-10-02
- **Etapa:** 0.6

## Contexto

A 0.6 introduz o vocabulário comum dos checkers e exige S2 (exaustão nunca é
aceitação/rejeição), S3 (resultado determinístico) e compile-fail com `trybuild`.
O plano `docs/plans/etapa-0.6.md` define o contrato implementado aqui.

## Decisões

- Produção depende somente de `logos-canon` por path. Sem novo crate externo na TCB
  de execução; `Reason` possui `Text` validado, permite texto vazio e não normaliza.
- `StepMeter::run(&Budget, FnOnce(&StepMeter) -> CheckOutcome)` é a única criação e
  finalização pública. Saldo e latch privados usam `Cell`, sem heap/locks/atomics.
  `charge(&self, u64)` compara/subtrai saldo: limite exato aceita, excesso dispara
  latch irreversível. Finalização retorna `Unknown(Steps)` mesmo se checker ignorar
  erro. Referência compartilhada impede substituição do medidor com `mem::swap`.
- Orçamento zero permite zero trabalho. Cobrar zero após exaustão continua erro.
  Não há reset, Clone, Default ou construção pública do medidor. Trabalho deve ser
  cobrado antes de executar; medidor não detecta trabalho não contabilizado nem
  interrompe closure mal escrita. Medidor é local, não Sync.
- Internos ignoram `wall_ms`; memória/tamanho são limites declarados para futuros
  consumidores, não monitoramento global de RSS. Tempo externo gera `Unknown(Time)`
  com máquina registrada pelo consumidor futuro. Nenhum supervisor nesta etapa.
- `verified_from_proof` e `refuted_from_countermodel` aceitam apenas `Accepted`;
  demais outcomes retornam `Err` intacto. Sem conversões genéricas de outcome para
  veredicto. Contexto de prova/contramodelo é explícito no nome.
- Formato novo `core/v1`, arrays com versão inteira 1, decodificados pelo tipo
  esperado, conforme `spec/core.md`. `canon/v1` e outros formatos não mudam.
  Tags/versões/aridades desconhecidas são erros, nunca defaults ou Unknown.
- `trybuild = =1.0.121` como dev-dependency (MIT OR Apache-2.0, MSRV 1.88).
  Fixar versão mantém dependências/diagnósticos reproduzíveis; não reutilizar
  exceção de doctests da 0.4. Transitivas resolvidas e respectivas licenças serão
  inventariadas abaixo antes do fechamento; allowlist inclui só grafo real.
- `serde_json = =1.0.151` também é dev-dependency direta dos testes de vetores.
  Já transitiva de trybuild, não adiciona crates ao grafo resolvido; reutilizar
  parser JSON evita manter parser artesanal só para expectativas dos vetores.
- CI Rust usa runners nativos x86_64 e aarch64; 1000 execuções por caso com bytes
  esperados comuns, sem relógio/RNG do SO. Evidência local não fecha S3 remoto.

## Fronteira dos tipos públicos

`Verdict` é enum público e decodificável, não credencial. Consumidor pode escrever
`Verdict::Refuted` ou match arbitrário; Rust não tipa variantes separadamente.
Compile-fail comprova ausência de conversões oferecidas pela biblioteca; testes
runtime e revisão comprovam comportamento das funções nomeadas. Não há prova de
que código arbitrário do consumidor respeita a política. Checker ativo, origem do
Accepted e autorização pertencem à 5.4. Não alterar enums/escopo do roadmap para
prometer isolamento inexistente.

## Alternativas recusadas

- Apenas retornar Result em charge: erro ignorado permitiria aceitar após limite.
- Referência mutável ao medidor: troca por medidor de chamada aninhada perderia latch.
- Cronômetro para checkers internos: resultado dependeria da máquina/carga.
- Catálogo de Reasons sem checkers: classificação prematura, fora do contrato.
- Conversão genérica Accepted → Verdict: não distingue prova de contramodelo.

## Dependências de desenvolvimento

Inventário resolvido por `cargo metadata`; dependências novas permitidas em
`deny.toml` (além de trybuild):

| Crate | Versão | Licença |
| --- | --- | --- |
| equivalent | 1.0.2 | Apache-2.0 OR MIT |
| glob | 0.3.4 | MIT OR Apache-2.0 |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 |
| indexmap | 2.14.2 | Apache-2.0 OR MIT |
| itoa | 1.0.18 | MIT OR Apache-2.0 |
| memchr | 2.8.3 | Unlicense OR MIT |
| serde / serde_core / serde_derive | 1.0.229 | MIT OR Apache-2.0 |
| serde_json | 1.0.151 | MIT OR Apache-2.0 |
| serde_spanned | 1.1.1 | MIT OR Apache-2.0 |
| target-tuple | 1.0.2 | MIT OR Apache-2.0 |
| termcolor | 1.4.1 | Unlicense OR MIT |
| toml | 1.1.6+spec-1.1.0 | MIT OR Apache-2.0 |
| toml_datetime | 1.1.1+spec-1.1.0 | MIT OR Apache-2.0 |
| toml_parser | 1.1.3+spec-1.1.0 | MIT OR Apache-2.0 |
| toml_writer | 1.1.2+spec-1.1.0 | MIT OR Apache-2.0 |
| winapi-util | 0.1.11 | Unlicense OR MIT |
| windows-link | 0.2.1 | MIT OR Apache-2.0 |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 |
| winnow | 1.0.4 | MIT |
| zmij | 1.0.23 | MIT |

Proc-macro2/quote/syn/unicode-ident já permitidos pelo ADR 0004. Nenhuma dependência
de desenvolvimento entra no grafo de execução do `logos-core`. Como `itoa` agora
é transitiva permitida, autoteste da allowlist passa a injetar `either`, ainda
proibida, e exigir diagnóstico específico de proibição.
