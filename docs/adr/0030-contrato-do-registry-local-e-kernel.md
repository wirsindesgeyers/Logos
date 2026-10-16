# ADR 0030 — Contrato do registry local e integração no kernel

- Estado: aceita; proposta A adotada explicitamente pelo mantenedor em2026-10-16
- Etapas: 4.5 e 5.4
- Data: 2026-10-16
- Base inspecionada: d401bad3472d84ea606e5260602512c28682f693
- Relacionadas: 0025, 0028, 0029

## Bloqueio confirmado

As dependências declaradas da4.5 existem: checker Rust e corpus da4.4;
auditor independente da3.6 com rederivação O→CNF da4.3. Os eventos
CheckerRegistered/Verified, CAS e Budget também existem.

O segundo critério da4.5, entretanto, exige literalmente:
“Trocar o binário por outro (mesmo nome) faz o kernel recusar o check.”
O próprio Fora do escopo4.5 exclui registro no kernel e atribui essa
integração à5.4. Na base inspecionada, logos-kernel/src/lib.rs contém
somente documentação e os dois atributos de lint; não há handler CERTIFY
nem CheckerRegistry. Implementá-los agora anteciparia5.1–5.4 e violaria
o escopo. Uma função chamada kernel em um teste não supre a dependência.

Reprodução somente de leitura na base inspecionada:

```sh
rtk proxy git show d401bad:crates/logos-kernel/src/lib.rs
rtk proxy git show d401bad:ROADMAP.md
rtk proxy git show d401bad:crates/logos-audit/src/verify.rs
rtk proxy git show d401bad:crates/logos-ledger/src/event_body.rs
```

AGENTS.md §1.3 exige: “Se algo de que a etapa depende não existe no
repositório, pare e registre como bloqueio em STATUS.md em vez de implementar.”
O STATUS já apontava esse risco antes desta análise. O pedido de continuação
foi aplicado à análise e ao plano; não foi tratado como adoção implícita de
uma mudança de contrato. Nenhum critério foi marcado nem código implementado.

## Decisão A — recusa local agora, integração preservada na5.4

Preservar a ordem e os entregáveis da4.5: build, hash, wrapper isolado,
registry local e auditor independente com dois checkers. Dividir o teste
de substituição de binário entre o mecanismo e o seu consumidor futuro.

Adoção explícita: “pode implementar A”. ROADMAP atualizado **antes do código**:

| Local | Texto proposto |
| --- | --- |
| 4.5, segundo critério | Trocar o binário por outro, mesmo com o mesmo nome, faz o registry local/wrapper recusar o check antes de executar o artefato desconhecido. Testar hash não permitido, substituição após registro e vínculo entre bytes identificados e executados. A integração no kernel permanece obrigatória na5.4. |
| 5.4, critério adicional | Teste integrado preservado da4.5: registrar checker conhecido, substituir o binário por outro de mesmo nome e enviar CERTIFY; o kernel recusa o check e não emite Verified. Conferir também checker desconhecido/inativo e preservar P3/P4. |

A dependência5.4→4.5 já existe; não criar dependência inversa. O registro
local não emite CheckerRegistered, não assume autoridade administrativa e
não concede citabilidade. A5.4 consome o mecanismo e emite os eventos reais.
O auditor continua sem depender de logos-ledger nem logos-kernel.

Os demais critérios4.5 permanecem integrais e desmarcados:

- Rebuild do zero com mesmo hash em duas máquinas reais. Dois diretórios ou
  duas execuções no mesmo host são apenas teste local de determinismo.
  Se só houver evidência de um host, registrar a pendência remota.
- Concordância dos dois checkers sobre a suíte4.4, incluindo as600 corrupções.
  Não substituir comparação direta por executar o Rust antes do cake_lpr.
  Diferenças do perfil RUP estrito em sintaxe/IDs/deleções/cauda exigem
  documentação e testes; sem reduzir a suíte silenciosamente.
- Recheck de100% dos Verified de camada1 do ledger de exemplo, com os dois
  checkers e rederivação O→CNF, recusando certificado trocado no CAS.
  Um ledger assinado de teste demonstra o auditor, não o handler futuro.

## Alternativa B — não adotada: manter o contrato literal e replanejar a ordem

Manter4.5 bloqueada. Preparar outro plano que explique como disponibilizar
kernel/registry/CERTIFY antes de exigir esse teste, resolvendo o ciclo
atual5.4→4.5. A reordenação precisa de atualização explícita do ROADMAP;
não antecipar fases por conveniência nem implementar um kernel provisório.

## Limites desta proposta

Não escolhe ainda dependências de isolamento nem uma revisão/binário
confiável. O plano4.5 exige fixá-los em decisão técnica antes do wrapper.
Não autoriza modificar cake_lpr, antecipar produtor SAT4.6, used_premises4.7,
handler5.4 ou fazer push. A escolha de A libera somente a fronteira descrita.

## Fonte externa consultada

O [upstream cake_lpr](https://github.com/tanyongkiam/cake_lpr) foi consultado
em2026-10-16: publica assembly pré-compilado, receita make, revisões HOL/CakeML
e interface de subprocesso. Isso permite planejar o build; não demonstra
reprodução em dois hosts, isolamento ou concordância com o corpus LOGOS.

Complemento técnico da implementação autorizada: [ADR0031](0031-build-e-isolamento-do-cake-lpr.md).
