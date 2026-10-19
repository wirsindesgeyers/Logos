# ADR 0038 — PROPOSE com snapshot autenticado

- Estado: aceito e implementado na5.3; verificação local concluída
- Data: 2026-10-19
- Base: 6374755
- Relacionadas: 0012,0034,0035; número0037 reservado à proposta científica preexistente

## Decisão

1. O handler usa EventProposal existente, resolve V no CAS, prepara o snapshot
   autenticado3.3 e chama generate1.5. Nenhuma obrigação fornecida pelo agente
   entra nesse caminho. Especulação não vazia é recusada antes de escrever.
2. A autoria registrada é a chave do kernel obtida do Genesis autenticado do
   próprio ledger. SO_PEERCRED autentica a conexão; UID não é persistido no
   evento e não permite atribuição individual posterior. Não alegar assinatura
   nem identidade criptográfica do agente. Sem novo campo de autoria no wire.
3. Scope vazio é o contexto opaco desta etapa. Assumptions de outro escopo,
   inclusive ancestrais usados, continuam recusadas pelo snapshot existente;
   a semântica de ASSUME/escopos pertence à5.7.
4. Serializar O e limitar seus bytes a1MiB (teto padrão do reader/checker)
   antes de publicar. Erros de canon/tamanho/CAS não geram eventos. Preparar
   a resposta antes dos appends; Proposed e ObligationCreated são as duas
   transições consumidoras existentes, sob o lock exclusivo do writer.
5. Falha entre appends pode deixar Proposed não citável e O órfão no CAS.
   Devolver erro sem Pending fictício nem retry automático; replay resolve
   persistência ambígua. Não acrescentar transação ou evento nesta etapa.
6. Novo resultado serializado propose-result/v1: [1,h(O),O_bytes], dentro do
   value livre do envelope kernel-protocol/v1 já existente. A versão do novo
   resultado começa em1; requisições, envelope externo e eventos não mudam.
   Vetores independentes fixam os bytes/hash da resposta e entram no --check.
   Não há decoder novo de entrada na TCB.
7. ObligationCreated conserva Route::Model, artifact=h(O), cnf=None. Seleção
   de checker/rota autorizadora pertence à5.4. Pending não é credencial.
8. O teste STATE14 reproduz época9, três referências citáveis, geração de O
   e os dois eventos pelo socket. Usa assumptions de escopo vazio existentes
   como premissas de fixture; não alega ingestão de Fact/Policy das Fases6/7.
   Também testa citação de Verified existente como fixture de replay, sem
   implementar CERTIFY nem confiar em veredicto fornecido pelo agente.

## Verificação

50 ataques P1 pelo socket real, resposta/hash/CAS/eventos/reabertura, limites,
escopo, erros de persistência e entrada pelo executável. Mutações compiladas
nas guardas efetivas, fontes restauradas, fuzz do decoder existente e full
ci/verify.sh --selftest antes do commit. Plano em docs/plans/etapa-5.3.md.

Evidência local: oito testes novos, executável real,50 recusas P1 e14
mutações compiladas detectadas; fuzz560503/61s sem crash e hashes
inalterados. Full verify/selftest VERIFY_OK/exit0 em1568.40s,252 Python+25
TCB. Resumo versionado em docs/plans/etapa-5.3-evidencias.json.

## Limites

O teto de saída não é um teto de RSS/wall para gerar/resolver premissas.
Nenhum pacote, parser JSON, formato histórico ou autoridade nova é introduzido.
