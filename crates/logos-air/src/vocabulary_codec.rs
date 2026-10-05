//! Codificação e decodificação estritas de `vocabulary/v1`.
//!
//! Implementa a serialização e desserialização canônica do vocabulário versionado
//! conforme a especificação normativa em `spec/vocabulary.md`.

use crate::error::AirError;
use crate::syntax::{Formula, Limits, Symbol};
use crate::vocabulary::{
    AttestorId, DomainSource, FactRef, Grounding, PolicyRef, SortDecl, SortDomain, SymbolDecl,
    Vocabulary, VocabularyError,
};
use logos_canon::{Array, Canon, CanonDecode, CanonError, Decoder, Text, Value};

const VOCABULARY_FORMAT_VERSION: u64 = 1;

fn array(items: Vec<Value>) -> Result<Value, CanonError> {
    Array::new(items).map(Value::Array)
}

fn items(value: &Value) -> Result<&[Value], CanonError> {
    match value {
        Value::Array(array) => Ok(array.items()),
        _ => Err(CanonError::WrongType),
    }
}

fn bytes32(value: &Value) -> Result<[u8; 32], CanonError> {
    match value {
        Value::Bytes(bytes) => {
            <[u8; 32]>::try_from(bytes.as_slice()).map_err(|_| CanonError::WrongType)
        }
        _ => Err(CanonError::WrongType),
    }
}

fn canon_error(error: VocabularyError) -> CanonError {
    match error {
        VocabularyError::Canon(error) => error,
        VocabularyError::Air(AirError::Canon(error)) => error,
        _ => CanonError::WrongType,
    }
}

fn domain_source_value(source: &DomainSource) -> Result<Value, CanonError> {
    match source {
        DomainSource::Fact(fact_ref) => {
            array(vec![Value::Unsigned(0), Value::Bytes(fact_ref.0.to_vec())])
        }
        DomainSource::Policy(policy_ref) => array(vec![
            Value::Unsigned(1),
            Value::Bytes(policy_ref.0.to_vec()),
        ]),
    }
}

fn domain_source_from_value(value: &Value) -> Result<DomainSource, CanonError> {
    match items(value)? {
        [Value::Unsigned(0), raw_ref] => Ok(DomainSource::Fact(FactRef(bytes32(raw_ref)?))),
        [Value::Unsigned(1), raw_ref] => Ok(DomainSource::Policy(PolicyRef(bytes32(raw_ref)?))),
        _ => Err(CanonError::WrongType),
    }
}

fn sort_domain_value(domain: &SortDomain) -> Result<Value, CanonError> {
    match domain {
        SortDomain::Open => array(vec![Value::Unsigned(0)]),
        SortDomain::Enumerated {
            constants,
            domain_source,
        } => {
            let constants_val = constants
                .iter()
                .map(Canon::to_value)
                .collect::<Result<Vec<_>, _>>()?;
            let source_val = domain_source_value(domain_source)?;
            array(vec![Value::Unsigned(1), array(constants_val)?, source_val])
        }
    }
}

fn sort_domain_from_value(value: &Value) -> Result<SortDomain, CanonError> {
    match items(value)? {
        [Value::Unsigned(0)] => Ok(SortDomain::Open),
        [Value::Unsigned(1), raw_constants, raw_source] => {
            let const_items = items(raw_constants)?;
            let mut constants = Vec::with_capacity(const_items.len());
            for item in const_items {
                constants.push(Symbol::from_value(item)?);
            }
            let domain_source = domain_source_from_value(raw_source)?;
            Ok(SortDomain::Enumerated {
                constants,
                domain_source,
            })
        }
        _ => Err(CanonError::WrongType),
    }
}

fn sort_decl_value(decl: &SortDecl) -> Result<Value, CanonError> {
    array(vec![
        decl.name.to_value()?,
        sort_domain_value(&decl.domain)?,
    ])
}

fn sort_decl_from_value(value: &Value) -> Result<SortDecl, CanonError> {
    match items(value)? {
        [raw_name, raw_domain] => {
            let name = Text::from_value(raw_name)?;
            let domain = sort_domain_from_value(raw_domain)?;
            Ok(SortDecl { name, domain })
        }
        _ => Err(CanonError::WrongType),
    }
}

fn grounding_value(grounding: &Grounding) -> Result<Value, CanonError> {
    match grounding {
        Grounding::Defined(formula) => array(vec![Value::Unsigned(0), formula.to_value()?]),
        Grounding::Anchored {
            attestor,
            procedure,
        } => array(vec![
            Value::Unsigned(1),
            attestor.0.to_value()?,
            procedure.to_value()?,
        ]),
        Grounding::Glossed(text) => array(vec![Value::Unsigned(2), text.to_value()?]),
    }
}

fn grounding_from_value(value: &Value, limits: Limits) -> Result<Grounding, VocabularyError> {
    match items(value).map_err(VocabularyError::Canon)? {
        [Value::Unsigned(0), raw_formula] => {
            let formula = Formula::from_value_with_limits(raw_formula, limits)?;
            Ok(Grounding::Defined(formula))
        }
        [Value::Unsigned(1), raw_attestor, raw_procedure] => {
            let attestor = Text::from_value(raw_attestor).map_err(VocabularyError::Canon)?;
            let procedure = Text::from_value(raw_procedure).map_err(VocabularyError::Canon)?;
            Ok(Grounding::Anchored {
                attestor: AttestorId(attestor),
                procedure,
            })
        }
        [Value::Unsigned(2), raw_text] => {
            let text = Text::from_value(raw_text).map_err(VocabularyError::Canon)?;
            Ok(Grounding::Glossed(text))
        }
        _ => Err(CanonError::WrongType.into()),
    }
}

fn symbol_decl_value(decl: &SymbolDecl) -> Result<Value, CanonError> {
    array(vec![
        decl.signature.to_value()?,
        grounding_value(&decl.grounding)?,
        decl.gloss_pt.to_value()?,
    ])
}

fn symbol_decl_from_value(value: &Value, limits: Limits) -> Result<SymbolDecl, VocabularyError> {
    match items(value).map_err(VocabularyError::Canon)? {
        [raw_signature, raw_grounding, raw_gloss] => {
            let signature = Symbol::from_value(raw_signature).map_err(VocabularyError::Canon)?;
            let grounding = grounding_from_value(raw_grounding, limits)?;
            let gloss_pt = Text::from_value(raw_gloss).map_err(VocabularyError::Canon)?;
            Ok(SymbolDecl {
                signature,
                grounding,
                gloss_pt,
            })
        }
        _ => Err(CanonError::WrongType.into()),
    }
}

impl Canon for Vocabulary {
    fn to_value(&self) -> Result<Value, CanonError> {
        let sorts_val = self
            .sorts()
            .iter()
            .map(sort_decl_value)
            .collect::<Result<Vec<_>, _>>()?;
        let symbols_val = self
            .symbols()
            .iter()
            .map(symbol_decl_value)
            .collect::<Result<Vec<_>, _>>()?;
        array(vec![
            Value::Unsigned(VOCABULARY_FORMAT_VERSION),
            Value::Unsigned(self.version()),
            array(sorts_val)?,
            array(symbols_val)?,
        ])
    }
}

impl CanonDecode for Vocabulary {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        Self::from_value_with_limits(value, Limits::default()).map_err(canon_error)
    }
}

impl Vocabulary {
    /// Decodifica um envelope `vocabulary/v1` completo sob os limites estruturais dados.
    ///
    /// # Errors
    /// Retorna erro canônico (inclusive bytes excedentes), erro de AIR em fórmulas,
    /// ou erro de validação semântica do vocabulário.
    pub fn decode_with_limits(bytes: &[u8], limits: Limits) -> Result<Self, VocabularyError> {
        let mut decoder = Decoder::new(bytes);
        let value = decoder.value()?;
        decoder.finish()?;
        Self::from_value_with_limits(&value, limits)
    }

    /// Reconstrói um vocabulário a partir de um valor canônico CBOR e limites dados.
    ///
    /// # Errors
    /// Retorna erro canônico caso a estrutura não siga o layout de `vocabulary/v1`,
    /// erro de AIR caso alguma fórmula exceda os limites ou viole o formato,
    /// ou erro semântico de validação do vocabulário (`VocabularyError`).
    pub fn from_value_with_limits(value: &Value, limits: Limits) -> Result<Self, VocabularyError> {
        let [
            Value::Unsigned(VOCABULARY_FORMAT_VERSION),
            Value::Unsigned(version),
            raw_sorts,
            raw_symbols,
        ] = items(value).map_err(VocabularyError::Canon)?
        else {
            return Err(CanonError::WrongType.into());
        };

        let sort_items = items(raw_sorts).map_err(VocabularyError::Canon)?;
        let mut sorts = Vec::with_capacity(sort_items.len());
        for sort_val in sort_items {
            sorts.push(sort_decl_from_value(sort_val).map_err(VocabularyError::Canon)?);
        }

        let symbol_items = items(raw_symbols).map_err(VocabularyError::Canon)?;
        let mut symbols = Vec::with_capacity(symbol_items.len());
        for symbol_val in symbol_items {
            symbols.push(symbol_decl_from_value(symbol_val, limits)?);
        }

        Self::new(*version, sorts, symbols)
    }
}
