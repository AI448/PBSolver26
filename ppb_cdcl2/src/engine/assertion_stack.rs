use std::hash::Hash;

use crate::{
    AssertionState, Literal, LiteralState, ParameterLowerBound, ParameterUpperBound, Predicate,
    Reason,
};

const NULL_ORDER: usize = usize::MAX;

#[derive(Clone)]
pub struct AssertionStack<ExplainKeyT>
where
    ExplainKeyT: Copy + Eq,
{
    variable_infos: Vec<VariableInfo>,
    parameter_lower_bound_infos: Vec<ParameterLowerBoundInfo>,
    parameter_upper_bound_infos: Vec<ParameterUpperBoundInfo>,
    assertion_infos: Vec<AssertionInfo<ExplainKeyT>>,
    decision_infos: Vec<DecisionInfo>,
    number_of_assigned_variables: usize,
}

// TODO: 検討 8byte にするか
#[derive(Clone)]
struct VariableInfo {
    value: bool,
    assertion_order: usize,
}

#[derive(Clone)]
struct ParameterLowerBoundInfo {
    parameter_lower_bound: ParameterLowerBound,
    assertion_order: usize,
}

#[derive(Clone)]
struct ParameterUpperBoundInfo {
    parameter_upper_bound: ParameterUpperBound,
    assertion_order: usize,
}

#[derive(Clone)]
struct AssertionInfo<ExplainKeyT>
where
    ExplainKeyT: Copy + Eq,
{
    predicate: Predicate,
    reason: Reason<ExplainKeyT>,
    decision_level: usize,
}

#[derive(Clone)]
struct DecisionInfo {
    assertion_order: usize,
}

impl<ExplainKeyT> Default for AssertionStack<ExplainKeyT>
where
    ExplainKeyT: Copy + Eq,
{
    fn default() -> Self {
        Self {
            variable_infos: Vec::default(),
            parameter_lower_bound_infos: Vec::default(),
            parameter_upper_bound_infos: Vec::default(),
            assertion_infos: Vec::default(),
            decision_infos: Vec::default(),
            number_of_assigned_variables: 0,
        }
    }
}

impl<ExplainKeyT> AssertionState for AssertionStack<ExplainKeyT>
where
    ExplainKeyT: Copy + Eq + Hash,
{
    type ExplainKey = ExplainKeyT;

    // ブール変数の数
    #[inline(always)]
    fn number_of_variables(&self) -> usize {
        self.variable_infos.len()
    }

    /// 表明された述語の数
    #[inline(always)]
    fn number_of_assertions(&self) -> usize {
        self.assertion_infos.len()
    }

    /// 値が割り当てられている変数の数
    #[inline(always)]
    fn number_of_assigned_variables(&self) -> usize {
        self.number_of_assigned_variables
    }

    /// 現在の決定レベル
    #[inline(always)]
    fn decision_level(&self) -> usize {
        self.decision_infos.len()
    }

    /// decision_level に対応する order の範囲
    #[inline(always)]
    fn order_range(&self, decision_level: usize) -> std::ops::Range<usize> {
        let start = if decision_level == 0 {
            0
        } else {
            self.decision_infos[decision_level - 1].assertion_order
        };
        let end = if decision_level < self.decision_infos.len() {
            self.decision_infos[decision_level].assertion_order
        } else {
            self.assertion_infos.len()
        };
        std::ops::Range { start, end }
    }

    #[inline(always)]
    fn assertion(&self, order: usize) -> Predicate {
        self.assertion_infos[order].predicate
    }

    #[inline(always)]
    fn literal_state(
        &self,
        literal: crate::Literal,
    ) -> impl LiteralState<ExplainKey = Self::ExplainKey> {
        AssertionStackLiteralState {
            decision_stack: self,
            literal,
        }
    }

    #[inline(always)]
    fn parameter_lower_bound_before(&self, order: usize) -> f64 {
        let k = match self
            .parameter_lower_bound_infos
            .binary_search_by_key(&order, |info| info.assertion_order)
            .into()
        {
            Ok(k) => k,
            Err(k) => k,
        };
        if k == 0 {
            f64::NEG_INFINITY
        } else {
            debug_assert!(self.parameter_lower_bound_infos[k - 1].assertion_order < order);
            self.parameter_lower_bound_infos[k - 1]
                .parameter_lower_bound
                .value()
        }
    }

    #[inline(always)]
    fn parameter_upper_bound_before(&self, order: usize) -> f64 {
        let k = match self
            .parameter_upper_bound_infos
            .binary_search_by_key(&order, |info| info.assertion_order)
            .into()
        {
            Ok(k) => k,
            Err(k) => k,
        };
        if k == 0 {
            f64::INFINITY
        } else {
            debug_assert!(self.parameter_upper_bound_infos[k - 1].assertion_order < order);
            self.parameter_upper_bound_infos[k - 1]
                .parameter_upper_bound
                .value()
        }
    }

    #[inline(always)]
    fn parameter_lower_bound(&self) -> f64 {
        self.parameter_lower_bound_before(usize::MAX)
    }

    #[inline(always)]
    fn parameter_upper_bound(&self) -> f64 {
        self.parameter_upper_bound_before(usize::MAX)
    }
}

impl<ExplainKeyT> AssertionStack<ExplainKeyT>
where
    ExplainKeyT: Copy + Eq,
{
    #[inline(always)]
    pub fn add_variable(&mut self, initial_value: bool) {
        self.variable_infos.push(VariableInfo {
            value: initial_value,
            assertion_order: NULL_ORDER,
        });
    }

    #[inline(always)]
    pub fn cached_phase(&self, variable: usize) -> bool {
        self.variable_infos[variable].value
    }

    #[inline(always)]
    pub fn assert(&mut self, assertion: Predicate, reason: Reason<ExplainKeyT>) {
        match assertion {
            Predicate::Literal(literal) => {
                assert!(self.variable_infos[literal.index()].assertion_order == NULL_ORDER);
                self.variable_infos[literal.index()] = VariableInfo {
                    value: literal.value(),
                    assertion_order: self.assertion_infos.len(),
                };
                self.number_of_assigned_variables += 1;
            }
            Predicate::ParameterLowerBound(parameter_lower_bound) => {
                assert!(
                    parameter_lower_bound.value()
                        > self
                            .parameter_lower_bound_infos
                            .last()
                            .map(|info| info.parameter_lower_bound.value())
                            .unwrap_or(f64::NEG_INFINITY)
                );
                assert!(
                    parameter_lower_bound.value()
                        <= self
                            .parameter_upper_bound_infos
                            .last()
                            .map(|info| info.parameter_upper_bound.value())
                            .unwrap_or(f64::INFINITY)
                );
                self.parameter_lower_bound_infos
                    .push(ParameterLowerBoundInfo {
                        parameter_lower_bound,
                        assertion_order: self.assertion_infos.len(),
                    });
            }
            Predicate::ParameterUpperBound(parameter_upper_bound) => {
                assert!(
                    parameter_upper_bound.value()
                        < self
                            .parameter_upper_bound_infos
                            .last()
                            .map(|info| info.parameter_upper_bound.value())
                            .unwrap_or(f64::INFINITY)
                );
                assert!(
                    parameter_upper_bound.value()
                        >= self
                            .parameter_lower_bound_infos
                            .last()
                            .map(|info| info.parameter_lower_bound.value())
                            .unwrap_or(f64::NEG_INFINITY)
                );
                self.parameter_upper_bound_infos
                    .push(ParameterUpperBoundInfo {
                        parameter_upper_bound,
                        assertion_order: self.assertion_infos.len(),
                    });
            }
        }
        if reason.is_decision() {
            self.decision_infos.push(DecisionInfo {
                assertion_order: self.assertion_infos.len(),
            });
        }
        self.assertion_infos.push(AssertionInfo {
            predicate: assertion,
            reason,
            decision_level: self.decision_infos.len(),
        });
    }

    pub fn backjump(&mut self, backjump_level: usize) {
        while self.decision_infos.len() > backjump_level {
            let assertion_info = self.assertion_infos.pop().unwrap();
            debug_assert!(assertion_info.decision_level == self.decision_infos.len());
            if assertion_info.reason.is_decision() {
                let decision = self.decision_infos.pop().unwrap();
                debug_assert!(decision.assertion_order == self.assertion_infos.len());
            }
            match assertion_info.predicate {
                Predicate::Literal(literal) => {
                    debug_assert!(
                        self.variable_infos[literal.index()].assertion_order
                            == self.assertion_infos.len()
                    );
                    self.variable_infos[literal.index()].assertion_order = NULL_ORDER;
                    self.number_of_assigned_variables -= 1;
                }
                Predicate::ParameterLowerBound(parameter_lower_bound) => {
                    debug_assert!(
                        self.parameter_lower_bound_infos
                            .last()
                            .unwrap()
                            .parameter_lower_bound
                            == parameter_lower_bound
                    );
                    debug_assert!(
                        self.parameter_lower_bound_infos
                            .last()
                            .unwrap()
                            .assertion_order
                            == self.assertion_infos.len()
                    );
                    self.parameter_lower_bound_infos.pop();
                }
                Predicate::ParameterUpperBound(parameter_upper_bound) => {
                    debug_assert!(
                        self.parameter_upper_bound_infos
                            .last()
                            .unwrap()
                            .parameter_upper_bound
                            == parameter_upper_bound
                    );
                    debug_assert!(
                        self.parameter_upper_bound_infos
                            .last()
                            .unwrap()
                            .assertion_order
                            == self.assertion_infos.len()
                    );
                    self.parameter_upper_bound_infos.pop();
                }
            }
        }
    }
}

struct AssertionStackLiteralState<'a, ExplainKeyT>
where
    ExplainKeyT: Copy + Eq,
{
    decision_stack: &'a AssertionStack<ExplainKeyT>,
    literal: Literal,
}

impl<'a, ExplainKeyT> AssertionStackLiteralState<'a, ExplainKeyT> where ExplainKeyT: Copy + Eq {}

impl<'a, ExplainKeyT> LiteralState for AssertionStackLiteralState<'a, ExplainKeyT>
where
    ExplainKeyT: Copy + Eq,
{
    type ExplainKey = ExplainKeyT;

    #[inline(always)]
    fn order(&self) -> Option<usize> {
        let order = self.decision_stack.variable_infos[self.literal.index()].assertion_order;
        if order != NULL_ORDER {
            Some(order)
        } else {
            None
        }
    }

    #[inline(always)]
    fn decision_level(&self) -> Option<usize> {
        if let Some(order) = self.order() {
            Some(self.decision_stack.assertion_infos[order].decision_level)
        } else {
            None
        }
    }

    #[inline(always)]
    fn is_assigned_before(&self, assertion_order: usize) -> bool {
        self.order().is_some_and(|o| o < assertion_order)
    }

    #[inline(always)]
    fn is_assigned(&self) -> bool {
        self.order().is_some()
    }

    #[inline(always)]
    fn is_true_before(&self, order: usize) -> bool {
        let info = &self.decision_stack.variable_infos[self.literal.index()];
        info.assertion_order < order && info.value == self.literal.value()
    }

    #[inline(always)]
    fn is_true(&self) -> bool {
        self.is_true_before(NULL_ORDER)
    }

    #[inline(always)]
    fn is_false_before(&self, order: usize) -> bool {
        let info = &self.decision_stack.variable_infos[self.literal.index()];
        info.assertion_order < order && info.value != self.literal.value()
    }

    #[inline(always)]
    fn is_false(&self) -> bool {
        self.is_false_before(NULL_ORDER)
    }

    #[inline(always)]
    fn reason(&self) -> Option<Reason<Self::ExplainKey>> {
        if let Some(order) = self.order() {
            Some(self.decision_stack.assertion_infos[order].reason)
        } else {
            None
        }
    }
}
