# ADR 0048 — Servidor web local não confiável

- Estado: **aceito** em 2026-10-22, por delegação do mantenedor, com a stack
  abaixo; revisável a pedido dele. Proposto em 2026-10-21.
- Base: 57c8a03
- Relacionadas: 0002, 0035 (transporte canônico), 0043 (cliente), 0045 (MCP), 0047,
  0049; `WS§3`, `WS§12`; Etapas 5.11, 9.2, 10.7, 23.1 e Apêndice G (WS.2).

## Contexto

O produto ganhará um cliente web local, opcional, ao lado da TUI. Um processo HTTP
local é uma superfície nova: navegador, sites de terceiros abertos no mesmo navegador
e outros processos locais podem alcançá-lo. A 9.2 já prevê um servidor local mínimo
para WebAuthn **do lado do kernel**, que mostra a paráfrase e coleta a assinatura;
ele faz parte da TCB de fidelidade (interface de confirmação) e não pode ser
confundido com a interface web de produto.

## Decisão proposta (normativa para a trilha WS)

1. **Processo separado e não confiável.** O servidor web é um processo distinto do
   kernel, do gateway e do serviço de memória. Não recebe chaves, credenciais nem
   capacidade de escrita no ledger; não abre os arquivos do ledger/CAS do kernel.
   Quando a 5.11 existir, roda sob usuário de SO próprio; antes, o teste da WS.2
   demonstra a ausência de acesso por permissões.
2. **Cliente do harness, como a TUI.** Fala com o harness pelo contrato de sessão da
   WS.1. Leituras de status passam por `Query`/`Subscribe` (diretamente, como a TUI da
   10.7, ou via harness, a decidir na WS.0). Não envia mensagens de escrita ao kernel
   e não fala com o gateway.
3. **Não é a interface de assinatura.** A página de confirmação/WebAuthn da 9.2/9.3
   é outro processo e outra origem, do lado do kernel. O cliente web pode indicar
   autorização pendente e abrir essa interface; não a hospeda, não a enquadra (frame)
   e não reformata a string.
4. **Loopback por padrão.** Bind em `127.0.0.1`/`::1`; bind não loopback recusado sem
   opção explícita e aviso. Localhost não é autenticação.
5. **Autenticação e origem.** Segredo de sessão aleatório gerado na inicialização e
   entregue pelo terminal (URL de uso único ou equivalente), trocado por cookie
   `HttpOnly`, `Secure` quando aplicável, `SameSite=Strict`; allowlist de `Host` e
   `Origin` em HTTP e WebSocket/SSE; sem CORS permissivo; tokens anti-CSRF nas
   mutações. Mitigar DNS rebinding, CSRF, cross-site WebSocket hijacking e path
   traversal.
6. **Conteúdo.** CSP estrita sem `unsafe-inline`/`unsafe-eval`; assets embutidos ou
   servidos de diretório fixo, sem CDN em runtime; nenhuma requisição externa por
   padrão.
7. **API.** Schemas estritos, campos desconhecidos rejeitados, limites de payload,
   stream, conexões e taxa; streaming com IDs de evento e cursor, reconexão sem
   duplicação, cancelamento e idempotência; sem endpoints administrativos genéricos.
8. **Comando.** Nome provisório `logos serve`; nome, porta e empacotamento finais em
   WS.0/23.1.

## Escolhas pendentes (WS.0), com critérios

| Escolha | Alternativas | Critérios |
| --- | --- | --- |
| Linguagem do servidor | (a) Rust, coerente com TUI/CLI do ADR 0002, cliente do mesmo contrato de sessão; (b) Python separado do harness; (c) dentro do processo do harness | isolamento de processo, superfície de dependências, reuso do contrato WS.1, testes adversariais de HTTP, ausência de chaves; (c) é desaconselhada por misturar falhas da UI com o harness |
| Frontend | (a) SPA TypeScript com build pinado e assets embutidos; (b) HTML do servidor com interatividade mínima; (c) módulos ES sem build | CSP sem eval, offline, tamanho, acessibilidade, testabilidade headless, manutenção por uma pessoa |
| Transporte navegador | WebSocket × SSE + POST | reconexão por cursor, cancelamento, proteção de origem |
| Projeção de dados | canon/v1 → JSON para o navegador × CBOR no navegador | o navegador não é autoridade; projeção documentada e testada contra o contrato |

Não se escolhe stack por preferência estética; a WS.0 registra a decisão com
alternativas, licenças e consequências, atualizando este ADR.

## Consequências

- TCB: inalterada. O servidor web fica fora da TCB de correção e de fidelidade.
- Risco: superfície web nova (Apêndice B, `TW1–TW13`); revisão de segurança na WS.9.
- A TUI continua sendo cliente suportado; se a revisão de segurança da web falhar,
  a web não é empacotada (plano B do Apêndice E).

## Limites

Nada implementado. Mitigações enunciadas aqui são requisitos de etapas futuras,
não propriedades existentes.

## Decisão de stack (2026-10-22, por delegação)

| Escolha | Decisão | Motivo | Reverter se |
| --- | --- | --- | --- |
| Servidor | Crate Rust próprio fora da TCB (`logos-web`, nome provisório), processo separado; HTTP com `axum`/`tokio` | ADR 0002 põe a interface em Rust; reusa o contrato WS.1 como a TUI; tipos fortes para validar `Host`/`Origin`/limites; ecossistema maduro e licenças MIT. A árvore de dependências fica fora da TCB, mas entra em `deny.toml` com revisão na WS.2 | a árvore de dependências for desproporcional na revisão da WS.2 (plano B: servidor HTTP síncrono mínimo) |
| Transporte navegador | **SSE** (servidor → navegador, `id:` por evento, reconexão com `Last-Event-ID`) + **POST** com token anti-CSRF para comandos | Cursor e reconexão nativos; elimina a classe de sequestro de WebSocket entre origens (TW4); cancelamento e idempotência ficam em POSTs com chave | precisar de fluxo bidirecional de baixa latência que SSE+POST não atenda |
| Autenticação | URL de uso único impressa no terminal → cookie `HttpOnly`, `SameSite=Strict`, segredo de 256 bits por inicialização | Não depende de "localhost"; nada reutilizável fica no histórico do navegador | — |
| Frontend | TypeScript + **Preact**, empacotado com `esbuild` em versão fixada, assets embutidos no binário; sem CDN; identidade visual híbrida *Swiss Lab + Editorial Científico* (`WS§5.4`: monograma `⟦ Λ ⊢ ⟧`, três vozes tipográficas e tokens `glifo + borda + cor`, com contraste anti-spoofing em relação ao recibo de assinatura da 9.2) | Componentes para cartões e painéis com ~4 KB de runtime; sem `eval`, compatível com a CSP estrita; build reprodutível; uma pessoa consegue manter | acessibilidade ou testes headless exigirem outra base |
| Projeção | O servidor converte canon/v1 em JSON documentado por schema (`spec/client-session.md`, WS.1); o navegador nunca é autoridade | JSON é nativo no navegador; a projeção é testada contra o contrato | — |
| Testes de navegador | Playwright headless com registro de rede e CSP; `axe-core` para acessibilidade | Cobre XSS/exfiltração (TW1/TW2) e acessibilidade (WS.5/WS.9) | — |

Versões exatas e a revisão de dependências entram no plano da WS.2; nenhuma
dependência é adicionada antes disso.
