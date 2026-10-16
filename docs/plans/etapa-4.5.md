# Etapa 4.5 — integração do cake_lpr

Base: d401bad. Estado: implementada e verificada localmente; rebuild em segundo
host/CI/ARM pendentes. A adotada explicitamente pelo mantenedor
em2026-10-16, conforme [ADR0030](../adr/0030-contrato-do-registry-local-e-kernel.md).
ROADMAP4.5/5.4 ajustado antes do código.

## 1. Interfaces de entrada

- LratChecker::new/check_lrat/check_with_used da4.4, cnf/v1 e lrat-rup/v1;
  corpus offline SAT-2002 de200 CNFs/provas e600 corrupções reproduzíveis,
  14 vetores manuais e manifesto de hashes.
- Budget/CheckOutcome da0.6, Hash<Checker> no domínio logos/checker/v1,
  ExternalDigest para SHA-256 cru de binário e hashes tipados CNF/certificate.
- CAS de arquivos da0.5/3.5, eventos CheckerRegistered/Verified da3.1,
  auditor independente3.6 com rederivação O→CNF da4.3.
- Upstream cake_lpr: assembly/FFI e receitas disponíveis; revisão/toolchain,
  licença, binários por arquitetura e digests precisam ser fixados e medidos.
- Ausente: handler CERTIFY/registry do kernel5.4. Não implementar nesta etapa.

## 2. Interfaces de saída

Interfaces implementadas em logos-lrat:

- CakeRegistry::register(&Path) -> Result<CakeRegistry,CakeError> só aceita
  hash fixado por arquitetura. identity() -> Hash<CakeBinary> usa o domínio
  logos/checker/v1; SHA-256 cru do manifesto externo permanece separado.
- CakeRegistry::check_from_cas(&self, &Path, Hash<Cnf>, Hash<LratCertificate>,
  &Budget) -> Result<CheckOutcome,CakeError>; check_bytes também recebe snapshots
  e ambos os hashes. decode_cake_output é parser puro da saída completa.
  Identidade/isolamento/erros seguem ADR0031 e spec/cake-lpr.md.
- Executável fixado, script de build e registro externo no manifesto da TCB.
- Auditor capaz de rederivar CNF e rechecar toda prova camada1 com ambos os
  checkers; política local explícita para selecionar binário confiável.
  Nenhum binário do CAS é executado só por constar de um ledger assinado.
- Relatório de comparação e artefatos por host/arquitetura. Nada emite Verdict,
  claim, autorização ou eventos administrativos em nome do kernel ausente.

## 3. Tarefas em ordem

1. Resolver ADR0030; se A adotada, ajustar critérios4.5/5.4 no ROADMAP antes
   do código. Se B, replanejar dependências, mantendo implementação bloqueada.
2. Fixar revisão upstream/toolchain/arquitetura e revisar Makefile/FFI;
   documentar alcance de verificação CakeML e dependências nativas/isolamento.
   Arquivos: novo ADR técnico, spec/cake-lpr.md, ferramentas de build e manifesto.
   Teste: build limpo, digests independentes e comparação reproduzível por host.
3. Projetar execução segura compatível com forbid(unsafe_code): limites reais
   de memória/tempo, ausência de rede, ambiente limpo, saída limitada, cleanup
   de processos e arquivos. Falha de isolamento recusa a execução. Novas
   dependências precisam de ADR/deny; não decidir pela disponibilidade apenas.
   Fixar vínculo entre bytes hasheados/executados e entre bytes CAS/checados,
   incluindo trocas concorrentes. Testes adversários de substituição e limites.
4. Implementar registry e wrapper em logos-lrat; listar fontes no manifesto.
   Confirmar P2 e leitura de objetos tipados; sucesso do processo sozinho não
   significa prova aceita. Saída malformada, erro, kill e timeout não viram
   Accepted/Refuted. Novos parsers/decoders exigem fuzz e vetores apropriados.
5. Rodar comparação direta do cake_lpr real e Rust sobre200 provas válidas e
   600 corrupções da4.4. Arquivos: harness/corpus e scripts CI. Registrar cada
   divergência; não mascarar com filtro Rust ou invocar fake checker para
   satisfazer esse critério. Casos de limites usam controles positivos.
6. Estender auditor/eventos internos/CLI para correlacionar Verified a O/CNF/
   certificado/Budget/identidade e rechecar com ambos, mantendo a independência.
   Fixture de ledger de exemplo com provas reais e export sem banco; adulterar
   certificado/CNF/binário/CAS. Cada Verified de camada1 deve ser rechecado.
7. Ligar build/smokes/comparação/auditor aos workflows e ci/verify.sh; atualizar
   AGENTS.md se a verificação mudar. Demonstrar por mutação os testes da TCB,
   fuzz oficial, medição TCB e verificação completa com autoteste.
8. Atualizar STATUS/diário/roadmap apenas com evidências observadas; commit
   local após VERIFY_OK. Rebuild em segundo host permanece pendente se não
   observado. Nenhum push autorizado.

## 4. Mapeamento dos critérios

| Critério original | Demonstração necessária |
| --- | --- |
| Mesmo hash em duas máquinas | Script de rebuild limpo com revisão/toolchain fixadas e relatórios de dois hosts distintos, para o mesmo alvo. ARM tem identidade própria; não exigir hashes iguais entre ISAs. |
| Troca de binário recusada pelo kernel | A adotada: testes reais no registry/wrapper4.5 demonstrados; critério integrado no kernel preservado em5.4, sem alegar que ele existe agora. |
| Concordância com suíte4.4 | Ambos os checkers reais executados sobre os mesmos200 positivos e600 negativos; relatório sem divergências, falhas ou skips encobertos. |
| Auditor100% + certificado trocado | Ledger de exemplo autenticado com provas reais; contagem de Verified camada1 igual à de recheques por ambos, antes/depois de export; certificado adulterado recusado. |

## 5. Riscos e decisões em aberto

ADR0030 resolveu o bloqueio de contrato com adoção explícita de A. Rebuild em dois hosts é evidência externa
pendente, não licença para reduzir o critério. O suporte LRAT/LPR upstream
é mais amplo que o perfil RUP estrito; limites/sintaxe/caudas precisam de
controles. Ordem e bytes canônicos da CNF determinam os IDs originais.

rlimit e isolamento de rede exigem mecanismo real compatível com as regras
TCB; não usar std::os::unix::process::CommandExt::pre_exec com unsafe no crate.
Não executar via pathname mutável depois de conferir outro objeto; os mesmos
bytes identificados precisam chegar ao processo. Distinguir falta de recurso,
rejeição lógica e falha operacional; não emitir aceite a partir de stdout
parcial, exit0 genérico ou artefato não registrado.

Não modificar cake_lpr, abrir construtor selado de O ou introduzir produtor
SAT4.6/used_premises4.7/kernel5.4. Cadastro local não autentica autoridade
administrativa de um CheckerRegistered. Novos formatos exigem spec/versão/
vetores; alterações técnicas/dependências têm ADR próprio antes do código.

## 6. Estimativa

M, conforme ROADMAP. Build externo, isolamento e auditor exigem validação
além da biblioteca Rust. Segundo host é requisito de evidência e pode deixar
a etapa com pendência remota após a implementação local.

## 7. Evidência da análise inicial

Inspecionados STATUS/ROADMAP/AGENTS, kernel esqueleto, eventos existentes,
auditor e manifesto TCB em d401bad; upstream consultado em2026-10-16.
Na análise inicial não houve alteração de código/formato/critério nem build/
comparação/fuzz/full. O contrato foi resolvido posteriormente pela adoção
explícita de A; a implementação e a verificação estão registradas na seção8.

## 8. Evidência da implementação e verificação

- A adotada explicitamente; ROADMAP ajustado antes do código. ADR0031 fixa
  build, identidade, dependências e isolamento. C/source upstream preservados.
- Rebuild inicial conservava DWARF de compiler_rt com diretórios temporários;
  -s resolveu e reconstrução limpa x86_64 reproduziu hash48760ef54cd4…
  (dois builds locais, não dois hosts). ARM cross-build fixado; nativo remoto pendente.
- Protocolo upstream retorna0 também em erros. Primeiros testes de sandbox
  falharam porque FSIZE64KiB bloqueava a cópia ro-bind-data; teto ajustado ao
  maior snapshot com saída monitorada/decodificada até64KiB. Cinco testes
  iniciais passaram; versão atual tem3 unitários de sandbox+6 públicos.
- Primeiro corpus comparativo falhou em918.13s: enquanto ele usava a identidade
  inicial, outra instalação substituiu o artefato compartilhado pelo build
  com strip. O checker recusou corretamente o hash novo; foi interferência do
  harness. Instalação do corpus agora usa diretório exclusivo com cleanup.
  Repetir todo o corpus com a identidade final e perfil test final.
- Hash descartável160MiB no perfil dev:60.38s; profile test de crypto/sha2 em
  nível1, overflow/debug explícitos.9 testes atuais passaram em1.42s+2.12s.
- Auditor real3/3 com ambos e export sem banco passou; corrupções assinadas
  e substituição de binário recusadas. Dez mutantes compilados capturados no
  primeiro perfil; repetir após mudança do perfil test. Logs target/cake-mutations/.
- Fuzz oficial cake_output:2756198 execuções em61s, exit0 sem crash.
  target/fuzz-4.5-cake-output.log/.exit. Verificação completa ainda não rodada.

- Perfil final:9 testes passaram em1.05s+1.36s. As dez mutações foram
  repetidas no perfil final e detectadas por assertion failure real, sem
  contar erro de compilação. Apenas capabilities/causas de sinal e controles
  adicionais do auditor foram reforçados depois, cobertos pela full em curso.
- Fuzz final do decoder inalterado: 2821026 execuções/61s,
  exit0 sem crash, target/fuzz-4.5-cake-output-final.log/.exit.
- Medição intermediária: lrat856 (+439 próprios);11540+747130=758670 Rust, build-only64508.
  O aumento de runtime641931 é rustix49394/linux-raw-sys478851/libc113368/
  errno318. Cota superior inclui fontes inteiros/todas as plataformas, não
  contagem do código compilado. C/FFI/musl/assembly e isolamento nativo não
  estão nesse total. Dois hashes externos no manifesto, sem provar C/OS.
- ci/check-audit.sh:17 eventos,4/4 refutações,1 consistência,6/6 CNFs,
  3/3 LRAT por cada checker antes/depois do export sem banco;1 prova de outra
  rota continua explicitamente unchecked. Independência passou.

- Controle adicional além do corpus200/600:14 vetores manuais Rust passaram;
  observação nativa cake_lpr concordou em12, rejeitou duplicates (Rust aceita)
  e esgotou memória em sparse (Rust aceita sob o limite). Receita executada
  em etapa-4.5-perfis.py e resultados em spec/vectors/cake-lpr/profile-observations.json.
  Não há equivalência universal; auditor exige Accepted dos dois. Nenhum byte
  de CNF foi normalizado para esconder a diferença. FFI C354 LOC e assembly
  x86_64/ARM56341 LOC no total, medidos por tokei separadamente dos totais Rust;
  não somar duas arquiteturas como se ambas executassem na mesma verificação.

- Verificação completa em curso: Rust/Clippy passaram. Corpus200/600 passou
  em407.21s com ambos os checkers reais;12 regressões RUP/14 vetores e
  diferencial exaustivo anteriores preservados. Critérios locais de registry,
  concordância no corpus obrigatório e auditor demonstrados; primeiro critério
  continua desmarcado porque segundo host/CI não foram observados.

- Primeira full interrompida explicitamente antes do fim (exit-15): revisão
  encontrou conversão ampla de sinais128..255 em Unknown(Memory). Corrigida
  para erro operacional, salvo sinais conhecidos de CPU/FSIZE; novo probe
  SIGSEGV exige erro. Deadline é conferido antes de aceitar término observado.
  Primeiro corpus200/600 já havia passado, crash10k/24 também; índices/Python/
  selftest não concluíram nessa rodada. Logs target/verify-4.5-attempt1.log/.exit.
  Repetir a verificação completa sobre o código corrigido antes do commit.

- Correção final:4 unitários de sandbox+6 públicos passaram (1.15s+1.85s),
  com SIGSEGV real recusado como erro. Onze mutações compiladas capturadas
  após correção/perfil final, sem contar erro de receita/compilação. Receita
  de timeout precisou de contexto mais específico após separar SIGXCPU;
  validação de todas as receitas precede agora as compilações. Full reiniciada
  sobre essa fonte com RUST_TEST_NOCAPTURE=1 para progresso visível.

- Rustdoc do wrapper esclarecido: somente exaustão reconhecida retorna Unknown;
  falha de startup/isolamento/I/O continua erro operacional, inclusive quando
  um limite inviabiliza o launcher sem diagnóstico confiável. Sem mudança de
  comportamento nessa correção de texto. Nenhuma nova hipótese de causa.

- Verificação final integral sobre a correção: ci/verify.sh --selftest com
  RUST_TEST_NOCAPTURE=1 terminou em VERIFY_OK, exit0 persistido em
  target/verify-4.5-final.log/.exit. Corpus real200/600 concordou em380.65s;
  todos os25 testes LRAT passaram, incluindo os10 novos. Auditor e export
  rechecaram3/3 provas com ambos; workspace Rust/Clippy/smokes/TCB/manifesto/
  independência/supply-chain/vetores passaram.227 Python em78.33s e25 testes
  TCB em1.41s. Autoteste bloqueou todos os casos pelos diagnósticos corretos,
  sem avisos de tar; cargo-deny manteve avisos de licenças já conhecidos.
  Crash10k/24 SIGKILLs/9998 ACKs em54.06s sem perder append reconhecido;
  rebuild100k/cinco tabelas exatas;1M/20000 consultas p99=11389ns,
  cold rebuild243024ms excluído da latência. A tentativa interrompida não foi
  contada como verificação completa bem-sucedida.
- Medição final: lrat865 (+448 próprios);11549+747130=758679 Rust de correção,
  build-only64508. Relatórios target/tcb-4.5-final.json/.md; a cota superior
  runtime e os componentes nativos separados seguem as ressalvas acima.
  Rebuild limpo x86_64 final reproduziu48760ef54cd4… no host local. O primeiro
  critério continua desmarcado: segundo host/CI/execução ARM não observados;
  fuzz local61s não fecha ≥1h. Kernel5.4 preserva o teste integrado do ADR0030.
  STATUS aponta4.6 como próxima implementação; sem produtor SAT antecipado.
