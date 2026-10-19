# Revisão documental da memória a partir do UniiChat

Solicitada pelo mantenedor em 2026-10-19. Escopo: conferir fontes públicas,
corrigir a extensão proposta e tornar as etapas de memória acionáveis no
ROADMAP. Não implementar o runtime nem antecipar dependências das Fases 6–10.

1. **Entradas:** STATUS, ADR 0008, E§1–24, roadmap; UniiChat
   `0500bf6a8ea4bf0ce047488562022b1b6532cc30` e OptMem
   `1fb164cf39028047781f72ac3bb1e5a691c1dcb0`, clones públicos em `/tmp`.
2. **Saídas:** relatório com permalinks e evidência reproduzível; ADR 0039;
   contratos de contexto/compactor, folding em lotes, blocos/cache e falhas;
   etapas MEM com dependências, exclusões e critérios ainda desmarcados.
3. **Ordem:** ler implementação e testes → executar simuladores locais →
   conferir casos pequenos contra funções reais → registrar diferenças
   deliberadas → revisar spec/roadmap/STATUS → verificação completa → commit.
4. **Fechamento:** cada afirmação sobre upstream aponta para commit/função;
   casos de diferença de due, retry, contexto e cache têm reprodução; limites
   e denominadores dos testes são declarados; links/dependências e coerência
   interna revisados; `ci/verify.sh --selftest` com exit 0 e `VERIFY_OK`.
5. **Decisões/riscos:** não copiar truncamento/limites soft nem credenciais
   no Python; separar resultado do simulador de cache real; formato de wire
   externo é evidência, não contrato aceito de provider; ADR 0008 continua
   proposta e a próxima etapa principal continua 5.4.
6. **Tamanho:** M pelo trabalho de investigação/contratos; nenhuma nova
   dependência ou alteração de serialização/TCB nesta revisão documental.
