# Atestador `enum-source`

Atesta o fecho de um domínio: "os usuários são exatamente {…}".

- **AttestorId:** `enum-source`
- **Procedimento:** `enum-source/closure/v1`
- **Tópico (trust/v1):** `domain-closure`

## Símbolos ancorados

- Constantes do sort enumerado (um membro cada).

## Procedimento

1. Monta o vocabulário com o sort `Enumerated` e a fonte `Fact(r)`, em que `r` é o hash da fórmula (não do fato), o que desfaz o ciclo vocabulário ↔ fato.
2. O fato afirma a conjunção fecho ∧ nomes distintos (`logos_ground::domain_axioms(...).conjunction()`), a fórmula exata que o kernel exige da fonte (ADR 0062).
3. Confere que recalcular a conjunção com a fonte final dá a mesma fórmula; se não der, erro.

## Limites

O atestador declara a lista; quem garante que a lista é o mundo é a confiança no emissor e a procedência (L1). Mudar a lista exige novo vocabulário e novo fato.

O operador precisa conceder `Trusted(emissor, domain-closure)` na configuração de confiança; sem isso o kernel recusa o registro (6.2).
