# ADR 0005 — escolhas do CAS

- Data: 2026-10-02
- Estado: aceito
- Etapa: 0.5

## Contexto

CAS é TCB de correção: precisa preservar bytes, separar domínios, limitar memória de
objetos e publicar atomicamente com durabilidade. Auditor não depende de logos-ledger.
Contrato e premissas normativos em [cas/v1](../../spec/cas.md).

## Decisão

- Implementar em módulo `cas` de logos-ledger, sem crate novo. Única dependência direta
  adicionada: logos-crypto por path, já autorizado em `[bans].allow` de deny.toml.
  Reutiliza Hash, Domain, hash_raw e RawHasher; não introduz bibliotecas externas.
- Resolver somente tags reservadas exatas, retornando índice e diretório estático.
  Domain permanece inalterado. Limits usa array privado de u64 por domínio; padrão
  canonical 16 MiB, certificate/checker raw 256 MiB, overrides até isize::MAX.
- Bytes opacos: CAS não decodifica canon/v1. Hash é SHA-256(tag || bytes), com tags v1
  existentes. Novo layout é cas/v1; não altera canon/v1 nem domains/v1.
- put não copia slice. put_reader/get usam leitura bounded, crescimento geométrico
  com try_reserve_exact e probe externo; reserva falha gera Io(OutOfMemory).
- Unix/local FS e ancestrais confiáveis. open sincroniza raiz/ancestrais até /;
  todo writer sincroniza arquivo, faz rename e sincroniza shard/domínio/raiz, sem
  depender do criador. Concorrência não usa lock global; temporários exclusivos.
- Manter variantes Integrity, TooLarge e Io. Erro primário mais falha de limpeza usam
  erro composto privado em io::Error, kind primário, Display com ambos e source primário.
  Falha pós-publicação não promete rollback.
- Test hooks privados sob cfg(test), sem API de debug, para pausa na escrita real,
  concorrência de diretórios, falhas de sync/reserva/limpeza. Integrações usam API pública.
- Resolver puro privado em cas_domain.rs incluído por path no fuzz harness; nenhum
  parser de hashes/canon novo. Fuzz cobre tags arbitrárias e colisões de diretório.

## Consequências e verificação

Tabela fixa restringe extensão a atualização normativa explícita, evita traversal e
alocações de mapas/tags. Objeto ainda cabe em memória; streaming real é etapa futura.
Sincronizar ancestrais existentes custa I/O, mas elimina dependência de outro writer.
Limite de objeto não promete teto de RSS nem proteção contra operador hostil.

Quatro critérios do roadmap: corrupção de byte, 16 writers, TooLarge e três endereços
independentes via hashlib. Testes determinísticos mais mutações devem detectar remoção
de integridade/tag/limite/publicação atômica/sync de pais/propagação de erro de sync.
Isso prova protocolo lógico e tratamento de falhas, não resistência experimental a
queda de energia. CI remota e hora noturna de fuzz são evidências separadas do smoke local.

## Evidência local — 2026-10-02

`cargo test -p logos-ledger`: 17 testes passaram (6 unitários, 10 de API pública,
1 de vetores). Smoke temporário executado com arquivos reais: três exemplos da spec
gravados por `put`/`put_reader`, reabertura com bytes exatos, corrupção em disco
detectada como `Integrity` e limite zero recusando um byte com `TooLarge`.
Exemplo removido após demonstração; vetores e testes preservam entradas reproduzíveis.

Mutação manual isolada, com restauração após cada falha (todos retornaram exit 101
por falha de asserção, não erro de compilação):

| Mutação | Teste que falhou |
| --- | --- |
| Ignorar comparação de digest em `get` | `corrupted_byte_is_integrity_error` |
| `put` usar `ExternalDigest` sem tag | `cas_addresses_match_independent_vectors` |
| Ignorar `actual > limit` em `put` | `size_limits_at_boundary` |
| Abrir destino com create/truncate em vez de temporário exclusivo | `read_during_publication_never_sees_partial_object` |
| Sincronizar pais só quando shard não existia | `concurrent_directory_creation_is_durable` |
| Ignorar erro da sincronização da raiz na publicação | `every_publication_sync_failure_is_public_io` |

Falhas de reserva, limpeza e cada ponto de fsync são injetadas em hooks privados
`cfg(test)`; operações continuam usando arquivos reais. Não houve experimento de
power loss. CI remota e fuzz noturno ≥ 1 h continuam sem evidência.
