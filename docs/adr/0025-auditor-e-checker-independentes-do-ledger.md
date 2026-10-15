# ADR 0025 — Auditor e checker independentes do ledger

- Estado: aceita
- Etapa: 3.6
- Data: 2026-10-15
- Relacionadas: 0016, 0020, 0021, 0024

## Contexto

O checker puro da2.3 está pronto, mas model -> ledger para ler CAS impede
consumi-lo no auditor. O guard percorre inclusive arestas dev/all-features;
uma feature ou alias não resolve independência. Não falta checker lógico,
formato ou algoritmo: é necessário separar transporte de checagem.

## Decisão

1. Substituir métodos ModelChecker::*_from_cas por *_from_source, recebendo
   uma closure falível que entrega Option<Vec<u8>> para Hash<FiniteModel>.
   O checker recalcula h(M) independentemente da fonte, mantém P2 para O,
   soma O+M, um medidor e propagação de erros. Remover model -> ledger.
   Transferir regressões/smoke com CAS real para ledger, cuja dependência
   dev de model não retorna ao auditor. Não esconder arestas do guard.
2. Auditor fora da TCB de autorização, com decoder próprio de event/v1,
   encadeamento, épocas, assinatura e leitor CAS conforme specs. Dependências
   workspace: canon/crypto e air/core/model/obligation para checkers; rusqlite
   já fixado no workspace para ler SQLite. Nenhum pacote externo novo.
   Não usar índices, leitores ou tipos de eventos de logos-ledger.
3. Chave pública Ed25519 externa obrigatória em verify/export. SQLite aberto
   read-only/query_only, sob writer.lock existente, com snapshot que inclui
   WAL confirmado. Não criar diretório/lock nem depender de index.sqlite3.
   SQLite pode manter sidecars de coordenação. Não usar immutable.
4. Conferir estrutura de todos os eventos e integridade de referências CAS.
   Refuted exige igualdade dos bytes certificate/model sob os dois domínios,
   V de O conferido no CAS e Accepted do checker sob Budget registrado.
   Rechecar também testemunhas explícitas de consistência. Verified de prova
   fica explicitamente sem recheck até4.5; jamais relatar prova verificada
   por conferir assinatura. Unknown não é promoção nem promessa de reprodução.
   h(checker) confere artefato registrado, não executa binário não confiável.
5. Exportar audit-stream/v1: cabeçalho ASCII LOGOS-AUDIT/v1 seguido de LF,
   frames u32 big-endian e bytes canônicos integrais do evento. EOF só entre
   frames; limite16MiB por evento. verify aceita o stream sem SQLite no
   consumidor. Export autentica cadeia antes de publicar saída; não implica
   recheck lógico sem --cas. Novo transporte tem spec/vetor próprio, nenhum
   formato/hash/assinatura histórico muda.
6. Relatório distingue eventos autenticados, refutações rechecadas,
   consistências rechecadas, provas sem checker e falhas. Falha dá exit não
   zero; remoção de sufixo válido exige âncora externa da13.2. Não adicionar
   LRAT, kernel, fatos, política administrativa ou transparência nesta etapa.

## Consequências

A refatoração limitada toca a TCB e recebe mutações/regressões; decoders novos
do auditor não emitem autoridade e ficam fora dela. Testes de interoperabilidade
do appender ficam no crate ledger e invocam binário compilado separadamente,
sem dependência do auditor em ledger nem mesmo em dev. Fixture independente
assinada em Python e22 vetores históricos conferem o decoder do auditor.
O snapshot externo permanece relativo à chave/cabeça observadas, sem prova de
completude global, autoridade administrativa ou identidade do checker executado.
