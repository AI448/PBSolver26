use std::usize;

use either::Either;

use crate::{
    AssertionState, CompressedConstraint, Constraint, ImplicationReceiver, Integer, Propagator,
    PropagatorAddConstraint,
    propagator::{
        cardinality_propagator::{CardinalityPropagator, CardinalityPropagatorExplainKey},
        linear_propagator::{LinearPropagator, LinearPropagatorExplainKey},
    },
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompositeLinearPropagatorExplinKey {
    Cardinality(CardinalityPropagatorExplainKey),
    Linear(LinearPropagatorExplainKey),
}

impl<T> ImplicationReceiver<CardinalityPropagatorExplainKey> for T
where
    T: ImplicationReceiver<CompositeLinearPropagatorExplinKey>,
{
    fn receive_conflict(&mut self, explain_key: CardinalityPropagatorExplainKey) {
        self.receive_conflict(CompositeLinearPropagatorExplinKey::Cardinality(explain_key));
    }

    fn receive_literal_assertion(
        &mut self,
        literal: crate::Literal,
        explain_key: CardinalityPropagatorExplainKey,
        normalized_slack: f64,
    ) {
        self.receive_literal_assertion(
            literal,
            CompositeLinearPropagatorExplinKey::Cardinality(explain_key),
            normalized_slack,
        );
    }

    fn receive_parameter_lower_bound_assertion(
        &mut self,
        parameter_lower_bound: crate::ParameterLowerBound,
        explain_key: CardinalityPropagatorExplainKey,
    ) {
        self.receive_parameter_lower_bound_assertion(
            parameter_lower_bound,
            CompositeLinearPropagatorExplinKey::Cardinality(explain_key),
        );
    }
}

impl<T> ImplicationReceiver<LinearPropagatorExplainKey> for T
where
    T: ImplicationReceiver<CompositeLinearPropagatorExplinKey>,
{
    fn receive_conflict(&mut self, explain_key: LinearPropagatorExplainKey) {
        self.receive_conflict(CompositeLinearPropagatorExplinKey::Linear(explain_key));
    }

    fn receive_literal_assertion(
        &mut self,
        literal: crate::Literal,
        explain_key: LinearPropagatorExplainKey,
        normalized_slack: f64,
    ) {
        self.receive_literal_assertion(
            literal,
            CompositeLinearPropagatorExplinKey::Linear(explain_key),
            normalized_slack,
        );
    }

    fn receive_parameter_lower_bound_assertion(
        &mut self,
        parameter_lower_bound: crate::ParameterLowerBound,
        explain_key: LinearPropagatorExplainKey,
    ) {
        self.receive_parameter_lower_bound_assertion(
            parameter_lower_bound,
            CompositeLinearPropagatorExplinKey::Linear(explain_key),
        );
    }
}

#[derive(Clone)]
pub struct CompositeLinearPropagator<ValueT>
where
    ValueT: Integer,
{
    cardinality_propagator: CardinalityPropagator<ValueT>,
    linear_propagator: LinearPropagator<ValueT>,
    constraint: CompressedConstraint<ValueT>,
}

impl<ValueT> CompositeLinearPropagator<ValueT>
where
    ValueT: Integer,
{
    pub fn new() -> Self {
        Self {
            cardinality_propagator: CardinalityPropagator::new(),
            linear_propagator: LinearPropagator::new(),
            constraint: CompressedConstraint::default(),
        }
    }
}

impl<ValueT> Propagator for CompositeLinearPropagator<ValueT>
where
    ValueT: Integer,
{
    type ExplainKey = CompositeLinearPropagatorExplinKey;
    type ExplanationConstraint<'a>
        = impl Constraint<Value = ValueT>
    where
        Self: 'a;

    fn add_variable(&mut self) {
        self.cardinality_propagator.add_variable();
        self.linear_propagator.add_variable();
    }

    fn propagate(
        &mut self,
        state: &impl AssertionState,
        receiver: &mut impl ImplicationReceiver<Self::ExplainKey>,
    ) {
        self.cardinality_propagator.propagate(state, receiver);
        self.linear_propagator.propagate(state, receiver);
    }

    fn explain(
        &self,
        explain_key: Self::ExplainKey,
        state: &impl AssertionState,
    ) -> Self::ExplanationConstraint<'_> {
        match explain_key {
            CompositeLinearPropagatorExplinKey::Cardinality(explain_key) => {
                Either::Left(self.cardinality_propagator.explain(explain_key, state))
            }
            CompositeLinearPropagatorExplinKey::Linear(explain_key) => {
                Either::Right(self.linear_propagator.explain(explain_key, state))
            }
        }
    }

    fn backjump(&mut self, backjump_level: usize, state: &impl AssertionState) {
        self.cardinality_propagator.backjump(backjump_level, state);
        self.linear_propagator.backjump(backjump_level, state);
    }

    fn receive_involved_constraints(
        &mut self,
        involved_constraint: impl Iterator<Item = Self::ExplainKey> + Clone,
    ) {
        self.cardinality_propagator.receive_involved_constraints(
            involved_constraint.clone().filter_map(|e| match e {
                CompositeLinearPropagatorExplinKey::Cardinality(e) => Some(e),
                _ => None,
            }),
        );
        self.linear_propagator.receive_involved_constraints(
            involved_constraint.clone().filter_map(|e| match e {
                CompositeLinearPropagatorExplinKey::Linear(e) => Some(e),
                _ => None,
            }),
        );
    }

    fn reduce_learnt_constraints(&mut self, state: &impl AssertionState) {
        self.cardinality_propagator.reduce_learnt_constraints(state);
        self.linear_propagator.reduce_learnt_constraints(state);
    }
}

impl<ValueT, ConstraintT> PropagatorAddConstraint<ConstraintT> for CompositeLinearPropagator<ValueT>
where
    ValueT: Integer,
    ConstraintT: Constraint<Value = ValueT>,
{
    fn add_constraint(
        &mut self,
        constraint: ConstraintT,
        is_learnt: bool,
        state: &impl AssertionState,
        receiver: &mut impl ImplicationReceiver<Self::ExplainKey>,
    ) {
        // eprintln!("A {}", constraint.dump(usize::MAX, state));
        // 常に充足される制約は無視
        // if constraint.sup_rhs_before(state.order_range(0).end, state).is_finite_and(|sup| sup <= ConstraintT::Value::zero())
        if constraint.lower_bound() <= ConstraintT::Value::zero() {
            return;
        }

        // eprintln!("B {}", constraint.dump(usize::MAX, state));
        self.constraint.assign(constraint);
        self.constraint.strengthen2(state);

        // 常に充足される制約は無視
        // if self
        //     .constraint
        //     .sup_rhs_before(state.order_range(0).end, state).is_finite_and(|sup|sup <= ConstraintT::Value::zero())
        if self.constraint.lower_bound() <= ConstraintT::Value::zero() {
            return;
        }

        // eprintln!("C {}", self.constraint.dump(usize::MAX, state));
        if self
            .constraint
            .iter_terms()
            .all(|(_, c)| c == ValueT::one())
        {
            self.cardinality_propagator.add_constraint(
                &self.constraint,
                is_learnt,
                state,
                receiver,
            );
        } else {
            self.linear_propagator
                .add_constraint(&self.constraint, is_learnt, state, receiver);
        }
    }
}
