# Etapa9.1 — pré-checagem e bloqueio do contrato de vocabulário

Estado: não iniciada; bloqueio registrado em21/out/2026, baseced0cf3.

1. Entradas: AIR1.1, vocabulário1.2 e parser1.4 existentes. O contrato9.1
   requer templates por símbolo no campo `render_pt` do vocabulário.
2. Constatação: `SymbolDecl` em crates/logos-air/src/vocabulary.rs contém
   signature/grounding/gloss_pt. O codec vocabulary/v1 codifica três campos.
   spec/vocabulary.md §3.2 define gloss_pt como documentação livre, sem
   validação de idioma, conteúdo, significado ou não-vazio. Não é template.
3. Bloqueio: não há campo/render-context de templates pronto para a9.1.
   Aplicar AGENTS.md §1.3: parar e registrar, sem implementar a dependência
   ausente como parte implícita da etapa nem reinterpretar dados históricos.
4. Próximo trabalho: ADR sequencial para conciliar1.2/9.1; decidir representação
   de render_pt, vínculo com h(V), placeholders/limites/validação, compatibilidade
   e migração. Mapear codecs/consumidores Rust/Python, gramática de superfície,
   vetores e alvos fuzz. Nenhuma decisão de formato foi adotada nesta pré-checagem.
5. Após o pré-requisito: plano de implementação para render_pt/parse_pt,
   gramática fixa, limites, hash versionado, manifesto TCB de fidelidade,
   round-trip100k/profundidade6+vetores, mutações com controles e fuzz≥1h.
   Preparar corpus de50 políticas reais para revisão humana registrada;
   não declarar essa revisão feita por resultados automáticos.
6. Fora do escopo: outras línguas/elegância, formalizador5R.2, autorização9.2,
   atestados9.3, ações/políticas novas/MEM. Nenhum código alterado nesta sessão.

Verificação: somente leitura dos contratos/fontes e conferência do diff de
documentação. Sem testes de runtime/full verify, commit ou push nesta pré-checagem.
Nenhum critério de conclusão9.1 demonstrado ou marcado.
