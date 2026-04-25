mod assertion_stack;
mod assertion_stack_with_pricer;
mod implication_queue;

use std::hash::Hash;

use crate::{
    AssertionState, Literal, LiteralState, Predicate, Pricer, Propagator, PropagatorAddConstraint,
    Reason, engine::assertion_stack_with_pricer::AssertionStackWithPricer,
};
use implication_queue::{ConflictImplication, ImplicationQueue};
use index_collections::Map;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExplainKey<PropagatorExplainKeyT, OuterExplainKeyT>
where
    PropagatorExplainKeyT: Copy + Eq,
    OuterExplainKeyT: Copy + Eq,
{
    Propagator(PropagatorExplainKeyT),
    Outer(OuterExplainKeyT),
}

#[derive(Clone, Copy)]
pub enum Status<ExplainKeyT>
where
    ExplainKeyT: Copy,
{
    Conflict(ConflictStatus<ExplainKeyT>),
    Noconflict,
}

impl<ExplainKeyT> Status<ExplainKeyT>
where
    ExplainKeyT: Copy,
{
    #[inline(always)]
    pub fn is_conflict(&self) -> bool {
        matches!(self, Self::Conflict(..))
    }

    #[inline(always)]
    pub fn is_noconflict(&self) -> bool {
        matches!(self, Self::Noconflict)
    }
}

#[derive(Clone, Copy)]
pub enum ConflictStatus<ExplainKeyT>
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
pub struct Engine<PropagatorT, OuterExplainKeyT, PricerT>
where
    PropagatorT: Propagator,
    OuterExplainKeyT: Copy + Eq,
    PricerT: Pricer,
{
    assertion_stack:
        AssertionStackWithPricer<ExplainKey<PropagatorT::ExplainKey, OuterExplainKeyT>, PricerT>,
    implication_queue: ImplicationQueue<PropagatorT::ExplainKey>,
    propagator: PropagatorT,
}

impl<PropagatorT, OuterExplainKeyT, PricerT> AssertionState
    for Engine<PropagatorT, OuterExplainKeyT, PricerT>
where
    PropagatorT: Propagator,
    OuterExplainKeyT: Copy + Eq + Hash,
    PricerT: Pricer,
{
    type ExplainKey = ExplainKey<PropagatorT::ExplainKey, OuterExplainKeyT>;

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
    fn literal_state(
        &self,
        assertion: Literal,
    ) -> impl LiteralState<ExplainKey = Self::ExplainKey> {
        self.assertion_stack.literal_state(assertion)
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
        self.assertion_stack
            .parameter_lower_bound_before(usize::MAX)
    }

    #[inline(always)]
    fn parameter_upper_bound(&self) -> f64 {
        self.assertion_stack
            .parameter_upper_bound_before(usize::MAX)
    }
}

impl<PropagatorT, OuterExplainKeyT, PricerT> Engine<PropagatorT, OuterExplainKeyT, PricerT>
where
    PropagatorT: Propagator,
    OuterExplainKeyT: Copy + Eq + Hash,
    PricerT: Pricer,
{
    #[inline(never)]
    pub fn new(propagator: PropagatorT, pricer: PricerT) -> Self {
        Self {
            assertion_stack: AssertionStackWithPricer::new(pricer),
            propagator,
            implication_queue: ImplicationQueue::default(),
        }
    }

    #[inline(always)]
    pub fn status(&self) -> Status<PropagatorT::ExplainKey> {
        if let Some(conflict) = self.implication_queue.get_conflict() {
            Status::Conflict(match conflict {
                ConflictImplication::Constraint { explain_key } => {
                    ConflictStatus::Constraint { explain_key }
                }
                ConflictImplication::Literals {
                    variable,
                    explain_keys,
                } => ConflictStatus::Literals {
                    variable,
                    explain_keys,
                },
            })
        } else {
            Status::Noconflict
        }
    }

    #[inline(never)]
    pub fn add_variable(&mut self, initial_value: bool, initial_activity: f64) {
        self.assertion_stack
            .add_variable(initial_value, initial_activity);
        self.propagator.add_variable();
    }

    #[inline(never)]
    pub fn add_constraint<ConstraintT>(&mut self, constraint: ConstraintT, is_learnt: bool)
    where
        PropagatorT: PropagatorAddConstraint<ConstraintT>,
    {
        assert!(self.implication_queue.is_empty());
        self.propagator.add_constraint(
            constraint,
            is_learnt,
            &self.assertion_stack,
            &mut self.implication_queue,
        );
        self.propagate();
    }

    // #[inline(never)]
    // pub fn add_learnt_constraint<ConstraintT>(&mut self, constraint: ConstraintT, shadow_price: &Map<f64>, involved_constraints: impl Iterator<Item = <Self as AssertionState>::ExplainKey> + Clone)
    // where
    //     PropagatorT: PropagatorAddConstraint<ConstraintT>,
    // {
    //     assert!(self.implication_queue.is_empty());
    //     self.assertion_stack.update_activity(shadow_price);
    //     self.propagator.receive_involved_constraints(involved_constraints.filter_map(|e| match e {
    //         ExplainKey::Propagator(e) => Some(e),
    //         _ => None
    //     }));
    //     self.propagator.add_constraint(
    //         constraint,
    //         true,
    //         &self.assertion_stack,
    //         &mut self.implication_queue,
    //     );
    //     self.propagate();
    // }

    #[inline(never)]
    pub fn decision(&mut self, predicate: Predicate) {
        assert!(self.implication_queue.is_empty());
        self.assertion_stack.assert(predicate, Reason::Decision);
        self.propagate();
    }

    #[inline(never)]
    pub fn update_activity(
        &mut self,
        weight: &Map<f64>,
        involved_constraints: impl Iterator<Item = <Self as AssertionState>::ExplainKey> + Clone,
    ) {
        self.assertion_stack.update_activity(weight);
        self.propagator
            .receive_involved_constraints(involved_constraints.filter_map(|e| match e {
                ExplainKey::Propagator(e) => Some(e),
                _ => None,
            }));
    }

    // pub fn set_activity_time_constant(&mut self, time_constant: f64) {
    //     self.assertion_stack.set_activity_time_constant(time_constant);
    // }

    #[inline(always)]
    pub fn top_priority_unassigned_literal(&self) -> Option<Literal> {
        self.assertion_stack.top_priority_unassigned_literal()
    }

    #[inline(always)]
    pub fn explain(
        &self,
        explain_key: PropagatorT::ExplainKey,
    ) -> PropagatorT::ExplanationConstraint<'_> {
        self.propagator.explain(explain_key, &self.assertion_stack)
    }

    #[inline(never)]
    pub fn backjump(&mut self, backjump_level: usize) {
        self.implication_queue.clear();
        let backjump_order = self.assertion_stack.order_range(backjump_level).end;
        self.propagator
            .backjump(backjump_level, &self.assertion_stack);
        self.assertion_stack.backjump(backjump_level);
        debug_assert!(self.assertion_stack.number_of_assertions() == backjump_order);
        if backjump_level == 0 {
            self.propagator
                .reduce_learnt_constraints(&self.assertion_stack);
        }
    }

    #[inline(never)]
    fn propagate(&mut self) {
        // debug_assert!(self.implication_queue.is_empty());
        loop {
            self.propagator
                .propagate(&self.assertion_stack, &mut self.implication_queue);
            if self.implication_queue.get_conflict().is_some() {
                break;
            } else if let Some((predicate, explain_key)) = self.implication_queue.pop_propagation()
            {
                self.assertion_stack.assert(
                    predicate,
                    Reason::Implication {
                        explain_key: ExplainKey::Propagator(explain_key),
                    },
                );
            } else {
                break;
            }
        }
    }
}
