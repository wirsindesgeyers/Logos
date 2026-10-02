# Plano de etapa — 0.5: armazenamento endereçado por conteúdo (CAS)

- **Data:** 2026-10-02.
- **Estado:** planejamento concluído; implementação não iniciada.
- **Escopo:** Etapa 0.5 do `ROADMAP.md`, tamanho **P**, TCB de correção.
- **Referências:** roadmap §0, §2, §4 e 0.5; especificação conceitual §4 (recheck e
  certificados grandes), §5 (TCB) e §7 (PVC); `spec/domains.md`; ADRs 0001–0004.
- **Local:** módulo `cas` de `logos-ledger`, já responsável pelo CAS no mapa de crates.
  Não criar `logos-cas`; `logos-audit` continuará independente de `logos-ledger`.

**Fora do escopo:** GC (3.5), replicação, compressão, armazenamento em streaming real,
SQLite, eventos, assinaturas do ledger, staging/protocolo do kernel, conversão de erros
em veredictos, implementação do auditor ou de PVCs. A API aceita `Read`, mas a v1 pode
manter o objeto em memória. Não fechar pendências remotas das etapas anteriores por
suposição nem alterar seus critérios.

## 1. Interfaces de entrada

| Interface existente | Uso nesta etapa |
| --- | --- |
| `logos_crypto::Domain { const TAG: &'static [u8]; }` | Selecionar domínio, diretório e limite; manter trait inalterada. |
| `Hash<T>`, `Hash::as_bytes`, `Hash::from_bytes` | Endereço tipado de 32 bytes e comparação do digest lido. |
| `hash_raw::<T>(&[u8]) -> Hash<T>` | Endereço de um objeto já disponível em memória. |
| `RawHasher::<T>::new/update/finalize` | Hash incremental dos blocos recebidos por `put_reader`. |
| `tags::RESERVED` e `spec/domains.md` | Tabela de tags, nomes e classificação canonical/raw. |
| `std::fs`, `std::io`, `std::path` | Arquivos, leitura limitada, publicação atômica e sincronização. |
| Workspace, lints, manifesto e scripts CI | Governança existente da TCB. |

Dependências técnicas da 0.4 presentes. Nenhum bloqueio técnico identificado para
implementar a 0.5. CI remota e fuzz longo da 0.3/0.4 seguem pendentes; não são evidência
positiva de conclusão. `logos-canon` não será dependência direta do CAS: seus bytes são
opacos aqui, embora o crypto já dependa dele transitivamente.

Padrões a reutilizar: diretórios temporários isolados e limpeza dos testes de
`crates/logos-crypto/tests/security.rs`; vetores TSV e geração independente em
`tools/gen_crypto_vectors.py`; manifesto explícito em `tcb/manifest.toml`.

## 2. Interfaces de saída

Exportar de `logos_ledger` os tipos abaixo; demais helpers ficam privados. Assinaturas
planejadas, não implementação já disponível:

```rust
pub struct Cas { /* raiz e limites privados */ }
pub struct Limits { /* overrides privados, indexados pela tag */ }

pub enum CasError {
    Integrity,
    TooLarge { limit: u64, actual: u64 },
    Io(std::io::Error),
}

impl Default for Limits { /* padrões definidos em spec/cas.md */ }

impl Limits {
    pub fn with_domain_limit<T: logos_crypto::Domain>(
        self,
        limit: u64,
    ) -> Result<Self, CasError>;
}

impl Cas {
    pub fn open(
        root: impl AsRef<std::path::Path>,
        limits: Limits,
    ) -> Result<Self, CasError>;

    pub fn put<T: logos_crypto::Domain>(
        &self,
        bytes: &[u8],
    ) -> Result<logos_crypto::Hash<T>, CasError>;

    pub fn put_reader<T: logos_crypto::Domain>(
        &self,
        reader: impl std::io::Read,
    ) -> Result<logos_crypto::Hash<T>, CasError>;

    pub fn get<T: logos_crypto::Domain>(
        &self,
        hash: &logos_crypto::Hash<T>,
    ) -> Result<Option<Vec<u8>>, CasError>;
}
```

### Contratos propostos para ADR 0005 e `spec/cas.md`

- `root` é a própria pasta CAS: `Cas::open("cas", ...)` produz
  `cas/<domínio>/ab/cdef…`, sem acrescentar outro `cas` ao caminho.
- Endereço: `SHA-256(TAG || bytes)`, exatamente como `spec/domains.md`; nenhum envelope
  extra, salt, normalização ou dupla codificação. Digest em 64 dígitos hexadecimais
  minúsculos; dois primeiros formam a pasta, os 62 restantes formam o nome do arquivo.
- Nome do diretório: resolver **somente tags reservadas exatas** pela tabela existente;
  `logos/certificate/v1\0` corresponde a `certificate-v1`. Não interpretar uma tag
  arbitrária como caminho. Tag desconhecida, inclusive malformada, retorna
  `Io` com `ErrorKind::InvalidInput`, sem fallback. Domínios futuros já reservados
  funcionam com tipos marcadores; domínio novo exige extensão normativa e ADR.
- Padrões: domínios canonical, 16 MiB; certificate e checker (raw), 256 MiB.
  Overrides por tag; limite zero permite somente objeto vazio. Documentar valores em
  bytes e unidades binárias. Limite exato é aceito, primeiro byte além dele é recusado.
- Overrides acima de `isize::MAX` (máximo representável por `Vec<u8>`) retornam
  `Io(InvalidInput)` em `with_domain_limit`, antes de qualquer leitura ou alocação.
  Conversões de tamanho e somas são checadas; falha de reserva retorna `Io` com
  `ErrorKind::OutOfMemory`, nunca `TooLarge`, bytes truncados ou aborto deliberado.
- CAS não valida `canon/v1`: o chamador entrega a codificação correta. A validação
  semântica/canônica pertence à camada consumidora, não ao armazenamento.
- `get`: ausência do objeto final retorna `Ok(None)`; arquivo presente com conteúdo
  incorreto retorna `Integrity`; falhas de acesso/leitura retornam `Io`. Não converter
  permissões negadas ou leitura interrompida em ausência. Aplicar limite também na leitura.
- `TooLarge.actual` é tamanho conhecido para `put`; em leitura limitada, é quantidade
  observada até a recusa (não alegar comprimento total de um fluxo não consumido).
- Cada `Cas::open` bem-sucedido garante a durabilidade da raiz: criar diretórios necessários
  e sincronizar a cadeia da raiz CAS até `/`, de baixo para cima, mesmo se já existir.
  Cada writer sincroniza arquivo, faz rename e sincroniza shard, domínio e raiz CAS,
  nessa ordem, antes de retornar sucesso; nunca depende do writer que criou diretórios.
  Falha de sincronização retorna `Io`, mesmo depois de publicação; não promete rollback.
  Durabilidade supõe filesystem local com rename atômico e suporte a fsync de diretório;
  não prometer equivalência em NFS ou hardware defeituoso.
- Base Unix, coerente com `FileSigner`. Raiz e diretórios ancestrais sob controle do
  operador confiável; não prometer isolamento contra operador/SO malicioso ou troca
  hostil de diretórios. Documentar essa premissa, sem ampliar para sandbox de filesystem.

Essas escolhas são propostas deste plano. Registrá-las em ADR 0005 antes do código;
se a implementação exigir outra escolha, atualizar plano e ADR, sem mudança silenciosa.
Não há mudança em `canon/v1` nem nas tags da 0.4: o novo layout será especificado como
`cas/v1`, com vetores próprios.

## 3. Tarefas em ordem

### 3.1. Fixar contrato normativo

**Arquivos:** criar `docs/adr/0005-escolhas-do-cas.md` e `spec/cas.md`.

Escrever tabela completa tag → diretório → codificação → limite, regra exata de hash,
layout, APIs, publicação, ausência/integridade, precedência de limite e premissas de
filesystem. Preservar independência do auditor: especificação suficiente sem importar
Rust. Não criar tipos concretos de fórmula, certificado ou modelo antes de suas etapas.

**Fecha com:** revisão da tabela contra `spec/domains.md`, especificação de três exemplos
reproduzíveis e ADR sem decisões técnicas implícitas.

### 3.2. Implementar configuração e endereçamento

**Arquivos:** `crates/logos-ledger/Cargo.toml`, `src/lib.rs`; criar `src/cas.rs` e
`src/cas_domain.rs`; `tcb/manifest.toml`. Dependência direta apenas de `logos-crypto`, por path.

Implementar `Limits`, `CasError`, resolver de tags, layout e `Cas::open`. Registrar todo
novo arquivo de produção no manifesto; manter `forbid(unsafe_code)` e lints existentes.
Resolver sem interpretar strings hostis; não adicionar `NAME` ao trait `Domain`.

**Fecha com:** testes em `crates/logos-ledger/tests/cas.rs` para tag desconhecida/malformada,
separação de domínios e limites independentes; `tcb/count.sh check` na verificação final.

### 3.3. Implementar escrita limitada e publicação atômica

**Arquivos:** `src/cas.rs`, `tests/cas.rs` e suporte privado de testes se necessário.

1. Resolver domínio/limite antes de tocar o destino; `put` recusa tamanho excedido antes
   de escrever e não copia a slice para um `Vec` intermediário.
2. `put_reader` lê blocos com buffer fixo, com tamanho de leitura limitado ao saldo
   disponível, alimenta `RawHasher` e guarda no máximo `limit` bytes no `Vec`.
   Ao atingir limite, lê um único byte em buffer separado: EOF aceita tamanho exato;
   byte presente retorna `TooLarge { limit, actual: limit + 1 }`, sem acrescentá-lo
   ao `Vec`, alimentar hash ou solicitar reserva para detectá-lo. Limite zero usa
   diretamente esse probe. Tratar `Interrupted`, overflow e erro do reader sem
   truncamento silencioso; falha não publica objeto parcial.
   Crescer capacidade por metas geométricas limitadas ao limite configurado, usando
   `try_reserve_exact` antes de acrescentar bytes; não pré-alocar limite inteiro para
   objetos pequenos nem realocar a cada bloco. Reserva solicitada não excede limite;
   allocator pode conceder capacidade extra — limite de objeto não é teto total de RSS.
3. Criar diretório de domínio e shard. Não usar existência nem autoria de `mkdir` como
   prova de durabilidade; outra operação pode ter criado diretório e falhado antes de fsync.
4. Criar temporário exclusivo no shard com `create_new`; nome PID + contador atômico,
   repetindo apenas colisões de nome. Nunca compartilhar temporário entre escritores.
5. Escrever todos os bytes, sincronizar arquivo, renomear no mesmo filesystem para
   endereço final; sincronizar shard, domínio e raiz CAS, incondicionalmente, antes do
   sucesso. Raiz e ancestrais já foram sincronizados pelo próprio `Cas::open`.
   Writers concorrentes do mesmo objeto publicam bytes idênticos, sem lock global
   e sem expor arquivo parcial. Qualquer falha de fsync impede retorno de sucesso.
6. Remover temporário em falha quando ainda existir. Preservar erro primário e expor
   falha de limpeza, se houver; não esconder erro de durabilidade ou prometer rollback.

**Leitura limitada:** `bounded_read_stops_at_first_excess_byte` mede bytes consumidos
por reader real controlado, em limite zero e fronteiras; excesso consome só `limit + 1`
bytes e nunca publica. `allocation_failure_is_io` injeta falha de reserva no helper
interno: `put_reader`/`get` devem devolver `Io(OutOfMemory)`, sem dados parciais.
Verificar como diagnóstico que probe não solicita reserva; não transformar capacidade
incidental de `Vec` em contrato de teste permanente.

**Fecha com:** `put_reader_matches_put`, `reader_error_does_not_publish`,
`concurrent_put_same_object` (16 threads, `Barrier`, sem `sleep`), e round-trip após
fechar/reabrir CAS. Observar que concorrência prova atomicidade lógica, não queda de energia.

**Publicação observada:** `read_during_publication_never_sees_partial_object`, com
arquivo real e writer pausado por canais depois de escrever prefixo, antes de completar
escrita/rename. Reader chama `get` durante a pausa: no primeiro put, só ausência; após
publicação, bytes completos. Na substituição de objeto correto existente, reader recebe
bytes completos também durante a pausa; nunca prefixo nem `Integrity` causado pelo writer.
Ponto de pausa deve estar na escrita real, inclusive quando temporário for removido por
mutação, para que escrever diretamente no destino seja detectado. Sem `sleep` ou polling.

**Durabilidade sob concorrência:** `concurrent_directory_creation_is_durable`, com
dois handles CAS independentes e sincronização por canais: A cria domínio/shard e pausa
antes de fsync; B encontra diretórios existentes e publica. B precisa concluir sua própria
sincronização de shard, domínio e raiz antes do sucesso, sem esperar A. A falha em seguida.
`sync_failure_is_io` injeta falha em cada sincronização da raiz/ancestrais no `open`,
arquivo no `put` e diretórios na publicação; observar erro público, inclusive após rename.
Colocar cenários com pontos de controle no módulo de testes unitários de `src/cas.rs`,
sob `cfg(test)`, exercitando arquivos e APIs reais; manter integrações sem hooks em
`tests/cas.rs`. Não criar API pública de debug nem simular o CAS inteiro. Teste de ordem
de chamadas é complementar: só vale junto ao erro observável e ao cenário entre writers.

### 3.4. Implementar leitura com checagem de integridade

**Arquivos:** `src/cas.rs`, `tests/cas.rs`.

Abrir objeto final, ler de forma limitada (não confiar só no metadata), recalcular hash
sobre os bytes efetivamente lidos e comparar com endereço pedido antes de entregar `Vec`.
Recusar arquivo acima do limite; não expor conteúdo parcial. Só `NotFound` na abertura do
objeto corresponde a ausência; falhas posteriores continuam erros.
Reutilizar algoritmo bounded da 3.3, incluindo probe externo, reserva falível e crescimento
controlado. EOF até limite permite checagem do hash; excesso tem precedência e retorna
`TooLarge`, sem calcular digest sobre conteúdo truncado.

**Fecha com:** `corrupted_byte_is_integrity_error`, `missing_object_is_none`,
`oversized_get_is_too_large`, `bounded_read_stops_at_first_excess_byte`,
`allocation_failure_is_io`, `unrepresentable_limit_is_invalid_input`, e erro de I/O
distinguível de ausência. Usar arquivos pequenos e limites de teste, sem alocar 256 MiB.

### 3.5. Provar interoperabilidade e resistência dos testes

**Arquivos:** criar `tools/gen_cas_vectors.py`, `spec/vectors/cas/vectors.tsv`,
`crates/logos-ledger/tests/cas_vectors.rs`; integrar check de vetores em
`ci/verify.sh` e `.github/workflows/ci.yml`, seguindo os checks crypto existentes.

Script Python usa somente biblioteca padrão (`hashlib`), com regra transcrita de
`spec/cas.md`; não importa `logos_client`, Rust ou helpers crypto. Três exemplos:
certificado raw vazio, certificado raw binário e obrigação canônica de bytes explicitados
no documento. Gravar tag, bytes, digest e caminho esperado; oferecer modo `--check`.
Rust grava via CAS e compara endereço e localização reais com os vetores.

**Fecha com:** `cas_addresses_match_independent_vectors` e execução de `--check`.
Mutação manual deve fazer testes falharem ao remover checagem de integridade, omitir tag
no hash, ignorar limite, publicar diretamente sem tmp+rename, omitir fsync de pais quando
diretório já existe e suprimir erro de fsync. Associar cada mutante ao cenário que o mata.
Restaurar conteúdo e executar verificação completa após mutações; não adicionar framework
permanente de mutação. Esses testes detectam violação do protocolo, não demonstram
resistência experimental a power loss; registrar essa limitação.

### 3.6. Fuzz, smoke e fechamento

**Arquivos:** `fuzz/Cargo.toml`; criar `fuzz/fuzz_targets/cas_domain.rs`; docs do módulo,
`STATUS.md`, `ROADMAP.md` e relatório da TCB conforme padrão existente.

Fuzzar fronteira de resolução de tags arbitrárias: tag aceita deve ser reservada,
nome deve ser componente único seguro e tags aceitas distintas não podem produzir mesmo
nome. Isolar resolver puro em `src/cas_domain.rs`, com visibilidade `pub(crate)` e entrada
`&[u8]`; harness inclui esse módulo por `#[path]`, sem exportá-lo de `logos_ledger`.
Não inventar parser de CBOR/hex de hashes: consumir `Hash<T>` já existente. Se surgir outro
parser/decoder, incluir seu target também.

Smoke reproduzível: criar CAS temporário, gravar três exemplos, reabrir, ler bytes exatos,
corromper arquivo e observar `Integrity`; executar script Python para recalcular endereços.
Pode ser exemplo temporário removido após demonstração. Rodar comandos da seção 4,
atualizar STATUS (tabela, pendências, próximo passo e diário), e só marcar critérios
com evidência. Etapa seguinte será 0.6; suas dependências existentes e ARM pendente devem
ficar explícitos. Commit coerente `Etapa 0.5: armazenamento endereçado por conteúdo`,
apenas após verificação completa; sem push não solicitado.

## 4. Critério de conclusão → evidência

| Contrato do roadmap 0.5 | Evidência planejada |
| --- | --- |
| Corromper 1 byte faz `get` retornar integridade | `corrupted_byte_is_integrity_error`; smoke observa o erro após alteração real em disco. |
| 16 threads gravam mesmo objeto sem corrupção | `concurrent_put_same_object`; Barrier, resultados iguais e leitura exata após reabrir. Complementar atomicidade com `read_during_publication_never_sees_partial_object`. |
| Objeto acima do limite dá `TooLarge` | `size_limits_at_boundary`: limite − 1, limite e limite + 1 em `put`/`put_reader`/`get`, além de limite zero; override não afeta outro domínio. `bounded_read_stops_at_first_excess_byte` verifica parada sem consumir resto do fluxo. |
| Endereços de 3 objetos calculados independentemente | `tools/gen_cas_vectors.py --check` e `cas_addresses_match_independent_vectors`, baseados apenas em `spec/cas.md`. |

Comandos de verificação da implementação, não resultados já obtidos:

```sh
cargo test -p logos-ledger
uv run --project py python tools/gen_cas_vectors.py --check
cargo xtask tcb check
cargo xtask tcb report
ci/verify.sh --selftest
FUZZ_TOOLCHAIN=nightly-2026-09-28 ci/fuzz.sh 60 cas_domain
```

CI da etapa deve passar e arquivar ≥ 1 h do novo alvo noturno antes de fechar Definition
of Done global. Smoke local não substitui hora de fuzz nem CI; registrar pendência remota
em vez de marcar conclusão completa. Sem testes permanentes que só conferem texto-fonte,
wiring, nomes incidentais ou ecos de mocks.

## 5. Riscos e decisões em aberto

| Risco / escolha | Tratamento planejado |
| --- | --- |
| Nome derivado de tag permitir traversal ou colisão | Tabela de tags reservadas exatas; rejeição tipada via `Io(InvalidInput)`; teste e fuzz da resolução. |
| Limites virarem leitura/alocação ilimitada | No máximo `limit` bytes no Vec; probe externo de um byte, reserva falível com crescimento controlado, conversões checadas; mesmo algoritmo em `get`. Limite de objeto não promete teto de RSS. |
| Rename sem durabilidade da árvore | `open` sincroniza raiz/ancestrais; cada writer sincroniza arquivo, shard, domínio e raiz, sem depender do criador; corrida controlada e falhas de fsync injetadas. |
| Corrida entre writers / leitura parcial | Temporário exclusivo e rename no mesmo shard; 16 threads, reader durante escrita pausada e mutante sem publicação atômica. |
| Canon inválido ser tratado como certificado válido | CAS prova integridade de bytes, não validade semântica; consumidor faz checagem. |
| Arquivo corrompido já existente no endereço | `get` acusa erro; novo `put` dos bytes corretos pode substituí-lo atomicamente. Nenhum fast path baseado só em existência. |
| Fuzz helper exigir abertura desnecessária da API | Resolver desenho do harness antes de exportar símbolo; sem APIs apenas de debug na superfície pública. |
| Supor suporte fora de Unix ou filesystem local | Não prometer; documentar plataforma e suporte a fsync. ARM continua na 0.6. |

### Decisões propostas para fechar o desenho

- **Representação de `Limits`:** array privado de `u64`, com uma posição por domínio
  reservado, inicializado com os padrões da tabela normativa. O resolver privado retorna
  índice e nome estático; configuração e operações usam o mesmo resolver. Evitar mapas,
  strings alocadas e cópias de tags; nenhum índice faz parte do formato persistido.
  `with_domain_limit` substitui somente a posição do domínio escolhido, depois de validar
  tag e representabilidade. Não duplicar a tabela de resolução entre limites e caminhos.
- **Erro primário + limpeza:** manter as três variantes públicas de `CasError`.
  Se remover temporário também falhar, retornar `CasError::Io` contendo erro composto
  privado, com o `ErrorKind` do erro primário, ambos os erros no `Display` e o primário
  como `source()`. Não converter falha de limpeza em sucesso nem descartar erro original.
  Temporário já ausente (`NotFound`) não é falha de limpeza. Teste interno deve provocar
  falhas reais/controladas de escrita e remoção, conferir ambos os diagnósticos e o kind;
  caso de limpeza bem-sucedida preserva o erro original sem wrapper.
- **ADR:** registrar essas propostas junto aos contratos da seção 2 no ADR 0005 antes
  da implementação; documento de planejamento não equivale a decisão já implementada.

Nenhuma dependência externa nova prevista. Decisões remanescentes são de implementação,
não de escopo; qualquer divergência dos contratos acima exige atualizar plano e ADR.

## 6. Estimativa revisada

Mantida **P**, conforme roadmap. Escopo pequeno, APIs crypto prontas e apenas `std` para
filesystem. Principal risco é contrato de durabilidade/erro, não hashing. Validação remota
e janela de fuzz longo são condições de fechamento, não funcionalidades adicionais.
