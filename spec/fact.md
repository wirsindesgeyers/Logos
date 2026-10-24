# fact/v1 — fatos assinados (Etapa 6.1, ADR0061)

Fato é uma afirmação AIR assinada por um emissor. Assinatura válida **não** torna
o fato citável: o kernel só grava `FactRegistered` se a configuração de confiança
vigente no ledger concede `Trusted(emissor, tópico)` (trust.md, 6.2), e a citação
depende da janela de validade e da mesma concessão.

## 1. Objeto

```text
Fact     = [1, Formula, hV, issuer, topic, observed_at, valid, version, sig]
valid    = [0, first_u64, last_u64]      ; épocas inclusivas, first ≤ last
         | [1, until_unix_seconds_u64]   ; instante inclusivo
version  = [] | [[resource_Text, version_Text]]
issuer   = PublicKey (canon de logos-crypto, algoritmo explícito)
sig      = Signature (canon de logos-crypto)
```

- `Formula` é AIR/v1 integral; `hV` é bstr32 do vocabulário; `topic`, `resource`
  e `version` são Text não vazios de até 4096 bytes. `observed_at` é instante
  Unix em segundos, informativo nesta etapa.
- O emissor é guardado como chave pública (o `KeyId` é `h(PublicKey)`), para que
  qualquer leitor do ledger possa verificar a assinatura.
- Bytes assinados: canon do array dos oito primeiros campos (sem `sig`), com a tag
  de assinatura `logos/sig/fact/v1\0`.
- Identidade: `h(Fact)` com a tag de hash `logos/fact/v1\0` sobre os bytes canônicos
  completos (com `sig`). Namespace CAS `fact-v1`, teto de 1 MiB.
- Decode estrito: versão 1, aridade 9, `first ≤ last`, tipos exatos; qualquer outro
  formato é erro, nunca um fato "parecido".

## 2. Registro

`FactRegistered { fact: h(Fact) }`, corpo `[1, 16, h(Fact)]` (events.md §3). O
kernel grava o evento só se: o objeto está no CAS e decodifica; a configuração
vigente concede `Trusted(KeyId(emissor), tópico)` (trust.md §3); a assinatura verifica; o vocabulário está no CAS e a fórmula é
bem tipada nele; a janela ainda não passou (época corrente ≤ last, ou agora ≤
until); e o mesmo `h(Fact)` não foi registrado antes. Recusa não grava evento.

## 3. Citação

A projeção do índice (ledger-index/v5) lê o fato do CAS (leitura confere o hash) e
grava a linha `Fact` (status 8), a janela, o `KeyId` do emissor e o tópico na tabela
`facts`. Um fato vivo é citável na época `e` e instante `t` se a configuração
vigente concede `Trusted(emissor, tópico)` (senão o gerador recusa com `Untrusted`,
trust.md §3) e:

- `valid = [0, a, b]`: `a ≤ e ≤ b`;
- `valid = [1, u]`: `e ≥` época de registro e `t ≤ u`.

O snapshot do kernel resolve a época de aplicabilidade: a corrente se aplicável;
senão o limite mais próximo da janela (o gerador recusa com `InvalidEpoch`).
`Until` vencido marca a entrada como expirada (o gerador recusa com `Expired`,
logo após `Retracted` na precedência de obligation.md §3). Na replay da projeção
(`used` de um Verified), `Until` vale enquanto o fato estiver vivo: a replay não
depende do relógio; o instante é conferido pelo kernel ao regenerar a obrigação.

## 4. Fonte de sort enumerado (6.1b, ADR0062)

Um sort `Enumerated { constants, domain_source: Fact(r) }` de V só é usável se,
em PROPOSE e de novo em CERTIFY/REFUTE/WITNESS (sobre O regenerado):

1. `r = hash(Formula)` da conjunção `And(fecho, distinct…)` de V
   (`spec/finite-fragment.md`); senão `enumeration source does not match vocabulary`;
2. alguma premissa de O tem exatamente essa fórmula; senão
   `missing enumeration source fact` (axiomas separados, ainda que aceitos pelo
   grounder, não bastam);
3. toda premissa de O com essa fórmula resolve, no snapshot que gerou O, para uma
   entrada `Fact` (evento `FactRegistered` vivo e aplicável); hipótese ou Verified
   com a mesma fórmula é `enumeration source is not a registered fact`.

A premissa entra em O pelo gerador comum (citação do evento do fato), então a
fonte é o `PremiseRef` do evento `FactRegistered`, nunca bytes copiados de `r`.
Como ela é o suporte do sort no grounding, o `used` de qualquer prova que toque o
domínio inclui o evento do fato, e o índice o registra em deps. Fonte `Policy` é
recusada (`unresolved enumeration source`) até a 7.3. Recusa não grava evento.

## 5. Revogação (6.4, ADR0064)

- `Retracted` sobre um `FactRegistered` (só o admin) mata o fato; a cascata ATMS
  derruba os claims que dependiam dele sem rota alternativa.
- `IssuerRevoked { KeyId, since }` (admin) mata todo fato vivo do emissor com
  **época de registro** ≥ `since` (não `observed_at`, que o emissor assina) e
  impede fatos novos dele. Fatos anteriores a `since` continuam válidos.
- Em ambos, o kernel grava `Invalidated { causes: [raiz, premissa morta] }` para os
  claims derrubados; a raiz é o `Retracted` ou o `IssuerRevoked`.

## 6. Auditoria

O auditor independente confere o objeto no CAS e reverifica a assinatura do
emissor (`facts_verified` em audit.md). Ele não decide confiança no emissor.

Vetores: `spec/vectors/fact/fact.json` (gerador Python independente
`tools/gen_fact_vectors.py`).
