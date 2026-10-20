# Etapa10.1 — Cliente Python do protocolo

Estado: concluída localmente; CI remota pendente; base2c22e1e; ADR0043. Escopo só10.1, sem MCP/farm.

## 1. Entradas

canon Python0.3, AIR/proposal1.6, wire/kernel-protocol/v1 (5.2), handlers5.3–5.5,
CAS/staging existentes. Dependências do perfil de pesquisa presentes.

## 2. Saídas

Modelos Pydantic para sete requests; encode/decode e resultados conhecidos.
KernelClient com propose/certify/refute/witness/assume/query e subscribe async.
StagingUploader.put(bytes,domain) retorna referência candidata; nunca autoridade.

## 3. Tarefas → testes

1. Dependência Pydantic2.14.1/uv.lock, modelos, codec estrito: vetores históricos.
2. Transporte: framing, prazo, ID, erro; servidor adversário em socket Unix.
3. Subscribe: sequência/links/cursor completo, EOF parcial e cancelamento.
4. Upload: identidade de domínio, modo/UID, symlink, colisão, limites/publicação.
5. Fixture Rust de provisionamento e executável real: todas as sete mensagens,
   PROPOSE/CERTIFY/WITNESS/REFUTE aceitos, ASSUME Unavailable, Queries/Subscribe.
6. Ruff/mypy/pytest direcionados, CI/full verify/selftest, STATUS e commit/push.

## 4. Critério → artefato

Todas as mensagens v1 exercitadas pelo cliente contra logos-kernel real.
ASSUME tem request completo e erro atual explícito, sem alegar handler5.7.
O cliente cobre o protocolo existente integralmente; nenhum novo comando de
fase futura é inventado. Não fecha5R.1/MCP nem10.2/farm por antecipação.

## 5. Decisões e riscos

ADR0043: conexão por operação, sem retry, prazo absoluto e eventos tainted;
Pydantic estrito/congelado. Mesmo UID não isola processos (5.11 futura).
Upload síncrono é limitado; cancelamento de pedido pode deixar append/upload
sem resposta, tratado como ambíguo. Python não lê/escreve material privado.

## 6. Estimativa

P com decisões explícitas; Rust de apoio só fixture de testes fora da TCB.

## Verificação

Direcionados:21 testes de vetores/transporte/staging passaram; integração
real nas sete mensagens passou em62.90s. Ruff check/format e mypy de todo
Python passaram; Clippy workspace/all-targets passou. Import do cliente em
ambiente isolado sem dependências dev: CLIENT_RUNTIME_ONLY_OK.

A primeira integração excedeu o prazo de inicialização20s da fixture,
antes de o socket estar pronto; sem erro lógico do kernel. Prazo somente da
fixture aumentado para120s e reexecução passou. Ajustes de lint/formatação e
tipo AsyncGenerator concluídos. Não são execuções completas de verificação.

Sem fontes/formatos/dependências Rust da TCB alterados; único Rust novo é o
provisionador de fixture em examples. Não há parser novo na TCB nem requisito
novo de fuzz. A validação de mensagens Python é não confiável por construção.

Resultado final abaixo; critério encerrado somente após VERIFY_OK.


## Fechamento local

ci/verify.sh --selftest: VERIFY_OK/exit0 em1838.8s;
543 Rust/0 ignorados; Python: 274 passed in 150.89s (0:02:30), 25 passed in 1.39s.
464 entradas monitoradas permaneceram idênticas durante a execução.
Todos os fontes Rust em crates/*/src, manifests/lock/deny da TCB mantiveram
bytes idênticos ao HEAD2c22e1e; TCB15131+747130=762261, build-only64508.
Único Rust novo: provisionador de fixture fora da TCB. Sem formato wire novo.

Vinte e dois testes novos:21 vetores/transporte/staging e1 cenário integrado
com sete mensagens no executável real. ASSUME recusa3/Unavailable explicitamente;
nenhum handler futuro é alegado. Produto fora da TCB: cliente não autentica
assinaturas nem cria Claim/token/autorização. Sem novo fuzz/mutação de TCB,
pois seus fontes não mudaram. Recibos/hashes em etapa-10.1-evidencias.json
e etapa-10.1-inputs.json; log local target/verify-10.1.log.

CI/ARM/noturno ainda não consultados. Próximo: plano10.2 (prover farm de pesquisa),
sem antecipar MCP5R.1 antes da farm.
