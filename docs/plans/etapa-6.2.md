# Etapa6.2 — confiança por tópico

Estado: implementada localmente (2026-10-23). Decisão em
[ADR 0063](../adr/0063-confianca-por-topico.md).

1. **Entradas:** fact/v1, `FactRegistered`, snapshot e índice v4 (6.1); mecanismo
   `Authority` de eventos administrativos (3.1, 5.10).
2. **Saídas:** `logos_ledger::{TrustConfig, TrustRoot}` (trust/v1, tag de hash
   `logos/trust/v1`, CAS `trust-v1`); `EventBody::TrustConfigPublished` (tag17);
   índice ledger-index/v5 (`facts.issuer/topic`, `trust`, `trust_configs`);
   `LedgerEntry::untrusted` e `Reject::Untrusted`; `logos_kernel::facts::{publish_trust,
   register}` (sem `TrustRoots`); consultas `trust_grant`, `trust_version`,
   `fact_trust`; auditor `trust_configs=`; fuzz `trust_decode`; vetores
   `spec/vectors/trust/trust.json` e evento tag17 (Python independente).
3. **Critério → evidência:**
   - *`db-monitor` assinando fato de tópico `backup` → não citável*:
     `logos-kernel/tests/facts.rs::criterion_db_monitor_signing_a_backup_fact_is_not_citable`
     (registro recusado sem evento e sem linha no índice; cada emissor no próprio
     tópico citável com a concessão explícita; v2 tira `db` e o fato antigo deixa
     de ser citável, PROPOSE recusado; auditor com `facts=2, trust_configs=2`;
     índice reconstruído igual; sem configuração ninguém atesta).
   - Admin, versão e formato: `trust_config_needs_the_admin_an_increasing_version_and_a_valid_object`.
   - Defesas fora do kernel: `projection_and_auditor_refuse_a_fact_registered_without_trust`,
     `projection_refuses_used_facts_after_trust_is_withdrawn_and_stale_versions`,
     `auditor_decodes_trust_configs_strictly` (decoder do auditor estrito por conta própria).
   - Vetores: `logos-ledger/tests/trust.rs` (decode/hash, 12 inválidos, regra exata
     por emissor e tópico).
   - Mutações 20/20 (`etapa-6.2-mutacoes.json`); fuzz `trust_decode` 120s/1.095.923
     execuções sem crash.
4. **Riscos:** chave do admin fora do ledger (auditor não a autentica); retirada de
   confiança não derruba claims antigos (6.4); `level` sem semântica ainda.
5. **Tamanho:** `M`, TCB.
