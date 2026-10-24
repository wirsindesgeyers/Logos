//! Árvore Merkle ordenada de regras e provas de inclusão e de não-pertinência (7.2, ADR 0070).
//!
//! Folha = `(chave de indexação, h(regra))`, ordenada por chave e depois por `h(regra)`. A chave é
//! `(classe, símbolo-cabeça da ação)` (vazio = qualquer ação), o que permite provar "não existe
//! regra `deny` que case com a ação `a`" mostrando duas folhas adjacentes que a envolvem. A árvore
//! é a de RFC 9162 §2.1 (divisão na maior potência de 2 menor que `n`), com hashes separados por
//! domínio (`merkle-leaf`, `merkle-node`, `merkle-empty`).

use logos_canon::{Array, Canon, CanonDecode, CanonError, Text, Value};
use logos_crypto::{Domain, RawHasher, hash, tags};

use crate::{Class, Policy, PolicyError, Rule, shape};

/// Teto de folhas (igual ao teto de regras).
const MAX_LEAVES: u64 = crate::MAX_RULES as u64;
/// Teto de passos de um caminho de auditoria (uma árvore de 2^63 folhas já seria absurda).
const MAX_PATH: usize = 64;

struct LeafDomain;
impl Domain for LeafDomain {
    const TAG: &'static [u8] = tags::MERKLE_LEAF;
}
struct NodeDomain;
impl Domain for NodeDomain {
    const TAG: &'static [u8] = tags::MERKLE_NODE;
}
struct EmptyDomain;
impl Domain for EmptyDomain {
    const TAG: &'static [u8] = tags::MERKLE_EMPTY;
}

impl Domain for Rule {
    const TAG: &'static [u8] = tags::RULE;
}

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

/// Chave de indexação: `(classe, símbolo)`; ordem por tag da classe e depois bytes do símbolo.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct IndexKey {
    class: u64,
    symbol: String,
}

impl IndexKey {
    /// Chave de consulta: regras de `class` cuja cabeça é `symbol` (vazio = qualquer ação).
    #[must_use]
    pub fn new(class: Class, symbol: &str) -> Self {
        Self {
            class: class.tag(),
            symbol: symbol.to_owned(),
        }
    }

    /// Chave de uma regra: `allow`/`deny` pelo símbolo da ação na cabeça, `derive` pelo predicado
    /// definido, `require` sempre vazia.
    ///
    /// # Errors
    /// Regra fora da forma da sua classe (nunca ocorre numa regra de política validada).
    pub fn of_rule(rule: &Rule) -> Result<Self, PolicyError> {
        let symbol = match rule.class {
            Class::Require => String::new(),
            class => {
                let tree = shape::Tree::new(&rule.formula);
                let shape = shape::analyse(rule.id.as_str(), class, &tree)?;
                match class {
                    Class::Derive => shape.head,
                    _ => shape.action,
                }
            }
        };
        Ok(Self::new(rule.class, &symbol))
    }

    /// Classe da chave.
    ///
    /// # Errors
    /// Tag de classe desconhecida (chave decodificada de bytes corrompidos).
    pub fn class(&self) -> Result<Class, CanonError> {
        Class::from_tag(self.class)
    }

    /// Símbolo-cabeça; vazio é o curinga.
    #[must_use]
    pub fn symbol(&self) -> &str {
        &self.symbol
    }
}

/// Uma folha: chave de indexação e `h(regra)`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Leaf {
    key: IndexKey,
    rule: [u8; 32],
}

impl Leaf {
    /// Folha de uma regra.
    ///
    /// # Errors
    /// Regra fora da forma da classe ou falha de codificação.
    pub fn of_rule(rule: &Rule) -> Result<Self, PolicyError> {
        Ok(Self {
            key: IndexKey::of_rule(rule)?,
            rule: *hash(rule)?.as_bytes(),
        })
    }

    /// Chave de indexação.
    #[must_use]
    pub const fn key(&self) -> &IndexKey {
        &self.key
    }

    /// `h(regra)`.
    #[must_use]
    pub const fn rule_hash(&self) -> &[u8; 32] {
        &self.rule
    }

    fn digest(&self) -> Result<[u8; 32], CanonError> {
        let mut hasher = RawHasher::<LeafDomain>::new();
        hasher.update(&logos_canon::to_canon(self)?);
        Ok(*hasher.finalize().as_bytes())
    }
}

impl Canon for Leaf {
    fn to_value(&self) -> Result<Value, CanonError> {
        array(vec![
            Value::Unsigned(self.key.class),
            Text::new(self.key.symbol.as_str())?.to_value()?,
            Value::Bytes(self.rule.to_vec()),
        ])
    }
}

impl CanonDecode for Leaf {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        let [class, symbol, rule] = items(value)? else {
            return Err(CanonError::WrongType);
        };
        let class = u64::from_value(class)?;
        Class::from_tag(class)?;
        Ok(Self {
            key: IndexKey {
                class,
                symbol: Text::from_value(symbol)?.as_str().to_owned(),
            },
            rule: bytes32(rule)?,
        })
    }
}

fn node(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = RawHasher::<NodeDomain>::new();
    hasher.update(left);
    hasher.update(right);
    *hasher.finalize().as_bytes()
}

/// Raiz de uma árvore vazia.
#[must_use]
pub fn empty_root() -> [u8; 32] {
    *RawHasher::<EmptyDomain>::new().finalize().as_bytes()
}

/// Maior potência de 2 estritamente menor que `n` (`n ≥ 2`).
fn split(n: usize) -> usize {
    1usize << (usize::BITS - 1 - (n - 1).leading_zeros())
}

fn root_of(hashes: &[[u8; 32]]) -> [u8; 32] {
    match hashes {
        [] => empty_root(),
        [only] => *only,
        _ => {
            let (left, right) = hashes.split_at(split(hashes.len()));
            node(&root_of(left), &root_of(right))
        }
    }
}

fn path_of(index: usize, hashes: &[[u8; 32]]) -> Vec<[u8; 32]> {
    if hashes.len() <= 1 {
        return Vec::new();
    }
    let (left, right) = hashes.split_at(split(hashes.len()));
    if index < left.len() {
        let mut path = path_of(index, left);
        path.push(root_of(right));
        path
    } else {
        let mut path = path_of(index - left.len(), right);
        path.push(root_of(left));
        path
    }
}

/// Caminho de auditoria de uma folha (RFC 9162 §2.1.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InclusionProof {
    /// Posição da folha (0-based).
    pub index: u64,
    /// Número de folhas da árvore.
    pub size: u64,
    /// Irmãos do caminho, da folha para a raiz.
    pub path: Vec<[u8; 32]>,
}

impl Canon for InclusionProof {
    fn to_value(&self) -> Result<Value, CanonError> {
        array(vec![
            Value::Unsigned(self.index),
            Value::Unsigned(self.size),
            array(self.path.iter().map(|h| Value::Bytes(h.to_vec())).collect())?,
        ])
    }
}

impl CanonDecode for InclusionProof {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        let [index, size, path] = items(value)? else {
            return Err(CanonError::WrongType);
        };
        let path = items(path)?;
        if path.len() > MAX_PATH {
            return Err(CanonError::WrongType);
        }
        Ok(Self {
            index: u64::from_value(index)?,
            size: u64::from_value(size)?,
            path: path.iter().map(bytes32).collect::<Result<_, _>>()?,
        })
    }
}

/// Confere o caminho de `leaf` contra `root` (algoritmo de verificação do RFC 9162 §2.1.3.2).
#[must_use]
pub fn verify_inclusion(root: &[u8; 32], leaf: &Leaf, proof: &InclusionProof) -> bool {
    if proof.size == 0 || proof.size > MAX_LEAVES || proof.index >= proof.size {
        return false;
    }
    leaf.digest()
        .ok()
        .is_some_and(|digest| climb(root, digest, proof))
}

/// Sobe do `digest` da folha até a raiz seguindo `proof.path` (RFC 9162 §2.1.3.2).
fn climb(root: &[u8; 32], mut computed: [u8; 32], proof: &InclusionProof) -> bool {
    let (mut position, mut last) = (proof.index, proof.size - 1);
    for sibling in &proof.path {
        if last == 0 {
            return false;
        }
        if position & 1 == 1 || position == last {
            computed = node(sibling, &computed);
            // Aqui `position == last > 0` quando é par, então o laço termina antes de zero.
            while position & 1 == 0 {
                position >>= 1;
                last >>= 1;
            }
        } else {
            computed = node(&computed, sibling);
        }
        position >>= 1;
        last >>= 1;
    }
    last == 0 && &computed == root
}

/// A árvore de uma política: folhas ordenadas e a raiz `r_v`.
#[derive(Debug, Clone)]
pub struct PolicyTree {
    leaves: Vec<Leaf>,
    digests: Vec<[u8; 32]>,
    root: [u8; 32],
}

impl PolicyTree {
    /// Constrói a árvore das regras da política.
    ///
    /// # Errors
    /// Regra fora da forma da classe ou falha de codificação.
    pub fn new(policy: &Policy) -> Result<Self, PolicyError> {
        let mut leaves = policy
            .rules()
            .iter()
            .map(Leaf::of_rule)
            .collect::<Result<Vec<_>, _>>()?;
        leaves.sort();
        let digests = leaves
            .iter()
            .map(Leaf::digest)
            .collect::<Result<Vec<_>, _>>()?;
        let root = root_of(&digests);
        Ok(Self {
            leaves,
            digests,
            root,
        })
    }

    /// Raiz `r_v`.
    #[must_use]
    pub const fn root(&self) -> &[u8; 32] {
        &self.root
    }

    /// Folhas em ordem de `(chave, h(regra))`.
    #[must_use]
    pub fn leaves(&self) -> &[Leaf] {
        &self.leaves
    }

    fn inclusion(&self, index: usize) -> InclusionProof {
        InclusionProof {
            index: index as u64,
            size: self.leaves.len() as u64,
            path: path_of(index, &self.digests),
        }
    }

    /// Prova de inclusão da folha de `rule_hash`, se a regra está na árvore.
    #[must_use]
    pub fn prove_inclusion(&self, rule_hash: &[u8; 32]) -> Option<(Leaf, InclusionProof)> {
        let index = self.leaves.iter().position(|l| &l.rule == rule_hash)?;
        Some((self.leaves[index].clone(), self.inclusion(index)))
    }

    /// Prova de que alguma regra tem a chave `key` (a primeira folha com essa chave).
    #[must_use]
    pub fn prove_present(&self, key: &IndexKey) -> Option<(Leaf, InclusionProof)> {
        let index = self.leaves.iter().position(|l| &l.key == key)?;
        Some((self.leaves[index].clone(), self.inclusion(index)))
    }

    /// Prova de que nenhuma regra tem a chave `key`; `None` se ela existe.
    #[must_use]
    pub fn prove_non_membership(&self, key: &IndexKey) -> Option<NonMembership> {
        if self.leaves.iter().any(|l| &l.key == key) {
            return None;
        }
        // Primeiro índice cuja chave passa de `key`.
        let after = self.leaves.partition_point(|l| &l.key < key);
        let size = self.leaves.len() as u64;
        let left = after
            .checked_sub(1)
            .map(|i| (self.leaves[i].clone(), self.inclusion(i)));
        let right = self
            .leaves
            .get(after)
            .map(|leaf| (leaf.clone(), self.inclusion(after)));
        Some(NonMembership { size, left, right })
    }

    /// Provas de `NoRule(class, a, v)` para uma ação de símbolo `symbol`: nenhuma regra da classe
    /// com esse símbolo nem com o curinga. `None` se alguma existe.
    #[must_use]
    pub fn prove_no_rule(&self, class: Class, symbol: &str) -> Option<NoRuleProof> {
        let wildcard = self.prove_non_membership(&IndexKey::new(class, ""))?;
        let specific = if symbol.is_empty() {
            None
        } else {
            Some(self.prove_non_membership(&IndexKey::new(class, symbol))?)
        };
        Some(NoRuleProof { specific, wildcard })
    }
}

/// Folha adjacente com seu caminho de auditoria.
pub type Neighbour = (Leaf, InclusionProof);

/// Prova de que nenhuma folha tem uma chave: as vizinhas imediatas (ou a borda da árvore).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonMembership {
    /// Número de folhas da árvore.
    pub size: u64,
    /// Última folha com chave menor, se houver.
    pub left: Option<Neighbour>,
    /// Primeira folha com chave maior, se houver.
    pub right: Option<Neighbour>,
}

fn neighbour_value(side: &Option<Neighbour>) -> Result<Value, CanonError> {
    match side {
        None => array(vec![]),
        Some((leaf, proof)) => array(vec![array(vec![leaf.to_value()?, proof.to_value()?])?]),
    }
}

fn neighbour_from(value: &Value) -> Result<Option<Neighbour>, CanonError> {
    match items(value)? {
        [] => Ok(None),
        [pair] => match items(pair)? {
            [leaf, proof] => Ok(Some((
                Leaf::from_value(leaf)?,
                InclusionProof::from_value(proof)?,
            ))),
            _ => Err(CanonError::WrongType),
        },
        _ => Err(CanonError::WrongType),
    }
}

impl Canon for NonMembership {
    fn to_value(&self) -> Result<Value, CanonError> {
        array(vec![
            Value::Unsigned(self.size),
            neighbour_value(&self.left)?,
            neighbour_value(&self.right)?,
        ])
    }
}

impl CanonDecode for NonMembership {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        let [size, left, right] = items(value)? else {
            return Err(CanonError::WrongType);
        };
        Ok(Self {
            size: u64::from_value(size)?,
            left: neighbour_from(left)?,
            right: neighbour_from(right)?,
        })
    }
}

impl Canon for NoRuleProof {
    fn to_value(&self) -> Result<Value, CanonError> {
        let specific = match &self.specific {
            None => array(vec![])?,
            Some(proof) => array(vec![proof.to_value()?])?,
        };
        array(vec![specific, self.wildcard.to_value()?])
    }
}

impl CanonDecode for NoRuleProof {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        let [specific, wildcard] = items(value)? else {
            return Err(CanonError::WrongType);
        };
        let specific = match items(specific)? {
            [] => None,
            [proof] => Some(NonMembership::from_value(proof)?),
            _ => return Err(CanonError::WrongType),
        };
        Ok(Self {
            specific,
            wildcard: NonMembership::from_value(wildcard)?,
        })
    }
}

/// Confere que a árvore de raiz `root` não tem nenhuma folha com a chave `key`.
#[must_use]
pub fn verify_non_membership(root: &[u8; 32], key: &IndexKey, proof: &NonMembership) -> bool {
    let bound =
        |side: &Neighbour| side.1.size == proof.size && verify_inclusion(root, &side.0, &side.1);
    match (&proof.left, &proof.right) {
        (None, None) => proof.size == 0 && root == &empty_root(),
        (Some(left), None) => bound(left) && left.1.index + 1 == proof.size && &left.0.key < key,
        (None, Some(right)) => bound(right) && right.1.index == 0 && &right.0.key > key,
        (Some(left), Some(right)) => {
            bound(left)
                && bound(right)
                && left.1.index + 1 == right.1.index
                && &left.0.key < key
                && &right.0.key > key
        }
    }
}

/// Confere que a árvore tem uma folha com a chave `key` (inclusão de uma folha dessa chave).
#[must_use]
pub fn verify_present(
    root: &[u8; 32],
    key: &IndexKey,
    leaf: &Leaf,
    proof: &InclusionProof,
) -> bool {
    &leaf.key == key && verify_inclusion(root, leaf, proof)
}

/// `NoRule(class, a, v)`: não-pertinência da chave específica e do curinga da classe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoRuleProof {
    /// Chave `(classe, símbolo da ação)`; `None` quando o símbolo é o curinga.
    pub specific: Option<NonMembership>,
    /// Chave `(classe, curinga)`.
    pub wildcard: NonMembership,
}

/// Confere `NoRule(class, symbol, root)`: nenhuma regra da classe casa com a ação.
#[must_use]
pub fn verify_no_rule(root: &[u8; 32], class: Class, symbol: &str, proof: &NoRuleProof) -> bool {
    let specific = match (&proof.specific, symbol.is_empty()) {
        (Some(p), false) => verify_non_membership(root, &IndexKey::new(class, symbol), p),
        (None, true) => true,
        _ => false,
    };
    specific && verify_non_membership(root, &IndexKey::new(class, ""), &proof.wildcard)
}
