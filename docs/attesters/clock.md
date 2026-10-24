# Atestador `clock`

O relógio local concorda com uma fonte de referência dentro de uma tolerância.

- **AttestorId:** `clock`
- **Procedimento:** `clock/skew-vs-reference/v1`
- **Tópico (trust/v1):** `time`

## Símbolos ancorados

- `clock_synced`: o relógio local está dentro da tolerância da referência.

## Procedimento

1. Lê o instante local `now` e o instante da fonte de referência (`TimeSource`; o padrão é o relógio do sistema, em testes é injetado).
2. Calcula `skew = |now − referência|` em segundos.
3. Afirma `clock_synced` se `skew ≤ tolerância`; senão afirma `¬clock_synced`. Fato com validade `Until(now + ttl)`.
4. A versão registrada é `clock` → `skew=<n>s;tolerance=<t>s`.

## Limites

Não prova a hora real, só a concordância com a fonte configurada. Se a fonte for o próprio relógio local, o desvio é sempre 0 e o fato é vazio: o operador deve apontar uma fonte independente.

O operador precisa conceder `Trusted(emissor, time)` na configuração de confiança; sem isso o kernel recusa o registro (6.2).
