//! `parse_pt`: texto paraphrase-pt/v1 → AIR, por descida recursiva estrita.
//!
//! Aceita exatamente os textos que o renderizador produz: frases fixas
//! byte a byte, decimais canônicos, `⟦ ⟧` balanceados e templates exatos.
//! Profundidade limitada antes de cada descida; nenhum valor por padrão.

use logos_air::{Formula, FormulaNode, Sort, Term, TermNode, check};

use crate::{
    ParaphraseError,
    grammar::{
        AND_OPEN, CLOSE, CLOSE_CHAR, EQ_MIDDLE, EQ_OPEN, EXISTS_BODY, EXISTS_OPEN, FALSE,
        FORALL_BODY, FORALL_OPEN, GROUP_CLOSE, IFF_MIDDLE, IFF_OPEN, IMPLIES_MIDDLE, IMPLIES_OPEN,
        LIST_FIRST, LIST_NEXT, MANY_CONDITIONS, NOT_OPEN, ONE_CONDITION, OPEN, OPEN_CHAR, OR_OPEN,
        SORT_OPEN, TRUE, parse_decimal, variable_ordinal,
    },
    paraphrase::Paraphraser,
};

struct Parser<'p, 'v, 't> {
    paraphraser: &'p Paraphraser<'v>,
    text: &'t str,
    position: usize,
    binders: Vec<Sort>,
}

/// Conteúdo de um par `⟦ ⟧`: segmentos literais e termos dos slots internos.
struct Bracketed<'t> {
    start: usize,
    segments: Vec<&'t str>,
    slots: Vec<Vec<TermNode>>,
}

pub(crate) fn parse(paraphraser: &Paraphraser<'_>, text: &str) -> Result<Formula, ParaphraseError> {
    if text.len() > paraphraser.limits.max_bytes {
        return Err(ParaphraseError::TooLarge);
    }
    let mut parser = Parser {
        paraphraser,
        text,
        position: 0,
        binders: Vec::new(),
    };
    let mut nodes = Vec::new();
    parser.formula(&mut nodes, 1)?;
    if parser.position != text.len() {
        return Err(parser.syntax());
    }
    let formula = Formula::new(nodes, paraphraser.limits.air)?;
    check(paraphraser.vocabulary, &formula)?;
    Ok(formula)
}

impl<'t> Parser<'_, '_, 't> {
    fn syntax(&self) -> ParaphraseError {
        ParaphraseError::Syntax {
            offset: self.position,
        }
    }

    fn rest(&self) -> &'t str {
        self.text.get(self.position..).unwrap_or("")
    }

    fn accept(&mut self, literal: &str) -> bool {
        if self.rest().starts_with(literal) {
            self.position += literal.len();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, literal: &str) -> Result<(), ParaphraseError> {
        if self.accept(literal) {
            Ok(())
        } else {
            Err(self.syntax())
        }
    }

    fn guard(&self, depth: usize) -> Result<(), ParaphraseError> {
        if depth > self.paraphraser.limits.air.max_depth {
            Err(ParaphraseError::DepthExceeded)
        } else {
            Ok(())
        }
    }

    /// Lê até o próximo caractere que não pertence ao conjunto aceito.
    fn take_while(&mut self, accept: impl Fn(char) -> bool) -> &'t str {
        let rest = self.rest();
        let end = rest.find(|c: char| !accept(c)).unwrap_or(rest.len());
        self.position += end;
        &rest[..end]
    }

    fn group(&mut self, nodes: &mut Vec<FormulaNode>, depth: usize) -> Result<(), ParaphraseError> {
        self.formula(nodes, depth)?;
        self.expect(GROUP_CLOSE)
    }

    fn formula(
        &mut self,
        nodes: &mut Vec<FormulaNode>,
        depth: usize,
    ) -> Result<(), ParaphraseError> {
        self.guard(depth)?;
        let next = depth + 1;
        if self.accept(TRUE) {
            nodes.push(FormulaNode::True);
        } else if self.accept(FALSE) {
            nodes.push(FormulaNode::False);
        } else if self.accept(EQ_OPEN) {
            let left = self.slot(next)?;
            self.expect(EQ_MIDDLE)?;
            let right = self.slot(next)?;
            nodes.push(FormulaNode::Eq(left, right));
        } else if self.rest().starts_with(OPEN) {
            self.atom(nodes, next)?;
        } else if self.accept(NOT_OPEN) {
            nodes.push(FormulaNode::Not);
            self.group(nodes, next)?;
        } else if self.accept(AND_OPEN) {
            let index = nodes.len();
            nodes.push(FormulaNode::And(0));
            let count = self.list(nodes, next)?;
            if let Some(node) = nodes.get_mut(index) {
                *node = FormulaNode::And(count);
            }
        } else if self.accept(OR_OPEN) {
            let index = nodes.len();
            nodes.push(FormulaNode::Or(0));
            let count = self.list(nodes, next)?;
            if let Some(node) = nodes.get_mut(index) {
                *node = FormulaNode::Or(count);
            }
        } else if self.accept(IMPLIES_OPEN) {
            nodes.push(FormulaNode::Implies);
            self.formula(nodes, next)?;
            self.expect(IMPLIES_MIDDLE)?;
            self.group(nodes, next)?;
        } else if self.accept(IFF_OPEN) {
            nodes.push(FormulaNode::Iff);
            self.formula(nodes, next)?;
            self.expect(IFF_MIDDLE)?;
            self.group(nodes, next)?;
        } else if self.accept(FORALL_OPEN) {
            let sort = self.binder(FORALL_BODY)?;
            nodes.push(FormulaNode::Forall(sort.clone()));
            self.quantified(sort, nodes, next)?;
        } else if self.accept(EXISTS_OPEN) {
            let sort = self.binder(EXISTS_BODY)?;
            nodes.push(FormulaNode::Exists(sort.clone()));
            self.quantified(sort, nodes, next)?;
        } else {
            return Err(self.syntax());
        }
        Ok(())
    }

    fn quantified(
        &mut self,
        sort: Sort,
        nodes: &mut Vec<FormulaNode>,
        depth: usize,
    ) -> Result<(), ParaphraseError> {
        self.binders.push(sort);
        let body = self.group(nodes, depth);
        self.binders.pop();
        body
    }

    /// `xK do tipo ⟦S⟧` + corpo; `K` precisa ser o próximo binder.
    fn binder(&mut self, body: &str) -> Result<Sort, ParaphraseError> {
        let start = self.position;
        let name = self.take_while(|c| c.is_ascii_alphanumeric());
        if variable_ordinal(name) != Some(self.binders.len() + 1) {
            self.position = start;
            return Err(self.syntax());
        }
        self.expect(SORT_OPEN)?;
        let sort_start = self.position;
        let name = self.take_while(|c| c != OPEN_CHAR && c != CLOSE_CHAR);
        let sort = self
            .paraphraser
            .sorts
            .get(name)
            .cloned()
            .ok_or(ParaphraseError::UnknownSort { offset: sort_start })?;
        self.expect(body)?;
        Ok(sort)
    }

    fn list(
        &mut self,
        nodes: &mut Vec<FormulaNode>,
        depth: usize,
    ) -> Result<usize, ParaphraseError> {
        let start = self.position;
        let digits = self.take_while(|c| c.is_ascii_digit());
        let count = parse_decimal(digits)
            .filter(|&count| count <= self.paraphraser.limits.air.max_nodes)
            .ok_or(ParaphraseError::Syntax { offset: start })?;
        self.expect(if count == 1 {
            ONE_CONDITION
        } else {
            MANY_CONDITIONS
        })?;
        for index in 0..count {
            self.expect(if index == 0 { LIST_FIRST } else { LIST_NEXT })?;
            self.group(nodes, depth)?;
        }
        Ok(count)
    }

    fn atom(&mut self, nodes: &mut Vec<FormulaNode>, depth: usize) -> Result<(), ParaphraseError> {
        let bracketed = self.bracketed(depth)?;
        let index = *self.paraphraser.predicates.get(&bracketed.segments).ok_or(
            ParaphraseError::UnknownTemplate {
                offset: bracketed.start,
            },
        )?;
        let symbol = self.symbol(index)?;
        let limits = self.paraphraser.limits.air;
        let args = bracketed
            .slots
            .into_iter()
            .map(|slot| Term::new(slot, limits))
            .collect::<Result<Vec<_>, _>>()?;
        nodes.push(FormulaNode::Pred { symbol, args });
        Ok(())
    }

    fn slot(&mut self, depth: usize) -> Result<Term, ParaphraseError> {
        let mut out = Vec::new();
        self.slot_into(&mut out, depth)?;
        Ok(Term::new(out, self.paraphraser.limits.air)?)
    }

    /// `⟦` termo `⟧`: variável `xK` ou template de termo preenchido.
    fn slot_into(&mut self, out: &mut Vec<TermNode>, depth: usize) -> Result<(), ParaphraseError> {
        let bracketed = self.bracketed(depth)?;
        if bracketed.slots.is_empty()
            && let Some(ordinal) = bracketed.segments.first().and_then(|s| variable_ordinal(s))
        {
            let binders = self.binders.len();
            let sort = ordinal
                .checked_sub(1)
                .and_then(|level| self.binders.get(level))
                .cloned()
                .ok_or(ParaphraseError::UnboundVariable {
                    offset: bracketed.start,
                })?;
            let index = binders
                .checked_sub(ordinal)
                .and_then(|index| u64::try_from(index).ok())
                .ok_or_else(|| self.syntax())?;
            out.push(TermNode::Var { index, sort });
            return Ok(());
        }
        let index = *self.paraphraser.terms.get(&bracketed.segments).ok_or(
            ParaphraseError::UnknownTemplate {
                offset: bracketed.start,
            },
        )?;
        let symbol = self.symbol(index)?;
        out.push(TermNode::App {
            symbol,
            args: bracketed.slots.len(),
        });
        for slot in bracketed.slots {
            out.extend(slot);
        }
        Ok(())
    }

    /// Lê `⟦ … ⟧`; cada `⟦` interno abre um slot de termo.
    fn bracketed(&mut self, depth: usize) -> Result<Bracketed<'t>, ParaphraseError> {
        self.guard(depth)?;
        self.expect(OPEN)?;
        let start = self.position;
        let mut segments = Vec::new();
        let mut slots = Vec::new();
        loop {
            segments.push(self.take_while(|c| c != OPEN_CHAR && c != CLOSE_CHAR));
            if self.accept(CLOSE) {
                break;
            }
            if !self.rest().starts_with(OPEN) {
                return Err(self.syntax());
            }
            let mut slot = Vec::new();
            self.slot_into(&mut slot, depth + 1)?;
            slots.push(slot);
        }
        Ok(Bracketed {
            start,
            segments,
            slots,
        })
    }

    fn symbol(&self, index: usize) -> Result<logos_air::Symbol, ParaphraseError> {
        self.paraphraser
            .vocabulary
            .symbols()
            .get(index)
            .map(|decl| decl.signature.clone())
            .ok_or_else(|| self.syntax())
    }
}
