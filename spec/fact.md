# fact/v1 — fatos assinados (Etapa 6.1, ADR0061)

Fato é uma afirmação AIR assinada por um emissor. Assinatura válida **não** torna
o fato citável: o kernel só grava `FactRegistered` para emissores da raiz de
confiança (configuração do operador; política por tópico na 6.2), e a citação
depende da janela de validade.

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
kernel grava o evento só se: o objeto está no CAS e decodifica; o emissor está na
raiz de confiança; a assinatura verifica; o vocabulário está no CAS e a fórmula é
bem tipada nele; a janela ainda não passou (época corrente ≤ last, ou agora ≤
until); e o mesmo `h(Fact)` não foi registrado antes. Recusa não grava evento.

## 3. Citação

A projeção do índice (ledger-index/v4) lê o fato do CAS (leitura confere o hash) e
grava a linha `Fact` (status 8) e a janela na tabela `facts`. Um fato vivo é
citável na época `e` e instante `t` se:

- `valid = [0, a, b]`: `a ≤ e ≤ b`;
- `valid = [1, u]`: `e ≥` época de registro e `t ≤ u`.

O snapshot do kernel resolve a época de aplicabilidade: a corrente se aplicável;
senão o limite mais próximo da janela (o gerador recusa com `InvalidEpoch`).
`Until` vencido marca a entrada como expirada (o gerador recusa com `Expired`,
logo após `Retracted` na precedência de obligation.md §3). Na replay da projeção
(`used` de um Verified), `Until` vale enquanto o fato estiver vivo: a replay não
depende do relógio; o instante é conferido pelo kernel ao regenerar a obrigação.

## 4. Auditoria

O auditor independente confere o objeto no CAS e reverifica a assinatura do
emissor (`facts_verified` em audit.md). Ele não decide confiança no emissor.

Vetores: `spec/vectors/fact/fact.json` (gerador Python independente
`tools/gen_fact_vectors.py`).
