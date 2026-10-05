# Plano — Etapa 1.6: formato da proposta

Fonte: ROADMAP 1.6; ADR 0013. Etapa P com decisões de formato e confiança.
Sem subagentes, por pedido do mantenedor.

## 1. Entradas

- Formula, Limits e Formula::decode_with_limits (air/v1, Etapa 1.1).
- PremiseRef de evento e generate/Proposal emprestada (Etapa 1.5).
- Canon/Hash<Vocabulary> e JSON serde já no lockfile, fora da TCB.
- Ledger real não é dependência; nenhuma implementação de 3.3/5.2/5.8.

## 2. Saídas

- logos_protocol::Proposal { text_pt: UntrustedText, goal: Formula,
  cites: Vec<PremiseRef>, speculative_on: Vec<ClaimId>, vocab: Hash<Vocabulary> }.
- decode(&[u8]) -> Result<Proposal, ProtocolError>;
  encode(&Proposal) -> Result<Vec<u8>, ProtocolError>.
- ClaimId::from_bytes/as_bytes e UntrustedText::new/as_untrusted_str.
- spec/protocol.md e protocol.schema.json; vectors/protocol/vectors.json.
- proposal/v1 novo, sem mudar air/v1, obligation/v1 ou canon/v1.

## 3. Tarefas em ordem

1. Fixar contrato/ADR e adicionar crate fora da TCB ao workspace/deny.
2. Implementar types/codec/error, decoder estrito e encoder simétrico.
3. Testes: campos extras/duplicados, faltantes/tipos/versões/refs, CBOR errado,
   limites inclusivos e texto NFD/controles. Inspecionar schema conforme contrato.
4. Vetores Python independentes usando encoder stdlib já em gen_air_vectors;
   teste Rust compara wire fixo, AIR bytes e conteúdo decodificado.
5. Smoke público: proposta real → decoder → gerador com snapshot em memória;
   dois textos/ordens produzem mesmo O; Pending não ganha citabilidade.
6. Integrar smoke/gerador na verificação local/CI, rodar full selftest,
   atualizar STATUS/ROADMAP/plano de fase e commit somente após sucesso.

## 4. Critérios → evidência

- Extras rejeitados/contrabando: testes em tests/proposal.rs, ambos os níveis
  e goal contendo envelope de O; smoke tenta campo obligation.
- P1 preliminar: teste inspeciona schema, fecha additionalProperties e percorre
  propriedades/definições/refs para impedir Obligation/Fact/Policy/Signature,
  dependência/status. Único payload de agente é Proposal.
- Taint: teste de O invariável frente a textos incompatíveis com canon/v1,
  sem usar texto na entrada do gerador.
- Formato novo: vetores fixos e --check, versão obrigatória.

## 5. Riscos e decisões

ADR 0013 fixa JSON fora da TCB, hex AIR e ClaimId opaco. Schema é preliminar,
limites em bytes e semântica AIR precisam do decoder. Leitura de vocabulário,
autenticidade e especulação não serão resolvidas aqui. LSP Rust indisponível:
rust-analyzer ausente na toolchain stable; nenhuma API exportada existente muda.
Preservar docs de memória e logs preexistentes fora do commit.

## 6. Tamanho

P mantido; apenas uma mensagem, reutilizando decoder AIR e dependências prontas.
Sem transporte/framing, servidor, mensagens CERTIFY/ações, especulação ou kernel.

## 7. Evidência observada

- 17 testes Rust passaram; schema P1 inspecionado e seis vetores de wire/goal/
  refs/taint conferidos, --check Python independente passou.
- Derive serde aceitou struct posicional como array: regressão falhou antes,
  exigência deserialize_map nos dois níveis corrigiu; regressão passou depois.
- Decoder permissivo e Fact no schema foram mutantes compilados; cada teste
  correspondente falhou com exit 101 por comportamento, não compilação.
  Ambos restaurados antes de full verify.
- Smoke público terminou PROTOCOL_SMOKE_OK, exit 0, com JSON → candidato →
  generate real. Texto e ordem não mudaram bytes/h(O); Pending permaneceu recusado.
- Clippy chunks_exact corrigido para as_chunks e alocação única de goal;
  ruff format ajustou literal sem mudar vetores. Exemplo da spec usa bytes AIR reais.
- ci/verify.sh --selftest: VERIFY_OK, exit 0, 175 Python + 25 TCB, workspace Rust,
  guards, smokes e todos os geradores. Dois avisos de licença e syn 2/3 em deny,
  sem erro; nenhum pacote externo novo no lockfile.
- cargo xtask tcb report, exit 0: correção 4610+78287=82897, fidelidade
  1550+80034=81584, build-only 51581; tudo inalterado. Snapshot da árvore modificada
  sobre 07f1467, não snapshot limpo de commit. CI/ARM não observados; sem push.
