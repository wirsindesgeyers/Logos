# Atestador `backup-svc`

Mock do serviço de backup: registra backups com timestamp e atesta se há um recente.

- **AttestorId:** `backup-svc`
- **Procedimento:** `backup-svc/registry/v1`
- **Tópico (trust/v1):** `backup`

## Símbolos ancorados

- `backup_recent(banco)`: existe um backup do banco mais novo que a idade máxima.

## Procedimento

1. O serviço registra `record(timestamp)` a cada backup concluído.
2. Na observação ignora timestamps no futuro (relógio errado ou registro falso) e toma o mais recente.
3. Afirma `backup_recent` se `now − último ≤ idade máxima`; senão a negação. A versão é `banco` → `backup_at=<ts>`.

## Limites

É um mock: não verifica que o backup é restaurável, só que foi registrado.

O operador precisa conceder `Trusted(emissor, backup)` na configuração de confiança; sem isso o kernel recusa o registro (6.2).
