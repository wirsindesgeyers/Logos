# ADR 0039 — contratos de memória conferidos no código do UniiChat

- Data: 2026-10-19.
- Estado: aceita para revisão documental e planejamento solicitado pelo mantenedor.
  ADR 0008 continua proposta; não declara runtime de memória implementado/adotado.
- Fontes e evidências: [auditoria](../research/uniichat-memory.md), UniiChat
  `0500bf6a8ea4bf0ce047488562022b1b6532cc30`, OptMem
  `1fb164cf39028047781f72ac3bb1e5a691c1dcb0`.

## Contexto

A extensão de 04/out seguiu uma descrição OptChat. O código oficial agora
disponível resolve ambiguidades materiais: duas views, histerese, due, blocos,
single flight e retries. Confundir OptMem, a descrição e o UniiChat produz
implementações diferentes sob os mesmos nomes. O mantenedor pediu melhorar
spec/roadmap com base no source. A próxima etapa principal continua 5.4.

## Decisão

1. E§5–6 passa a especificar ContextView persistida separada, cutoff de folha
   no start/pai no end, contexto pronto congelado sem dependência circular;
   retries com blocos iniciais + uma nota substituída, sem transcript crescente.
2. Folding usa o due literal `(H+1-start)/child_count`, com aritmética inteira
   checked e testemunha H=8. Batches HIGH→LOW preservam prefixos entre lotes.
   No LOGOS, frame inteiro escapado tem teto; LOW=max(HIGH/2,F), F é o piso
   conservador de E§6, e publicação de batch espera LOW. ContextView pode
   omitir ranges declarados; ConversationView nunca omite história.
3. Render de cache em blocos de quatro linhas, uma marca na view e tail
   variável após ela. Separar nodes imutáveis, prompt cache remoto, coordenação
   efêmera e caches de retrieval/provas. Single flight só começa no send após
   autorização/orçamento/fila; liberação no evento do adapter, com falhas,
   cancel/deadline e namespace seguro. Chaves SHA-256 dos campos reais do wire.
4. Preservar divergências LOGOS: originais integrais e WAL/FULL; hard byte
   limits, Locator explícito, backoff/deadlines e 8 jobs por store, sem copiar
   clipping, oversize soft, retry infinito, cache global Bun 64 ou credenciais
   no Python. Formatos/provider fields do upstream não são contratos aceitos
   para nosso adapter; precisam de verificação própria quando implementados.
5. Inserir trilha **proposta** MEM.0–MEM.7 no Apêndice F do ROADMAP, com
   dependências, exclusões, testes negativos, simulação e medição real.
   Nenhuma etapa recebe checkmark por esta pesquisa. Referências em etapas
   existentes são pontos de integração futura, sem dependência reversa.

## Consequências

O roadmap passa a conter escopo concreto da memória, sem reordenar Fases 0–23.
MEM.0 decide adoção/encoding/egress antes de código; dependência ausente vira
bloqueio, não implementação antecipada. A TCB/protocolo kernel, endereços CAS
atuais e formatos normativos não mudam nesta revisão. Preservam-se limites
L1–L3 e não autoridade dos resumos/índices. A referência externa fica fixada
por commit/hash; evolução posterior exige nova comparação, não seguir main.

Dados estruturais de 04/out não demonstram a fórmula ou lotes corrigidos.
Testes contra o upstream demonstram observações daquele código, sem fechar
testes de implementação LOGOS. Cache simulado não prova hit/custo real,
recall semântico, durabilidade, compatibilidade de API ou isolamento.
