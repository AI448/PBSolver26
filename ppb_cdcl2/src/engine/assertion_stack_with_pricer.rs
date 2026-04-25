use std::hash::Hash;

use index_collections::Map;

use crate::{
    AssertionState, Literal, LiteralState, Predicate, Pricer, Reason,
    engine::assertion_stack::AssertionStack,
};

#[derive(Clone)]
pub struct AssertionStackWithPricer<ExplainKeyT, PricerT>
where
    ExplainKeyT: Copy + Eq,
    PricerT: Pricer,
{
    assertion_stack: AssertionStack<ExplainKeyT>,
    pricer: PricerT,
}

impl<ExplainKeyT, PricerT> AssertionState for AssertionStackWithPricer<ExplainKeyT, PricerT>
where
    ExplainKeyT: Copy + Eq + Hash,
    PricerT: Pricer,
{
    type ExplainKey = <AssertionStack<ExplainKeyT> as AssertionState>::ExplainKey;

    #[inline(always)]
    fn number_of_variables(&self) -> usize {
        self.assertion_stack.number_of_variables()
    }

    #[inline(always)]
    fn number_of_assertions(&self) -> usize {
        self.assertion_stack.number_of_assertions()
    }

    #[inline(always)]
    fn number_of_assigned_variables(&self) -> usize {
        self.assertion_stack.number_of_assigned_variables()
    }

    #[inline(always)]
    fn decision_level(&self) -> usize {
        self.assertion_stack.decision_level()
    }

    #[inline(always)]
    fn order_range(&self, decision_level: usize) -> std::ops::Range<usize> {
        self.assertion_stack.order_range(decision_level)
    }

    #[inline(always)]
    fn assertion(&self, order: usize) -> Predicate {
        self.assertion_stack.assertion(order)
    }

    #[inline(always)]
    fn literal_state(&self, literal: Literal) -> impl LiteralState<ExplainKey = Self::ExplainKey> {
        self.assertion_stack.literal_state(literal)
    }

    #[inline(always)]
    fn parameter_lower_bound_before(&self, order: usize) -> f64 {
        self.assertion_stack.parameter_lower_bound_before(order)
    }

    #[inline(always)]
    fn parameter_upper_bound_before(&self, order: usize) -> f64 {
        self.assertion_stack.parameter_upper_bound_before(order)
    }

    #[inline(always)]
    fn parameter_lower_bound(&self) -> f64 {
        self.assertion_stack.parameter_lower_bound()
    }

    #[inline(always)]
    fn parameter_upper_bound(&self) -> f64 {
        self.assertion_stack.parameter_upper_bound()
    }
}

impl<ExplainKeyT, PricerT> AssertionStackWithPricer<ExplainKeyT, PricerT>
where
    ExplainKeyT: Copy + Eq + Hash,
    PricerT: Pricer,
{
    #[inline(always)]
    pub fn new(pricer: PricerT) -> Self {
        Self {
            assertion_stack: AssertionStack::default(),
            pricer,
        }
    }

    #[inline(always)]
    pub fn add_variable(&mut self, initial_value: bool, initial_activity: f64) {
        self.assertion_stack.add_variable(initial_value);
        self.pricer.add_variable(0, initial_activity);
    }

    #[inline(never)]
    pub fn update_activity(&mut self, weight: &Map<f64>) {
        self.pricer.update_activity(weight);
    }

    #[inline(always)]
    pub fn top_priority_unassigned_literal(&self) -> Option<Literal> {
        if let Some((variable, (_priority, _activity))) = self.pricer.peek() {
            Some(Literal::new(
                variable,
                self.assertion_stack.cached_phase(variable),
            ))
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn assert(&mut self, assertion: Predicate, reason: Reason<ExplainKeyT>) {
        self.assertion_stack.assert(assertion, reason);
        if let Predicate::Literal(literal) = assertion {
            self.pricer.set_to_assigned(literal.index());
        }
    }

    #[inline(never)]
    pub fn backjump(&mut self, backjump_level: usize) {
        for order in (self.assertion_stack.order_range(backjump_level).end
            ..self.assertion_stack.number_of_assertions())
            .rev()
        {
            if let Predicate::Literal(literal) = self.assertion_stack.assertion(order) {
                self.pricer.set_to_unassigned(literal.index());
            }
        }
        self.assertion_stack.backjump(backjump_level);
    }
}
