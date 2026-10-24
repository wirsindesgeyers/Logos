# Etapa6.4 — revogação de fatos e de atestadores

Estado: implementada localmente (2026-10-24). Decisão em
[ADR 0064](../adr/0064-revogacao-de-fatos-e-emissores.md).

1. **Entradas:** fatos e confiança (6.1/6.2), cascata ATMS e `Retractions` (5.9),
   mecanismo `Authority`.
2. **Saídas:** `EventBody::IssuerRevoked` (tag18); índice ledger-index/v6
   (`revoked_issuers`); `facts::revoke_issuer` e `IssuerRevocation`;
   `LedgerLiveIndex::issuer_revoked`; `Retractions::retract` aceita fato (admin);
   `reconcile` trata fato morto como raiz; auditor recusa fato de emissor revogado e
   revogação dupla; vetor de evento tag18 (Python independente).
3. **Critério → evidência:**
   - *Revogar `backup-svc` invalida todos os claims que dependiam de fatos dele, e
     só esses*: `logos-kernel/tests/facts.rs::criterion_revoking_backup_svc_invalidates_exactly_the_claims_that_depended_on_it`
     (claim no fato comprometido e claim transitivo caem com
     `causes = [h(IssuerRevoked), premissa]`; claim de outro emissor e claim de fato
     do mesmo emissor anterior a `since` ficam Verified; só o admin revoga; reconcile
     idempotente; segunda revogação e fato novo do emissor recusados; índice
     reconstruído e auditor com recheck LRAT/cake concordam).
   - Retratação de um fato: `admin_retracts_a_single_fact_and_its_dependents_fall`.
   - Defesas fora do kernel: `projection_and_auditor_refuse_facts_of_a_revoked_issuer_and_double_revocations`.
   - Mutações 11/11 (`etapa-6.4-mutacoes.json`).
4. **Riscos:** `since` por época de registro depende de o operador saber quando o
   comprometimento começou; revogação é terminal (sem reabilitar emissor; rotação de
   chaves fora do escopo).
5. **Tamanho:** `P`, TCB.
