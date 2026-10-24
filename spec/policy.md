# Políticas — policy/v1 e sintaxe de superfície (Etapa 7.1, ADR 0069)

Política é um conjunto de regras em AIR, escritas por humanos direto em lógica (fidelidade F4),
versionadas e ligadas a um vocabulário. O kernel só consome os **bytes canônicos** `policy/v1`;
o texto de superfície (§5) é conveniência fora da TCB. Árvore Merkle e raiz assinada são da 7.2;
avaliação, da 7.3.

## 1. Objeto

```text
Policy = [1, version_u64, hV, rules]
rules  = [Rule…]                       ; ordem estrita por `id` (bytes), sem repetição
Rule   = [id_Text, class_u64, Formula, comment_Text]
class  = 0 allow | 1 deny | 2 require | 3 derive
```

- `version ≥ 1`; `hV` é o hash do vocabulário (vocabulary/v1) em que todas as fórmulas valem.
- `id`: texto não vazio de até 256 bytes; no máximo 4096 regras; comentário de até 64 KiB;
  política de até 4 MiB canônicos.
- Identidade: `h(Policy)` com a tag de hash `logos/policy/v1\0` sobre os bytes canônicos.
- O **comentário é glosa em português e não é normativo**: nada na avaliação o lê.
- Decode estrito: tipos exatos, aridade 4, classes 0–3, ordem por `id`; qualquer outro formato é erro.

## 2. Validação (TCB)

`Policy::new` e `Policy::decode(bytes, vocabulary)` aplicam, nesta ordem, e recusam a política
inteira na primeira violação (nada é aceito parcialmente):

1. `version ≠ 0`, `h(V)` igual ao do vocabulário fornecido, ids únicos e ordenados, tetos;
2. **sort-check** de cada fórmula contra o vocabulário (`logos_air::check`, 1.3);
3. se há regras, o vocabulário declara `pred Allowed(S)` unário;
4. **forma** por classe (§3);
5. **estratificação** (§4).

## 3. Classes e forma de Horn restrita

`allow`, `deny` e `derive` são regras de Horn: `∀x̄. corpo → cabeça` (ou só a cabeça). O corpo é uma
conjunção de átomos, igualdades, `true`, átomos negados e blocos negados `¬∃ȳ.(conjunção positiva)`;
`∃` positivo no corpo só introduz variáveis. Fora disso (`∨`, `↔`, `→` aninhado, `∀` ou `false` no
corpo, negação dupla) a regra não é Horn e só pode ser `require`.

| classe | cabeça | efeito |
| --- | --- | --- |
| `allow` | `Allowed(t)` | `Allowed(a)` fica derivável se o corpo vale |
| `deny` | `¬Allowed(t)` | veto; prevalece sobre `allow` |
| `derive` | `P(t)` com `P ≠ Allowed` | predicado auxiliar definido pela política |
| `require` | FOL geral que menciona `Allowed` | restrição que exige certificado (rota L2, 7.3) |

**Semântica deny-overrides:** `Allowed(a)` só é derivável se alguma regra `allow` dispara e nenhuma
`deny` aplicável dispara. Formalmente, `Allowed ≡ AllowFires ∧ ¬DenyFires`.

Fora do escopo desta etapa, e registrado para a 7.3: segurança por faixa (range restriction: toda
variável da cabeça ocorrer em literal positivo do corpo) e a ordem de avaliação.

## 4. Estratificação

Grafo de dependência com nós por predicado mais `AllowFires` e `DenyFires`: o corpo de cada regra
aponta para os predicados que usa (aresta negativa se o uso é negado ou está em bloco negado);
`Allowed → AllowFires` (positiva) e `Allowed → DenyFires` (negativa). A política é recusada
(`NegativeCycle`) se alguma aresta negativa liga dois nós do mesmo componente fortemente conexo,
inclusive por um único predicado que se nega a si mesmo. Recursão positiva é permitida. Um corpo
`deny` que dependa de `Allowed` fecha ciclo com a negação do deny-overrides e é recusado.

## 5. Sintaxe de superfície (`logos-policy-text`, fora da TCB)

```text
policy v12 {
  rule r1 allow: forall x:Action. forall e:Source.
      kind_of(x) = respond and says_request(e, x) and external(e) -> Allowed(x)
    comment "Responder a pedido externo é permitido."
  rule r9 deny: forall x:Action. is_drop(x) and
      (not exists t:Moment. (executed(backup_of(x), t) and recent(t))) -> not Allowed(x)
}
```

- `rule ID CLASSE: FÓRMULA [comment "texto"]`, `;` opcional entre regras, `#` comenta até o fim da
  linha; `ID` é identificador (`[A-Za-z_][A-Za-z0-9_]*`) ou string.
- A fórmula é a sintaxe `air-text/v1` (`spec/air-surface.md`): `and`, `or`, `not`, `->`, `<->`,
  `forall x:S.`, `exists x:S.`. Um quantificador vai até o delimitador, então parentesize
  `(not exists t:Moment. (...))` antes de `->`.
- Uma regra termina em `;`, `}`, no próximo `rule ID CLASSE:` ou em `comment "…"` fora de
  parênteses; os termos `rule` e `comment` só são reservados nessas posições.
- Texto fora de NFC ou com code points não atribuídos é erro, nunca normalizado em silêncio.
- A impressão (`print_policy`) é determinística e relida dá a mesma política.

Os exemplos do roadmap usam `∧`, `∃`, `Says(e, Pedido(x))`: em texto vira `and`, `exists` e
`says_pedido(e, x)` (`says_P` levantado, ADR 0068).

## 6. Vetores

`spec/vectors/policy/policy.json` (gerador Python independente `tools/gen_policy_vectors.py`, parser
AIR e canon Python): 3 políticas válidas com hash, 14 inválidas com o motivo esperado, e o
vocabulário canônico (`vocabulary.bin`, também semente de fuzz). O Rust confere byte a byte.
