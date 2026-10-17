# Corpus SAT4.6

corpus.json fixa600 nomes/famílias/status esperados/h(O)/h(V), antes da
execução dos solvers:300 obrigações válidas e300 pares refutáveis. Regenerar
com tools/gen_sat_vectors.py; --check exige identidade integral. As fórmulas
e construção determinística estão em py/logos_provers/sat_cases.py.
O gerador não consulta solver/checker. Não são fontes autenticadas de política.

O smoke exige300 CNFs positivas distintas e premissas consistentes, e inclui
sete controles de reconstrução fora desse denominador. Resultados/provas
são gerados pelo solver real em ci/check-sat.sh e conferidos pelos checkers.
Transporte e pressupostos em spec/sat-producer.md; protocolos negativos em
py/tests/test_sat.py. Nenhum novo certificado/encoding da TCB.
