# Etapa 5.4 — CERTIFY e registry de checkers

- Estado: implementada e verificada localmente; base `d8f8fcc`; CI remota pendente.
- Escopo: camada 1 LRAT e REFUTE/modelo; sem Alethe, avaliação direta,
  WITNESS5.5, política de tentativas ou bloqueio de re-proposta5.6.

## 1. Entradas

PROPOSE5.3, CAS e replay autenticado do appender; ModelChecker2.3;
ground_for_check/clausify4.2–4.3, LratChecker4.4, CakeRegistry4.5 e
used_premises4.7 existem. Nenhuma dependência ausente encontrada.
Transportes CERTIFY/REFUTE e imports já existem na5.2.

## 2. Saídas

`CheckerRegistry` associa formato e hash a implementação local conhecida;
registro/revogação vêm de eventos administrativos conferidos contra a chave
local. `CertificationHandlers` implementa PROPOSE, CERTIFY e REFUTE.
Recuperação de `Claim<Pending>` fica privada em Transitions e exige replay,
par Proposed/ObligationCreated, época atual e ausência de conclusão anterior.
Resposta `check-result/v1` identifica evento e resultado; não recebe outcome
do cliente. Rejected retorna Pending via CertificateRejected; Unknown mantém
tentativa registrada e permite nova tentativa, sem autorizar.

## 3. Tarefas e demonstrações

1. ADR0041 e extensão `certificate-rejected-body/v2`, tag13; preservar todos
   os bytes históricos, decoder independente do auditor e referências CAS.
   Testes de outcome, aridade/versão, vetores independentes e fuzz event_decode.
2. Registry local e histórico ativo/revogado, sem carregar código indicado pelo
   agente/CAS. Identidades de checkers Rust são os bytes do executável atual;
   cake usa hash fixado/execução por snapshot da4.5. Testes de desconhecido,
   revogado e substituição após cadastro, inclusive pelo socket CERTIFY.
3. Recuperação privada e validação contextual: replay autenticado, h(O),
   proposta, época e citações; rederivar O com snapshot atual antes do check.
   Testes de reabertura, mistura de contextos, época e conclusão duplicada.
4. CERTIFY rederiva grounding/CNF dos bytes CAS de O; recusa DomainSource
   opaco (ADR0026). Checker Rust fornece fecho para used, inclusive quando
   cake participa; não aceitar sem ambos os resultados requeridos.
   Accepted registra Verified com ConsistencyUnknown explícito; WITNESS fica5.5.
5. REFUTE carrega bytes CAS exatos e usa ModelChecker ativo. Nenhuma rejeição
   de prova/modelo vira Refuted; Unknown registra Budget e limites.
6. Integração real STATE15–16/refutação, ataques P3/P4 e mutações compiladas;
   atualizar specs, vetores, manifesto, métricas TCB e documentação.
7. Fuzz direcionado, `ci/verify.sh --selftest`, STATUS e commit coerente;
   CI remota e fuzz noturno só marcados depois de observados.

## 4. Critérios → evidência

| Critério | Evidência prevista |
| --- | --- |
| P3/P4 | testes de outcomes/registro/identidades/contextos e revisão dos únicos chamadores de accept/refute; mutações compiladas |
| STATE15–16 e variante | integração PROPOSE→CERTIFY→Verified e PROPOSE→REFUTE→Refuted pelo transporte real, replay e auditor |
| ADR0030 | registro cake conhecido, substituição pelo mesmo nome, CERTIFY recusado sem Verified; desconhecido/revogado recusados |

## 5. Decisões e riscos

ADR0041 fixa corpo novo v2 sem reescrever event/v1. Recuperação não abre
construtor público de Claim; histórico assinado não substitui conferência
lógica. Revogação é terminal nesta etapa. Custo de replay de recuperação
será limitado e explícito; otimização não pode confiar no cache em disco.
Falha operacional do cake não vira Rejected/Unknown inventado.

## 6. Estimativa

Mantém M: integração de TCB, novo corpo serializado, auditor independente,
histórico e testes adversariais; nenhuma ampliação de fase.

## 7. Revisão dos caminhos de emissão

Os únicos chamadores de Transitions::accept/refute em produção estão no
finish privado de certify.rs. CERTIFY chega a ele depois de registro ativo,
recuperação privada, rederivação contextual, checker LRAT e projeção used.
No formato cake, o Rust usado para o fecho também precisa estar ativo e
aceitar; cake deve aceitar e revalidar o executável conhecido em cada chamada.
REFUTE obtém outcome exclusivamente do ModelChecker ativo. O parâmetro model
separa as duas chamadas privadas; o cliente não fornece outcome nem Claim.

finish só chama accept/refute no ramo Accepted; Rejected e Unknown seguem
transições diferentes. Transitions confere novamente sessão/época/h(O)/outcome;
o catálogo confere os desfechos antes de assinar/persistir. Resposta só segue
append confirmado. Used vem do relatório privado ligado a O/CNF/certificado,
nunca da cópia de cites. Construção pública/retag de Claim continuam bloqueados
pelos testes compile-fail existentes. Esta revisão e os testes gerados são
evidência local; formalização mecanizada permanece na Fase12.

## 8. Verificação e ocorrências

- Onze integrações novas em tests/certify.rs; teste do processo real ampliado
  para PROPOSE/CERTIFY/REFUTE via staging, com identidade do executável.
- Vetores:23 eventos históricos preservados e1 CertificateRejected adicionado;
  quatro valores check-result/v1 novos, sem modificar requests/propose-result.
- Dez mutações detectadas por asserção, cada uma precedida de controle positivo
  exit0; cinco fontes restaurados byte a byte. Recibo final
  target/mutations-5.4/report.json; reprodução isolada com
  `rtk proxy python3 docs/plans/etapa-5.4-mutacoes.py`.
  Mutações reexecutadas com controle positivo por caso após corrigir a fixture
  de enumeração. A primeira rodada tinha nove recusas por asserção e uma falha
  de construção da fixture, que não conta como mutante detectado. Artefatos
  anteriores arquivados em target/mutations-5.4-fixture-invalid/.
- Fuzz oficial: event_decode148179/61s e kernel_message639796/61s, exit0,
  sem crash;152,79s incluindo build. Logs históricos preservados; recibo
  target/fuzz-5.4/report.json. Não satisfaz duração/execução remota noturna.
- TCB:14788 próprias+747130 runtime=761918; build-only64508. Kernel2238,
  ledger3819; aumento próprio672. Sem pacote externo novo.
- Corrigidos durante desenvolvimento: import não usado, clones de Budget Copy
  e localização do módulo de testes no Clippy. Teste antigo do processo
  esperava somente Genesis no head; passou a conferir os dois registros reais
  e o fluxo completo. Primeira chamada isolada do Ruff usou configuração da
  raiz; configuração py/pyproject.toml reaplicada, a mesma da CI.
- Primeira execução completa parou em246,22s/exit101: fixture REFUTE enumerada
  usava domínio vazio, que FiniteModel::new recusa (InvalidDomains). Corrigida
  para enumeração unitária/modelo válido;11 integrações passaram em28,91s.
  Recibo/log preservados em target/verify-5.4-fixture.*. Nenhum critério fechado
  com essa execução; repetir full depois dos controles positivos de mutação.

- Segunda execução completa iniciada após11 integrações positivas e10/10
  mutações compiladas detectadas com controles positivos. Nenhum fonte de
  produção mudou ao corrigir a fixture; fuzz anterior continua aplicável.

- Segunda verificação completa:286,21s/exit101;11 integrações novas passaram,
  mas o replay de vetores do ledger ainda fixava a última sequência em22.
  Atualizada para23 (24 vetores); busca global não encontrou outra contagem
  antiga ligada ao catálogo. Recibo/log em target/verify-5.4-catalog-count.*.
  Fontes de produção inalterados; nenhuma falha marcada como sucesso.

## 9. Fechamento local

- [x] P3/P4: integrações geradas, revisão dos caminhos privados e10 mutações
  compiladas detectadas, cada uma com controle positivo exit0.
- [x] STATE15–16 e refutação por transporte/processo reais, persistência,
  citabilidade e auditor independente com rederivação CNF e Rust/cake.
- [x] ADR0030: cake conhecido aceita; substituição pelo mesmo nome recusa
  CERTIFY sem Verified. Desconhecido, inativo e assinatura administrativa
  forjada também recusados.
- [x] Terceira execução ci/verify.sh --selftest: VERIFY_OK/exit0 em1468.7s;
  455 entradas de runtime/spec/fuzz/CI/TCB inalteradas.
  Rust:533 aprovados/0 ignorados.

Recibos versionados: etapa-5.4-evidencias.json, etapa-5.4-inputs.json e
etapa-5.4-mutacoes.json. Log local target/verify-5.4.log, SHA-256
0848c50e02862c422073a70e34abcaeae2e0ff65e18583e9ad52d4294cf31000.
As duas execuções que falharam e a primeira mutação inválida não contam
como sucesso; suas causas/artefatos estão preservados acima. CI remota,
ARM e duração noturna permanecem pendentes. Próxima etapa principal:5.5.

Aviso não fatal na execução final: `tar: .: file changed as we read it`
na cópia temporária do selftest. Todos os controles concluíram com o motivo
esperado; comando final exit0/VERIFY_OK e455 entradas com hashes inalterados.
