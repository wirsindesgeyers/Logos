# CAS — cas/v1

Especificação normativa da Etapa 0.5. Objetos são bytes opacos; CAS não valida canon/v1
nem certificados. Auditor e PVC podem recalcular endereços sem código Rust.

## 1. Endereço e domínios

`digest = SHA-256(tag || bytes)`; tag ASCII da tabela seguida de exatamente um byte
`0x00`. Sem envelope, salt ou normalização. Hexadecimal minúsculo de 64 dígitos.
A raiz fornecida já é a pasta CAS: caminho relativo `<diretório>/<hex[0:2]>/<hex[2:64]>`.
Exemplo de raiz: `cas/`. Somente tags exatas abaixo são aceitas; tag desconhecida gera
`Io(InvalidInput)` antes de tocar objeto ou ler fluxo. Um domínio novo exige ADR e versão
normativa; estas tags são as mesmas de [domains.md](domains.md).

| Tag (ASCII, + `0x00`) | Diretório | Codificação do consumidor | Limite padrão (bytes) |
| --- | --- | --- | --- |
| `logos/key/v1` | key-v1 | canonical | 16777216 |
| `logos/formula/v1` | formula-v1 | canonical | 16777216 |
| `logos/vocab/v1` | vocab-v1 | canonical | 16777216 |
| `logos/proposal/v1` | proposal-v1 | canonical | 16777216 |
| `logos/obligation/v1` | obligation-v1 | canonical | 16777216 |
| `logos/certificate/v1` | certificate-v1 | raw | 268435456 |
| `logos/checker/v1` | checker-v1 | raw | 268435456 |
| `logos/model/v1` | model-v1 | canonical | 16777216 |
| `logos/event/v1` | event-v1 | canonical | 16777216 |
| `logos/fact/v1` | fact-v1 | canonical | 16777216 |
| `logos/policy/v1` | policy-v1 | canonical | 16777216 |
| `logos/contract/v1` | contract-v1 | canonical | 16777216 |
| `logos/action/v1` | action-v1 | canonical | 16777216 |
| `logos/paraphrase/v1` | paraphrase-v1 | canonical | 16777216 |
| `logos/token/v1` | token-v1 | canonical | 16777216 |
| `logos/assumption/v1` | assumption-v1 | canonical | 16777216 |
| `logos/cnf/v1` | cnf-v1 | raw | 16777216 |
| `logos/trust/v1` | trust-v1 | canonical | 16777216 |

16 MiB = 16777216 bytes; 256 MiB = 268435456 bytes. CAS calcula hash dos bytes recebidos,
inclusive em domínios canonical: consumidor é responsável pela codificação correta.

## 2. API e limites

`logos_ledger::{Cas, Limits, CasError}`; módulo público `cas`:

- `Cas::open(root: impl AsRef<Path>, limits: Limits) -> Result<Cas, CasError>`.
- `Limits::default()` fornece tabela acima.
- `limits.with_domain_limit::<T: Domain>(limit: u64) -> Result<Limits, CasError>`
  substitui apenas limite daquela tag; acima de `isize::MAX` gera `Io(InvalidInput)`.
- `cas.put::<T: Domain>(&[u8]) -> Result<Hash<T>, CasError>`.
- `cas.put_reader::<T: Domain>(impl Read) -> Result<Hash<T>, CasError>`.
- `cas.get::<T: Domain>(&Hash<T>) -> Result<Option<Vec<u8>>, CasError>`.

Limite zero aceita objeto vazio; limite exato é aceito. `TooLarge { limit, actual }`
usa tamanho total conhecido em `put`; em fluxo/get usa bytes observados (`limit + 1`),
não supõe tamanho do restante. Leitura consome no máximo limite mais probe externo de
um byte; guarda no máximo limite bytes no Vec. Crescimento geométrico com reserva
falível; `OutOfMemory` é erro I/O, não `TooLarge`. Limite não é teto de RSS. `Interrupted`
na leitura é repetido; outros erros são propagados. Falha não publica objeto parcial.

`get` retorna `None` só para `NotFound` ao abrir destino. Erro posterior é `Io`.
Excesso precede integridade; em EOF dentro do limite recalcula digest e compara com
endereço antes de entregar bytes. Divergência é `Integrity`; nunca devolve bytes errados.
Erros não são veredictos: camada futura converte `TooLarge` em `Unknown(Size)`.

## 3. Publicação e durabilidade

Base Unix, filesystem local com rename atômico e fsync de diretórios. Operador controla
raiz e ancestrais; não é sandbox contra SO/operador malicioso ou troca hostil de diretórios.
Sem promessa em NFS, hardware defeituoso ou filesystem sem suporte necessário.

`open` cria raiz e sincroniza sua cadeia de ancestrais até `/`, de baixo para cima,
mesmo se já existentes. Cada writer cria domínio/shard, abre temporário exclusivo no
shard (`create_new`, PID + contador, repetir somente colisões), escreve todos os bytes,
sincroniza arquivo, renomeia ao destino, sincroniza shard, domínio e raiz nessa ordem.
Nunca depende de quem criou diretórios. Não há fast path baseado em existência.
Concorrência pode substituir objeto do mesmo hash com bytes idênticos; `put` correto
repara objeto corrompido. Reader vê ausência ou objeto completo, nunca escrita parcial.

Falha de sincronização retorna `Io`, inclusive após rename; erro não promete rollback.
Remover temporário restante em falha: `NotFound` é limpeza já satisfeita. Se limpeza
falhar, `Io` preserva kind primário, ambos os diagnósticos e primário em `source()`.
GC, replicação, compressão e streaming real ficam fora de cas/v1.

Etapa3.5 acrescenta `Cas::open_existing(root, limits)` (não cria/sincroniza
diretórios) e integra o store ao appender. A CLI oferece GC somente dry-run;
contrato em [ledger-cas.md](ledger-cas.md), sem alteração do layout cas/v1.

## 4. Exemplos independentes

Tabela de entrada para vetores; bytes hex vazios representados por `-`:

| Exemplo | Tag (ASCII, + `0x00`) | Bytes (hex) |
| --- | --- | --- |
| certificate-empty | `logos/certificate/v1` | `-` |
| certificate-binary | `logos/certificate/v1` | `0001ff804c4f474f53` |
| obligation-canonical | `logos/obligation/v1` | `a1616101` |
| cnf-empty | `logos/cnf/v1` | `7020636e66203020300a` |

O exemplo obligation-canonical é canon/v1 de mapa `{"a": 1}`, sem afirmar semântica de obrigação.
Script independente `tools/gen_cas_vectors.py` lê apenas este documento e usa `hashlib`.
Vetores em `spec/vectors/cas/vectors.tsv` incluem tag, bytes, digest e caminho relativo.
`--check` exige igualdade exata com arquivo versionado.

Etapa4.3/ADR0028 acrescenta namespace cnf-v1 (DIMACS), com limite padrão
16 MiB e hash separado. Objetos e endereços anteriores permanecem iguais.
