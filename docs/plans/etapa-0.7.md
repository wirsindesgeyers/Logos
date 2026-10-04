# Plano da Etapa 0.7 — ameaças e escopo

Data: 2026-10-04. Escopo: [ROADMAP](../../ROADMAP.md) Etapa 0.7;
especificação conceitual §1, §5, §6, §10 e §13.
Sem dependência técnica. Não implementar mitigação nem modelo formal.

## 1. Interfaces de entrada

- [Especificação conceitual](../spec/LOGOS-proof-carrying-agents.md): tese,
  TCB por propriedade, garantia do gateway, 18 ameaças, integridade de justificativa.
- [Manifesto](../../tcb/manifest.toml): classificação de crates e `verified`;
  listar um esqueleto não demonstra a propriedade que ele implementará.
- [ADR 0005](../adr/0005-escolhas-do-cas.md) e
  [ADR 0006](../adr/0006-veredictos-e-orcamentos.md): premissas de filesystem,
  limites por objeto, latch de passos, enums públicos sem autoridade.
- Estado das primitivas 0.3–0.6; kernel/gateway ainda sem implementação.
  Verificação remota pendente não será apresentada como observada.

## 2. Interfaces de saída

Sem API Rust, formato serializado ou assinatura de função novos.

- [threat-model.md](../threat-model.md): catálogo `A1`–`A8`, ativos,
  fronteiras e matriz bidirecional atacante ↔ ameaça `1`–`18`.
- [guarantees.md](../guarantees.md): `G1` (tese), `G2` (gateway),
  `G3` (justificativa), cada qual com enunciado, premissas, TCB explícita
  por propriedade e estado de implementação.
- [limits.md](../limits.md): L1–L3, limites fundamentais e TOCTOU parcial.
- Apêndice B preserva as etapas de mitigação existentes, mas distingue
  redução de risco de solução dos limites fundamentais.

## 3. Tarefas em ordem

1. Revisar primitivas e suas fronteiras com a spec/ADRs; registrar achados
   comprovados no STATUS. Não tratar ausência de fases futuras como bug.
2. Escrever os três documentos; conferir manualmente cada enunciado contra
   §1/§5/§6/§13 e cada linha da matriz contra §10 e Apêndice B.
3. Executar script abaixo: cobertura total de ameaças, referências recíprocas,
   TCB/premissas/estado por garantia, limites fundamentais e links locais.
   Ele confere estrutura/rastreabilidade, não prova a semântica dos enunciados.
4. Rodar `ci/verify.sh --selftest`, incluindo smoke público real do core;
   atualizar `STATUS.md` e critérios 0.7 no roadmap segundo evidência.
   Commit somente após verificação completa passar; sem push.

## 4. Critérios → evidência reproduzível

| Critério do roadmap | Evidência |
| --- | --- |
| Três documentos; TCB explícita em cada garantia | Documentos revisáveis; script confere G1–G3 e campos obrigatórios |
| Toda ameaça de §10 no Apêndice B com mitigação ou declaração fundamental | Matriz 1–18 no roadmap; script compara os três conjuntos de IDs e classificação fundamental |
| Cada atacante aponta para ameaça e vice-versa | Duas tabelas em threat-model; script confere igualdade das arestas |

Na raiz do repositório, executar:

```sh
python3 - <<'PY'
from pathlib import Path
import re

root = Path.cwd()

def text(path):
    return (root / path).read_text()

def numbered_rows(body):
    return {int(m[1]): m[2] for m in re.finditer(r'^\| (\d+) \| (.+)$', body, re.M)}

spec = text('docs/spec/LOGOS-proof-carrying-agents.md')
spec_threats = spec.split('## 10. Ameaças e failure modes', 1)[1].split('## 11.', 1)[0]
roadmap = text('ROADMAP.md').split('## Apêndice B', 1)[1].split('## Apêndice C', 1)[0]
model = text('docs/threat-model.md')
actors = {}
for m in re.finditer(r'^\| (A\d+) \| ([^\n]+)$', model, re.M):
    cells = [c.strip() for c in m[2].split('|')]
    actors[m[1]] = set(map(int, re.findall(r'\b\d+\b', cells[-2])))
assert set(actors) == {f'A{i}' for i in range(1, 9)}
rows = numbered_rows(model)
expected = set(range(1, 19))
assert set(rows) == set(numbered_rows(spec_threats)) == set(numbered_rows(roadmap)) == expected
for actor, threats in actors.items():
    assert threats and threats <= expected, actor
    assert threats == {i for i, row in rows.items() if actor in re.findall(r'\bA\d+\b', row)}, actor
for i, row in rows.items():
    assert re.findall(r'\bA\d+\b', row), i
for i, row in numbered_rows(roadmap).items():
    cells = [c.strip() for c in row.split('|')]
    assert re.search(r'\b\d+\.\d+\b', cells[2]) or 'fundamental — só declarada' in row, i
for i in [1, 2, 3, 13, 16]:
    assert 'fundamental — só declarada' in numbered_rows(roadmap)[i], i
assert 'Parcialmente fundamental' in numbered_rows(roadmap)[10]
guarantees = text('docs/guarantees.md')
sections = re.findall(r'^## (G\d+) — .*?\n(.*?)(?=^## |\Z)', guarantees, re.M | re.S)
assert {name for name, _ in sections} == {'G1', 'G2', 'G3'}
for name, body in sections:
    for field in ['Enunciado', 'Premissas', 'TCB de correção', 'TCB de fidelidade', 'Dependências de progresso', 'Estado']:
        assert f'**{field}.**' in body, (name, field)
limits = text('docs/limits.md')
for name in ['L1', 'L2', 'L3']:
    assert re.search(rf'^## {name} — ', limits, re.M), name
assert set(numbered_rows(limits)) == {1, 2, 3, 10, 13, 16}
paths = ['docs/plans/etapa-0.7.md', 'docs/threat-model.md', 'docs/guarantees.md', 'docs/limits.md']
for path in paths:
    for link in re.findall(r'(?<!!)\[[^\]\n]+\]\(([^\s)]+)\)', text(path)):
        if '://' not in link and not link.startswith('#'):
            target = link.split('#', 1)[0]
            assert (root / path).parent.joinpath(target).exists(), (path, link)
print('DOCS_0_7_OK: 18 ameaças, 8 atacantes, 3 garantias, L1-L3 e links locais')
PY
```

## 5. Riscos e decisões

- Prometer mediação implementada a partir de tipos/CAS: separar estado atual
  das propriedades projetadas e não chamar enum de credencial.
- Confundir assinatura com verdade, cadeia assinada com visão global única,
  conformidade com intenção ou proveniência da prova com influência no LLM.
- Tratar memória/tamanho declarados no core como supervisão de processo.
- Operador/kernel equivocado ou malicioso é atacante de bifurcação; pressupor
  kernel íntegro para G1–G3 não elimina a ameaça 9 em auditorias de terceiros.
- Não há decisão técnica nova nem divergência de escopo: aplica spec e ADRs
  existentes. Não requer ADR novo. Mitigações permanecem nas etapas futuras.

## 6. Tamanho

`P`, como roadmap: documentação e rastreabilidade; sem alteração de TCB,
protocolo, serialização ou dependências.

## 7. Evidência local — 2026-10-04

- Script da §4 executado: `DOCS_0_7_OK: 18 ameaças, 8 atacantes, 3 garantias,
  L1-L3 e links locais`. Revisão dos enunciados contra §1/§5/§6/§13 e dos
  limites contra §1/§6/§10; nenhum formato/API/TCB alterado.
- `ci/verify.sh --selftest`: `VERIFY_OK`, incluindo `CORE_SMOKE_OK` em x86_64,
  diferencial Rust×Python, 104 testes Python + 7 da TCB e bloqueios do selftest.
  Dois avisos `license-not-encountered` do cargo deny; verificação sem falha.
- Smoke Rust temporário fora do repositório: rejeição de cabeçalho não mínimo,
  chave duplicada/profundidade excessiva; assinatura válida só no domínio/mensagem
  corretos; latch de passos e fronteira de enums públicos/trabalho não cobrado;
  CAS real com reabertura, detecção de corrupção e recusa por tamanho.
  Resultado `REVIEW_SMOKE_OK`; temporários removidos após execução.
- Revisão direta das primitivas não identificou bug novo no recorte examinado.
  Revisores paralelos falharam por indisponibilidade de modelo, sem relatório.
  Isto não é auditoria exaustiva nem prova formal. CI remota/ARM/fuzz noturno
  continuam pendentes; documentação não os fecha.
