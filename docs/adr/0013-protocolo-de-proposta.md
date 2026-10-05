# ADR 0013 — Proposta transmitida e texto não confiável

Estado: aceito para Etapa 1.6, 2026-10-05.

## Contexto

ROADMAP 1.6 recebe Formula e PremiseRef prontos e exige decoder estrito e schema
sem campos de autoridade. Proposal emprestada de logos-obligation é entrada
resolvida do gerador, não mensagem de agente. Ledger e servidor são etapas futuras.

## Decisão

- Criar logos-protocol fora da TCB, como logos-air-text. Nenhum crate TCB passa
  a depender dele. JSON pode falhar na disponibilidade, mas não constrói O,
  Fact/Policy, status, certificado ou assinatura. Só generate produz O.
- Formato novo proposal/v1: JSON UTF-8 com envelope exato
  {"version":1,"proposal":{text_pt,goal,cites,speculative_on,vocab}}.
  Campos obrigatórios, extras e duplicatas rejeitados em ambos os objetos.
  Não há mensagens alternativas ou framing nesta etapa.
- goal transmite hex minúsculo de canon(Formula), envelope air/v1 completo.
  Decoder AIR existente confere canonicalidade, lógica, estrutura e limites;
  não aceitar texto de fórmula nem objeto de obrigação nessa posição.
- PremiseRef e h(V) usam 64 dígitos hex minúsculos. ClaimId é identificador
  opaco de 32 bytes, com tipo próprio; não é PremiseRef nem prova de existência.
  A atribuição/autenticação de ClaimId pertence ao ledger futuro, não a um
  algoritmo de hash inventado pelo decoder.
- text_pt vira UntrustedText, sem Canon/Domain/Display implícito. UTF-8
  arbitrário é preservado, inclusive NFD, controles escapados e caracteres
  que canon/v1 recusaria. Nenhuma normalização/truncamento ou interpretação.
  O acesso chama-se as_untrusted_str. Texto não integra entrada de generate.
- Limites inclusivos: mensagem 3 MiB; text_pt 64 KiB UTF-8 decodificado;
  goal 1 MiB CBOR; cada lista 1024 refs; AIR 128/100000. São limites de
  admissão, não compromisso de RSS ou tempo. Encode e decode aplicam os mesmos.
- Dependências serde 1.0.229/serde_json 1.0.151 já existem no lockfile/allowlist,
  agora runtime apenas fora da TCB; AIR/canon/crypto/obligation são workspace.
  Não há parser TCB novo ou alteração de formatos/domínios existentes.
  Medição após full verify manteve TCB 4610+78287 e build-only 51581;
  comparação com lockfile anterior mostrou apenas novo crate workspace.
  cargo-deny avisou syn 2/3 já existentes, sem falha; não é aumento de TCB.
- Schema normativo em spec/protocol.schema.json. Restrições de bytes UTF-8,
  CBOR canônico e tipagem exigem decoder/gerador: JSON Schema sozinho não basta.
- Decoder representa speculative_on sem habilitar especulação. Pending continua
  NotCitable; não fornecer adapter que descarte essas marcações silenciosamente.
  Resolver h(V), decidir especulação e autenticar ledger são integração futura.

## Alternativas rejeitadas

- Serde/JSON dentro da TCB: amplia confiança sem necessidade para um candidato.
- Receber O, status ou assinatura: quebra a fronteira P1.
- Usar texto natural como objetivo: mistura fidelidade e prova formal.
- Reusar Proposal emprestada como wire type: omite texto, ClaimId e h(V).
- Normalizar text_pt com Text canônico: altera dado tainted sem necessidade.

## Consequências

Propostas continuam candidatas sem autoridade. Schema/testes fecham P1 preliminar;
não provam autenticidade, verdade ou isolamento de um servidor ainda inexistente.
Novo formato tem versão inicial 1 e vetores fixos; futuros campos exigem versão
nova, não um decoder permissivo. Memória proposta não é dependência desta etapa.
