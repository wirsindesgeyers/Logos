# Atestador `sandbox-run`

Execução determinística isolada que atesta `computed(h_script, h_input, h_output)`.

- **AttestorId:** `sandbox-run`
- **Procedimento:** `sandbox-run/bwrap-prlimit/v1`
- **Tópico (trust/v1):** `computation`

## Símbolos ancorados

- `computed(script, entrada, saída)` sobre constantes `sha256:<hex>` do sort `Digest`.

## Procedimento

1. Executa o script (`/usr/bin/sh`) sobre a entrada dentro de `/usr/bin/bwrap` (namespaces de usuário, PID, rede, IPC e UTS; só `/usr` somente leitura; script e entrada montados somente leitura) sob `/usr/bin/prlimit` (CPU, memória, tamanho de arquivo, sem core; o número de processos fica contido pelo namespace de PID, já que `RLIMIT_NPROC` conta o usuário inteiro e quebraria o isolamento em hosts ocupados).
2. Roda **duas vezes** e exige a mesma saída; saídas diferentes não atestam nada. Código de saída ≠ 0, estouro de tempo ou de saída (1 MiB) são erro.
3. Confere que script e entrada não mudaram durante a execução; calcula SHA-256 dos três artefatos.
4. Cria um vocabulário por execução com as três constantes (`sha256:<hex>`); o fato afirma `computed` sobre elas. Sem `bwrap`/`prlimit` não há fallback sem isolamento.

## Limites

Fecha a cadeia dado bruto → receita → saída por hashes; não prova que o dado reflete o mundo. Não-determinismo escondido que se repita nas duas execuções (ex.: dependência do host) passa.

O operador precisa conceder `Trusted(emissor, computation)` na configuração de confiança; sem isso o kernel recusa o registro (6.2).
