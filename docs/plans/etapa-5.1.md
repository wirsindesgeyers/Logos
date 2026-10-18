# Plano da Etapa 5.1 — typestate de claims

2026-10-18; base a74d2b9. Estado: concluída localmente; CI desta etapa não
observada. Escopo ROADMAP5.1.

## 1. Entradas

EventBody/Event assinados (3.1), Verdict/CheckOutcome (0.6) e Ledger::append
durável (3.2), todos implementados. Nenhuma dependência ausente. O appender
existente evita inventar um sink público que pudesse forjar persistência.

## 2. Saídas

- Claim<S> e SignedEvent<S>, campos privados, sem Clone/Copy/Default/decoder.
  S: Draft, Pending, Verified, Refuted, Unknown, Invalidated. Apenas getters.
- Módulo privado transitions: única construção dos claims; Transitions
  interno empresta o writer e vincula os claims à sessão por identidade Arc.
  propose → Draft; submit(Draft) → Pending; accept/refute/unknown(Pending)
  → estado correspondente; invalidate(Verified) → Invalidated.
- Cada operação chama append uma vez e publica o novo tipo só após sucesso.
  Falha consome o estado anterior; eventual commit sem resposta é resolvido
  por replay/reabertura, não por retry automático.
- spec/state-machine.md: tabela estável para os futuros handlers/modelo formal.

## 3. Tarefas e testes

1. ADR0034, tabela e este plano antes do código: explicitar Proposed como
   entrada de Draft, dois eventos do PROPOSE5.3 e limites da autorização5.4.
2. src/state.rs, src/transitions.rs e exports; dependências locais em
   Cargo/deny/lock, manifesto TCB. Validar sessão/época/h(O), desfecho e
   citações antes do append; recibo tipado imutável após commit.
3. Testes unitários com ledger/CAS reais: seis operações, sequências
   pseudoaleatórias e oráculo independente de estados/tags, um evento por
   sucesso e zero por erro; assinaturas, hashes, replay e interleaving.
4. trybuild: construção externa, retag, conversões, Clone/reuso e
   Refuted→Verified impossíveis; controle positivo público. Mutações manuais
   compiladas provam sensibilidade de guardas e de evento único.
5. fmt/clippy/testes direcionados, manifesto/independência/relatório TCB;
   ci/verify.sh --selftest antes de commit; STATUS e critérios com evidências.

## 4. Critérios → evidência

- Construção externa/Refuted→Verified → testes trybuild e controle positivo;
  testes internos conferem consumo/mapeamento dos tipos.
- Evento único → sequências aleatórias no writer SQLite real, contagem por
  seq e conferência de prev/hash/assinatura/tipo; erros não avançam a cabeça.
- TCB → mutações compiladas, manifesto e full verify/selftest. Sem parser,
  decoder ou formato novo: não há alvo de fuzz novo nesta etapa.

## 5. Riscos e decisões

ADR0034: não aceitar Event/CheckOutcome externos como autorização. A API de
transição é interna até sua integração nos handlers5.3/5.4, com precondição
de checker ativo/evidência real para accept/refute. Unknown e Refuted não
têm caminho para Verified. Rejected não é mudança de typestate; evento de
tentativa/política de repetição pertence à5.4. Cascata/recheck pertencem
à5.9/5.10. Nenhum requisito de etapa futura é marcado como implementado.

## 6. Estimativa

M mantida: fronteira de construção/recibos, integração durável e testes de
sequências, incluindo verificação completa. Nenhum pacote externo novo.

## 7. Evidências

- Oito testes unitários passaram (164.96s), incluindo1024 transições em oito
  sequências de128 operações, quatro claims intercalados por sequência;
  cada evento/assinatura/hash/seq/prev e replay de reabertura conferidos.
- Doze casos compile-fail e controle positivo passaram ao gerar expectativas;
  diagnósticos conferidos (campos privados, E0308/E0599/E0277/E0382).
  Reexecução sem overwrite faz parte da verificação integral.
- Clippy -D warnings, manifesto e independência do auditor passaram.
  Nove mutações compilaram e falharam por asserções; fontes restauradas por
  SHA256, receita etapa-5.1-mutacoes.json e logs target/claim-mutations/.
- TCB Rust12245+747130=759375 (+241 próprios); kernel243, build-only64508.
  Nenhum pacote externo novo; canon/core/crypto/ledger/obligation runtime.
- Primeira ci/verify.sh --selftest: exit101 em29.39s; teste do auditor
  encontrou bwrap NETLINK_ROUTE Operation not permitted dentro do sandbox
  Codex. Sem fallback/relaxamento. Log target/verify-5.1-sandbox.* preservado.
  Reexecução integral fora do sandbox autorizada.
- Rodada integral final: ci/verify.sh --selftest terminou com VERIFY_OK e
  exit0 em2415.30s. Logs/exit/tempo em target/verify-5.1-final.*. Oito testes
  unitários passaram novamente em161.07s e trybuild12+controle em6.19s, sem
  overwrite. O snapshot de35 arquivos de código/configuração/especificação
  permaneceu idêntico ao fim (target/claim-verified-sources.json).
- Rust/Clippy/smokes, CNF5k sem divergências, auditor independente, TCB/
  manifesto/deny/audit/vetores e autoteste passaram;252 Python em174.01s e
  25 TCB em2.61s. Avisos conhecidos deny de licença não encontrada e syn
  duplicado, sem falhas. Autoteste bloqueou todas as violações pelo motivo
  esperado; corpus SAT-2002 200/600 concordou em474.75s; produtor SAT300/300
  provas,300/300 contramodelos,300 consistências e sete controles passaram
  (607 casos/907 rechecks).
- Crash10k/24 SIGKILLs/9997 ACKs em119.30s sem perdas; rebuild100k/cinco
  tabelas exatas;1M/20000 consultas p99=36964ns, cold rebuild465562ms fora
  da latência. Ambos os critérios5.1 demonstrados e marcados. Sem novo alvo
  fuzz porque não há parser/decoder novo. CI/ARM/fuzz longo e segundo host4.5
  anteriores seguem pendentes. README/STATUS atualizados; próximo5.2.
