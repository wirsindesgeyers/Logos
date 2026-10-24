# Atestador `db-monitor`

Existência de tabela, contagem de conexões e versão de linha de um banco.

- **AttestorId:** `db-monitor`
- **Procedimento:** `db-monitor/probe/v1`
- **Tópico (trust/v1):** `db`

## Símbolos ancorados

- `table_exists(tabela)`.
- `connections_within_limit(banco)`: conexões abertas ≤ limite.
- `row_observed(tabela)`: a linha existe; a versão é a impressão digital dos valores.

## Procedimento

1. Usa uma `DbProbe`. `SqliteProbe` abre o arquivo SQLite **somente leitura** e consulta `sqlite_master` e a linha por `rowid` (nome de tabela restrito a `[A-Za-z0-9_]`).
2. A impressão da linha é o SHA-256 dos valores com tag de tipo e tamanho (`1` e `'1'` não colidem).
3. SQLite não tem contagem de conexões: a sonda devolve erro em vez de um número inventado. `MockPgProbe` é o mock do `pg-monitor` (sem rede nesta etapa).
4. Ausência de tabela ou de linha vira a negação; falha da sonda vira erro, nunca `false`.

## Limites

Observação de um instante; o monitor não vê transações em andamento. `pg-monitor` real fica para uma etapa posterior.

O operador precisa conceder `Trusted(emissor, db)` na configuração de confiança; sem isso o kernel recusa o registro (6.2).
