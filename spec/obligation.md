# obligation/v1 — geração selada e leitura contextual

Etapas 1.5/2.3; ADRs 0012/0016. Transporte canon/v1, AIR FOL-ms-eq/v1, Vocabulary/v1.
Gerador e reader pertencem à TCB de correção. Objeto lógico não é Verified, prova,
autorização, evento ou atestado de fidelidade.

## 1. Fronteira e entrada

`generate(Proposal<'_>, &impl LedgerView) -> Result<Obligation, Reject>` é a única
construção pública de Obligation. Campos privados; não implementar Default,
CanonDecode, From/TryFrom de bytes/Value/Proposal nem setter. Métodos de leitura,
Canon (somente saída) e Domain não permitem ingestão de obrigação externa.
A representação de proposta abaixo é entrada mínima em memória, não protocolo
1.6. Parser textual/citações nominais não conferem autoridade.

API Rust:

```rust
pub struct Proposal<'a> {
    pub vocabulary: &'a Vocabulary,
    pub goal: &'a Formula,
    pub cites: &'a [PremiseRef],
}
pub struct PremiseRef { /* digest privado */ }
// from_event_hash([u8;32]); event_hash() -> &[u8;32]
pub enum EntryKind {
    Fact, Policy, Verified, Assumption,
    Pending, Refuted, Unknown, Rejected, Invalidated, Proposal,
}
pub struct LedgerEntry<'a> {
    pub kind: EntryKind,
    pub epoch: u64,
    pub retracted: bool,
    pub vocabulary: &'a Vocabulary,
    pub formula: &'a Formula,
}
pub enum LedgerError { Unavailable, InvalidRecord }
pub trait LedgerView {
    fn current_epoch(&self) -> u64;
    fn lookup(&self, reference: PremiseRef)
        -> Result<Option<LedgerEntry<'_>>, LedgerError>;
}
```

LedgerView é fronteira confiável: representa um snapshot consistente, associa
lookup ao evento pedido e valida autenticidade/proveniência/escopo no leitor.
Sem acesso a relógio ou rede pelo gerador. Getter da época é lido uma vez.
Implementação real depende da 3.3, ausente; testes/smoke usam snapshot em memória.
Unavailable/InvalidRecord são erros, nunca ausência. Formula/Vocabulary já têm
construtores estruturais validados; decoder 2.3 produz tipo distinto (§6), não Obligation.
`LedgerEntry.epoch` é a época de aplicabilidade resolvida pelo snapshot, não
necessariamente a época de emissão imutável do evento. O reader futuro decide
se o evento ainda é válido; o gerador exige aplicabilidade na época corrente.

## 2. Referências e canonização

PremiseRef contém **o digest do evento citável**, não h(Formula), hash de texto,
ID de claim ou hash do payload Fact/Policy. A notação ROADMAP `h(PremiseRef)`
designa esse digest existente; não o hashear novamente. Ordenação lexicográfica
unsigned dos 32 bytes, equivalente à ordem hexadecimal minúscula. Deduplicar por
esse digest; duas referências distintas à mesma fórmula permanecem distintas.
PremiseRef não carrega kind/época: provenance pertence ao evento/view.

O envelope completo é um único array CBOR:

```text
[1, 0, "FOL-ms-eq/v1", 1, vocab_digest32, premises, goal]
premises = [[event_digest32, Formula], ...]
goal = Formula
```

Primeiro 1: versão obligation/v1; 0: tag de tipo Obligation neste formato.
Segundo 1: `encoder_version` canon/v1. Formula é envelope AIR completo,
não seus bytes dentro de bytestring. Digests são bytestrings de exatamente 32
bytes. Premises em ordem estritamente crescente; vazias permitidas. Goal e
premissas não são simplificadas, reordenadas internamente nem traduzidas.
`vocab_digest32 = SHA256(b"logos/vocab/v1\0" || canon(V_alvo))`.
Lógica igual à do objetivo e de todas as premissas (v1 só FOL-ms-eq/v1).

`h(O) = SHA256(b"logos/obligation/v1\0" || canon(O))`, tag já reservada
em spec/domains.md. Obligation implementa Domain/Canon para usar logos_crypto::hash.
Nenhuma tag de hash nova, alteração em AIR/Vocabulary/canon ou versão antiga.
Época, kind, status, retracted, ordem original/duplicatas de citações, texto da
proposta e nomes de binders não entram em O. Identidade é sequente puro; mesma
entrada lógica válida em duas épocas produz os mesmos bytes/hash.

## 3. Algoritmo, rejeições e precedência

1. `check(V_alvo, goal)`; falha => `Reject::GoalSort(SortError)`.
2. Calcular h(V_alvo); erro canônico => `Reject::Canon(CanonError)`.
3. Copiar referências, ordenar e deduplicar; ler época do snapshot uma vez.
4. Para cada referência ordenada: lookup; erro => Ledger, None => UnknownCitation.
5. Se retracted => Retracted; época diferente => InvalidEpoch; kind fora de
   Fact/Policy/Verified/Assumption => NotCitable. Pending nunca é hipótese
   automaticamente; Unknown/Rejected/Refuted/Invalidated não são Verified.
6. Exigir `V_alvo.compatibility_with(V_origem) == Compatible` (inclui igualdade).
   Direção importa: alvo preserva exatamente sorts/domínios/assinaturas/grounding/
   gloss anteriores. Versão e ordem por si só não invalidam compatibilidade 1.2.
   Caso contrário => IncompatibleVocabulary; nunca tradução implícita.
7. `check(V_alvo, premise.formula)`; falha => PremiseSort. Copiar fórmula aceita
   uma vez para armazenamento próprio. View é responsável por ligar fórmula ao
   vocabulário original autenticado, não aceita rótulo arbitrário de fonte.
8. Construir O privado com lógica, encoder 1, h(V_alvo), premissas e goal próprios.

Erros públicos (com payloads precisos): UnknownCitation(PremiseRef),
Retracted(PremiseRef), InvalidEpoch { reference, expected, actual },
NotCitable { reference, kind }, IncompatibleVocabulary(PremiseRef),
GoalSort(SortError), PremiseSort { reference, error }, Ledger { reference, error },
Canon(CanonError). Sem conversão de rejeição em veredicto. Referências inválidas
múltiplas têm diagnóstico determinístico pela menor referência ordenada.
Não procurar entradas duplicadas mais de uma vez. Nenhuma inferência de fórmula
por conteúdo do hash, glosa ou cache mutável.

## 4. Assumptions e não-objetivos

Assumption citável continua premissa explícita em Γ e seu evento permanece na
identidade. O não transforma isso em fato incondicional. Escopo/proveniência
válidos vêm da view; propagação condicional/eventos/tokens depende de fases futuras.
Gerador não prova consistência, não fecha obrigação, não expande Defined, não
executa solvers e não verifica assinatura de ledger inexistente. LedgerView fake
não prova autenticidade real. Sem CNF/SMT/Lean, especulação ou protocolo 1.6.

## 5. Evidência reproduzível

20 vetores fixos em spec/vectors/obligation/, AST e bytes/hashes produzidos por
Python stdlib (CBOR independente) + hashlib, sem importar LOGOS nem chamar Rust. Testes
Rust geram O via API e comparam bytes/hash; variantes de ordem/duplicatas/época e
identidade das referências testam canonização. Rejeições cobrem todos os kinds,
época, retratação, falhas da view, sorts e compatibilidade direcional.
Compile-fail externo cobre struct literal, decoder, Default e conversão; controle
positivo só usa generate. Smoke público e mutantes de TCB demonstram comportamento.
Etapa 1.5 não introduziu parser; Etapa 2.3 acrescenta reader contextual e alvo
obligation_decode, sem abrir a construção selada de Obligation.

## 6. Reader contextual para checkers (Etapa 2.3)

`decode_for_check(bytes, Hash<Obligation>, &Vocabulary, DecodeLimits)`
retorna `Result<CheckedObligation,DecodeError>`. Campos privados e accessors
vocab/premises/goal; CheckedObligation MUST NOT implementar Canon, Domain,
CanonDecode ou conversão para Obligation. Não é objeto emitido por generate.

Ordem de processamento MUST ser: teto de bytes, hash_raw::<Obligation> dos
bytes originais comparado ao esperado, Decoder canônico com finish exato,
envelope §2 exato, h(V) real, referências estritamente crescentes, AIR e
sort-check de cada fórmula fechada (incluindo goal). Hash divergente MUST
recusar antes de parsing; MUST NOT hashear apenas um reencoding da AST.
Duplicatas/ordem inválida MUST ser InvalidPremises, nunca normalizadas.

DecodeLimits inclusivos: max_bytes (padrão 1.048.576), max_premises (4.096) e
air=Limits padrão (depth=128 por fórmula, nodes=100.000 agregados entre todas
as premissas e goal, inclusive termos). Bytes antes de materializar Value;
quantidade de premissas antes da reserva tipada. Decoder canônico conserva
MAX_DEPTH independente; excedê-lo é Canon(DepthExceeded). Limites AIR/bytes/
premissas produzem Unknown(Size); falhas de reserva tipada Unknown(Memory).
Materialização Value não promete hard RSS. Demais erros preservam Canon/Air/
Sort, HashMismatch ou VocabularyMismatch; nunca objeto parcial/padrão.

Reader MUST NOT afirmar que uma referência é citável/autêntica ou válida na
época atual. h(O) esperado MUST ser obtido de obrigação gerada pelo monitor:
um atacante que fornece bytes e recalcula hash não obtém proveniência. P1
permanece em generate/LedgerView; P2 liga dados lógicos ao hash esperado.

Reprodução: `cargo test -p logos-obligation --test decoding`; positivos
reutilizam os 20 vetores independentes da geração; negativos fixos adicionais
em [vectors/obligation/decoder.json](vectors/obligation/decoder.json), com
hashes SHA-256 dos bytes originais. Sem alteração de versão/layout/domain.
Fuzz: `FUZZ_TOOLCHAIN=nightly-2026-09-28 ci/fuzz.sh 60 obligation_decode`.
Checkers/limites compartilhados: [evaluation.md](evaluation.md) §5.

