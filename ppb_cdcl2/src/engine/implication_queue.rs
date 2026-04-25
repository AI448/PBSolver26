use std::collections::VecDeque;

use index_collections::{Comparator, HeapedMap, ValueComparator};
use ordered_float::NotNan;

use crate::{ImplicationReceiver, Literal, ParameterLowerBound, Predicate};

#[derive(Clone, Copy)]
pub enum ConflictImplication<ExplainKeyT>
where
    ExplainKeyT: Copy,
{
    Constraint {
        explain_key: ExplainKeyT,
    },
    Literals {
        variable: usize,
        explain_keys: [ExplainKeyT; 2],
    },
}

#[derive(Clone)]
pub struct ImplicationQueue<ExplainKeyT>
where
    ExplainKeyT: Copy,
{
    conflict_constraints: VecDeque<ExplainKeyT>,
    conflict_literal_assertions: HeapedMap<
        ConflictLiteralAssertion<ExplainKeyT>,
        ValueComparator<VariableConflictComparator>,
    >,
    noconflict_literal_assertions: HeapedMap<
        NoconflictLiteralAssertion<ExplainKeyT>,
        ValueComparator<VariablePropagationComparator>,
    >,
    parameter_lower_bound_assertion: Option<(ParameterLowerBound, ExplainKeyT)>,
}

impl<ExplainKeyT> Default for ImplicationQueue<ExplainKeyT>
where
    ExplainKeyT: Copy,
{
    fn default() -> Self {
        Self {
            conflict_constraints: VecDeque::default(),
            conflict_literal_assertions: HeapedMap::default(),
            noconflict_literal_assertions: HeapedMap::default(),
            parameter_lower_bound_assertion: None,
        }
    }
}

#[derive(Clone)]
struct ConflictLiteralAssertion<ExplainKeyT>
where
    ExplainKeyT: Copy,
{
    normalized_slacks: [NotNan<f64>; 2],
    explain_keys: [ExplainKeyT; 2],
}

#[derive(Default, Clone)]
struct VariableConflictComparator {}

impl<ExplainKeyT> Comparator<ConflictLiteralAssertion<ExplainKeyT>> for VariableConflictComparator
where
    ExplainKeyT: Copy,
{
    #[inline(always)]
    fn cmp(
        &self,
        lhs: &ConflictLiteralAssertion<ExplainKeyT>,
        rhs: &ConflictLiteralAssertion<ExplainKeyT>,
    ) -> std::cmp::Ordering {
        // スラックの合計が小さい方を優先
        (lhs.normalized_slacks[0] + lhs.normalized_slacks[1])
            .cmp(&(rhs.normalized_slacks[0] + rhs.normalized_slacks[1]))
    }
}

#[derive(Clone)]
struct NoconflictLiteralAssertion<ExplainKeyT>
where
    ExplainKeyT: Copy,
{
    normalized_slack: NotNan<f64>,
    explain_key: ExplainKeyT,
    value: bool,
}

#[derive(Default, Clone)]
struct VariablePropagationComparator {}

impl<ExplainKeyT> Comparator<NoconflictLiteralAssertion<ExplainKeyT>>
    for VariablePropagationComparator
where
    ExplainKeyT: Copy,
{
    #[inline(always)]
    fn cmp(
        &self,
        lhs: &NoconflictLiteralAssertion<ExplainKeyT>,
        rhs: &NoconflictLiteralAssertion<ExplainKeyT>,
    ) -> std::cmp::Ordering {
        // スラックが小さい方を優先
        lhs.normalized_slack.cmp(&rhs.normalized_slack)
    }
}

impl<ExplainKeyT> ImplicationQueue<ExplainKeyT>
where
    ExplainKeyT: Copy,
{
    pub fn is_empty(&self) -> bool {
        self.conflict_constraints.is_empty()
            && self.conflict_literal_assertions.is_empty()
            && self.noconflict_literal_assertions.is_empty()
            && self.parameter_lower_bound_assertion.is_none()
    }

    pub fn get_conflict(&self) -> Option<ConflictImplication<ExplainKeyT>> {
        if let Some(&explain_key) = self.conflict_constraints.front() {
            Some(ConflictImplication::Constraint { explain_key })
        } else if let Some((&variable, conflict_literals)) =
            self.conflict_literal_assertions.first()
        {
            Some(ConflictImplication::Literals {
                variable,
                explain_keys: conflict_literals.explain_keys,
            })
        } else {
            None
        }
    }

    pub fn pop_propagation(&mut self) -> Option<(Predicate, ExplainKeyT)> {
        assert!(self.conflict_constraints.is_empty());
        assert!(self.conflict_literal_assertions.is_empty());
        if let Some((variable, noconflict_literal)) = self.noconflict_literal_assertions.pop_first()
        {
            Some((
                Predicate::Literal(Literal::new(variable, noconflict_literal.value)),
                noconflict_literal.explain_key,
            ))
        } else if let Some((parameter_lower_bound, explain_key)) =
            self.parameter_lower_bound_assertion.take()
        {
            Some((
                Predicate::ParameterLowerBound(parameter_lower_bound),
                explain_key,
            ))
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.conflict_constraints.clear();
        self.conflict_literal_assertions.clear();
        self.noconflict_literal_assertions.clear();
        self.parameter_lower_bound_assertion = None;
    }
}

impl<ExplainKeyT> ImplicationReceiver<ExplainKeyT> for ImplicationQueue<ExplainKeyT>
where
    ExplainKeyT: Copy + Eq,
{
    fn receive_conflict(&mut self, explain_key: ExplainKeyT) {
        self.conflict_constraints.push_back(explain_key);
    }

    fn receive_parameter_lower_bound_assertion(
        &mut self,
        parameter_lower_bound: ParameterLowerBound,
        explain_key: ExplainKeyT,
    ) {
        if self
            .parameter_lower_bound_assertion
            .is_none_or(|(lb, _)| parameter_lower_bound.value() > lb.value())
        {
            self.parameter_lower_bound_assertion
                .replace((parameter_lower_bound, explain_key));
        }
    }

    fn receive_literal_assertion(
        &mut self,
        literal: Literal,
        explain_key: ExplainKeyT,
        normalized_slack: f64,
    ) {
        let normalized_slack = NotNan::new(normalized_slack).unwrap();
        if let Some(conflict) = self.conflict_literal_assertions.get(literal.index()) {
            if normalized_slack < conflict.normalized_slacks[literal.value() as usize] {
                let (normalized_slacks, explain_keys) = if literal.value() as usize == 0 {
                    (
                        [normalized_slack, conflict.normalized_slacks[1]],
                        [explain_key, conflict.explain_keys[1]],
                    )
                } else {
                    (
                        [conflict.normalized_slacks[0], normalized_slack],
                        [conflict.explain_keys[0], explain_key],
                    )
                };
                self.conflict_literal_assertions.insert(
                    literal.index(),
                    ConflictLiteralAssertion {
                        normalized_slacks,
                        explain_keys,
                    },
                );
            }
        } else if let Some(boolean_assertion) =
            self.noconflict_literal_assertions.get(literal.index())
        {
            if boolean_assertion.value != literal.value() {
                let (normalized_slacks, explain_keys) = if literal.value() as usize == 0 {
                    (
                        [normalized_slack, boolean_assertion.normalized_slack],
                        [explain_key, boolean_assertion.explain_key],
                    )
                } else {
                    (
                        [boolean_assertion.normalized_slack, normalized_slack],
                        [boolean_assertion.explain_key, explain_key],
                    )
                };
                self.conflict_literal_assertions.insert(
                    literal.index(),
                    ConflictLiteralAssertion {
                        normalized_slacks,
                        explain_keys,
                    },
                );
                self.noconflict_literal_assertions.remove(literal.index());
            } else if normalized_slack < boolean_assertion.normalized_slack {
                self.noconflict_literal_assertions.insert(
                    literal.index(),
                    NoconflictLiteralAssertion {
                        normalized_slack,
                        explain_key,
                        value: literal.value(),
                    },
                );
            }
        } else {
            self.noconflict_literal_assertions.insert(
                literal.index(),
                NoconflictLiteralAssertion {
                    normalized_slack,
                    explain_key,
                    value: literal.value(),
                },
            );
        }
    }
}
