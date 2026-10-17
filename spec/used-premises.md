# Dependências do certificado — Etapa4.7

Contrato em memória, [ADR0033](../docs/adr/0033-dependencias-do-certificado-e-suporte-finito.md).
Não é formato serializado. ground/v1, cnf/v1, lrat-rup/v1 e eventos continuam
com os mesmos bytes/versões; nenhum decoder novo.

## Entradas, identidades e saída

```rust
used_premises(
    report: &LratReport,
    map: &ProvenanceMap,
    expected_obligation: Hash<Obligation>,
    budget: &Budget,
) -> Result<UsedPremises, UsedError>
```

ProvenanceMap privado só vem de clausify de GroundProgram validado; contém
h(O), h(CNF), origens e suporte contextual alinhados aos IDs1..C. Reemitir a
CNF/mapa de O/V confiáveis, como na4.3. Não aceitar mapas fornecidos pelo
agente. LratReport privado só expõe IDs e identidades quando a prova inteira
foi Accepted. h(certificado) usa os bytes inteiros, inclusive comentários e
deleções válidos após a primeira vazia. Cobrar esse hash no Budget LRAT.

Conferir Accepted, presença dos IDs/identidade do certificado, h(O) esperado,
h(CNF) e alinhamento de suportes. Exigir IDs positivos, estritamente crescentes
e no intervalo original. Nenhum ID derivado/apagado é remapeado por heurística;
o fecho histórico do checker já resolve as arestas validadas.

Para cada ID usado, incluir Premise(ref) ou DomainAxiom(ref), mais o suporte
daquela cláusula. NegatedGoal e Definitional não criam referências próprias.
Ordenar e deduplicar: saída é conjunto de PremiseRef, não lista de citações.
UsedPremises guarda referências e identidades O/CNF/certificado privadas;
getters não concedem autoridade nem emitem Verified.

Unknown/Rejected não têm dependências válidas. Erro de identidade ou ID
inválido é erro explícito. Nenhuma saída parcial é publicada após exaustão.
O consumidor futuro5.4 ainda confere registry/checkers/evento/cited e contexto
antes de autorizar um claim. O índice existente usa somente used para deps.

## Suporte de domínio alcançado

Grounding confere e associa a cada sort enumerado as referências explícitas
de fecho e nomes únicos de O. A tradução de uma fórmula conserva os sorts
de quantificadores, variáveis e assinaturas de termos/predicados, seguindo
Defined transitivamente com pilha e marcação de símbolos. Unir as referências
desses sorts, ordenadas/únicas. Não confundir DomainSource opaco com fonte
autenticada; nenhuma nova premissa é criada.

A assertiva de cada premissa e a do objetivo negado recebem o suporte dessa
fórmula. Equivalências proposicionais de Tseitin não exigem suporte adicional.
A assertiva global de constraints recebe domain_support global se sua raiz
não é True: todas as guardas foram reunidas numa assertiva indivisível pelo
encoder1. Isso cobre Defined global inconsistente e enum vazio, mesmo quando
nenhuma assertiva de axioma explícito foi alcançada pela prova.

A união inclui somente suportes de cláusulas realmente alcançadas. Assim,
prova booleana que não alcança guardas finitas não herda domínio irrelevante;
igualdade/quantificação finita continua ligada aos axiomas que permitiram a
tradução, mesmo quando suas assertivas se tornam tautologias. Suporte por sort
é conservador: pode incluir nomes únicos em prova que só precisava de fecho,
ou axioma de quantificador tautológico. Não é núcleo mínimo nem minimização
semântica. Constraints global pode conservar mais sorts que um refinamento
por guarda conservaria; esta etapa não modifica CNF para ganhar precisão.

## Limites

Suporte do grounding usa max_steps e max_cells existentes: pilha e somatório
de referências por premissa/objetivo são limitados separadamente por max_cells.
O encoder limita cópias de suporte a1M referências agregadas. Reservas de
vetores são falíveis; exceder retorna Unknown(Size/Memory) de GroundError.

Checagem LRAT e projeção recebem Budgets separados, sem reinicialização oculta.
Na projeção, steps cobre varredura do mapa/IDs, suportes alcançados e ordenação;
mem_bytes conta capacidades reservadas para a união antes de deduplicar,
sem restituição. size_bytes conta8 bytes por ID,33 por origem e32 por referência
de suporte de cláusula; são unidades lógicas independentes do layout Rust.
Buffers emprestados não são RSS contabilizado. Limites inclusivos; overflow/
reserva falha ou exaustão dá erro/Unknown explícito. wall_ms não é imposto.

## Reprodução

```sh
cargo test -p logos-lrat --lib --test used
python3 tools/build_cake_lpr.py
cargo test -p logos-ledger --test used
ci/check-audit-independence.sh
ci/verify.sh --selftest
```

O teste do ledger gera O de cinco assumptions assinadas, confere LRAT nos
dois checkers e passa o conjunto extraído ao evento existente do harness.
Confere todas as consultas dependents; exatamente duas apontam ao claim.
Isto demonstra composição das interfaces existentes, sem implementar handler
ou autorização do kernel. Omissão da premissa necessária tem CNF satisfatível
com valoração independente completa e certificado antigo rejeitado.
