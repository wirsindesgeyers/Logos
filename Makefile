# Atalhos reproduzíveis; a verificação continua em ci/verify.sh (AGENTS.md §4).
.PHONY: demo-kernel
demo-kernel: ## Etapa 5.12: kernel ponta a ponta + auditoria independente
	ci/demo-kernel.sh
