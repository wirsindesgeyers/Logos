# cake-lpr-wrapper/v1

Contrato local4.5 sob ADR0030/A e ADR0031. Não altera event/v1,
obligation-body/v2, audit-stream/v1 ou lrat-rup/v1.

## Identidade e entrada

CakeRegistry::register(path) só aceita o ELF fixado de vendor/cake-lpr/lock.json
para a arquitetura compilada. SHA-256 cru identifica o artefato externo;
Hash<CakeBinary> é SHA-256(logos/checker/v1\0 || bytes), usado pelo evento.
Não aceitar hash escolhido pelo ledger como allowlist. O registro local não
assina/emite CheckerRegistered nem autentica autoridade administrativa.

check_from_cas usa namespaces cnf-v1 e certificate-v1; check_bytes permite
snapshots já obtidos do CAS, conferindo ambos os hashes tipados. Em cada check,
o executável é aberto novamente sem seguir symlink final, lido com limite e
conferido contra o build fixado e o cadastro anterior. Arquivos especiais são
recusados. Nenhum binário CAS é executado automaticamente.

Binário/CNF/prova são copiados para memfd selados WRITE/GROW/SHRINK/SEAL.
O processo recebe arquivos privados read-only copiados desses descritores,
nunca reabre path do CAS. Trocar o pathname após snapshot não troca os bytes
executados. Uma mudança antes do snapshot é detectada pelo hash.

## Isolamento e recursos

Linux, /usr/bin/prlimit e /usr/bin/bwrap confiáveis, namespaces de usuário/
PID/rede/IPC/UTS habilitados. Ambiente limpo/capabilities removidas; root privado read-only contendo
só checker estático, CNF e prova. Não montar host/proc/credenciais. Sem shell,
PATH ou fallback não isolado. Novas redes não recebem interfaces do host.

prlimit impõe RLIMIT_AS=mem_bytes, CPU soft=ceil(wall_ms/1000)/hard=soft+1, CORE=0 e FSIZE.
FSIZE admite o maior snapshot (ro-bind-data precisa copiá-lo), com mínimo64KiB.
O pai observa stdout/stderr e interrompe acima de64KiB; cada arquivo tem ainda
o teto rígido FSIZE. Há possível excesso entre duas sondagens, limitado por
FSIZE. Parser só lê no máximo65537 bytes e nunca aceita saída excessiva.
O pai usa relógio monotônico para o prazo do subprocesso e kill/wait;
namespace PID/die-with-parent encerram descendentes. O prazo não inclui
hash/leitura/snapshot no processo pai. Seu RSS não tem limite físico novo.

size_bytes limita soma CNF+prova antes do subprocesso. mem_bytes e wall_ms
positivos são necessários; Steps não mede instruções do programa externo.
Heap e stack CakeML recebem cada um floor(mem_bytes/3MiB), mínimo1MiB;
RLIMIT_AS continua o teto real do processo. O checker Rust usa seu medidor
lógico separadamente; auditor fornece o Budget registrado aos dois.
Exaustão reconhecida retorna Unknown; falha de setup/I/O/identidade é erro
operacional. Nenhum caso vira Verified/Refuted neste wrapper.

## Protocolo de saída completo

- Accepted somente exit0 + stdout exatamente `s VERIFIED UNSAT` seguido de
  LF + stderr vazio. Exit0 sozinho não é sucesso: upstream também usa0 em erros.
- Rejected para exit0/stdout vazio e erro de prova reconhecido do binário
  fixado: ausência de cláusula vazia ou Checking failed at line com LF final.
- Heap/stack esgotado retorna Unknown(Memory); prazo retorna Unknown(Time).
  Saída excessiva detectada pelo pai retorna Unknown(Size).
- CPU (SIGXCPU) e saída (SIGXFSZ) têm Unknown específicos. Outros sinais,
  incluindo SIGSEGV/SIGKILL sem causa demonstrada, são erro operacional;
  não presumir falta de memória. Deadline prevalece antes de aceitar término
  observado. Status/saída desconhecidos também são erro, nunca aceite parcial.

O parser puro decode_cake_output tem testes negativos e fuzz cake_output.
RUP Rust e LPR completo diferem em gramática/IDs/deleções/caudas. Comparar
a suíte4.4 diretamente, sem pré-filtrar a execução do cake_lpr pelo Rust.

## Auditor

verify_with_cake(..., Some(registry)) rederiva O→CNF e associa cada Verified
de rota LRAT ao h(O)/h(CNF) anterior. Exige identidade permitida e Accepted
em ambos os checkers com os mesmos bytes/Budget. Sem configuração do operador,
Verified LRAT falha. Outros tipos de prova permanecem explicitamente contados
como unchecked; não alegar recheck de camadas futuras. authenticate/export
continuam conferindo somente estrutura/cadeia/assinaturas.

CLI verify aceita --cake-lpr BIN; export não executa checkers. Auditor não
depende de logos-ledger/kernel, inclusive em dependências dev. Fixture com
appender real é produzida por subprocesso separado.

## Limites de compatibilidade observados

Dois dos14 vetores manuais4.4 têm diferenças registradas: duplicates é
aceito pelo Rust e rejeitado pelo cake_lpr; sparse é aceito pelo Rust e
esgota memória no cake_lpr com256MiB de AS/heap64MiB/stack64MiB. Os outros12
concordaram. Observações/reprodução em spec/vectors/cake-lpr/ e
docs/plans/etapa-4.5-perfis.py. Não alterar os bytes da CNF ou renumerar IDs
no wrapper para esconder essas diferenças. Recheck duplo exige ambos os
aceites; essa limitação pode reduzir a completude de produtores futuros.
Os critérios de comparação do corpus200/600 não alegam equivalência geral.
