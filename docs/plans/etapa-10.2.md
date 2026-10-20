# Etapa10.2 — farm de pesquisa

1. Entradas: cliente10.1; produtores2.4/2.5/4.6; O e h(O), V público
   e h(V) já referido em O. Encoder prover_bridge existente (ADR0032).
2. Saídas: ProverFarm.run(O,hO,V) async → resultado do checker, evidências
   de consistência e diagnósticos. Candidatos nunca são credenciais.
3. Implementar worker isolado Linux/bwrap, portfólio paralelo Z3/finder/SAT,
   testemunha independente, deadline e cancelamento com recolhimento; cache
   limitado em memória por h(O), sempre rechecado; integração cliente/staging.
   Arquivos py/logos_provers/farm*.py e testes adversariais/integração.
4. Critério: corpus reproduzível de500 O mistas distintas, p50 incluindo
   geração/checker, resultados ligados a eventos reais no ledger; script
   ci/check-farm.sh. Cache adulterado/rejeição/timeout não autorizam claims.
5. Decisões: ADR0044; V público explícito, nenhum acesso ao CAS privado.
   Reutilizar composição do grounder existente fora da autoridade do kernel,
   conforme ADR0032; não criar segundo encoder nem mudar formatos.
   Workers sem rede/host/credenciais, runtime e executáveis explicitamente
   montados somente leitura; sem fallback se isolamento falhar.
6. Tamanho M mantido. Sem MCP, camada2, ações, novo handler ou nova TCB.

Verificação completa e fechamento local registrados ao fim deste plano.

## Desenvolvimento e limites da evidência

- Integração de desenvolvimento com2 casos passou, incluindo cache rechecado
  e cache adulterado rejeitado antes do fallback. Não conta como corpus500.
- Nove testes cobrem rejeição/Unknown/cache, prazo/cancelamento, isolamento
  real (arquivo/ambiente/rede/mount somente leitura), produtores reais e
  stdout malformado/limite. Uma fixture inicial entregava bytes em vez de
  ObjectRef e outra não simulava Unknown; corrigidas após testes vermelhos.
- Primeiro sandbox SAT falhou porque o hard FSIZE externo64MiB era menor
  que a saída limitada do encoder (64MiB+4096). Elevado a128MiB; não houve
  fallback para execução fora do sandbox. SAT/prova/modelo reais passaram.
- Conferência inicial do benchmark buscava h(O) no nível errado do evento;
  corrigida para CheckRecord.hO e CheckOutcome Accepted. Dois casos passaram.
- Medição final usa kernel release (debug custava6–9s por veredicto por hash
  dos executáveis). Código runtime/formatos/dependências da TCB inalterados;
  somente fixture Rust em examples aceita V público opcional para o corpus.
- Corpus definitivo usa dez famílias proposicionais,250 válidas/250
  refutáveis,500 sintaxes distintas e nenhuma premissa autenticada fictícia.
  Não demonstra desempenho de quantificadores/modelos grandes ou camada2.

## Fechamento local

ci/verify.sh --selftest: VERIFY_OK/exit0 em2803.23s.
Corpus500/500 distintos,250 Verified/250 Refuted, p50=1.274575s até o checker.
Rust543/0 ignorados; Python: ['283 passed in 145.74s (0:02:25)', '25 passed in 1.55s'].
Cache rechecado e corrompido rejeitado antes do fallback; todos500 eventos com Accepted/h(O).
472 entradas monitoradas inalteradas. TCB runtime Rust idêntica à base3f3fc79.
Evidências/corpus/inputs em etapa-10.2-*.json. CI remota/ARM/noturno pendentes.
