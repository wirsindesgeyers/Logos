# ADR 0012 — Geração determinística de obrigações

Estado: aceito para Etapa 1.5, 2026-10-05.

## Contexto

ROADMAP 1.5 e spec conceitual §2/§3 exigem que apenas o monitor gere o sequente
canônico. AST/Vocabulary/sort-check 1.1–1.3 existem; ledger assinado 3.3 e
protocolo Proposal 1.6 não existem e estão fora do escopo desta etapa.

## Decisão

- Implementar logos-obligation na TCB; dependências de runtime apenas crates
  workspace logos-air, logos-canon e logos-crypto, já permitidos em deny.toml.
  Desenvolvimento usa trybuild/serde_json nas versões existentes dos testes core.
  Novos arquivos próprios entram no manifesto; nenhum pacote externo novo.
- Entrada mínima Proposal emprestada em memória; LedgerView é snapshot abstrato
  confiável com epoch e lookup tipado/falível. Não implementar ledger, autenticação
  ou rede; view futura valida autenticidade e escopo. Erro de leitura não é None.
- PremiseRef é digest de evento opaco; h(PremiseRef) no roadmap é identidade já
  hasheada, sem dupla hash. Ordenar bytes unsigned e deduplicar antes de lookup.
  Eventos diferentes de fórmula igual não colapsam.
- Fact, Policy, Verified, Assumption são citáveis se atuais/não retratados;
  demais estados são NotCitable. Assumption continua antecedente explícito,
  não fato incondicional. Época só governa elegibilidade, não identidade de O.
- Alvo deve preservar exatamente origem via compatibility_with da 1.2; versão
  e ordem não substituem comparação de significado. Sort-check objetivo e cada
  premissa contra alvo; nenhuma tradução/expansão Defined/simplificação lógica.
- Envelope obligation/v1 em spec/obligation.md, encoder canon/v1, tag existente
  logos/obligation/v1 NUL. Não alterar formatos/versionamento AIR/Vocabulary.
- Obligation próprio/imutável com campos privados; única construção generate.
  Saída Canon/Domain e accessors são leitura, não ingestão. Sem CanonDecode,
  Default, setters, From/TryFrom de dados externos. Sem novo parser de obrigação:
  não introduzir decoder/fuzz target que contradiria fronteira pública.
- Precedência de rejeições: goal, hash(V), refs ordenadas; lookup, retratação,
  época, citabilidade, compatibilidade, sort. Diagnóstico independe da ordem dada.
- 20 vetores Python/hashlib independentes, testes de comportamento e compile-fail
  externo com controle positivo; smoke real, mutação manual e full verify.

## Alternativas rejeitadas

- Receber O do agente ou decodificar O publicamente: abre atalho contra P1.
- Derivar eventos da fórmula/ID textual ou rehashear referência: perde provenance.
- Incluir época/status em O: sequente deixa de ser objeto lógico puro.
- Aceitar Pending/Unknown como hipótese: escopo de especulação 5.8, inseguro aqui.
- Criar protocolo ou ledger agora: anteciparia dependências 1.6/3.3.

## Consequências

TCB ganha geração e codec somente de saída; nenhum solver/LLM entra na confiança.
O e seu hash não concedem Verified/token. Segurança real ainda exige reader
confiável, ledger assinado, checker e monitor futuros. Memória proposta ADR 0008
não é dependência nem implementação desta etapa.
