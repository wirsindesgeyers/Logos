# Caderno de pesquisa — notebook/v1 (Etapa 5R.3)

Formato **fora da TCB**, implementado em `py/logos_notebook/bundle.py`; decisão em
[ADR0055](../docs/adr/0055-caderno-de-pesquisa-e-operacoes-cientificas.md). Um
bundle válido não prova, atesta nem autoriza nada: guarda formulações,
proveniência e referências a eventos do ledger. Vetores independentes em
[`vectors/notebook/`](vectors/notebook/) (`tools/gen_notebook_vectors.py`).

## 1. Envelope canon/v1

```text
bundle   = [1, "logos/notebook", project, [artifact...], [relation...]]
project  = [id_bytes16, title_Text, scope_Text, hV_bytes32, revision_u64]
artifact = [id_bytes16, kind_u, nature_u, revision_u64, title_Text, statement_Text,
            formula, provenance, [claim_ref...], [limitation_Text...],
            attachment, status_hint]
formula  = null | Formula air/v1 (valor canon, não bytes)
provenance = [source_Text, locator_Text, method_Text, actor_Text, recorded_at_u64]
claim_ref  = [role_u, hEvent_bytes32, null | hO_bytes32]
attachment = null | [media_type_Text, bytes]            (só DATA_TABLE, ≤ 4 MiB)
status_hint = null | [observed_seq_u64, label_Text]      (histórico, nunca atual)
relation = [source_id, relation_u, target_id]
```

`h(bundle) = SHA256("logos/notebook/v1\0" ‖ bytes)`. A tag não pertence ao
registro de domínios da TCB (`spec/domains.md`), que só lista domínios Rust.

| kind | nome | papéis aceitos |
| ---: | --- | --- |
| 0 | QUESTION | — |
| 1 | HYPOTHESIS | ASSUMPTION |
| 2 | PREDICTION | CHECK |
| 3 | EVIDENCE | ASSUMPTION |
| 4 | ASSUMPTION | ASSUMPTION |
| 5 | CONCLUSION | CHECK |
| 6 | CANDIDATE_LAW | ASSUMPTION, CHECK |
| 7 | DATA_TABLE | — |

`nature`: 0 hipótese, 1 evidência empírica, 2 fonte afirma φ, 3 modelo sugere φ,
4 salvo pelo usuário, 5 pedido de derivação. Descreve como o item entrou no
caderno (WS§11.1), não seu status lógico. `role`: 0 ASSUMPTION (evento
AssumptionDeclared, sem h(O)), 1 CHECK (evento de CERTIFY/REFUTE, com h(O)).
`relation`: 0 predicts, 1 tests, 2 derived_from, 3 alternative_to,
4 conflicts_with, 5 observed_in.

## 2. Regras do decoder

Aridades exatas, inteiros sem sinal, digests/IDs com o tamanho exato e textos
canon/v1 (NFC, atribuídos) até 64 KiB; título, tipo de mídia, limitação e rótulo
não vazios. Escopo começa por `session:`, `until:` ou `claim:` (ADR0054).
Recusar: kind/nature/role/relation desconhecidos; papel não aceito pelo kind
(**evidência ou hipótese com CHECK é erro**); CHECK sem h(O) ou ASSUMPTION com
h(O); PREDICTION/CONCLUSION/CANDIDATE_LAW sem fórmula; referência ASSUMPTION sem
fórmula; fórmula que não é air/v1; anexo fora de DATA_TABLE; IDs repetidos;
relação com extremo inexistente ou laço; relações fora da ordem lexicográfica
estrita `(source, relation, target)` ou repetidas. Limites: 4096 artefatos,
16384 relações, 1024 referências e 64 limitações por artefato, bundle até 16 MiB.
Nada é corrigido silenciosamente; o encoder relê o que emite com o decoder.

## 3. Semântica e limites

- O status de qualquer referência é consultado no kernel (Query status/open) ao
  exibir; `status_hint` é rotulado histórico (W1). Só um Verified é "provável
  sob Γ", sempre com as assumptions abertas; Refuted/Unknown ficam distintos.
- Evidência externa entra como nota (sem referência) ou como assumption
  explícita (ASSUME 5.7); nunca como fato, atestação ou resultado de checker.
- Retratar é operação do kernel (5.9) pelo operador; o caderno só reconsulta.
- O bundle guarda h(V), não V; formulações são reavaliadas no vocabulário vigente.
- Importação pela WS.4: IDs, referências e proveniência mapeiam campos de
  `ProjectArtifact`; não há promessa de compatibilidade além de notebook/v1.

## 4. Operações (py/logos_notebook)

`Lab` usa os mesmos componentes da 5R.1 (cliente 10.1, farm 10.2, staging).
`science` implementa experimento crucial, conflito mínimo (MUS checado) e leis
contra tabelas (CSV estrito `linha,<predicados>` com células `0`/`1`, cada linha
um model/v1 proposicional); `atms` recalcula o corte de retratação do zero
(ADR0052) e confere registros Invalidated; `report` gera Markdown/JSON.
Reprodução: `ci/check-notebook.sh` (relatório em `target/notebook-report.*`).
