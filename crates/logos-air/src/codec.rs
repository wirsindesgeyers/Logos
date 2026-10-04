//! Codificação e decodificação estritas de `air/v1`.

use crate::error::AirError;
use crate::shape::{Shape, Stats};
use crate::syntax::{
    Formula, FormulaNode, Limits, LogicId, Sort, Symbol, SymbolKind, SymbolResult, Term, TermNode,
};
use logos_canon::{Array, Canon, CanonDecode, CanonError, Decoder, Text, Value};

const AIR_FORMAT_VERSION: u64 = 1;

fn array(items: Vec<Value>) -> Result<Value, CanonError> {
    Array::new(items).map(Value::Array)
}

fn items(value: &Value) -> Result<&[Value], CanonError> {
    match value {
        Value::Array(array) => Ok(array.items()),
        _ => Err(CanonError::WrongType),
    }
}

fn count(value: &Value) -> Result<usize, CanonError> {
    usize::try_from(u64::from_value(value)?).map_err(|_| CanonError::IntOutOfRange)
}

fn count_value(count: usize) -> Result<Value, CanonError> {
    u64::try_from(count)
        .map(Value::Unsigned)
        .map_err(|_| CanonError::IntOutOfRange)
}

fn canon_error(error: AirError) -> CanonError {
    match error {
        AirError::Canon(error) => error,
        _ => CanonError::WrongType,
    }
}

fn node_records(value: &Value, expected_type: u64) -> Result<&[Value], AirError> {
    let [
        Value::Unsigned(AIR_FORMAT_VERSION),
        Value::Unsigned(type_tag),
        Value::Text(logic),
        records,
    ] = items(value)?
    else {
        return Err(CanonError::WrongType.into());
    };
    if *type_tag != expected_type {
        return Err(CanonError::WrongType.into());
    }
    LogicId::from_name(logic.as_str())?;
    Ok(items(records)?)
}

fn envelope(type_tag: u64, logic: LogicId, records: Vec<Value>) -> Result<Value, CanonError> {
    array(vec![
        Value::Unsigned(AIR_FORMAT_VERSION),
        Value::Unsigned(type_tag),
        Value::Text(Text::new(logic.as_str())?),
        array(records)?,
    ])
}

impl Canon for Sort {
    fn to_value(&self) -> Result<Value, CanonError> {
        array(vec![
            Value::Unsigned(AIR_FORMAT_VERSION),
            Value::Unsigned(0),
            self.name.to_value()?,
        ])
    }
}

impl CanonDecode for Sort {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        let [
            Value::Unsigned(AIR_FORMAT_VERSION),
            Value::Unsigned(0),
            name,
        ] = items(value)?
        else {
            return Err(CanonError::WrongType);
        };
        Ok(Self {
            name: Text::from_value(name)?,
        })
    }
}

impl Canon for Symbol {
    fn to_value(&self) -> Result<Value, CanonError> {
        let kind = match self.kind {
            SymbolKind::Const => 0,
            SymbolKind::Func => 1,
            SymbolKind::Pred => 2,
        };
        let arity = self
            .arity
            .iter()
            .map(Canon::to_value)
            .collect::<Result<Vec<_>, _>>()?;
        let result = match &self.result {
            SymbolResult::Sort(sort) => array(vec![Value::Unsigned(0), sort.to_value()?])?,
            SymbolResult::Bool => array(vec![Value::Unsigned(1)])?,
        };
        array(vec![
            Value::Unsigned(AIR_FORMAT_VERSION),
            Value::Unsigned(1),
            self.name.to_value()?,
            Value::Unsigned(kind),
            array(arity)?,
            result,
        ])
    }
}

impl CanonDecode for Symbol {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        let [
            Value::Unsigned(AIR_FORMAT_VERSION),
            Value::Unsigned(1),
            name,
            kind,
            arity,
            result,
        ] = items(value)?
        else {
            return Err(CanonError::WrongType);
        };
        let name = Text::from_value(name)?;
        let kind = match kind {
            Value::Unsigned(0) => SymbolKind::Const,
            Value::Unsigned(1) => SymbolKind::Func,
            Value::Unsigned(2) => SymbolKind::Pred,
            _ => return Err(CanonError::WrongType),
        };
        let arity = items(arity)?
            .iter()
            .map(Sort::from_value)
            .collect::<Result<Vec<_>, _>>()?;
        let result = match items(result)? {
            [Value::Unsigned(0), sort] => SymbolResult::Sort(Sort::from_value(sort)?),
            [Value::Unsigned(1)] => SymbolResult::Bool,
            _ => return Err(CanonError::WrongType),
        };
        Ok(Self {
            name,
            kind,
            arity,
            result,
        })
    }
}

fn term_node_value(node: &TermNode) -> Result<Value, CanonError> {
    match node {
        TermNode::Var { index, sort } => array(vec![
            Value::Unsigned(0),
            Value::Unsigned(*index),
            sort.to_value()?,
        ]),
        TermNode::App { symbol, args } => array(vec![
            Value::Unsigned(1),
            symbol.to_value()?,
            count_value(*args)?,
        ]),
    }
}

fn term_node(value: &Value) -> Result<TermNode, CanonError> {
    match items(value)? {
        [Value::Unsigned(0), index, sort] => Ok(TermNode::Var {
            index: u64::from_value(index)?,
            sort: Sort::from_value(sort)?,
        }),
        [Value::Unsigned(1), symbol, args] => Ok(TermNode::App {
            symbol: Symbol::from_value(symbol)?,
            args: count(args)?,
        }),
        _ => Err(CanonError::WrongType),
    }
}

impl Canon for Term {
    fn to_value(&self) -> Result<Value, CanonError> {
        let records = self
            .nodes
            .iter()
            .map(term_node_value)
            .collect::<Result<Vec<_>, _>>()?;
        envelope(2, self.logic_id(), records)
    }
}

impl CanonDecode for Term {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        Self::from_value_with_limits(value, Limits::default()).map_err(canon_error)
    }
}

impl Term {
    /// Decodifica um envelope `air/v1` completo sob os limites estruturais dados.
    ///
    /// # Errors
    /// Erros canônicos (inclusive bytes restantes), de lógica, forma ou limites.
    pub fn decode_with_limits(bytes: &[u8], limits: Limits) -> Result<Self, AirError> {
        let mut decoder = Decoder::new(bytes);
        let value = decoder.value()?;
        decoder.finish()?;
        Self::from_value_with_limits(&value, limits)
    }

    /// Constrói um termo de um valor canônico, validando o prefixo incrementalmente.
    ///
    /// # Errors
    /// Erros de formato, lógica não suportada, estrutura incompleta ou limites excedidos.
    pub fn from_value_with_limits(value: &Value, limits: Limits) -> Result<Self, AirError> {
        let records = node_records(value, 2)?;
        let mut shape = Shape::new(limits);
        let mut nodes = Vec::new();
        for record in records {
            shape.next_depth()?;
            let node = term_node(record)?;
            shape.push(node.children(), Stats::default())?;
            nodes.push(node);
        }
        let stats = shape.finish()?;
        Ok(Self { nodes, stats })
    }
}

fn formula_node_value(node: &FormulaNode) -> Result<Value, CanonError> {
    match node {
        FormulaNode::True => array(vec![Value::Unsigned(0)]),
        FormulaNode::False => array(vec![Value::Unsigned(1)]),
        FormulaNode::Eq(left, right) => array(vec![
            Value::Unsigned(2),
            left.to_value()?,
            right.to_value()?,
        ]),
        FormulaNode::Pred { symbol, args } => array(vec![
            Value::Unsigned(3),
            symbol.to_value()?,
            array(
                args.iter()
                    .map(Canon::to_value)
                    .collect::<Result<Vec<_>, _>>()?,
            )?,
        ]),
        FormulaNode::Not => array(vec![Value::Unsigned(4)]),
        FormulaNode::And(children) => array(vec![Value::Unsigned(5), count_value(*children)?]),
        FormulaNode::Or(children) => array(vec![Value::Unsigned(6), count_value(*children)?]),
        FormulaNode::Implies => array(vec![Value::Unsigned(7)]),
        FormulaNode::Iff => array(vec![Value::Unsigned(8)]),
        FormulaNode::Forall(sort) => array(vec![Value::Unsigned(9), sort.to_value()?]),
        FormulaNode::Exists(sort) => array(vec![Value::Unsigned(10), sort.to_value()?]),
    }
}

fn rebase_error(error: AirError, child: Limits, parent: Limits) -> AirError {
    match error {
        AirError::DepthExceeded { actual, .. } => {
            let actual = parent
                .max_depth
                .checked_sub(child.max_depth)
                .and_then(|offset| offset.checked_add(actual));
            match actual {
                Some(actual) => AirError::DepthExceeded {
                    limit: parent.max_depth,
                    actual,
                },
                None => AirError::InvalidStructure,
            }
        }
        AirError::NodesExceeded { actual, .. } => {
            let actual = parent
                .max_nodes
                .checked_sub(child.max_nodes)
                .and_then(|offset| offset.checked_add(actual));
            match actual {
                Some(actual) => AirError::NodesExceeded {
                    limit: parent.max_nodes,
                    actual,
                },
                None => AirError::InvalidStructure,
            }
        }
        other => other,
    }
}

fn embedded_term(value: &Value, limits: &mut Limits, parent: Limits) -> Result<Term, AirError> {
    let term = Term::from_value_with_limits(value, *limits)
        .map_err(|error| rebase_error(error, *limits, parent))?;
    limits.max_nodes = limits
        .max_nodes
        .checked_sub(term.stats.nodes)
        .ok_or(AirError::InvalidStructure)?;
    Ok(term)
}

fn formula_node(value: &Value, shape: &mut Shape, parent: Limits) -> Result<FormulaNode, AirError> {
    match items(value)? {
        [Value::Unsigned(0)] => Ok(FormulaNode::True),
        [Value::Unsigned(1)] => Ok(FormulaNode::False),
        [Value::Unsigned(2), left, right] => {
            let mut limits = shape.child_limits()?;
            let left = embedded_term(left, &mut limits, parent)?;
            let right = embedded_term(right, &mut limits, parent)?;
            Ok(FormulaNode::Eq(left, right))
        }
        [Value::Unsigned(3), symbol, args] => {
            let symbol = Symbol::from_value(symbol)?;
            let mut limits = shape.child_limits()?;
            let mut terms = Vec::new();
            for arg in items(args)? {
                terms.push(embedded_term(arg, &mut limits, parent)?);
            }
            Ok(FormulaNode::Pred {
                symbol,
                args: terms,
            })
        }
        [Value::Unsigned(4)] => Ok(FormulaNode::Not),
        [Value::Unsigned(5), children] => Ok(FormulaNode::And(count(children)?)),
        [Value::Unsigned(6), children] => Ok(FormulaNode::Or(count(children)?)),
        [Value::Unsigned(7)] => Ok(FormulaNode::Implies),
        [Value::Unsigned(8)] => Ok(FormulaNode::Iff),
        [Value::Unsigned(9), sort] => Ok(FormulaNode::Forall(Sort::from_value(sort)?)),
        [Value::Unsigned(10), sort] => Ok(FormulaNode::Exists(Sort::from_value(sort)?)),
        _ => Err(CanonError::WrongType.into()),
    }
}

impl Canon for Formula {
    fn to_value(&self) -> Result<Value, CanonError> {
        let records = self
            .nodes
            .iter()
            .map(formula_node_value)
            .collect::<Result<Vec<_>, _>>()?;
        envelope(3, self.logic_id(), records)
    }
}

impl CanonDecode for Formula {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        Self::from_value_with_limits(value, Limits::default()).map_err(canon_error)
    }
}

impl Formula {
    /// Decodifica um envelope `air/v1` completo sob os limites estruturais dados.
    ///
    /// # Errors
    /// Erros canônicos (inclusive bytes restantes), de lógica, forma ou limites.
    pub fn decode_with_limits(bytes: &[u8], limits: Limits) -> Result<Self, AirError> {
        let mut decoder = Decoder::new(bytes);
        let value = decoder.value()?;
        decoder.finish()?;
        Self::from_value_with_limits(&value, limits)
    }

    /// Constrói uma fórmula de um valor canônico, incluindo o orçamento de seus termos.
    ///
    /// # Errors
    /// Erros de formato, lógica não suportada, estrutura incompleta ou limites excedidos.
    pub fn from_value_with_limits(value: &Value, limits: Limits) -> Result<Self, AirError> {
        let records = node_records(value, 3)?;
        let mut shape = Shape::new(limits);
        let mut nodes = Vec::new();
        for record in records {
            shape.next_depth()?;
            let node = formula_node(record, &mut shape, limits)?;
            shape.push(node.children(), node.embedded_stats()?)?;
            nodes.push(node);
        }
        let stats = shape.finish()?;
        Ok(Self { nodes, stats })
    }
}
