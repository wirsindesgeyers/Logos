# ADR 0008 — extensão proposta de memória de longo prazo

- Estado: proposta; especificada, não implementada.
- Data: 2026-10-04.
- Escopo: [extensão Long-Term Memory](../spec/LOGOS-long-term-memory.md).

## Contexto

O mantenedor pediu extensão da spec madura, não redesign nem implementação antecipada.
A spec §§3/5/12/13 já separa contexto/retrieval não confiável de justificativa checada.
OptChat oferece histórico original, árvore binária, folding e zoom; CAS/SQLite,
Source/ContentOf/Origin/Says/Extracted e os protocolos existentes resolvem partes
que não devem ser duplicadas. O repo ainda não implementa kernel/gateway/harness.

## Decisões propostas

- Episodic Memory para história, Knowledge Memory para documentos, Router pequeno no
  harness. Nenhum output desses módulos vira Fact/Policy/Verified/Seal/capability.
- Originais imutáveis; event IDs globais por store/domínio, sequenciador único, árvore
  diádica alinhada e generations imutáveis. Stores diferentes não compartilham ACLs.
- Free nodes sem modelo, compactor contextual separado, view incremental persistida,
  settle e turnos fresh. Budgets reais de UTF-8/frame; Locator explícito em falha,
  nunca oversize/truncamento publicado. Defaults experimentais, não garantias empíricas.
- Registry/versões/proveniência e locators; lexical+dense/RRF/rerank; índices reconstruíveis,
  extrações pinadas quando usadas, IEEE754 Bytes derivados e scores sem floats em canon/v1.
- Reusar CAS com raiz de memória isolada do kernel. Novos domínios raw artifact/manifest
  exigem extensão normativa/versionada e vetores antes de implementação; não reutilizar
  certificate/event para armazenar documentos. Nenhum endereço existente muda.
- Custódia/acesso Rust fora da TCB de soundness; Python sem chaves/credenciais;
  chamadas remotas/pagas e egress via mecanismos autorizados existentes. CLI/TUI somente
  adições; nenhum renderizador concorrente de Seal nem protocolo kernel de memória.
- Knowledge Graph não incluído sem ganho medido; manter DAG epistemológico existente.

## Consequências

Extensão completa em E§1–24 com schemas, algoritmos, recovery, invariantes, critérios,
benchmarks e pontos cirúrgicos para §§2–13/novo §15. Não altera AIR/canon/v1/Proof Ledger
ou aceita semântica de conteúdo por similarity. A aprovação do ADR e inserção de etapas
no ROADMAP são explícitas; nenhum checkmark de código é criado. Dependências futuras
reais e regras TCB/deny/fuzz/manifesto continuam obrigatórias.
