# ADR 0031 — Build e isolamento do cake_lpr

- Estado: aceito para implementação da4.5, sob contrato A do ADR0030
- Data: 2026-10-16
- Base: d401bad

## Decisões

1. Fixar upstream a36874a8b750b43fe4b385b8ddbf5b033e46a3fa. Conservar
   assembly x86_64/ARMv8, basis_ffi.c e licença sem alterações em arquivo
   comprimido vendor/cake-lpr/source.tar.xz, com digests por arquivo.
   O assembly referencia HOL0ae7030/CakeMLfb377b4 conforme README upstream.
   Não reconstruir a prova HOL nesta etapa nem alegar verificação do C/OS.
2. Build estático Linux-musl com Zig0.15.1 distribuído oficialmente,
   downloads fixados por SHA-256, alvo explícito/baseline, sem build-id/debug e strip de símbolos (incluindo compiler_rt),
   ambiente/cache controlados. Identidades distintas para x86_64/aarch64.
   Script reproduzível guarda relatório do host e compara hash esperado.
   Binários de teste offline acompanham fontes/licença; testes não baixam
   compilador nem resolvem solver. Dois builds locais não fecham dois hosts.
3. Wrapper estrito Linux sem unsafe próprio. Acrescentar rustix=1.1.5,
   default-features=false, features std/fs, para memfd e seals seguros.
   Acrescentar rustix e suas dependências transitivas à allowlist do deny;
   medir seu custo na TCB. A implementação usa somente APIs seguras.
4. Bubblewrap e util-linux prlimit instalados pelo operador são parte da
   base operacional confiável (assim como loader/kernel), não checkers
   registrados pelo ledger. Caminhos absolutos, sem busca por PATH ou shell.
   Falhar se ausentes/incapazes de isolamento. Nenhuma execução alternativa
   sem sandbox. Requer namespaces de usuário habilitados no host/runner.
5. Identificar executável por SHA-256 cru fixado por arquitetura e pelo hash
   tipado logos/checker/v1. Abrir como arquivo regular sem seguir symlink,
   limitar leitura e conferir em cada execução. Copiar executável/CNF/prova
   para memfd selados contra escrita/crescimento/encolhimento; conferir hashes
   dos bytes copiados, sem reabrir pathname no processo. Bubblewrap recebe
   descritores e cria arquivos read-only privados com ro-bind-data. Troca
   concorrente do caminho não pode trocar os bytes já identificados.
6. prlimit impõe RLIMIT_AS/CPU/FSIZE/CORE. Bubblewrap cria explicitamente
   namespaces de rede/PID/usuário/IPC/UTS, sem compartilhar rede, limpa ambiente/remove capabilities,
   sem montagens do host: apenas executável estático e as duas entradas.
   PID namespace e die-with-parent garantem encerramento dos descendentes.
   O pai impõe prazo monotônico e mata/recolhe o launcher em expiração/erro.
   Saídas em arquivos limitados por FSIZE; não bloquear em pipes sem leitura.
7. Budget.size_bytes limita CNF+prova; mem_bytes/wall_ms precisam ser positivos
   para checker externo. Steps não representa instruções CakeML: não alegar
   equivalência de combustível. Rust usa seu Budget lógico e cake_lpr usa
   limites de SO, aplicados separadamente na auditoria. Cada checker recebe
   o Budget registrado; nenhum Unknown/erro operacional vira aceitação.
8. Aceite externo exige exit0, stdout exatamente s VERIFIED UNSAT seguido de
   LF e stderr vazio. Erros de prova reconhecidos são Rejected; exaustão
   reconhecida de memória/tempo/tamanho é Unknown. Sinais inesperados e
   saída/launcher desconhecidos são erro operacional.
   Parser pequeno da saída terá vetores, mutações e fuzz, sem interpretar
   sucesso parcial. Registry local não emite eventos nem possui autoridade.
9. Auditor mantém parser próprio e independência de ledger/kernel, correlaciona
   Verified camada1 à obrigação/CNF derivada antes do evento e exige os dois
   checkers reais. Configuração do cake_lpr vem do operador; ledger não escolhe
   executável. Sucesso informa contagens separadas de provas rechecadas/pendentes.
   Formatos event/v1 e audit-stream/v1 não mudam.

## Validação e limites

Comparação direta sobre200/600 casos4.4, sem filtro Rust antes do cake_lpr;
testes adversários P2/CAS/binário/limites/saída/isolamento/cleanup e auditor
antes/depois do export. Perfil RUP Rust é mais estrito que LPR upstream;
diferenças fora do corpus são explícitas e não ampliam o suporte Rust.
Mutações compiladas demonstram os testes TCB; fuzz oficial do parser de saída.
Medição da TCB inclui rustix e declara C/assembly e ferramentas nativas à parte.
Build e isolamento são engenharia adicional à verificação do checker CakeML.

Fontes: [cake_lpr](https://github.com/tanyongkiam/cake_lpr/tree/a36874a8b750b43fe4b385b8ddbf5b033e46a3fa),
[Zig0.15.1](https://ziglang.org/download/0.15.1/release-notes.html),
[rustix](https://docs.rs/rustix/1.1.5/rustix/fs/index.html).

## Descobertas durante implementação

- O binário upstream retorna exit0 também em erros de prova; saída completa
  reconhecida distingue aceite/rejeição. Testes com o binário real fixam isso.
- Zig compiler_rt conservava DWARF com diretório temporário apesar de -g0.
  -s no link removeu essa origem de não determinismo; rebuild limpo reproduziu
  o hash x86_64. Isso ainda não é demonstração em dois hosts.
- RLIMIT_FSIZE=64KiB impedia bubblewrap de copiar o executável com ro-bind-data.
  O teto rígido precisa admitir o maior snapshot; o pai limita saída a64KiB
  por observação e parser, com excesso transitório limitado por FSIZE.
  Não alegar teto físico exato64KiB para escrita assíncrona.

O corpus externo acrescenta hashes dos certificados (centenas de MB), além
dos h(CNF) existentes. O perfil test otimiza logos-crypto e sha2 em nível1,
com overflow-checks/debug-assertions explícitos, como logos-lrat desde4.4.
Produção não muda. Rodar as mutações e verificação completa nesse perfil.

Medição descartável no perfil dev:160MiB de SHA-256 em60.38s no host
carregado; o custo dos hashes justificou a otimização restrita do perfil test.

## Compatibilidade observada além do corpus obrigatório

Nos14 vetores manuais4.4, o caso duplicates é Accepted pelo Rust, mas
Rejected pelo cake_lpr: sua redução não trata literais repetidos como
singleton. O caso sparse é Accepted pelo Rust e Unknown(Memory) no cake_lpr
sob AS256MiB/heap64MiB/stack64MiB. Os outros12 concordaram. Resultados em
spec/vectors/cake-lpr/profile-observations.json; reprodução em
[etapa-4.5-perfis.py](../plans/etapa-4.5-perfis.py), incluindo teste real dos
vetores Rust. São controles de perfil/recursos, distintos dos200 benchmarks
históricos e600 corrupções que os critérios4.4/4.5 comparam diretamente.

Não normalizar CNF/prova silenciosamente para obter aceite externo: mudaria
os bytes de P2/IDs. O auditor requer Accepted dos dois e recusa esses casos
quando não há aceite de ambos; não ampliar o resultado de um checker ao
outro. Concordância no corpus200/600 não é equivalência universal de perfis.
A receita de namespace usa opções confirmadas também no
[bubblewrap0.8.0](https://github.com/containers/bubblewrap/blob/v0.8.0/bubblewrap.c);
execução remota Ubuntu/ARM continua pendente.

Sinais inesperados (p.ex. SIGSEGV e SIGKILL sem causa demonstrada) são erro
operacional, sem fallback para Memory. SIGXCPU/FSIZE têm causas específicas;
mensagens exatas do runtime identificam heap/stack. Probe real de SIGSEGV e
mutação dessa classificação demonstram a recusa explícita. Deadline é
conferido antes de aceitar um término observado depois do prazo.
