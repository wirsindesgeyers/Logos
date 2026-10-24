# Etapa6.3 — SDK de atestador e atestadores de referência

Estado: implementada localmente (2026-10-24). Decisão em
[ADR 0065](../adr/0065-sdk-de-atestador.md).

1. **Entradas:** fact/v1, `FactRegistered`, confiança por tópico e revogação
   (6.1, 6.2, 6.4); `Anchored` do vocabulário (1.2); `bwrap`/`prlimit` (4.5).
2. **Saídas:** `logos-attest` (`Attester`, `attest`, `submit`, `Observation`, helpers de
   vocabulário/fórmula; `reference::{clock, fs_stat, db_monitor, backup_svc, enum_source,
   sandbox_run}`; `example_vocabulary`, `REFERENCES`); `docs/attesters/*.md`;
   `logos ledger trust-publish|fact-register|revoke-issuer`; pacote Python `logos_attest`
   (`Attester`, `Ed25519Signer`, `attest`, `Attestation.write`, `Clock`); vetor
   `spec/vectors/attest/clock.json` (`tools/gen_attest_vectors.py`).
3. **Critério → evidência:**
   - *Cada atestador de referência tem teste de integração produzindo um fato citável*:
     `logos-attest/tests/reference.rs` — `clock_attests_sync_and_skew`,
     `fs_stat_attests_presence_hash_and_absence`,
     `db_monitor_attests_sqlite_tables_rows_and_mock_pg_connections`,
     `backup_svc_attests_recent_stale_and_ignores_future_backups`,
     `enum_source_fact_is_the_enumeration_source_the_kernel_requires`,
     `sandbox_run_chains_script_input_and_output_hashes` (cada um registra no ledger real
     com confiança publicada e confere `is_citable`); fora do tópico ou sem confiança o
     registro é recusado sem evento (`an_attester_outside_its_topic_or_untrusted_…`);
     o auditor recheca os fatos (`auditor_rechecks_every_reference_fact`).
   - *O vocabulário de exemplo tem todos os símbolos operacionais `Anchored` ou
     `Defined`*: `example_vocabulary_is_fully_anchored_to_documented_procedures`
     (nenhum `Glossed`; `AttestorId` e procedimento batem com `REFERENCES`; cada
     documento cita id, procedimento e tópico).
   - Operador: `logos-cli/tests/facts.rs` (confiança, registro, repetição, versão não
     crescente, revogação de emissor, chave errada, flags malformadas).
   - Python: `py/tests/test_attest.py` (assinatura verifica com a tag documentada,
     observações inválidas levantam erro, arquivo de chave 0600, agente não importa o
     SDK) e paridade com Rust em `logos-attest/tests/vectors.rs` (bytes idênticos; fato
     do Python decodifica, verifica e vira citável).
   - Mutações 15/15 mortas (`etapa-6.3-mutacoes.json`); duas sobreviveram na primeira
     rodada (fronteira da idade do backup, script alterado durante a execução) e ganharam
     teste.
4. **Riscos:** sem protocolo para registrar fato num kernel em execução (entrega por
   biblioteca ou CLI com o kernel parado); `pg-monitor` é mock; fatos de arquivo/banco
   são TOCTOU por natureza; `sandbox-run` depende de `bwrap`/`prlimit` e não tem
   fallback sem isolamento; chave do atestador é responsabilidade do operador.
5. **Tamanho:** `M`, fora da TCB de correção.
