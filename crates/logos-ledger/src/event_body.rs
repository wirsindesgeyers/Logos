//! Catálogo de eventos, tipos associados e regras de validação estrutural.
//!
//! Implementa os 14 corpos de eventos normativos de `spec/events.md` e ADR 0020.
//! A validação de corpos assegura invariantes de tipos, limites de tamanho e
//! consistência de evidências registradas sem inferir DAG histórica ou autoridade externa.

use logos_air::{Formula, Limits, Vocabulary};
use logos_canon::{Array, Canon, Text, Value};
use logos_core::{Budget, CheckOutcome};
use logos_crypto::{Domain, Hash, KeyId, PublicKey, Signature, tags};
use logos_obligation::Obligation;

use crate::{Event, EventError};

/// Limite inclusivo de referências por lista de eventos (`spec/events.md` §5).
pub const MAX_REFS: usize = 1024;

/// Limite inclusivo de bytes UTF-8 por campo de texto (`spec/events.md` §5).
pub const MAX_TEXT_BYTES: usize = 64 * 1024;

/// Marcador de tipo de domínio para certificados de prova (`logos/certificate/v1\0`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Certificate;

impl Domain for Certificate {
    const TAG: &'static [u8] = tags::CERTIFICATE;
}

/// Marcador de tipo de domínio para binários/especificações de checkers (`logos/checker/v1\0`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Checker;

impl Domain for Checker {
    const TAG: &'static [u8] = tags::CHECKER;
}

/// Marcador de tipo de domínio para modelos finitos e contramodelos (`logos/model/v1\0`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModelObject;

impl Domain for ModelObject {
    const TAG: &'static [u8] = tags::MODEL;
}

/// Rota de checagem ou demonstração de obrigação (`spec/events.md` §2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Route {
    /// Rota de busca ou conferência de modelo finito.
    Model = 0,
    /// Rota de checagem proposicional LRAT.
    Lrat = 1,
    /// Rota de checagem SMT Alethe.
    Alethe = 2,
    /// Rota de prova interativa Lean.
    Lean = 3,
}

/// Par de chave e assinatura de autoridade de governança ou emissão (`spec/events.md` §2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Authority {
    /// Identificador da chave pública da autoridade.
    pub key: KeyId,
    /// Assinatura sobre o payload canônico de autoridade.
    pub sig: Signature,
}

/// Texto não confiável que preserva bytes UTF-8 originais sem normalização canônica (`spec/events.md` §2).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaintedText(pub(crate) String);

impl TaintedText {
    /// Cria uma nova instância de texto tainted se respeitar o limite de 64 KiB.
    ///
    /// # Errors
    /// Retorna [`EventError::Limit`] se o tamanho da string em bytes exceder 64 KiB.
    pub fn new(s: String) -> Result<Self, EventError> {
        if s.len() > MAX_TEXT_BYTES {
            return Err(EventError::Limit("tainted text exceeds 64 KiB"));
        }
        Ok(Self(s))
    }

    /// Retorna a referência à fatia de string não confiável sem normalização.
    #[must_use]
    pub fn as_untrusted_str(&self) -> &str {
        &self.0
    }

    /// Consome o invólucro retornando a string original.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }

    /// Retorna os bytes UTF-8 preservados.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

/// Dados da proposta de afirmação conforme `proposal/v1` (`spec/events.md` §2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventProposal {
    /// Texto descritivo original preservando UTF-8 sem normalização.
    pub text_pt: TaintedText,
    /// Fórmula objetivo da proposta.
    pub goal: Formula,
    /// Citações históricas referenciadas pela proposta (preservam ordem e repetições).
    pub cites: Vec<Hash<Event>>,
    /// Identificadores de suposições especulativas não autenticadas.
    pub speculative_on: Vec<[u8; 32]>,
    /// Vocabulário sob o qual o objetivo é formulado.
    pub vocabulary: Hash<Vocabulary>,
}

/// Marcador de bytes DIMACS canônicos (cnf/v1), sem executar solver.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CnfObject;
impl Domain for CnfObject {
    const TAG: &'static [u8] = tags::CNF;
}

/// Identidade/versionamento da derivação proposicional registrada.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CnfRecord {
    /// Hash com domínio dos bytes DIMACS canônicos.
    pub hash: Hash<CnfObject>,
    /// Versão do encoder; atualmente somente 1 é reconhecida.
    pub encoder_version: u64,
}

/// Registro completo de uma checagem executada (`spec/events.md` §2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckRecord {
    /// Identificador da obrigação checada.
    pub obligation: Hash<Obligation>,
    /// Identificador do certificado de prova ou contramodelo.
    pub certificate: Hash<Certificate>,
    /// Identificador do checker responsável.
    pub checker: Hash<Checker>,
    /// Orçamento alocado ao checker (não medição de recursos consumidos).
    pub budget: Budget,
    /// Resultado retornado pelo checker.
    pub outcome: CheckOutcome,
}

/// Status de consistência das premissas da obrigação (`spec/events.md` §2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Consistency {
    /// Consistência inconclusiva ou sem tentativa de checagem.
    Unknown {
        /// Tentativa opcional com resultado Unknown/Rejected.
        check: Option<CheckRecord>,
    },
    /// Modelo das premissas com checagem aceita; não é contramodelo do objetivo.
    Witness {
        /// Hash do modelo finito testemunha.
        model: Hash<ModelObject>,
        /// Checagem do modelo comprovando aceitação.
        check: CheckRecord,
    },
}

/// Catálogo normativo dos 15 corpos de eventos assinados (`spec/events.md` §3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventBody {
    /// Evento inicial de ancoragem do ledger.
    Genesis {
        /// Versão do ledger (deve ser 1).
        ledger_version: u64,
        /// Digest SHA-256 cru do binário do kernel.
        kernel_binary: [u8; 32],
        /// Chave de governança do kernel.
        kernel_key: KeyId,
    },
    /// Publicação e atestação de compatibilidade de um vocabulário.
    VocabPublished {
        /// Identificador do vocabulário.
        vocabulary: Hash<Vocabulary>,
        /// Indicador de compatibilidade retroativa.
        compatible: bool,
        /// Assinatura da autoridade de vocabulário.
        authority: Authority,
    },
    /// Registro de proposta de declaração ou meta.
    Proposed {
        /// Conteúdo da proposta.
        proposal: EventProposal,
        /// Autor signatário da proposta.
        author: KeyId,
    },
    /// Criação de obrigação canônica direcionada a uma rota.
    ObligationCreated {
        /// Identificador da obrigação formal.
        obligation: Hash<Obligation>,
        /// Rota de checagem.
        route: Route,
        /// Digest SHA-256 cru do artefato direcionado ao provador.
        artifact: [u8; 32],
        /// Metadados CNF; Some usa obligation-body/v2 somente em rota Lrat.
        cnf: Option<CnfRecord>,
    },
    /// Registro de obrigação verificada formalmente com evidências.
    Verified {
        /// Checagem principal aceita.
        check: CheckRecord,
        /// Conjunto estritamente ordenado de eventos citados.
        cited: Vec<Hash<Event>>,
        /// Subconjunto estritamente ordenado de eventos utilizados na prova.
        used: Vec<Hash<Event>>,
        /// Evidência/status de consistência das premissas.
        consistency: Consistency,
    },
    /// Registro de obrigação refutada por contramodelo aceito.
    Refuted {
        /// Checagem do contramodelo aceito.
        check: CheckRecord,
        /// Identificador do modelo refutador.
        model: Hash<ModelObject>,
    },
    /// Registro de checagem inconclusiva por esgotamento de recursos.
    UnknownRecorded {
        /// Checagem com desfecho Unknown.
        check: CheckRecord,
        /// Identificador canônico da máquina/ambiente de execução.
        machine: Text,
    },
    /// Certificado rejeitado; não é refutação e preserva Pending.
    CertificateRejected {
        /// Checagem com resultado Rejected.
        check: CheckRecord,
    },
    /// Tentativa de consistência anexada sem alterar um evento anterior.
    ConsistencyRecorded {
        /// Pending ou Verified da instância checada.
        target: Hash<Event>,
        /// Resultado de check_consistency; nunca refutação.
        check: CheckRecord,
        /// Candidato checado, inclusive quando rejeitado.
        model: Hash<ModelObject>,
    },
    /// Declaração de pressuposto ou axioma contextual.
    AssumptionDeclared {
        /// Vocabulário de contextualização.
        vocabulary: Hash<Vocabulary>,
        /// Fórmula formal do pressuposto.
        formula: Formula,
        /// Escopo textual associado ao pressuposto.
        scope: Text,
        /// Autor que declarou o pressuposto.
        author: KeyId,
    },
    /// Retratação formal de um evento prévio por autoridade.
    Retracted {
        /// Evento alvo da retratação.
        target: Hash<Event>,
        /// Justificativa textual canônica.
        reason: Text,
        /// Assinatura da autoridade competente.
        authority: Authority,
    },
    /// Invalidação encadeada de afirmação decorrente de premissas invalidadas.
    Invalidated {
        /// Afirmação invalidada.
        claim: Hash<Event>,
        /// Caminho causal ordenado e não vazio de causas da invalidação.
        causes: Vec<Hash<Event>>,
    },
    /// Avanço de época administrativa do ledger.
    EpochAdvanced {
        /// Novo número de época.
        new_epoch: u64,
        /// Motivo textual canônico da transição.
        reason: Text,
        /// Compromisso Merkle opaco; não autentica ou publica uma política.
        policy_root: Option<[u8; 32]>,
        /// Vocabulário associado à transição; não implica compatibilidade.
        vocab: Option<Hash<Vocabulary>>,
    },
    /// Registro e habilitação de um verificador de provas.
    CheckerRegistered {
        /// Identificador do checker registrado.
        checker: Hash<Checker>,
        /// Formato de prova aceito pelo checker.
        format: Text,
        /// Assinatura de autorização administrativa.
        authority: Authority,
    },
    /// Recheck de um certificado Verified com outro checker (5.10, ADR0056).
    ///
    /// Accepted reativa a rota suspensa pela revogação do checker original;
    /// Rejected a invalida e abre a cascata; Unknown mantém a suspensão.
    Rechecked {
        /// Evento Verified cujo certificado foi rechecado.
        route: Hash<Event>,
        /// Mesmos h(O)/h(certificado) da rota, com o checker novo e o desfecho.
        check: CheckRecord,
    },
    /// Registro de um fato assinado por emissor da raiz de confiança (6.1, ADR0061).
    ///
    /// O objeto fact/v1 fica no CAS; o kernel só grava este evento depois de
    /// conferir emissor, assinatura e tipagem da fórmula.
    FactRegistered {
        /// Fato registrado (identidade fact/v1 no CAS).
        fact: Hash<crate::Fact>,
    },
    /// Publicação de uma configuração de confiança por tópico (6.2, ADR0063).
    ///
    /// O objeto trust/v1 fica no CAS; a assinatura do admin cobre `h(config)`.
    TrustConfigPublished {
        /// Configuração publicada (identidade trust/v1 no CAS).
        config: Hash<crate::TrustConfig>,
        /// Assinatura de autorização administrativa.
        authority: Authority,
    },
    /// Revogação de um emissor comprometido (6.4, ADR0064): mata todo fato dele
    /// registrado na época `since` ou depois e recusa fatos novos dele.
    IssuerRevoked {
        /// Emissor revogado.
        issuer: KeyId,
        /// Primeira época de registro afetada.
        since: u64,
        /// Assinatura de autorização administrativa.
        authority: Authority,
    },
    /// Conteúdo externo atribuído (6.6, ADR0068): o extrator afirma que `source` disse
    /// `formula` (literal `Says`-lifted). É premissa de confiança "extração por X",
    /// citável como assumption de escopo `session:extracted`; nunca é fato.
    Extracted {
        /// Vocabulário da fórmula.
        vocabulary: Hash<Vocabulary>,
        /// Nome da constante de sort `Source` que identifica a fonte.
        source: Text,
        /// SHA-256 bruto do conteúdo de onde a fórmula foi extraída.
        content: [u8; 32],
        /// Literal `Says`-lifted atribuído à fonte.
        formula: Formula,
        /// Extrator responsável (pode retratar a própria extração).
        extractor: KeyId,
    },
    /// Revogação formal de um verificador de provas previamente registrado.
    CheckerRevoked {
        /// Identificador do checker revogado.
        checker: Hash<Checker>,
        /// Motivo textual canônico da revogação.
        reason: Text,
        /// Assinatura de autoridade revogadora.
        authority: Authority,
    },
}

/// Escopo fixo das premissas `Extracted` (6.6): condição aberta única de quem as cita.
pub const EXTRACTED_SCOPE: &str = "session:extracted";

fn validate_text(text: &Text, message: &'static str) -> Result<(), EventError> {
    if text.as_str().len() > MAX_TEXT_BYTES {
        return Err(EventError::Limit(message));
    }
    Ok(())
}

fn validate_strictly_sorted_refs(refs: &[Hash<Event>]) -> Result<(), EventError> {
    if refs.len() > MAX_REFS {
        return Err(EventError::Limit("refs list exceeds maximum 1024"));
    }
    for window in refs.windows(2) {
        if window[0] >= window[1] {
            return Err(EventError::Invalid(
                "refs must be strictly sorted and without duplicates",
            ));
        }
    }
    Ok(())
}

fn validate_formula(formula: &Formula) -> Result<(), EventError> {
    let limits = Limits::default();
    if formula.depth() > limits.max_depth || formula.node_count() > limits.max_nodes {
        return Err(EventError::Limit("formula exceeds AIR default limits"));
    }
    Ok(())
}

impl EventProposal {
    pub(crate) fn validate(&self) -> Result<(), EventError> {
        if self.cites.len() > MAX_REFS || self.speculative_on.len() > MAX_REFS {
            return Err(EventError::Limit("proposal references"));
        }
        validate_formula(&self.goal)
    }
}

impl CheckRecord {
    pub(crate) fn validate(&self) -> Result<(), EventError> {
        if let CheckOutcome::Rejected(reason) = &self.outcome
            && reason.as_str().len() > MAX_TEXT_BYTES
        {
            return Err(EventError::Limit("check reason text exceeds 64 KiB"));
        }
        Ok(())
    }
}

impl Consistency {
    pub(crate) fn validate(&self) -> Result<(), EventError> {
        match self {
            Self::Witness { check, .. } if check.outcome != CheckOutcome::Accepted => {
                return Err(EventError::Invalid("Witness requires Accepted"));
            }
            Self::Unknown { check: Some(check) } if check.outcome == CheckOutcome::Accepted => {
                return Err(EventError::Invalid("Unknown cannot contain Accepted"));
            }
            _ => {}
        }
        match self {
            Self::Witness { check, .. } | Self::Unknown { check: Some(check) } => check.validate(),
            Self::Unknown { check: None } => Ok(()),
        }
    }
}

impl EventBody {
    /// Retorna a tag numérica normatizada da variante (`spec/events.md` §3).
    #[must_use]
    pub const fn tag(&self) -> u64 {
        match self {
            Self::Genesis { .. } => 0,
            Self::VocabPublished { .. } => 1,
            Self::Proposed { .. } => 2,
            Self::ObligationCreated { .. } => 3,
            Self::Verified { .. } => 4,
            Self::Refuted { .. } => 5,
            Self::UnknownRecorded { .. } => 6,
            Self::CertificateRejected { .. } => 13,
            Self::ConsistencyRecorded { .. } => 14,
            Self::AssumptionDeclared { .. } => 7,
            Self::Retracted { .. } => 8,
            Self::Invalidated { .. } => 9,
            Self::EpochAdvanced { .. } => 10,
            Self::CheckerRegistered { .. } => 11,
            Self::CheckerRevoked { .. } => 12,
            Self::Rechecked { .. } => 15,
            Self::FactRegistered { .. } => 16,
            Self::TrustConfigPublished { .. } => 17,
            Self::IssuerRevoked { .. } => 18,
            Self::Extracted { .. } => 19,
        }
    }

    /// Retorna a autoridade associada se o corpo contiver assinatura administrativa.
    #[must_use]
    pub fn authority(&self) -> Option<&Authority> {
        match self {
            Self::VocabPublished { authority, .. }
            | Self::Retracted { authority, .. }
            | Self::CheckerRegistered { authority, .. }
            | Self::CheckerRevoked { authority, .. }
            | Self::TrustConfigPublished { authority, .. }
            | Self::IssuerRevoked { authority, .. } => Some(authority),
            _ => None,
        }
    }

    /// Valida invariantes estruturais, limites e consistência sem sort-check de lógica.
    ///
    /// # Errors
    /// Retorna [`EventError::Invalid`] para inconsistências lógicas de estado ou ordenação,
    /// ou [`EventError::Limit`] para violações de limites inclusivos de tamanho.
    pub fn validate(&self) -> Result<(), EventError> {
        match self {
            Self::Genesis { ledger_version, .. } => {
                if *ledger_version != 1 {
                    return Err(EventError::Invalid("Genesis ledger_version must be 1"));
                }
            }
            Self::VocabPublished { .. }
            | Self::FactRegistered { .. }
            | Self::TrustConfigPublished { .. }
            | Self::IssuerRevoked { .. } => {}
            Self::Proposed { proposal, .. } => proposal.validate()?,
            Self::ObligationCreated { route, cnf, .. } => {
                if let Some(c) = cnf
                    && (*route != Route::Lrat || c.encoder_version != 1)
                {
                    return Err(EventError::Invalid("CNF route/encoder version"));
                }
            }
            Self::Verified {
                check,
                cited,
                used,
                consistency,
            } => {
                check.validate()?;
                consistency.validate()?;
                if check.outcome != CheckOutcome::Accepted {
                    return Err(EventError::Invalid(
                        "Verified check outcome must be Accepted",
                    ));
                }
                validate_strictly_sorted_refs(cited)?;
                validate_strictly_sorted_refs(used)?;
                for u in used {
                    if cited.binary_search(u).is_err() {
                        return Err(EventError::Invalid("used must be a subset of cited"));
                    }
                }
                match consistency {
                    Consistency::Witness {
                        check: witness_check,
                        ..
                    } => {
                        if witness_check.obligation != check.obligation {
                            return Err(EventError::Invalid(
                                "Consistency Witness obligation must match main check",
                            ));
                        }
                    }
                    Consistency::Unknown {
                        check: Some(unknown_check),
                    } => {
                        if unknown_check.obligation != check.obligation {
                            return Err(EventError::Invalid(
                                "Consistency Unknown obligation must match main check",
                            ));
                        }
                    }
                    Consistency::Unknown { check: None } => {}
                }
            }
            Self::Refuted { check, .. } => {
                check.validate()?;
                if check.outcome != CheckOutcome::Accepted {
                    return Err(EventError::Invalid(
                        "Refuted check outcome must be Accepted",
                    ));
                }
            }
            Self::ConsistencyRecorded { check, .. } | Self::Rechecked { check, .. } => {
                check.validate()?;
            }
            Self::CertificateRejected { check } => {
                check.validate()?;
                if !matches!(check.outcome, CheckOutcome::Rejected(_)) {
                    return Err(EventError::Invalid("CertificateRejected requires Rejected"));
                }
            }
            Self::UnknownRecorded { check, machine } => {
                check.validate()?;
                if !matches!(check.outcome, CheckOutcome::Unknown(_)) {
                    return Err(EventError::Invalid(
                        "UnknownRecorded check outcome must be Unknown",
                    ));
                }
                validate_text(machine, "machine text exceeds 64 KiB")?;
            }
            Self::AssumptionDeclared { formula, scope, .. } => {
                validate_formula(formula)?;
                validate_text(scope, "scope text exceeds 64 KiB")?;
            }
            Self::Extracted {
                source, formula, ..
            } => {
                validate_formula(formula)?;
                if source.as_str().is_empty() {
                    return Err(EventError::Invalid("Extracted source must not be empty"));
                }
                validate_text(source, "source text exceeds 64 KiB")?;
            }
            Self::Retracted { reason, .. } => {
                validate_text(reason, "reason text exceeds 64 KiB")?;
            }
            Self::Invalidated { causes, .. } => {
                if causes.is_empty() {
                    return Err(EventError::Invalid("causes path must not be empty"));
                }
                if causes.len() > MAX_REFS {
                    return Err(EventError::Limit("causes path exceeds 1024"));
                }
            }
            Self::EpochAdvanced { reason, .. } => {
                validate_text(reason, "reason text exceeds 64 KiB")?;
            }
            Self::CheckerRegistered { format, .. } => {
                validate_text(format, "format text exceeds 64 KiB")?;
            }
            Self::CheckerRevoked { reason, .. } => {
                validate_text(reason, "reason text exceeds 64 KiB")?;
            }
        }
        Ok(())
    }

    /// Produz os bytes canônicos da mensagem a ser assinada pela autoridade (`spec/events.md` §4).
    ///
    /// O payload é `canon([1, 1, body_without_authority_signature])`, substituindo
    /// `Authority` por apenas `KeyId`. Retorna `Ok(None)` para eventos sem autoridade.
    ///
    /// # Errors
    /// Retorna erro se a codificação canônica falhar.
    pub fn authority_message(&self) -> Result<Option<Vec<u8>>, EventError> {
        self.validate()?;
        let body_without_sig = match self {
            Self::VocabPublished {
                vocabulary,
                compatible,
                authority,
            } => Value::Array(Array::new(vec![
                Value::Unsigned(1),
                Value::Unsigned(1),
                vocabulary.to_value()?,
                Value::Bool(*compatible),
                authority.key.to_value()?,
            ])?),
            Self::Retracted {
                target,
                reason,
                authority,
            } => Value::Array(Array::new(vec![
                Value::Unsigned(1),
                Value::Unsigned(8),
                target.to_value()?,
                reason.to_value()?,
                authority.key.to_value()?,
            ])?),
            Self::CheckerRegistered {
                checker,
                format,
                authority,
            } => Value::Array(Array::new(vec![
                Value::Unsigned(1),
                Value::Unsigned(11),
                checker.to_value()?,
                format.to_value()?,
                authority.key.to_value()?,
            ])?),
            Self::CheckerRevoked {
                checker,
                reason,
                authority,
            } => Value::Array(Array::new(vec![
                Value::Unsigned(1),
                Value::Unsigned(12),
                checker.to_value()?,
                reason.to_value()?,
                authority.key.to_value()?,
            ])?),
            Self::TrustConfigPublished { config, authority } => Value::Array(Array::new(vec![
                Value::Unsigned(1),
                Value::Unsigned(17),
                config.to_value()?,
                authority.key.to_value()?,
            ])?),
            Self::IssuerRevoked {
                issuer,
                since,
                authority,
            } => Value::Array(Array::new(vec![
                Value::Unsigned(1),
                Value::Unsigned(18),
                issuer.to_value()?,
                Value::Unsigned(*since),
                authority.key.to_value()?,
            ])?),
            _ => return Ok(None),
        };

        let message = Value::Array(Array::new(vec![
            Value::Unsigned(1),
            Value::Unsigned(1),
            body_without_sig,
        ])?);
        let mut bytes = Vec::with_capacity(crate::event::encoded_size(&message)?);
        message.encode_into(&mut bytes);
        Ok(Some(bytes))
    }

    /// Confere a assinatura de autoridade e a correspondência do KeyId com a chave pública esperada.
    ///
    /// # Errors
    /// Retorna [`EventError::Invalid`] se o evento não contiver autoridade ou a chave divergir,
    /// ou erro criptográfico caso a assinatura não verifique no domínio `logos/sig/event/v1\0`.
    pub fn verify_authority(&self, public_key: &PublicKey) -> Result<(), EventError> {
        let authority = match self.authority() {
            Some(auth) => auth,
            None => {
                return Err(EventError::Invalid(
                    "event body does not have authority signature",
                ));
            }
        };

        let expected_key_id = public_key.key_id()?;
        if authority.key != expected_key_id {
            return Err(EventError::Invalid("authority key mismatch"));
        }

        let message_bytes = self
            .authority_message()?
            .ok_or(EventError::Invalid("missing authority payload"))?;

        logos_crypto::verify::<Event>(&message_bytes, &authority.sig, public_key)?;
        Ok(())
    }
}
