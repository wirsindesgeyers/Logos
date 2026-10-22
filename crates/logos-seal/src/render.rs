//! `render_pt`: AIR fechada e bem tipada → texto paraphrase-pt/v1.
//!
//! Recursão limitada pela profundidade conferida antes de começar; cada
//! subfórmula fica entre parênteses e cada átomo/argumento entre `⟦ ⟧`.

use std::fmt::Write as _;

use logos_air::{Formula, FormulaNode, Term, TermNode, check};

use crate::{
    ParaphraseError,
    grammar::{
        AND_OPEN, CLOSE, EQ_MIDDLE, EQ_OPEN, EXISTS_BODY, EXISTS_OPEN, FALSE, FORALL_BODY,
        FORALL_OPEN, GROUP_CLOSE, IFF_MIDDLE, IFF_OPEN, IMPLIES_MIDDLE, IMPLIES_OPEN, LIST_FIRST,
        LIST_NEXT, MANY_CONDITIONS, NOT_OPEN, ONE_CONDITION, OPEN, OR_OPEN, SORT_OPEN, TRUE,
        VARIABLE_PREFIX,
    },
    paraphrase::Paraphraser,
};

struct Renderer<'p, 'v> {
    paraphraser: &'p Paraphraser<'v>,
    out: String,
}

pub(crate) fn render(
    paraphraser: &Paraphraser<'_>,
    formula: &Formula,
) -> Result<String, ParaphraseError> {
    let limits = paraphraser.limits.air;
    if formula.depth() > limits.max_depth || formula.node_count() > limits.max_nodes {
        return Err(ParaphraseError::DepthExceeded);
    }
    check(paraphraser.vocabulary, formula)?;
    let mut renderer = Renderer {
        paraphraser,
        out: String::new(),
    };
    let mut position = 0;
    renderer.formula(formula.nodes(), &mut position, 0)?;
    if position != formula.nodes().len() {
        return Err(ParaphraseError::RoundTripMismatch);
    }
    Ok(renderer.out)
}

impl Renderer<'_, '_> {
    fn push(&mut self, text: &str) -> Result<(), ParaphraseError> {
        if self.out.len() + text.len() > self.paraphraser.limits.max_bytes {
            return Err(ParaphraseError::TooLarge);
        }
        self.out.push_str(text);
        Ok(())
    }

    fn group(
        &mut self,
        nodes: &[FormulaNode],
        position: &mut usize,
        binders: usize,
    ) -> Result<(), ParaphraseError> {
        self.formula(nodes, position, binders)?;
        self.push(GROUP_CLOSE)
    }

    fn formula(
        &mut self,
        nodes: &[FormulaNode],
        position: &mut usize,
        binders: usize,
    ) -> Result<(), ParaphraseError> {
        let node = nodes
            .get(*position)
            .ok_or(ParaphraseError::RoundTripMismatch)?;
        *position += 1;
        match node {
            FormulaNode::True => self.push(TRUE),
            FormulaNode::False => self.push(FALSE),
            FormulaNode::Eq(left, right) => {
                self.push(EQ_OPEN)?;
                self.slot(left, binders)?;
                self.push(EQ_MIDDLE)?;
                self.slot(right, binders)
            }
            FormulaNode::Pred { symbol, args } => {
                let template = self
                    .paraphraser
                    .by_name
                    .get(symbol.name.as_str())
                    .ok_or(ParaphraseError::RoundTripMismatch)?;
                let segments = template.segments.clone();
                if segments.len() != args.len() + 1 {
                    return Err(ParaphraseError::RoundTripMismatch);
                }
                self.push(OPEN)?;
                for (index, segment) in segments.iter().enumerate() {
                    self.push(segment)?;
                    if let Some(arg) = args.get(index) {
                        self.slot(arg, binders)?;
                    }
                }
                self.push(CLOSE)
            }
            FormulaNode::Not => {
                self.push(NOT_OPEN)?;
                self.group(nodes, position, binders)
            }
            FormulaNode::And(count) => self.list(AND_OPEN, *count, nodes, position, binders),
            FormulaNode::Or(count) => self.list(OR_OPEN, *count, nodes, position, binders),
            FormulaNode::Implies => {
                self.push(IMPLIES_OPEN)?;
                self.formula(nodes, position, binders)?;
                self.push(IMPLIES_MIDDLE)?;
                self.group(nodes, position, binders)
            }
            FormulaNode::Iff => {
                self.push(IFF_OPEN)?;
                self.formula(nodes, position, binders)?;
                self.push(IFF_MIDDLE)?;
                self.group(nodes, position, binders)
            }
            FormulaNode::Forall(sort) | FormulaNode::Exists(sort) => {
                let (open, body) = if matches!(node, FormulaNode::Forall(_)) {
                    (FORALL_OPEN, FORALL_BODY)
                } else {
                    (EXISTS_OPEN, EXISTS_BODY)
                };
                self.push(open)?;
                self.variable(binders + 1)?;
                self.push(SORT_OPEN)?;
                self.push(sort.name.as_str())?;
                self.push(body)?;
                self.group(nodes, position, binders + 1)
            }
        }
    }

    fn list(
        &mut self,
        open: &str,
        count: usize,
        nodes: &[FormulaNode],
        position: &mut usize,
        binders: usize,
    ) -> Result<(), ParaphraseError> {
        self.push(open)?;
        self.push(&count.to_string())?;
        self.push(if count == 1 {
            ONE_CONDITION
        } else {
            MANY_CONDITIONS
        })?;
        for index in 0..count {
            self.push(if index == 0 { LIST_FIRST } else { LIST_NEXT })?;
            self.group(nodes, position, binders)?;
        }
        Ok(())
    }

    fn variable(&mut self, ordinal: usize) -> Result<(), ParaphraseError> {
        let mut name = String::new();
        let _ = write!(name, "{VARIABLE_PREFIX}{ordinal}");
        self.push(&name)
    }

    fn slot(&mut self, term: &Term, binders: usize) -> Result<(), ParaphraseError> {
        let mut position = 0;
        self.push(OPEN)?;
        self.term(term.nodes(), &mut position, binders)?;
        if position != term.nodes().len() {
            return Err(ParaphraseError::RoundTripMismatch);
        }
        self.push(CLOSE)
    }

    fn term(
        &mut self,
        nodes: &[TermNode],
        position: &mut usize,
        binders: usize,
    ) -> Result<(), ParaphraseError> {
        let node = nodes
            .get(*position)
            .ok_or(ParaphraseError::RoundTripMismatch)?;
        *position += 1;
        match node {
            TermNode::Var { index, .. } => {
                let index =
                    usize::try_from(*index).map_err(|_| ParaphraseError::RoundTripMismatch)?;
                let ordinal = binders
                    .checked_sub(index)
                    .filter(|&ordinal| ordinal >= 1)
                    .ok_or(ParaphraseError::RoundTripMismatch)?;
                self.variable(ordinal)
            }
            TermNode::App { symbol, args } => {
                let template = self
                    .paraphraser
                    .by_name
                    .get(symbol.name.as_str())
                    .ok_or(ParaphraseError::RoundTripMismatch)?;
                let segments = template.segments.clone();
                if segments.len() != args + 1 {
                    return Err(ParaphraseError::RoundTripMismatch);
                }
                for (index, segment) in segments.iter().enumerate() {
                    self.push(segment)?;
                    if index < *args {
                        self.push(OPEN)?;
                        self.term(nodes, position, binders)?;
                        self.push(CLOSE)?;
                    }
                }
                Ok(())
            }
        }
    }
}
