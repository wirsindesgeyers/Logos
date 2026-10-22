# LOGOS Spec Extension — Workspace, interface web e artefatos

2026-10-21 · proposta · citada como `WS§N`

> **Estado.** Proposta especificada, **nada implementado**. Decisões em
> [ADR 0047](../adr/0047-produto-unico-com-clientes-e-workspaces.md),
> [ADR 0048](../adr/0048-servidor-web-local-nao-confiavel.md) e
> [ADR 0049](../adr/0049-artefatos-visualizacoes-e-documentos.md), todas **propostas**.
> Etapas, dependências e critérios estão no
> [Apêndice G do ROADMAP](../../ROADMAP.md#apêndice-g--interface-web-workspaces-e-artefatos-trilha-proposta).
> Esta extensão não altera a [spec principal](LOGOS-proof-carrying-agents.md) (`§N`),
> a [extensão de memória](LOGOS-long-term-memory.md) (`E§N`), formatos de `spec/`,
> a TCB nem critérios já demonstrados.

## 1. Propósito

O LOGOS continua sendo **um** ambiente conversacional de agentes: um harness, um
kernel, um ledger. Esta extensão acrescenta, **por cima** deles, uma camada de
experiência: clientes TUI/CLI e web local, sessões reabríveis, projetos opcionais
e persistentes, artefatos com proveniência, visualizações interativas, documentos
exportáveis e um workspace científico como organização especializada.

A frase que governa a extensão:

> **A interface mostra, organiza e pede. O kernel decide. O ledger registra.
> A memória lembra. Nenhum deles empresta autoridade aos outros.**

Fora do escopo desta extensão: novo checker, nova lógica, novo evento de ledger,
segunda memória, segundo harness, gateway alternativo, colaboração remota
multiusuário, hospedagem em nuvem, marketplace, revisão editorial automática,
laboratório autônomo e qualquer pretensão de verdade empírica certificada.

## 2. Tese de produto: um LOGOS, várias superfícies

| Requisito | Decisão |
| --- | --- |
| Um chat, um agente | Um único harness (`logos_agent`, Fases 5R/10) atende conversa comum, programação, matemática, filosofia, análise de documentos e pesquisa. Não há "LOGOS Reason" e "LOGOS Research" como assistentes distintos (ADR 0047). |
| Perfil de pesquisa | A "vertical de pesquisa" do ADR 0037 é um **perfil** de capacidades e painéis do mesmo agente; não troca o modelo de agente nem concede autoridade a fontes. |
| Sessões | Cada conversa tem `SessionId`, histórico e estado próprios; é reabrível e nunca é mesclada silenciosamente com outra. |
| Clientes | TUI/CLI Rust permanecem (ADR 0002). Web local é **superfície adicional**, opcional, sem caminho privilegiado ao kernel. |
| Verificação em qualquer conversa | AIR, provas, refutações, explicações e auditoria ficam disponíveis no chat comum sempre que aplicáveis. Nenhum "modo científico" é pré-requisito. |
| Projetos | Opcionais. Chat comum funciona sem projeto e não herda contexto de projeto não selecionado. |
| Workspace científico | Organização visual e funcional (painéis) sobre os mesmos objetos; tudo que ele faz também é alcançável pelo chat. |
| Memória | Reutiliza Episodic + Knowledge + Memory Router (`E§`, trilha MEM) e o Proof Ledger. Nenhuma memória de terceiros (Hermes, resumos próprios do cliente) a substitui. |

### 2.1 MVP proposto

Chat web local + sessões + cartões de claim + LaTeX básico + gráfico declarativo +
salvar e recuperar um artefato. Entregue por WS.1, WS.2, WS.4 e WS.5/WS.7 nas suas
versões mínimas; itens experimentais vêm depois. O MVP é fechado em WS.0.

Ordem de governança: (1) preservar o caminho crítico do roadmap; (2) trabalhar em
camadas — fixtures → status real → chat com harness → projetos duráveis → contexto
compartilhado → pesquisa avançada; (3) nenhuma ferramenta científica separada;
(4) nenhuma segunda memória; (5) dados de projeto não são ledger; (6) decisões de
stack reversíveis, registradas em ADR, sem acoplar a lógica formal a biblioteca de UI.

## 3. Fronteiras de autoridade

| Componente | Pode | Não pode | Base de confiança |
| --- | --- | --- | --- |
| Kernel + ledger | Emitir estados de claim, eventos assinados, respostas de `Query`/`Subscribe` | — | TCB de correção (inalterada) |
| Renderizador `render_pt`/`parse_pt` (9.1) e interface de confirmação do lado do kernel (9.2/9.3) | Produzir e exibir a string assinável | — | TCB de fidelidade (inalterada) |
| Harness (`logos_agent`) | Conduzir a conversa, chamar ferramentas, pedir `PROPOSE`/`CERTIFY`, compor contexto | Escrever fatos, políticas, obrigações ou `Verified`; ter chaves | Não confiável (progresso) |
| Serviço de memória (MEM) | Guardar originais, views, índices; servir navegação/retrieval read-only | Escrever no ledger/CAS do kernel; criar autoridade (`E§14`) | Custódia, não correção |
| Registry de projetos (WS.3) | Organizar projetos, vínculos e artefatos por referência | Criar evento de ledger, `Fact`, `Policy`, `Verified`, `Seal` ou capability | Custódia, não correção |
| Servidor web local (WS.2) | Servir assets, encaminhar sessão ao harness, projetar status consultado | Assinar, ter chaves, escrever no ledger, falar com o gateway, hospedar a página de assinatura | Não confiável |
| Navegador, frontend, renderers de gráfico/LaTeX/PDF, buscadores, LLM | Exibir e transformar dados não confiáveis | Determinar status epistêmico; executar efeitos | Não confiáveis |

Consequências obrigatórias:

1. **Status vem de consulta.** Todo badge de status (`Verified`, `Refuted`, `Unknown`,
   `Pending`, `Invalidated`, consistência registrada) é derivado de uma resposta do
   kernel com cursor/época; texto do LLM ou do harness nunca o produz (W1).
2. **Escrita só pelo protocolo existente.** Pedido de verificação feito na UI vira
   `PROPOSE`/`CERTIFY` via harness, sujeito à regra de citação (5.3). Não existe rota
   web que fale diretamente com o kernel para escrita, nem com o gateway (W2).
3. **Assinatura fora do cliente web.** A string que o usuário assina (9.2) é exibida
   pela interface de confirmação do lado do kernel. O cliente web pode **indicar** que
   há autorização pendente e abrir essa interface; não a hospeda, não a imita e não
   reformata a string (W3).
4. **Seal só do kernel.** Quando a Fase 15 existir, o cliente exibe o `Seal` renderizado
   pela API única da 15.1. Antes disso, só cartões de status (5R.4). Nenhum selo visual
   próprio do frontend; o nome `Seal` não é reutilizado.
5. **Usar o status disponível na etapa vigente.** O contrato de apresentação carrega só
   campos que existem; campos futuros (fidelidade F0–F4, grounding agregado) aparecem
   como "não disponível", não como valor.

## 4. Quatro camadas de dados, quatro semânticas

| Camada | Responde | Quem escreve | O que **não** é |
| --- | --- | --- | --- |
| Episodic Memory (`E§3–7`) | O que foi registrado como acontecido, com originais e zoom | Capture owner da memória | Prova, fato, autorização atual |
| Knowledge Memory (`E§8–12`) | Onde conteúdo relevante pode existir; fontes, versões, extrações | Ingestão autorizada | Verdade da fonte; `retrieval_score` não é escore de verdade |
| Proof Ledger (Fases 3/5) | Quais inferências foram certificadas, sob quais premissas e limites | Kernel | Banco genérico de UI ou de projetos |
| Project Workspace (esta extensão) | Como objetos estão organizados, relacionados e visualizados num projeto | Registry de projetos, por ação explícita do usuário/harness | Uma nova fonte de autoridade; cópia autoritativa de claims |

Nenhum *save*, favorito, pin, importação, resumo, embedding, resultado de busca,
visualização, documento gerado ou associação a projeto converte informação em
`Fact`, `Policy`, `Verified`, `Seal` ou capacidade de ação (W4). Conteúdo
recuperado continua dado não confiável, inclusive para instruções (W8, `E§14`).

## 5. Experiência do usuário

### 5.1 Navegação (conceitual, não layout obrigatório)

- Nova conversa; sessões recentes (reabrir, renomear, arquivar).
- Projetos: criar, buscar, arquivar, reabrir. Ao selecionar: sessões associadas,
  hipóteses, fontes, artefatos, gráficos, experimentos e documentos.
- Biblioteca/arquivos com referências e proveniência (Knowledge Memory, quando existir).
- Auditoria e resultados de verificação ligados a eventos reais do ledger.
- Configurações de modelo, budgets, ferramentas, autorizações e apresentação.

O chat é o centro. Um painel contextual secundário abre visualizações sem tirar o
usuário da conversa. A TUI oferece o mesmo modelo em forma textual; recursos
gráficos ausentes na TUI degradam para tabela/texto/link, não somem silenciosamente.

### 5.2 Modos de visualização (não são modos de autoridade)

| Modo | Mostra | Não é |
| --- | --- | --- |
| Chat | Mensagens e artefatos inline | Um estado do kernel |
| Inspect | Claim, prova, contramodelo, dependências `used`, fontes, status, versão, hashes | Recheck (auditoria é `logos-audit`) |
| Explore | Gráficos e DAGs com seleção e filtro | Recalcular silenciosamente |
| Project | Objetos persistentes e sua organização | Ledger |
| Research workspace | Projeto com painéis científicos (WS§11) | Outro agente ou outro ledger |

### 5.3 Jornadas de referência

**J1 — Chat comum, filosofia.** "Analise este argumento aristotélico, formalize as
premissas quando possível, procure objeções, mostre a árvore de inferência e exporte
um PDF em LaTeX com conclusões e ressalvas." Sem projeto. A resposta separa:
validade formal (status do kernel), fidelidade da formalização (paráfrase 9.1 e
confirmação; F3/Seal só nas Fases 9.3/15) e verdade das premissas (fora do sistema, L1).
A "árvore" mostra o que o certificado disponível sustenta: para LRAT (Fase 4),
premissas `used` e DAG de claims, **não** uma dedução natural; árvores de prova
legíveis dependem de formatos com estrutura (Alethe 14.x, Lean 20.x).

**J2 — Chat comum, análise visual.** "Leia estes dados, gere um gráfico com
intervalos de confiança, explique limitações, deixe-me salvar." O gráfico é um
artefato `Visualization` (WS§9): referência aos dados por hash, transformação,
spec declarativa, versão do renderer e limitações declaradas. O IC é resultado de
uma transformação identificada, não um teorema; "salvar" exige projeto (WS§7.3).

**J3 — Projeto científico persistente.** "Retome o projeto sobre malária no Amazonas,
abra H3, localize as evidências contrárias e compare com o experimento da semana
passada." O harness compõe contexto autorizado por snapshot (WS§8), abre os painéis,
consulta o estado atual de cada `ClaimReference` e mostra as referências originais.
Não inventa acesso a estudos, não confunde resumo com evidência e não apresenta
associação estatística como causalidade provada.

**J4 — Várias sessões, um projeto.** Uma sessão revisa literatura, outra analisa
dados, outra discute hipóteses. Os artefatos salvos são consultáveis pelas três
dentro de snapshots versionados. Não se compartilham prompts, instruções do sistema,
credenciais, autorizações ou permissões entre sessões.

## 6. Fluxos operacionais

Cada fluxo tem estados de erro explícitos. "Erro explícito" significa mensagem e
código estruturado na UI e no stream; nunca um valor vazio que pareça sucesso.

| # | Fluxo | Estados que precisam existir | Primeira etapa |
| --- | --- | --- | --- |
| F1 | Criar projeto; associar/desassociar sessão | vazio; conflito de revisão; projeto arquivado; sessão já vinculada; falha de armazenamento sem ack | WS.3 |
| F2 | Continuar projeto em nova sessão | snapshot resolvido; degradação (índice/fonte ausente); acesso revogado; sem projeto ativo | WS.6 (anexos explícitos já em WS.4) |
| F3 | Salvar do chat hipótese/fonte/gráfico/prova/experimento/documento | dados transitórios não preservados; tamanho acima do limite; sem projeto; timeout de armazenamento | WS.4 |
| F4 | Recuperar artefato, expandir original, verificar origem | original íntegro; adulterado (digest diverge); ausente; external best-effort indisponível | WS.4 |
| F5 | Comparar duas versões de hipótese/gráfico | revisões incompatíveis; dados de uma versão irrecuperáveis | WS.4/WS.5 |
| F6 | Retratar/corrigir evidência ou hipótese | nova revisão + histórico preservado; claims dependentes só mudam pelo fluxo 5.9 do kernel, nunca por "invalidação" de UI | WS.4 |
| F7 | Exportar/compartilhar projeto | redaction/ACL explícitas; artefato restrito excluído com aviso; compartilhamento remoto **fora do escopo da v1** | WS.9 |
| F8 | Abrir claim e navegar a premissas, resultados, limites e hashes | estado atual consultado; claim retratado/invalidado; kernel indisponível ("status não consultado") | WS.1 (inline), WS.5 (explorador) |
| F9 | Exportar relatório formalizado | texto livre marcado; sem teorema sem `ledger_ref`; compilador falhou; status como snapshot datado | WS.7 |
| F10 | Alternar ou retirar projeto ativo | próxima chamada sem contexto do projeto anterior; caches invalidados | WS.4 (anexos), WS.6 (router) |

Casos ainda não suportados (ações reais, autorização assinada, selo completo,
FOL geral, Lean, estatística certificada) aparecem como "indisponível nesta versão"
com o nome da etapa que os entrega, nunca simulados.

## 7. Projetos e artefatos

### 7.1 Registry de projetos

Um **Project Workspace Registry** organiza referências estáveis a objetos que já
existem em outras camadas. Não é Knowledge Graph, não é banco paralelo de provas e
não é ledger. Persistência proposta (ADR 0047, a confirmar em WS.0 junto com MEM.0):
namespace `Artifact`/`Project` no store da memória (`E§13` já reserva o namespace
`Artifact` no protocolo auxiliar), reutilizando custódia, idempotência, ACL/scope,
CAS da memória, pins e GC de MEM.0/MEM.1. Se o ADR 0008 for rejeitado, WS.3 fica
bloqueada até nova decisão por ADR; não se cria um `memory.sqlite` concorrente.

### 7.2 Contratos propostos (não normativos)

Tipos de trabalho para a spec normativa futura (`spec/project-registry.md`, a
criar em WS.3/WS.4 com vetores). Seguem `E§19`: envelope
`[format_version, type_tag, record]`, campos ASCII fixos, enums discriminados,
campos desconhecidos rejeitados, `canon/v1`, SHA-256 com domínio próprio, **sem
floats** em registros canônicos; dados numéricos ficam em blobs originais (CSV,
Parquet, JSON etc.) referenciados por hash, e parâmetros canônicos usam inteiros ou
decimais em texto com escala declarada.

```text
Project {
  project_id: Id128, owner_scope, title, description?, created_at: Time,
  state: Active | Archived, revision: u64, visibility_policy_ref?,
  memory_store_ref?, knowledge_snapshot_ref?, metadata
}
ProjectSessionLink {
  project_id, session_id, linked_at, actor, context_policy,
  valid_from_revision, unlinked_at?
}
ProjectArtifact {
  artifact_id, project_id, type, title, revision, source_refs[],
  original_artifact_ref?: ArtifactReference, manifest_ref?, created_by, created_at,
  parent_revision?, epistemic_links[], limitations[], status_hint?,
  integrity_state, scope_id
}
ProjectRelation { from_id, relation_type, to_id, provenance_ref?, revision }
SavedVisualization {
  artifact_id, data_refs[], transformation_ref?, specification_ref,
  renderer_id, renderer_version, parameters_ref?, output_snapshot_ref?, limitations[]
}
ProjectContextSnapshot {
  project_id, project_revision, memory_snapshot_refs[], knowledge_snapshot_refs[],
  selected_artifact_refs[], selection_recipe_ref, scope_id, generated_at, degradation[]
}
```

`status_hint` é o status visto no momento do save, rotulado como histórico; **nunca**
é exibido como status atual (W1). `epistemic_links` aponta para `ClaimRef`/`EventRef`
do ledger, `DocumentKey` e `EventId` da memória, sem copiar seus corpos.

Decisões que a spec normativa precisa fechar antes de código: derivação de IDs
(atribuídos × derivados de payload acíclico, `E§19`), domínio de hash, limites de
título/metadata/arrays/relações, ciclos permitidos em `ProjectRelation` (proposta:
grafo arbitrário, mas `parent_revision` acíclico), exclusão lógica × arquivamento,
pin/GC, referências cruzadas entre stores (hash igual **não** é permissão,
`E§14`), e migração de formato.

### 7.3 Tipos de artefato

| Tipo | Conteúdo mínimo | MVP | Depois |
| --- | --- | --- | --- |
| `ResearchQuestion`, `Hypothesis` | formulação, versões, incertezas, relações; previsões opcionais | WS.4 | painéis WS.8 |
| `EvidenceReference` | fonte original, fragmento/locator, proveniência, limitações; status de **extração**, não de verdade | WS.4 (locator externo `best_effort` sem MEM.6) | `DocumentKey` resolvível com MEM.6 |
| `ClaimReference` | identidade no ledger (`h(O)`/evento), fórmula, `h(V)`, checker, status consultado | WS.4 | Seal com 15.8 |
| `Dataset`/`DataTable` | blob original, esquema, licença, origem, hash, unidades, versão | WS.4 | atestação de origem via 6.x |
| `Analysis`/`Experiment` | protocolo, inputs, ambiente, script, seeds, resultados e falhas | — | WS.8 (execução isolada exige etapa própria) |
| `Visualization` | spec declarativa, dados, transformações, renderer, parâmetros, snapshot derivado opcional | WS.4 (persistência), WS.5 (interação) | — |
| `ProofDocument`/`Report` | fonte estruturada, template, bibliografia, claims referenciados, PDF derivado | WS.7 | — |
| `BibliographyCollection`/`DocumentCollection` | referências e versões; extrações rotuladas como extrações | — | WS.8 com MEM.6 |

O caderno da 5R.3 (hipóteses, previsões, evidências, assumptions, conclusões) é a
primeira instância concreta desses tipos; WS.4 o importa sem perda (IDs, status
consultáveis, proveniência) em vez de criar um segundo modelo.

### 7.4 Operações distintas

| Operação | Efeito | Não faz |
| --- | --- | --- |
| Save | Cria artefato/revisão persistida; confirma **só após** armazenamento durável | Tornar a afirmação verdadeira |
| Pin | Protege bytes/originais contra GC | Mudar status |
| Reference | Liga objeto existente (ledger, memória, outro artefato) sem copiar | Copiar autoridade |
| Promote to hypothesis | Cria `Hypothesis` a partir de texto/claim | Criar claim no kernel |
| Request verification | Harness faz `PROPOSE`/`CERTIFY` normais; resultado é evento do kernel | Atalho para `Verified` |
| Retract (artefato) | Nova revisão marcada retratada; histórico preservado | Invalidar claims (isso é 5.9, no kernel) |
| Archive / Unlink | Some da navegação padrão | Apagar originais, ledger ou artefatos pinados |

Autosave de rascunho é permitido para texto do usuário e não substitui confirmação
de Save/Request verification. Se uma visualização depende de dados transitórios
(por exemplo, saída de ferramenta não capturada), o Save informa o que precisa ser
preservado e recusa salvar como reprodutível sem esses bytes. Atualização cria
revisão; links antigos preservam identidade e podem indicar obsolescência.

## 8. Contexto de projeto compartilhado (Project Context Router)

O roteador de contexto de projeto é uma **rota adicional do Memory Router**
(`E§12`: none/episodic/knowledge/both), não um terceiro sistema. Implementação
real depende de MEM.5/MEM.6 (WS.6). Antes disso, a única continuidade entre
sessões é o **anexo explícito** (WS.4): o usuário escolhe artefatos e o harness
inclui só esses, com ACL revalidada.

Composição, sob autorização e orçamento explícitos:

```text
input atual + políticas/instruções vigentes
  + snapshot autorizado da Episodic Memory
  + recuperação da Knowledge Memory
  + artefatos relevantes do Project Workspace
  + referências consultáveis do Proof Ledger (estado consultado agora)
  => contexto com origem, versão, limites e degradações
  => o LLM decide os próximos passos; o kernel verifica claims/efeitos
```

Requisitos:

1. Projeto ativo e domínio de autorização são explícitos; sessão sem projeto não
   herda contexto por acidente (W9).
2. Snapshots/revisões/cutoffs estáveis e referências aos originais.
3. Recuperação limitada por budget, ranking e quotas; registrar fontes
   selecionadas e falhas; nunca alegar recall completo.
4. Sem duplicação: o mesmo conteúdo não entra como resumo episódico, chunk e
   artefato ao mesmo tempo (deduplicação por origem declarada).
5. História narrada, documento recuperado, hipótese, atestação empírica e prova
   certificada chegam ao LLM como tipos distintos, nunca fundidos (M11).
6. Versões conflitantes, fontes ausentes, retratações e invalidações aparecem como
   degradação; nunca continuidade inventada.
7. Conteúdo recuperado não injeta roles de sistema ou instruções privilegiadas (M12, W8).
8. ACL revalidada em cada fetch, zoom, render, export e cache; troca de projeto ou
   revogação invalida contexto e cache incompatíveis (W5).
9. `ClaimReference` recuperado consulta o estado vigente; descrição salva ou resumo
   antigo não é status atual.
10. Métricas: latência, qualidade de recuperação com gabarito, custo e taxa de
    referência ao original — não só cache hit (Apêndice D).

Isolamento e colaboração futura: a v1 assume instalação/usuário local conforme
decisões existentes, mas IDs e escopos não impedem colaboração futura.
Compartilhar conhecimento de projeto não compartilha credenciais, autorização,
identidade ou política.

## 9. Visualizações

### 9.1 Famílias

- Quantitativas: séries, dispersão, distribuição, barras, intervalos, resíduos,
  diagnósticos e comparações.
- Exploração de tabelas: filtros, unidades, faltantes, transformações, origem.
- Grafos: hipóteses ↔ premissas ↔ evidências ↔ conclusões; DAG de claims com
  dependências normativas `used` (4.7) e versões.
- Lógica/matemática: fórmulas, sequentes, contramodelos (2.6), limites do checker,
  premissas em aberto; árvores de prova só para formatos que as têm (WS§5.3 J1).
- Linha do tempo da pesquisa: versões, experimentos, retratações, decisões.
- Domínios especializados (mapas etc.) só com dados e ferramentas compatíveis;
  não fazem parte do MVP.

### 9.2 Contrato de visualização declarativa (`spec/visualization.md`, futuro)

Lista **fechada** de componentes suportados e dados referenciados. Cada
visualização persistida declara: tipo e `schema/version`; fontes de dados e
digests; transformação/receita reproduzível; parâmetros, escalas, unidades e
semântica estatística (o que é média, IC de qual método, n); versão do renderer e
resultado de validação; identidade do artefato e política de acesso; snapshot
exportável (SVG/PNG/PDF) marcado como **derivado**; vínculos a claims, evidências e
limitações. A escolha de bibliotecas (Vega-Lite, ECharts, Cytoscape, KaTeX,
MathJax ou alternativas) é do ADR 0049/WS.0, por critérios, não por estética.
Renderizar uma fórmula para leitura não substitui `render_pt` (9.1) em atos de
assinatura.

### 9.3 Segurança de renderização

- Nenhum JavaScript enviado pelo LLM ou por documentos recuperados roda como
  componente confiável. Componentes são código do cliente; a spec carrega só dados.
- Markdown/HTML sanitizados; CSP sem `unsafe-inline`/`unsafe-eval`; conteúdo que
  precise de execução ativa fica em sandbox isolado sem acesso aos dados do projeto.
- SVG, links, imagens e fontes não fazem requisições externas por padrão (sem
  exfiltração por URL).
- Limites de bytes, profundidade, nós, arestas, séries, pontos e frames; excesso é
  erro declarado, não travamento nem truncamento silencioso.
- Nenhum badge `Verified` vindo do modelo; status derivado de consulta e marcado
  com idade/cursor quando possivelmente obsoleto (W1).
- Controles interativos com efeito real seguem os gates da Fase 8/9.2; nunca
  callback direto ao gateway.

### 9.4 Inspeção e obsolescência

Selecionar ponto, nó ou fórmula abre detalhe e fonte original. Quando dados,
vocabulário (`h(V)`), certificado ou referência mudam, a UI distingue
**congelada** (snapshot fiel ao estado salvo), **atualizada** (recalculada a pedido,
nova revisão), **stale** (entradas mudaram, não recalculada) e **irrecuperável**
(originais ausentes). Não recalcula silenciosamente nem exibe um gráfico novo como
se fosse o antigo (W7). Mesma regra dos cartões da 5R.4.

## 10. Documentos, LaTeX e PDF

- Relatórios técnicos, análise filosófica, demonstrações e documentos científicos
  com fonte estruturada e editável; PDF é **derivado** da fonte e das referências.
- Formato-fonte (LaTeX, Typst ou outro) decidido no ADR 0049/WS.0.
- Enunciados de claims vêm do renderizador da 9.1 (quando existir) ou da AIR
  impressa, com `ledger_ref`; texto do LLM é marcado como tal. Nenhuma inferência
  não verificada aparece como teorema.
- Equações, bibliografia, figuras com legenda/fonte, tabelas e notas de limitações.
- Reprodutibilidade: template, versão do renderer/compilador, artefatos referenciados
  e opções de compilação registrados. Se o PDF não for byte-idêntico entre execuções,
  isso é declarado e a reprodutibilidade é medida sobre a fonte e as versões.
- Compilação em sandbox (padrão bwrap/prlimit da 4.5/10.2): sem chaves, sem rede,
  sem shell-escape, sem leitura fora dos inputs declarados, com limites de tempo/RSS.
- O PDF registra o status das afirmações **no momento da geração** (cursor/época do
  ledger). Retratação posterior não altera bytes antigos, mas o projeto marca o
  relatório como desatualizado. Nada de "selo" desenhado no PDF.

## 11. Workspace científico

Camada opcional sobre o chat; todas as tarefas científicas continuam possíveis
pelo chat padrão (inventário verificado em WS.8).

| Painel | Conteúdo | Entrega |
| --- | --- | --- |
| Overview | pergunta, objetivos, hipóteses ativas, pendências, artefatos recentes | WS.8 (MVP) |
| Hypothesis Board | hipóteses, assumptions, formulações alternativas, previsões, versões | WS.8 (MVP) |
| Evidence Explorer | estudos/datasets/resultados com fonte, método, limites declarados, contradições identificadas; sem *truth score* | WS.8 (MVP) |
| Argument/Proof Graph | claims, status, certificados, contramodelos, dependências, premissas abertas | WS.5 + WS.8 |
| Experiments & Analysis | protocolos, scripts, datasets, ambientes, parâmetros, resultados, falhas | depois do MVP; execução exige etapa própria |
| Bibliography & Documents | biblioteca, originais, extrações, citações, documentos gerados | WS.8 com MEM.6 |
| Visualization Gallery | gráficos persistentes comparáveis | WS.5 + WS.8 |
| Research Timeline | decisões, versões, retratações, experimentos por sessão | WS.8 |

Estatística/desenho experimental continuam **fora do escopo** do repositório
(5R.3). Não há "LOGOS Science RFC" no repositório; qualquer capacidade científica
avançada exige especificação e etapa próprias, não esta extensão.

Distinções que a UI e o harness preservam:

| Rótulo | Significa | Não implica |
| --- | --- | --- |
| provable under Γ | inferência certificada relativa a premissas | verdade de Γ |
| empirical evidence | observação/estudo com proveniência, método e incerteza | φ |
| hypothesis | proposição investigável | conclusão |
| statistical association | correlação medida sob um método | causalidade |
| source says φ | atribuição (`Says`, 6.6/5R.3) | φ |
| model suggests φ | saída de LLM/modelo | φ |
| user-saved φ | φ guardado num projeto | φ |

Um gráfico matematicamente correto não comprova que o dataset representa o
fenômeno. Hash ou assinatura garante identidade/integridade sob premissas, não
metodologia adequada nem verdade empírica (L1).

## 12. Runtime local e clientes

```text
┌───────────────────────┐    ┌───────────────────────┐
│ TUI / CLI (Rust)      │    │ Web UI local (opcional)│
└───────────┬───────────┘    └───────────┬───────────┘
            │  contrato de sessão (WS.1)   │ via servidor web local (WS.2)
            └───────────────┬────────────┘
                            ▼
              Harness (logos_agent, não confiável)
              ├─ visualizações / relatórios (não confiáveis)
              ├─ registry de projetos (custódia, não autoridade)
              └─ Memory Router (MEM), rota de projeto (WS.6)
                            │
            ┌───────────────┴───────────────┐
            ▼                               ▼
   Episodic/Knowledge + projetos       Kernel + Proof Ledger
   (store da memória, MEM)             (Query/Subscribe; escrita só via protocolo)
                                            │
                                   políticas/gateway (quando existirem)
```

Diagrama conceitual; processos, IPC e linguagens finais são decididos no ADR 0048
e em WS.0 à luz do código. Regras:

- Bind em `127.0.0.1`/`::1` por padrão; bind não loopback é recusado sem opção
  explícita e aviso; "localhost" não é autenticação.
- Autenticação do navegador por segredo de sessão gerado na inicialização
  (entregue ao usuário pelo terminal), cookie `HttpOnly`/`SameSite=Strict` ou
  equivalente decidido no ADR 0048; allowlist de `Host` e `Origin`
  (DNS rebinding, CSRF, cross-site WebSocket); sem CORS permissivo.
- API com schemas estritos, limites de payload/stream/conexões, sem endpoints
  administrativos genéricos; assets servidos de diretório fixo sem path traversal.
- Streaming com IDs de evento e cursor, reconexão sem duplicar mensagem final ou
  artefato, cancelamento, idempotência e distinção incompleto × concluído.
- Abas/sessões isoladas: eventos e autorizações de uma não aparecem em outra.
- O processo web não recebe chaves do kernel/gateway, não abre os arquivos do
  ledger/CAS do kernel e não envia mensagens de escrita ao kernel (W2).
- Linux é a plataforma inicial, como no resto do roadmap.

## 13. Invariantes

Prefixo `W` (workspace/web), distinto de `P1–P8`, `S1–S4` e `M1–M13`.

- **W1.** Status epistêmico exibido por qualquer cliente é derivado de consulta ao
  kernel/ledger com cursor/época; texto do LLM ou do harness nunca recebe badge
  verificado, e status histórico salvo é rotulado como histórico.
- **W2.** Servidor web, frontend e renderers não têm chave, credencial nem
  capacidade de escrita no ledger; não falam com o gateway; escrita no kernel só
  pelo harness e pelo protocolo existente.
- **W3.** Strings assináveis (9.1/9.2/9.3) são exibidas byte a byte com hash; a
  assinatura acontece na interface do lado do kernel, nunca no cliente web.
- **W4.** Save, pin, reference, vínculo a projeto, importação, resumo, embedding ou
  visualização não criam `Fact`, `Policy`, `Verified`, `Seal` nem capability.
- **W5.** Objetos e contexto de um escopo não são resolvidos em outro sem
  autorização vigente; revogação vale para handles, caches e snapshots antigos.
- **W6.** Artefato persistido é recuperável até seus originais (dados, spec,
  receita) ou falha com erro declarado; nunca há substituição silenciosa.
- **W7.** Projeção obsoleta é marcada stale; nunca é recalculada silenciosamente
  como se fosse a anterior.
- **W8.** Conteúdo recuperado, importado ou renderizado é dado: não vira instrução
  privilegiada nem executa código no cliente.
- **W9.** Chat sem projeto funciona e não herda contexto de projeto não selecionado.

## 14. Ameaças

Ameaças `TW1–TW13` e atacantes `AW1–AW3` estão em
[`docs/threat-model.md`](../threat-model.md#superfície-de-interface-web-e-workspaces-proposta)
e no [Apêndice B](../../ROADMAP.md#apêndice-b--matriz-de-ameaças--etapas). Limites
em [`docs/limits.md`](../limits.md#interfaces-projetos-e-visualizações-proposta).

## 15. Testes mínimos (atribuídos à primeira etapa em que são testáveis)

| Teste | Etapa |
| --- | --- |
| Fixtures Verified/Refuted/Unknown/Pending/Invalidated/stale/erro renderizadas; texto livre nunca com badge; status forjado no payload exibido como não consultado | WS.1 |
| Strings marcadas como assináveis passam byte a byte (multibyte, bidi, espaços) | WS.1 |
| Origin externa, Host rebinding, CSRF, WebSocket cross-site, path traversal, payload grande recusados; bind não loopback exige opção | WS.2 |
| Reconexão sem duplicar final/artefato; cancelamento distingue incompleto; abas isoladas | WS.2 |
| Inventário de rotas: nenhuma escrita no ledger, gateway, assinatura ou mint por rota web | WS.2 |
| Projeto A não resolve objetos de B; ID/escopo forjado e acesso revogado recusados; crash/restart preserva o que teve ack | WS.3 |
| Chat sem projeto funciona e não recebe contexto de projeto | WS.3 |
| Salvar/reabrir/versionar/recuperar original; adulteração e ausência dão erro; GC não remove pinado | WS.4 |
| `ClaimReference` reaberto mostra estado atual (retratado/invalidado), Refuted/Unknown distintos de Verified | WS.4 |
| Anexo explícito: só artefatos escolhidos entram no contexto; retirar projeto limpa a próxima chamada | WS.4 |
| Valores plotados conferem com os dados; mudança de dados/`h(V)`/certificado marca stale | WS.5 |
| HTML/SVG/Markdown malicioso não executa nem faz requisição externa; limites de complexidade recusam excesso | WS.5 |
| Três sessões do mesmo projeto recuperam artefato; outro projeto não recebe; snapshot antigo pós-revogação falha | WS.6 |
| Injeção em PDF, histórico, legenda, dataset, resultado MCP e nome de projeto permanece dado | WS.6 |
| Índice/provider indisponível degrada explicitamente | WS.6 |
| Compilador recusa shell-escape/leitura externa/rede; PDF com status datado; nenhum teorema sem `ledger_ref` | WS.7 |
| Retratação de premissa atualiza painéis sem apagar histórico; associação nunca rotulada causal | WS.8 |
| Export/import/restore com integridade/ACL/redaction; revisão de segurança por ameaça | WS.9 |
| Falha de provider, compactor, banco, indexador ou compilador produz erro explícito | etapa do componente (WS.4–WS.7, MEM) |

Utilidade (WS.9, sem prometer ganho antes de medir): tempo para recuperar
hipótese/fonte; precisão/completude de referências com gabarito; reprodução de
gráfico/documento por terceiro; fidelidade de visualizações aos dados; tempo de
auditoria e identificação de premissas; custo/latência por artefato e sessão;
contexto de projeto × chat sem recuperação; taxa de correções humanas e falhas
explícitas; acessibilidade, desempenho e legibilidade dos estados. Estudos com
pessoas seguem 11.8.

## 16. Decisões em aberto

1. Aceitação dos ADRs 0047–0049 (todos propostos).
2. Linguagem e processo do servidor web; framework/ausência de framework do
   frontend; bibliotecas de gráfico, grafo, matemática e formato de documento (ADR
   0048/0049, WS.0).
3. Persistência do registry no store da memória × store próprio, condicionada à
   adoção do ADR 0008 (MEM.0).
4. Formato do contrato de sessão entre harness e clientes (canon/v1 no socket local;
   projeção JSON para o navegador?) — WS.1.
5. Política de compartilhamento/exportação com redaction (WS.9); colaboração remota
   fica fora até etapa própria.
