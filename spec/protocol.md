# proposal/v1 — mensagem candidata do agente

Etapa 1.6, ADR 0013. Este é o único tipo de mensagem de agente definido até aqui;
transporte/framing e demais mensagens estão fora do escopo. Versão inicial 1.

## 1. Envelope JSON

Documento JSON UTF-8 completo, sem bytes não brancos após o valor:

```json
{"version":1,"proposal":{"text_pt":"objetivo em português","goal":"8401036c464f4c2d6d732d65712f7631818100","cites":[],"speculative_on":[],"vocab":"0000000000000000000000000000000000000000000000000000000000000000"}}
```

Goal do exemplo é True em air/v1; vocab zero é só identificador sintático,
não vocabulário existente/autenticado. version DEVE ser inteiro JSON 1.
Objeto raiz tem exatamente version/proposal; payload tem exatamente os cinco
campos acima. Todos são obrigatórios. Extras, duplicatas (inclusive chaves
escapadas equivalentes), null ou tipos errados DEVEM ser rejeitados. Ordem de
chaves e whitespace JSON não importam; escapes JSON válidos são decodificados.
Encoder escreve ordem fixa version/proposal e text_pt/goal/cites/speculative_on/vocab;
essa saída não é canon/v1 nem recebe domínio/hash autoritativo próprio.

## 2. Campos

- text_pt: String UTF-8 não confiável. Preservar sequência Unicode integral,
  sem normalizar NFC, interpretar instruções ou promover para política/fato.
  Rust usa UntrustedText e acesso explícito as_untrusted_str; sem Canon/Domain.
- goal: string hex minúscula, de comprimento par e não vazia, contendo exatamente
  canon(Formula), envelope air/v1 completo. Formula::decode_with_limits aplica
  lógica FOL-ms-eq/v1, codificação mínima, Unicode canônico, estrutura e limites.
  Não aceita AST JSON, texto AIR, bytes de O ou bytes canônicos sobrando.
  Bem formado não implica bem tipado ou verdadeiro; generate faz sort-check.
- cites: array de digests de evento (PremiseRef), cada um exatamente 64 dígitos
  ASCII [0-9a-f]. Ordem/duplicatas são preservadas pelo protocolo e normalizadas
  exclusivamente pelo gerador. Não confundir hash do evento com hash da fórmula.
- speculative_on: array de ClaimId opacos, também 32 bytes/64 dígitos hex. Tipo
  distinto de PremiseRef. Decoder não atribui/autentica IDs nem decide escopo.
  Array não vazio é representável, não autorização de citar Pending ou agir.
- vocab: h(Vocabulary), 32 bytes/64 dígitos hex minúsculos. Decoder não resolve
  esse hash. Integração futura deve selecionar V confiável cujo hash coincida.

Não existe campo de Obligation, Fact, Policy, dependência calculada, status,
certificado, token ou Signature do kernel. Também não há tal campo no envelope.
Esses conteúdos em text_pt continuam apenas texto tainted, nunca autoridade.

## 3. Limites inclusivos

| Entrada | Máximo |
| --- | --- |
| JSON completo em bytes | 3 * 1024 * 1024 |
| text_pt, UTF-8 após escapes | 64 * 1024 |
| goal, CBOR após hex | 1024 * 1024 |
| cites | 1024 entradas |
| speculative_on | 1024 entradas |
| Formula, profundidade/nós agregados | 128 / 100000 |

Rejeitar excesso, não truncar/normalizar ou omitir campo. Verificar tamanho do
JSON antes de parsing; arrays param ao exceder orçamento. Limites de mensagem
não são garantia de RSS/tempo de execução. Encode aplica as mesmas restrições;
nenhum erro produz substituto null/objetivo padrão/lista vazia.

## 4. Fronteira de confiança e API

logos-protocol é fora da TCB. decode(bytes) retorna candidato tipado em memória,
não entrada autenticada. encode(&proposal) retorna JSON para o mesmo contrato.
Schema normativo: protocol.schema.json (JSON Schema Draft 2020-12). Schema impõe
forma/hex/contagens; UTF-8 em bytes, CBOR canônico e semântica requerem decoder.
maxLength de texto é limite necessário de code points, não substitui 64 KiB.

logos_obligation::Proposal emprestada continua input mínimo resolvido, não este
wire type. Nenhum caller anterior muda. text_pt e speculative_on não existem
na entrada lógica do gerador. Não há adapter que resolva V ou descarte especulação:
integração 5.2/5.8 deve conferir h(V), política de especulação e LedgerView confiável.
Até lá Pending continua Reject::NotCitable mesmo se marcado no payload.
O protocolo não implementa ledger, certificados, status, assinaturas, servidor,
transporte, framing, ações ou memória de longo prazo.

## 5. Evidência reproduzível

- tests/proposal.rs: extras, duplicatas, tipos, versões, hex, AIR/O, limites,
  schema/P1 e texto tainted invariável na obrigação gerada.
- spec/vectors/protocol/vectors.json, reproduzidos por
  tools/gen_protocol_vectors.py (--check), independentes do decoder Rust.
- cargo run -p logos-protocol --example protocol_smoke: decode e geração real
  via APIs públicas; snapshot em memória não prova autenticidade de ledger.

Formato novo começa em v1. Alterar campos/representações exige nova versão,
atualizar esta spec/schema/vetores; não aceitar campos futuros silenciosamente.
