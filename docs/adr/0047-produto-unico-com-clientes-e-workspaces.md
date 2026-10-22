# ADR 0047 — Um produto, um harness, vários clientes e workspaces de projeto

- Estado: **proposto** (2026-10-21); aguarda decisão do mantenedor em WS.0.
- Base: 57c8a03
- Relacionadas: 0002 (linguagens), 0008/0039 (memória, propostas/aceita para
  planejamento), 0037 (trilha científica, aceita), 0045 (MCP de pesquisa), 0048, 0049;
  [`docs/spec/LOGOS-workspace-and-web-ui.md`](../spec/LOGOS-workspace-and-web-ui.md)
  (`WS§`); Apêndice G do `ROADMAP.md`.

## Contexto

O mantenedor pediu que o roadmap descreva o produto como **um único ambiente
conversacional de agentes**, com TUI/CLI e interface web local, verificação formal
integrada ao chat comum, visualizações, projetos persistentes opcionais e um
workspace científico especializado, sem diluir a arquitetura de confiança.

Estado real em 57c8a03: kernel com PROPOSE/CERTIFY/REFUTE/WITNESS e Query/Subscribe
(5.2–5.5), cliente async (10.1), farm (10.2), MCP de pesquisa (5R.1), auditor (3.6) e
templates `render_pt` (9.1a). `logos_agent`, `logos-tui`, `logos-seal` e
`logos-gateway` são esqueletos; nenhuma etapa MEM, 9.1, 5R.2+ ou 10.3–10.8 foi
implementada. O ADR 0037 adotou "LOGOS Research" como primeira vertical; a Fase 5R
prevê "grafo por projeto" (5R.3) e chat/TUI de pesquisa (5R.5). A extensão de memória
(`E§13`) já reserva o namespace `Artifact` no protocolo auxiliar.

Riscos de deixar implícito: criar dois assistentes ("reason" × "research"), um
segundo harness para a web, um banco de projetos concorrente com a memória ou com o
ledger, ou badges de "verificado" produzidos pela interface.

## Decisão proposta

1. **Um harness e um ledger.** Conversa comum, matemática, filosofia, programação e
   pesquisa usam o mesmo `logos_agent` e o mesmo kernel/ledger. A "vertical LOGOS
   Research" do ADR 0037 passa a ser lida como **perfil de pesquisa** (capacidades e
   painéis) desse mesmo agente. O ADR 0037 e a Fase 5R permanecem válidos; esta
   leitura não muda nenhum critério deles.
2. **Clientes são superfícies.** TUI/CLI Rust (ADR 0002) continuam; uma interface web
   local opcional é acrescentada (ADR 0048). Todos consomem o mesmo contrato de
   sessão do harness (WS.1), sem caminho privilegiado ao kernel.
3. **Status vem do kernel.** Clientes exibem status derivado de consulta com cursor;
   nunca constroem `Verified`, assinam, emitem `Seal` próprio ou hospedam a
   interface de assinatura (invariantes W1–W3).
4. **Projetos como camada de organização.** Um Project Workspace Registry guarda
   projetos, vínculos sessão↔projeto, artefatos e relações **por referência** a
   objetos do ledger, da memória e do CAS da memória. Não é ledger, não cria eventos
   de ledger, não é Knowledge Graph e não cria autoridade (W4).
5. **Persistência sobre a memória.** Proposta preferida: namespace `Artifact`/`Project`
   no store da memória (MEM.0/MEM.1), reutilizando custódia, ACL/scope, idempotência,
   pins e GC. Se o ADR 0008 for rejeitado, WS.3 fica bloqueada até nova decisão por
   ADR; não criar um segundo `memory.sqlite`.
6. **Contexto compartilhado reutiliza o Memory Router** (`E§12`) com uma rota de
   projeto (WS.6), condicionada a MEM.5/MEM.6. Antes disso, só anexos explícitos.
7. **Workspace científico é organização.** Painéis sobre os mesmos objetos; tudo
   também acessível pelo chat comum; nenhuma autoridade nova para fontes.
8. **Trilha transversal.** Registrar a trilha como Apêndice G (WS.0–WS.9), sem
   renumerar as Fases 0–23 e sem alterar o caminho crítico do primeiro paper nem a
   ordem da 5R.

## Alternativas consideradas

1. **Produto separado para pesquisa** (rejeitada): duplicaria harness e regras de
   apresentação e quebraria a tese de "um LOGOS". O ADR 0037 já rejeitara produto
   separado.
2. **Projetos dentro do ledger** (rejeitada): misturaria organização de UI com
   eventos epistêmicos, aumentaria a TCB e criaria tentação de "verificar por save".
3. **Store próprio de projetos, independente da MEM** (alternativa de reserva): mais
   simples de começar, mas duplicaria custódia/ACL/GC. Só se a MEM não for adotada,
   com ADR próprio.
4. **Workspace científico como modo obrigatório** (rejeitada): bloquearia o chat
   comum e criaria a impressão de outro agente.

## Consequências

- TCB: nenhuma mudança. Todos os componentes novos ficam fora da TCB de correção e
  de fidelidade; a interface de confirmação da 9.2 permanece a única superfície de
  assinatura.
- Memória: WS.3+ dependem da adoção de MEM.0/MEM.1; contexto compartilhado (WS.6)
  depende de MEM.5/MEM.6, que por sua vez dependem de 8.9/9.2. Isso adia o projeto
  com contexto automático; o roadmap declara essa espera em vez de antecipar.
- Gateway: nenhuma rota de cliente fala com o gateway; ações seguem a Fase 8/9.2.
- 5R: o caderno da 5R.3 é a primeira instância dos tipos de artefato; WS.4 o importa
  sem perda. 5R.5 não ganha a web como critério.
- Custo: trilha grande (WS.0–WS.9). Não está no caminho crítico do paper 1.

## Limites

- Nada implementado por este ADR; nenhuma caixa do roadmap é marcada.
- Organização, visualização e memória não verificam metodologia empírica nem
  tornam o agente infalível (L1–L3 permanecem).
