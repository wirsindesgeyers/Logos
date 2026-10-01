# Domínios de hash e de assinatura

Especificação normativa (roadmap, Etapa 0.4). Implementação: `crates/logos-crypto`. Vetores em
[`vectors/domains/`](vectors/domains/) e [`vectors/crypto/`](vectors/crypto/).

## 1. Hash com separação de domínio

```
H_T(x) = SHA-256( tag_T ‖ bytes(x) )          (32 bytes)
```

- `bytes(x)` é a codificação canônica `canon/v1` de `x` ([`encoding.md`](encoding.md)) para os
  domínios **canônicos**, ou os bytes do objeto para os domínios **crus** (`raw`).
- `tag_T` é uma sequência de bytes fixa por domínio, da forma `logos/<nome>/v<k>` seguida de um
  único byte `0x00`. Nenhuma tag contém outro `0x00` e nenhuma é prefixo de outra; portanto
  `tag ‖ bytes` nunca é ambíguo e dois domínios nunca produzem o mesmo hash para a mesma entrada
  (a menos de colisão do SHA-256).
- O tipo carrega o domínio: um hash calculado no domínio `formula` não é aceito onde se espera
  um hash de `obligation`.
- `hash_raw` aceita também entrada em fluxo (hash incremental): `tag` primeiro, depois os bytes
  na ordem. O resultado é idêntico ao de uma só chamada.

### Domínios reservados

| Nome | Tag (ASCII, + `0x00`) | Codificação | Introduzido em |
| --- | --- | --- | --- |
| key | `logos/key/v1` | canonical | 0.4 |
| formula | `logos/formula/v1` | canonical | 1.1 |
| vocab | `logos/vocab/v1` | canonical | 1.2 |
| proposal | `logos/proposal/v1` | canonical | 1.6 |
| obligation | `logos/obligation/v1` | canonical | 1.5 |
| certificate | `logos/certificate/v1` | raw | 2.3 |
| checker | `logos/checker/v1` | raw | 4.5 |
| model | `logos/model/v1` | canonical | 2.1 |
| event | `logos/event/v1` | canonical | 3.1 |
| fact | `logos/fact/v1` | canonical | 6.1 |
| policy | `logos/policy/v1` | canonical | 7.1 |
| contract | `logos/contract/v1` | canonical | 8.1 |
| action | `logos/action/v1` | canonical | 8.2 |
| paraphrase | `logos/paraphrase/v1` | canonical | 9.1 |
| token | `logos/token/v1` | canonical | 8.3 |
| assumption | `logos/assumption/v1` | canonical | 5.7 |

Os domínios acima só são *reservados* na Etapa 0.4: os tipos que os usam nascem nas etapas
indicadas. Um domínio novo ou uma mudança de tag exige entrada nesta tabela, ADR e novos vetores;
mudar o conteúdo de um domínio existente é uma nova versão (`v2`), nunca uma edição da `v1`.

### Digest externo

`ExternalDigest = SHA-256(bytes)` **sem tag**. Serve só para conferir binários de terceiros
contra o hash que eles próprios publicam (ex.: `cake_lpr`, Etapa 4.5) e para o manifesto da TCB.
Nunca é aceito onde se espera um `Hash<T>`. O ledger registra sempre `hash_raw` no domínio
`checker`.

## 2. Algoritmos, chaves e assinaturas

- Algoritmos (`alg`): `"ed25519"` (implementado) e `"es256"` (reservado para a Etapa 9.2, hoje
  recusado com erro tipado). Qualquer outro nome é erro (`UnknownAlgorithm`).
- **Chave pública** (canônica): mapa `{"alg": <nome>, "key": <bytes>}`. Ed25519: 32 bytes, ponto
  válido e de ordem não pequena; chaves fracas são recusadas.
- **Assinatura** (canônica): mapa `{"alg": <nome>, "sig": <bytes>}`. Ed25519: 64 bytes.
  Chaves e assinaturas com chaves de mapa extras ou faltando são rejeitadas.
- **KeyId** = `H_key(canon(PublicKey))`. Como o algoritmo faz parte da chave canônica, os mesmos
  bytes de chave sob outro algoritmo são outra identidade.
- Uma assinatura só verifica contra uma chave do **mesmo** algoritmo (`AlgorithmMismatch`).

## 3. Domínios de assinatura

```
assinatura = Ed25519( chave_privada, sigtag_D ‖ bytes )
```

`sigtag_D` segue as mesmas regras de forma e de prefixo-livre das tags de hash, com o prefixo
`logos/sig/`. Uma assinatura de um domínio nunca verifica em outro (um evento nunca vale como
token). A verificação é a **estrita** do Ed25519 (RFC 8032, com cofator; rejeita assinaturas
maleáveis).

| Nome | Tag (ASCII, + `0x00`) | Uso | Introduzido em |
| --- | --- | --- | --- |
| event | `logos/sig/event/v1` | evento do ledger (kernel) | 3.2 |
| token | `logos/sig/token/v1` | token de ação (kernel) | 8.3 |
| fact | `logos/sig/fact/v1` | fato (atestador) | 6.1 |
| policy | `logos/sig/policy/v1` | raiz de política (autoridade) | 7.2 |
| contract | `logos/sig/contract/v1` | contrato de ferramenta (admin) | 8.1 |
| authorization | `logos/sig/authorization/v1` | autorização do usuário | 9.2 |
| override | `logos/sig/override/v1` | override humano | 8.7 |
| fidelity | `logos/sig/fidelity/v1` | atestado de fidelidade | 9.3 |

Os vetores de Ed25519 do RFC 8032 (`vectors/crypto/ed25519-rfc8032.tsv`) valem para a primitiva
sem tag; os de `vectors/crypto/sig-domain.tsv` cobrem a mensagem com tag.

## 4. Arquivo de chave

Formato (`FileSigner::load`): a semente de 32 bytes em **64 dígitos hexadecimais minúsculos**,
com um `\n` final opcional, e nada mais. O arquivo precisa ser regular (não link simbólico) e não
ter permissão alguma para grupo/outros (`modo & 0o077 == 0`; tipicamente `0600`); caso contrário a
carga é recusada (`InsecurePermissions`). A permissão é conferida no arquivo já aberto.

A chave fica em memória do processo que assina. HSM/TEE e rotação de chaves são da Etapa 13.1,
como outras implementações da trait `Signer`.
