# ADR 0007 — Sintaxe AIR plana e air/v1

- Status: aceito
- Data: 2026-10-04
- Etapa: 1.1

## Contexto

FOL-ms-eq/v1 precisa de sorts, assinaturas, termos e fórmulas canônicos,
alfa-equivalência por De Bruijn e recusa segura de profundidade 10⁶.
Árvore recursiva em Box/Vec pode estourar a pilha ao descartar até uma entrada
rejeitada. Canon/v1 limita aninhamento CBOR a 128; limite lógico configurável
não deve depender acidentalmente desse limite do transporte.

## Decisão

- Term e Formula são sequências **planas em pré-ordem**, validadas e privadas,
  com nós públicos de sintaxe e contagens explícitas de filhos em App/And/Or.
  Eq/Pred contêm termos planos, nunca subfórmulas recursivas.
- Stack explícita de filhos restantes valida raiz única, árvore completa,
  profundidade e número de nós. Estatísticas de termos embutidos são somadas;
  construir um filho com limite maior não burla o orçamento do pai.
- Profundidade: raiz 1, filho +1, termos em Eq/Pred são filhos; max_nodes conta
  nós de fórmulas e termos, não metadata de sorts/assinaturas. Limites padrão
  128/100_000. Zero não admite nó algum. Limites não fazem parte da identidade.
- Sort é nome Text NFC; Symbol inclui nome, kind Const/Func/Pred, lista de sorts
  dos argumentos e resultado Sort/Bool. Identidade por conteúdo, sem arena de
  IDs/vocabulário nesta etapa. Sintaxe mal sortada permanece representável;
  a 1.3 fará sort-check sobre vocabulário 1.2.
- Variáveis usam índice De Bruijn u64 e sort. Binder mais próximo = 0;
  quantificadores carregam sort, nunca nome. Variáveis livres/índices fora do
  contexto não são decididos pelo parser estrutural desta etapa.
- LogicId tem apenas FOL-ms-eq/v1; Term e Formula incluem identificador explícito
  no fio. Decoder recusa lógica desconhecida, versão/tag/aridade inválida e
  estrutura incompleta. CanonDecode usa limites padrão; API explícita admite
  limites customizados e preserva erros estruturais detalhados.
- Formato novo **air/v1**, envelopes `[1, type, ...]`, conforme tabela normativa
  de `spec/air.md` e contrato no plano 1.1. Sort tag 0, Symbol 1, Term 2,
  Formula 3. Records prefixos mantêm profundidade CBOR independente da lógica.
  Formula usa domínio já reservado `logos/formula/v1\0`; sem tags novas nem
  mudança de canon/v1/core/v1/domains.
- Sem ordenação AC, simplificação, equivalência lógica ou redução de binders.
  Alfa-renomeação desaparece porque nomes não existem na identidade canônica.
- Dependências novas de logos-air: somente logos-canon e logos-crypto por path,
  já na allowlist. Nenhum crate externo novo. Gerador de vetores usa biblioteca
  padrão Python com CBOR independente, sem importar implementação de produção.

## Consequências

Representação, Clone, Drop e percurso lógico não recursivos; segurança de stack
não depende de acesso correto a um destrutor especial. Consumidores futuros
percorrem prefixo por contagens, não recebem referências de arena que possam
apontar para outra árvore. Ser plano não torna a sintaxe uma credencial.

Decoder de canon ainda materializa Value antes da conversão AST. Limites de
profundidade/nós não prometem teto global de memória, tamanho de strings/assinaturas
nem orçamento de bytes de rede. O consumidor deve impor limite do frame/CAS.
Vetores, mutações e fuzz validam contratos; não são prova formal de soundness.

## Alternativas rejeitadas

- Box recursivo com Drop manual iterativo: destruição exige stack/alocações
  especiais e pode falhar em caminhos de erro; prefixo plano evita o problema.
- Canon aninhado refletindo árvore: mistura limite de CBOR e limite lógico.
- Índices globais de símbolos ou interning: introduz vocabulário/ownership antes
  da 1.2 e permite identidade depender de tabela externa.
- Parser de nomes/binders na TCB: parser textual pertence à 1.4; consumidores
  externos resolvem nomes para índices, kernel só recebe a sintaxe canônica.
