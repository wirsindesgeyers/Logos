# Farm de provadores10.2 (não confiável)

`logos_provers.farm.ProverFarm(client, uploader, Sandbox(encoder,solver),
 timeout=30, cache_size=128).run(O,hO,V)` é async. Usar no mesmo event loop.
O/V são bytes públicos, limitados a1MiB cada. h(O) usa domínio obligation/v1;
V passa pela validação existente e precisa corresponder ao h(V) de O. O
chamador fornece V público; a farm não abre CAS/ledger privados nem chaves.

Quatro rotas independentes iniciam juntas: Z3 refute, finder refute, CaDiCaL
via encoder de apoio4.6 e Z3 witness. O primeiro candidato que o kernel aceita
vence; ausência limitada, erro, timeout e rejeição não são veredictos. Rotas
restantes de prova/refutação são canceladas/recolhidas após aceitação. A busca
de consistência permanece até terminar ou expirar o prazo global. WITNESS é
submetido se houver modelo; quando Refuted já encerrou a proposta, a recusa
remota é registrada em diagnostics. Falta de modelo não prova inconsistência.

`FarmResult.result` é CheckResult do kernel ou None, `witness` é a resposta
WITNESS ou None, `diagnostics` preserva falhas/inconclusivos. `verdict_elapsed`
mede até o primeiro aceite; `elapsed` inclui a busca independente de testemunha.
Resultados do cliente não são certificados portáteis nem autenticação de
assinatura: para isso usar auditor e raiz de confiança explícita.

Cache LRU volátil por h(O), limitado a cache_size (0..1024) e32MiB totais de
payload. Somente candidatos de rotas aceitas são guardados, sem resultado de
checker, credencial, ausência ou witness. Cada hit é novamente submetido ao
kernel; rejeição elimina entrada e deixa demais rotas concorrerem. Repropostas
exigem novo append/check. Um reinício elimina todo cache.

Workers Linux usam bwrap sem fallback: user/PID/network/IPC namespaces,
filesystem tmpfs, /usr/lib e runtime Python/Z3/duas bibliotecas do projeto e
executáveis explícitos somente leitura; /tmp privado. Não montar home, /etc,
ledger, socket, staging ou chaves. Supervisor usa apenas cliente10.1 e uploader
para integrar ao kernel. Cada processo recebe ambiente limpo e grupo próprio;
prlimit AS1GiB, FSIZE128MiB, NPROC64 e CORE0; produtores SAT mantêm limites
menores internos. Não são limites agregados de RSS/disco entre processos.
Stdout até32MiB e stderr até64KiB, coletados incrementalmente; tempo e
cancelamento matam/recolhem o processo. bwrap mata seu namespace PID quando
encerra, inclusive descendentes que mudaram de grupo. Falha de isolamento é
erro da rota, nunca execução direta no host.

Prazo monotônico global inclui validação síncrona limitada, produção, upload e
RPC; Python não preempta trabalho síncrono. Timeout/cancelamento de RPC pode
seguir um append já persistido. Não há retry automático: diagnóstico temporal
é inconclusivo para a chamada interrompida; recuperar estado pelo ledger antes
de decidir nova submissão. Erros remotos/I/O/protocolo de CERTIFY/REFUTE
propagam para evitar repetição ambígua. Os arquivos de staging publicados
seguem a propriedade/limpeza do contrato10.1.

`ci/check-farm.sh` reproduz500 obrigações distintas,250 tautologias e250
refutáveis, dez famílias proposicionais sem premissas, com checkers reais e
kernel release. Nenhuma Fact/Policy/ASSUME é antecipada. V publicado pela
fixture Rust de teste, que provisiona a chave sem entregá-la ao Python.
A medição exclui compilação/startup/PROPOSE e inclui produção até RPC aceito;
relatório também mede o tempo total com witness. Conferir500 eventos reais,
CheckOutcome Accepted e h(O), cache rechecado e cache corrompido rejeitado
antes do fallback. Evidência em target/farm-report.json. `--count` menor é
somente desenvolvimento e não encerra o critério500. CI remota é pendência
separada. Não cobre camada2, MCP, ações ou MEM.
