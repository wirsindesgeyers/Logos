# ADR 0067 — Chave do usuário (v1)

- Estado: aceito para a Etapa 6.5 (2026-10-24), por delegação do mantenedor. Revisável.
- Relacionadas: 0059 (isolamento do agente), 0063 (confiança por tópico), 0065.

## Contexto

A 6.5 pede um par Ed25519 local do usuário (`logos user init`), registrado como raiz de
confiança com os tópicos `authorize`, `confirm` e `assume`, com as assinaturas feitas
fora do processo do agente. Passkey/WebAuthn é da 9.2. A configuração de confiança
(trust/v1) é publicada inteira e versionada, então registrar o usuário não pode apagar as
raízes dos atestadores.

## Decisão

1. `logos user init --key ARQUIVO` cria a semente com `/dev/urandom`, modo 0600, por
   `O_EXCL` (não sobrescreve nem segue link simbólico), no formato de arquivo de chave do
   kernel, e imprime a chave pública e o `KeyId`. O caminho é do usuário; o sandbox do
   agente (5.11) não monta o home.
2. `logos ledger user-trust DIR --key KERNEL --user-public HEX` lê a configuração vigente
   (maior versão publicada), acrescenta a raiz `user` com os tópicos `assume`, `authorize`
   e `confirm` e publica a versão seguinte com a autoridade do kernel. Recusa registrar a
   mesma chave duas vezes.
3. **Não há comando de assinatura nesta etapa.** Os formatos do que o usuário assina
   (token de autorização, confirmação) nascem nas Fases 8 e 9; o requisito aqui é só que a
   chave exista fora do alcance do agente. O pentest de isolamento ganha `read_user_key` e
   `sign_as_user`: precisam funcionar fora do sandbox (controle positivo) e falhar dentro.

## Consequências

- Sem formato novo. O usuário ser raiz de confiança não dá a ele poder de registrar fato
  (`FactRegistered` exige emissor com tópico do fato); os tópicos servem às verificações das
  Fases 8 e 9.
- A chave do usuário continua em disco, legível pelo usuário de SO que roda a CLI; proteção
  contra malware do próprio usuário é da Fase 13 (hardware/passkey).
