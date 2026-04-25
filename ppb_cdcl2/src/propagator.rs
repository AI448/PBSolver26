mod cardinality_propagator;
mod composite_linear_propagator;
mod linear_propagator;
mod row_storage;

use std::hash::Hash;

use crate::{AssertionState, Literal, ParameterLowerBound};
pub use composite_linear_propagator::CompositeLinearPropagator;

pub trait ImplicationReceiver<ExplainKeyT> {
    fn receive_conflict(&mut self, explain_key: ExplainKeyT);

    fn receive_literal_assertion(
        &mut self,
        literal: Literal,
        explain_key: ExplainKeyT,
        normalized_slack: f64,
    );

    fn receive_parameter_lower_bound_assertion(
        &mut self,
        parameter_lower_bound: ParameterLowerBound,
        explain_key: ExplainKeyT,
    );
}

pub trait Propagator {
    type ExplanationConstraint<'a>
    where
        Self: 'a;

    type ExplainKey: Copy + Eq + Hash;

    fn add_variable(&mut self);

    fn propagate(
        &mut self,
        state: &impl AssertionState,
        receiver: &mut impl ImplicationReceiver<Self::ExplainKey>,
    );

    fn explain(
        &self,
        explain_key: Self::ExplainKey,
        state: &impl AssertionState,
    ) -> Self::ExplanationConstraint<'_>;

    fn backjump(&mut self, backjump_level: usize, state: &impl AssertionState);

    fn receive_involved_constraints(
        &mut self,
        involved_constraint: impl Iterator<Item = Self::ExplainKey> + Clone,
    );

    fn reduce_learnt_constraints(&mut self, state: &impl AssertionState);
}

pub trait PropagatorAddConstraint<ConstraintT>: Propagator {
    fn add_constraint(
        &mut self,
        constraint: ConstraintT,
        is_learnt: bool,
        state: &impl AssertionState,
        receiver: &mut impl ImplicationReceiver<Self::ExplainKey>,
    );
}
