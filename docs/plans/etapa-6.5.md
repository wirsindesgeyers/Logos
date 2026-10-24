# Etapa6.5 — chave do usuário (v1)

Estado: implementada localmente (2026-10-24). Decisão em [ADR 0067](../adr/0067-chave-do-usuario.md).

1. **Entradas:** raízes de confiança (6.2), isolamento do agente (5.11), arquivo de chave (0.4).
2. **Saídas:** `logos user init --key`; `logos ledger user-trust`; probes `read_user_key` e
   `sign_as_user` no pentest de isolamento (`ISOLATION_OK forbidden=11`).
3. **Critério → evidência:**
   - *O processo do agente não consegue produzir assinatura do usuário*:
     `tools/check_isolation.py` cria a chave com `logos user init` no home do operador;
     fora do sandbox o probe lê a chave e assina (controle 11/11); dentro do `bwrap` os 11
     falham. Mutação (expor `/home` no sandbox) faz `read_user_key`, `sign_as_user` e
     `read_tool_credentials` passarem e o pentest falhar.
   - Chave e raiz: `logos-cli/tests/user.rs` (modo 0600, sem sobrescrever nem seguir
     link, chaves distintas, raiz com os três tópicos, raízes anteriores preservadas,
     registro duplicado recusado, primeira configuração pode ser a do usuário).
4. **Riscos:** chave em disco do usuário de SO; sem comando de assinatura (Fases 8/9).
5. **Tamanho:** `P`.
