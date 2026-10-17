# CaDiCaL — produtor não confiável

Fonte upstream integral na revisão de lock.json, conservada por git archive
e XZ. LICENSE é upstream. Nenhuma modificação no solver; nenhum pacote novo
na TCB. Instalar com python3 tools/build_cadical.py (g++/GNU make, sem rede).
O build é por fonte fixada, não uma promessa de binário idêntico entre hosts.
Relatório do compilador/fonte/binário em target/cadical/cadical.build.json.

A API exige caminho explícito para o solver. Trocar esse executável só muda
o produtor não confiável: o candidato sempre precisa passar no checker.
Uso: --lrat --no-binary; modelos SAT usam saída padrão da competição.
