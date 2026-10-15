# ADR 0026 — Contrato de fragmentos e fontes de fechamento

- Estado: aceita; proposta A adotada explicitamente pelo mantenedor em2026-10-15
- Etapa: 4.1, com integrações definidas em4.2/5.4/6.1/7.3/7.5
- Data: 2026-10-15
- Base inspecionada: abbc5d4
- Relacionadas: 0009, 0012, 0022, 0023, 0025

## Bloqueio confirmado antes do código

As dependências declaradas1.2/1.5 existem. Entretanto, o segundo critério da
4.1 exige axiomas em O e deps de um claim, rastreáveis à fonte da enumeração;
o terceiro exige prova de ¬P(c), used extraído e compromisso de completude.
Esses caminhos não estão prontos:

| Interface | O que existe em abbc5d4 | O que falta para o critério |
| --- | --- | --- |
| DomainSource da1.2 | FactRef/PolicyRef são bytes32 opacos, sem atestação | Fact/FactRegistered6.1; política/raiz assinada7.2; resolver payload → evento citável |
| generate da1.5 | Premissas copiadas das citações resolvidas por LedgerView; O selado | Ingresso confiável de premissas derivadas com proveniência; não há injeção automática de axiomas |
| Índices3.3 | Projeta o array used de eventos Verified em deps | Extração de used do certificado4.7 e emissão por CERTIFY5.4; índice não cria prova |
| Checkers2.3 | Refutação/consistência por modelo | Certificado de prova da camada1, LRAT4.4/4.5 |
| Fechamento de predicado | Compromissos constam apenas no roadmap | Política Merkle7.2 ou projeção assinada7.5; completion e ponte semântica7.3 |

Na base inspecionada, logos-ground, logos-lrat, logos-kernel e logos-policy
ainda eram esqueletos.
O único struct Policy nos testes CLI é marcador de domínio para GC, não
política assinada. EpochAdvanced.policy_root da3.4 é compromisso opaco, não
PolicyPublished ou prova de completude. logos-audit explicita Verified sem
recheck LRAT; sua assinatura válida não supre um checker de prova ausente.

Há também uma diferença de identidade que não pode ser apagada por cast:
FactRef/PolicyRef identificam objetos; PremiseRef identifica evento citável
(spec/obligation.md §2). Copiar os32 bytes entre eles não demonstra o vínculo.
LedgerView falsa retornando EntryKind::Fact, Assumption tratada como fato,
ou Verified assinado com CheckOutcome::Accepted escolhido pelo teste não
demonstram a integração pedida. Não abrir o construtor selado de O/P1.

Reprodução da inspeção no commit-base (comandos apenas de leitura):

```sh
rtk proxy git show abbc5d4:crates/logos-air/src/vocabulary.rs
rtk proxy git show abbc5d4:crates/logos-obligation/src/generate.rs
rtk proxy git show abbc5d4:crates/logos-obligation/src/types.rs
rtk proxy git show abbc5d4:crates/logos-ledger/src/index_project.rs
rtk proxy git show abbc5d4:crates/logos-ground/src/lib.rs
rtk proxy git show abbc5d4:crates/logos-lrat/src/lib.rs
rtk proxy git show abbc5d4:crates/logos-kernel/src/lib.rs
rtk proxy git show abbc5d4:crates/logos-policy/src/lib.rs
```

AGENTS.md §1.3 manda parar se faltar dependência. A etapa foi bloqueada
antes da implementação, sem critério reduzido ou marcado durante a análise.
O mantenedor adotou explicitamente a proposta A abaixo; o ROADMAP foi ajustado antes do código.

## Decisão A — separar mecanismos genéricos e integrações

Preservar a ordem das fases e todas as exigências de proveniência, atribuindo
os testes integrados às etapas que recebem as fontes/checkers reais.

### Texto adotado para4.1

Entregáveis:

- Classificador estrutural de O/V, conferindo identidade de V e contexto
  tipado, que distingue FiniteDomain/General conforme o contrato de sorts e
  símbolos. Fragmento não é autorização, certificado ou resolução de fonte.
- Construtores lógicos de **candidatos** aos axiomas de fecho de domínio e
  nomes únicos a partir de Enumerated. Cada candidato preserva DomainSource,
  tem fórmula AIR tipada e resultado determinístico, com limites explícitos.
  Não cria PremiseRef de um hash de objeto, não modifica O nem emite claim.
- A inclusão autenticada desses candidatos em O/deps pertence às integrações
  abaixo. Completion de predicados com fontes de fatos/regras pertence à7.3;
  não inventar hoje um formato de compromisso de completude.

Critérios substitutos adotados:

1. Sort Open quantificado classifica General; casos enumerados/contradomínio
   Open e vocabulário incorreto recebem testes positivos/negativos. Não
   implementar roteamento de kernel7.4 nem autorização por simples classificação.
2. Candidatos de fecho/nomes únicos têm AIR exato, sort-check, fonte preservada
   e comportamento correto em enumeração vazia/singleton/múltipla. Em testes
   lógicos, usar um vocabulário de controle com domínio Open e as mesmas
   assinaturas para admitir modelos que violam os axiomas; comparar com
   modelos que os satisfazem pelo avaliador existente. Modelos contra o V
   Enumerated já obedecem ao fecho por construção e não servem como esses
   controles negativos. Isso não é
   demonstração de fonte autenticada ou de used extraído de certificado.
3. Rastrear os dois critérios integrados originais nos destinos a seguir,
   ainda desmarcados. Não declarar a Fase4 capaz de autorizar com fontes
   não resolvidas enquanto essas integrações não existirem.

### Responsabilidades e critérios que permanecem obrigatórios

| Destino adotado | Ajuste concreto ao contrato/dependências |
| --- | --- |
| 4.2 | Recebe classificador/candidatos da4.1; axiomas usados no grounding devem estar explícitos em O. Testes condicionais com premissas explícitas não demonstram fonte factual. Não acrescentar fecho implícito ao grounder para contornar a integração pendente. |
| 5.4 | Recusar uso autorizador de fontes de enumeração ainda não resolvidas. Não promover um DomainSource opaco a confiança; preservar P3/P4. |
| 6.1 | Acrescentar dependências4.1/4.7/5.4/3.3 para teste integrado de fonte Fact: fato assinado/registrado e resolvido pelo evento correto → axiomas em O; prova real usa esses axiomas → used/deps rastreiam a fonte. Fonte ausente/adulterada não é citável. |
| 7.3 | Preservar a ponte semântica existente e acrescentar explicitamente4.7/5.4/3.3: fontes de política7.2 autenticadas → axiomas em O/used/deps; ¬P(c) não tem prova sem completion e tem prova com ela, com certificado LRAT rechecado e fonte do compromisso nas dependências. |
| 7.5 | Após7.3, repetir integração para projeção assinada com as_of:h(head). Não fazer7.3 depender de7.5, que já depende de7.3; distinguir fonte de política e fonte de projeção. |

O critério de axiomas em O/deps fica preservado para os dois tipos de fonte:
Fact na6.1 e Policy na7.3. O critério de negação/used/completude fica na7.3
para políticas e é repetido na7.5 para projeções. Não transferir garantia a
uma etapa cujo objeto ainda não existe, nem fechar critérios com dublês.

Após a aceitação, os textos da4.1 e das etapas destinatárias foram
atualizados **no mesmo ajuste de contrato, antes do código**. O plano e o
contrato técnico abaixo especificam abrangência global do classificador,
limites de axiomas e enumeração vazia. Qualquer extensão futura
da identidade de premissa ou do wire de O exige ADR/spec/vetores/bump/fuzz;
não está autorizada implicitamente por esta divisão de responsabilidades.

## Alternativa B — não adotada: manter contrato integral e redesenhar a ordem

Conservar4.1 bloqueada. Planejar explicitamente como disponibilizar4.4/4.7,
5.4 e fontes6.1/7.2/7.5 antes de fechar os critérios, sem criar os ciclos
4.1 → 7.3 → 4.1 ou7.3 → 7.5 → 7.3. Não pular automaticamente para4.2 ou
implementar fases futuras por conveniência. Reordenação exige outro plano
revisável e atualização do ROADMAP antes de código.

## Contrato técnico da implementação genérica

- Análise conservadora de todo V, inclusive símbolos/Defined não utilizados,
  pois a avaliação existente valida definições globalmente. Const é função
  nula: resultado Open também impede FiniteDomain. Parâmetros Defined são
  universalmente interpretados; parâmetros Open também exigem General.
- Conferir h(V)==O.vocab e tipagem de todas as premissas/goal/Defined antes
  de retornar um fragmento. Erro/limite não vira General; todos os corpos
  Defined são visitados, sem expansão recursiva ou dependência de relevância.
- Candidatos contêm h(V), sort, DomainSource, fecho e desigualdades por pares
  i<j na ordem da enumeração. Fecho vazio é ∀x.False (Or(0)); singleton
  não gera desigualdade. Saída não implementa Canon/Domain/Obligation.
- Limites de entrada e saída em nós/declaracões/constantes/axiomas com
  aritmética checada e recusa antes da expansão quadrática. Recursos
  insuficientes preservam Unknown(Size/Memory) no erro; sem hard RSS/wall.
- Apenas ASTs existentes em memória. Nenhum parser, domínio ou formato novo;
  vetores de fórmulas usam AIR/v1 existente. Fonte opaca e bytes de payload
  não recebem autoridade nem identidade de evento. Nenhuma mutação em O.
- Dependências runtime novas de logos-ground apenas crates locais
  air/canon/core/crypto/obligation, já allowlisted; model/serde_json só dev
  para semântica e vetores. Manifesto lista os novos fontes TCB; fuzz estende
  vocabulary_decode existente, sem antecipar grounder ou parser de CNF.

A estimativa4.1 passa de P a M: dois núcleos TCB (classificação e emissão),
validação global de Defined, limites e testes semânticos/mutações. Fases não
foram reordenadas. O bloqueio original consta como evidência histórica, e a
escolha explícita do mantenedor libera somente o contrato genérico adotado.
