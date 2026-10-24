# Atestador `fs-stat`

Existência, hash e mtime de um arquivo, versionados pelo hash do conteúdo.

- **AttestorId:** `fs-stat`
- **Procedimento:** `fs-stat/lstat-sha256/v1`
- **Tópico (trust/v1):** `fs`

## Símbolos ancorados

- `file_present(arquivo)`: o arquivo existe e é regular.
- Constante por arquivo vigiado (alias no vocabulário).

## Procedimento

1. `lstat` do caminho configurado. Link simbólico é recusado (erro), nunca seguido.
2. Arquivo regular até 64 MiB: lê o conteúdo e calcula SHA-256; afirma `file_present(alias)`.
3. Ausente (ou não regular): afirma `¬file_present(alias)`, sem versão.
4. A versão registrada é `alias` → `sha256=<hex>;size=<n>;mtime=<s>`.

## Limites

O fato descreve o instante da leitura; o arquivo pode mudar depois (TOCTOU). A validade curta e o hash na versão deixam o consumidor detectar a mudança.

O operador precisa conceder `Trusted(emissor, fs)` na configuração de confiança; sem isso o kernel recusa o registro (6.2).
