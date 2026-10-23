# Etapa5.12 — kernel mínimo ponta a ponta

Estado: implementação local (2026-10-23). Decisão em
[ADR 0060](../adr/0060-demo-do-kernel-minimo.md).

1. **Entradas:** 5.1–5.11; atestador de teste stub (fatos reais na Fase 6).
2. **Saídas:** `crates/logos-kernel/examples/demo_kernel.rs`, `ci/demo-kernel.sh`,
   `Makefile` (`make demo-kernel`), passo no verify e na CI.
3. **Critério → evidência:**
   - *Script de demo: vocabulário → fatos de teste → 3 propostas (Verified via
     LRAT, Refuted via modelo, Unknown por timeout) → retratação de um fato →
     cascata → `logos-audit` verifica e recheca tudo*: `make demo-kernel` →
     `DEMO_KERNEL_OK events=16 … invalidated=1` e `AUDIT_OK events=16
     refutations_rechecked=1 … failures=0 … cnf_rederived=3 lrat_rust_rechecked=1
     lrat_cake_rechecked=1`, `DEMO_KERNEL_AUDIT_OK`.
   - *Reproduzível com um comando*: `make demo-kernel` (~10 s com o cache quente).
4. **Riscos:** fatos de teste são assumptions do atestador stub (ADR0060); o
   "timeout" é o teto de passos determinístico, não tempo de relógio.
5. **Tamanho:** `P`.
