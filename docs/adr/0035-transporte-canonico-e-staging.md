# ADR 0035 — Transporte canônico e staging do kernel

- Estado: aceito para implementação da5.2
- Data: 2026-10-18
- Base: 451e1ff

## Decisão

1. Novo kernel-protocol/v1, independente de proposal/v1 JSON, que permanece
   intacto e fora da TCB. O candidato Propose tem os mesmos cinco campos e
   limites da1.6, representados por EventProposal já existente na TCB. AIR
   continua usando seu decoder; texto tainted usa bytes UTF-8, sem NFC.
   Não ligar serde/JSON à TCB só para compartilhar um struct candidato.
2. Frames u32 big-endian com comprimento positivo, envelope CBOR canônico
   [1,id,tag,payload]. Aridades exatas; tags/versões/tipos/extras desconhecidos
   são erros. Nenhuma mensagem fornece obrigação pronta, autor ou veredicto.
3. Linux inicialmente, SO_PEERCRED via rustix1.1.5 seguro, já permitido.
   Novo uso net/process/fs; nenhuma dependência externa nova. UID permitido
   configurado pelo operador, nunca pelo frame. Socket em diretório controlado
   pelo kernel; não remover socket preexistente. Sem fallback de autenticação.
4. Conexões do mesmo UID têm framing, quota, taxa e prazo absoluto próprios.
   O único estado lógico compartilhado é Ledger sob Mutex. Contadores de
   admissão, cursor e arquivos temporários são estado operacional transitório.
   Handlers de mutação retornam Unavailable até suas etapas donas; nenhum
   resultado bruto é promovido a Claim/Verified. Query/Subscribe leem o ledger.
5. Subscribe envia eventos comprometidos em ordem e termina explicitamente
   por limite ou prazo, com cursor de continuação; acompanha novos appends
   enquanto a conexão está aberta. Outro socket do mesmo UID pode enviar
   comandos. Nunca mantém lock do ledger durante I/O de rede ou espera.
6. Referência a objeto é CAS ou staging com nome opaco e h(objeto) esperado.
   Capturar entrada por renameat para diretório privado, sem seguir symlinks;
   conferir arquivo regular/UID/link único, copiar sob teto para snapshot
   privado e conferir hash antes de publicar no CAS. Arquivo aberto pelo
   agente pode mudar: só os bytes do snapshot são publicados/checados.
   Falha não produz recibo de importação nem desfecho lógico. Staging e área
   privada devem estar no mesmo filesystem; EXDEV é erro sem fallback.
7. Formato novo começa em v1, com vetores independentes; formatos anteriores
   preservados. Novos fontes no manifesto TCB e alvo kernel_message para
   decoder/framing. CI noturna executa esse alvo por pelo menos14400s e
   arquiva logs; critério4h só é marcado após execução observada.

## Limites

UID identifica um principal do SO, não uma pessoa, e não distingue processos
hostis sob o mesmo UID. Separação de usuários/chaves é da5.11. Transporte e
importação não conferem certificado, fatos, citações ou autoridade administrativa;
handlers PROPOSE/CERTIFY/ASSUME permanecem nas5.3/5.4/5.7.
