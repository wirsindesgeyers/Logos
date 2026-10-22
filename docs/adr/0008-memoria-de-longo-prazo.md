# ADR 0008 — extensão proposta de memória de longo prazo

- Estado: proposta; especificada, não implementada. **Direção adotada** em
  2026-10-22 por delegação do mantenedor (ADR 0047); contratos normativos,
  formatos e vetores continuam sendo o entregável da MEM.0.
- Data: 2026-10-04.
- Escopo: [extensão Long-Term Memory](../spec/LOGOS-long-term-memory.md).
- Revisão documental: 2026-10-19, [ADR 0039](0039-memoria-baseada-no-codigo-uniichat.md)
  e [auditoria do source](../research/uniichat-memory.md). Estado da adoção não mudou.

## Contexto

O mantenedor pediu extensão da spec madura, não redesign nem implementação antecipada.
A spec §§3/5/12/13 já separa contexto/retrieval não confiável de justificativa checada.
OptChat oferece histórico original, árvore binária, folding e zoom; CAS/SQLite,
Source/ContentOf/Origin/Says/Extracted e os protocolos existentes resolvem partes
que não devem ser duplicadas. Em 04/out o repo ainda não implementava
kernel/gateway/harness; em 19/out o kernel chega à5.3, gateway/harness seguem
futuros. Não tratar esse contexto histórico como estado atual do STATUS.

## Decisões propostas

- Episodic Memory para história, Knowledge Memory para documentos, Router pequeno no
  harness. Nenhum output desses módulos vira Fact/Policy/Verified/Seal/capability.
- Originais imutáveis; event IDs globais por store/domínio, sequenciador único, árvore
  diádica alinhada e generations imutáveis. Stores diferentes não compartilham ACLs.
- Free nodes sem modelo, compactor contextual separado, view incremental persistida,
  settle e turnos fresh. Budgets reais de UTF-8/frame; Locator explícito em falha,
  nunca oversize/truncamento publicado. Defaults experimentais, não garantias empíricas.
- Revisão do source: ContextView separada e cutoff de pai no end, batches
  HIGH/LOW, due literal `(H+1-start)/child_count`, blocos estáveis de quatro
  linhas e coordenação de aquecimento single flight. Retries não acumulam
  transcript; caches/roles/tools/ACL são contratos explícitos em E§5–6.
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
ou aceita semântica de conteúdo por similarity. O mantenedor pediu a revisão
baseada no código em19/out: etapas **propostas** MEM.0–MEM.7 foram inseridas no
ROADMAP (Apêndice F), com dependências/exclusões e testes. Adoção/formatos ainda
são gate MEM.0; nenhum checkmark de código é criado. Dependências futuras reais
e regras TCB/deny/fuzz/manifesto continuam obrigatórias.

## Adoção da direção (2026-10-22)

Por delegação do mantenedor, a direção deste ADR é adotada: Episodic Memory,
Knowledge Memory e Memory Router fora da autoridade epistêmica, com originais
imutáveis, store isolado e Rust guardando/acessando. Motivo: o produto do ADR 0047
(projetos persistentes e contexto entre sessões) precisa de exatamente essa base, e
construir outra duplicaria custódia/ACL. Isto **não** encerra a MEM.0: decisões de
formato, domínios CAS, vetores, permissões e processos continuam sendo o seu
critério de conclusão. Nenhuma etapa MEM foi implementada.
