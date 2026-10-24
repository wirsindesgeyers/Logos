# trust/v1 — confiança por tópico (Etapa 6.2, ADR0063)

A configuração de confiança diz, para cada emissor, que tópicos ele pode atestar.
Ela é um objeto no CAS publicado por um evento assinado pelo admin; a vigente é a
de maior versão. Um fato só é registrável e citável se `Trusted(emissor, tópico)`
vale na configuração vigente.

## 1. Objeto

```text
TrustConfig = [1, version_u64, roots]          ; version ≥ 1
roots       = [Root, ...]                      ; 1..1024, ordem estrita de KeyId
Root        = [issuer, name_Text, topics, level_u64]
topics      = [Text, ...]                      ; 1..256, ordem estrita de bytes UTF-8
issuer      = PublicKey (canon de logos-crypto; KeyId = h(PublicKey))
```

- `name` e cada tópico: Text não vazio de até 4096 bytes. `name` é legível e não
  participa da regra. `level` é declarado pelo admin e não tem efeito nesta etapa.
- Tópicos comparam por igualdade exata de bytes (sem prefixo/hierarquia:
  `backup` não cobre `backup/offsite`).
- Identidade: `h(TrustConfig)` com a tag `logos/trust/v1\0` sobre os bytes
  canônicos; namespace CAS `trust-v1`, teto de 1 MiB. Decode estrito: versão de
  formato 1, aridades exatas, ordem/unicidade conferidas; nada é normalizado.
- O objeto não tem assinatura própria: a do admin está no evento e cobre `h(config)`.

## 2. Publicação

`TrustConfigPublished { config: h(TrustConfig), authority }`, corpo
`[1, 17, h, Authority]` (events.md §12). O kernel grava o evento só se o signer é a
chave administrativa configurada pelo operador, o objeto está no CAS e decodifica,
e `version` é maior que a vigente. A assinatura de autoridade segue events.md §4.

## 3. Regra

- **Registro** (`FactRegistered`, fact.md §2): exige `Trusted(KeyId(issuer), topic)`
  na configuração vigente; sem configuração publicada, nenhum fato é registrado.
- **Citação** (snapshot/gerador): fato cujo emissor não tem mais o tópico na
  configuração vigente é recusado com `Reject::Untrusted` (obligation.md §3), logo
  após `Expired`. Claims já Verified não caem: retirar confiança em massa é a
  revogação de emissor da 6.4.
- **Projeção**: configuração nova substitui as concessões; versão que não cresce é
  erro. `FactRegistered` sem concessão vigente nesse ponto e `used` de fato sem
  concessão na época do Verified são erros de reconstrução.
- **Premissa explícita**: a concessão que torna um fato citável é consultável
  (`fact_trust(h(FactRegistered))` → `h(TrustConfigPublished)`, `trust_grant`).
  Nesta etapa ela é premissa explícita no ledger, não fórmula AIR em O (P8, 17.4).

## 4. Auditoria

O auditor independente reaplica as configurações na ordem com decoder próprio e
estrito (mesmas regras de §1: ordem, unicidade, tetos), recusa versão que não
cresce e confere que cada fato registrado tinha concessão
vigente (`trust_configs` em audit.md). A chave do admin não está no ledger: o
auditor não confere a assinatura de autoridade contra ela.

Vetores: `spec/vectors/trust/trust.json` (gerador Python independente
`tools/gen_trust_vectors.py`).
