use index_collections::{
    HeapedMap, NaturalComparator, ReverseComparator, ValueComparator,
};
use ordered_float::OrderedFloat;

use crate::{
    Predicate,
    engine::{
        assertion_state::{AssertionState, LiteralState},
        pricer::PricerTrait,
    },
    utility::LiteralSet,
};

#[derive(Clone)]
pub struct UpdateOnlyAssignedPricer {
    time_constant: f64,
    priorities: Vec<(u64, OrderedFloat<f64>)>,
    unassigned_variables:
        HeapedMap<(u64, OrderedFloat<f64>), ReverseComparator<ValueComparator<NaturalComparator>>>,
}

impl UpdateOnlyAssignedPricer {
    pub fn new(time_constant: f64) -> Self {
        Self {
            time_constant,
            priorities: Vec::default(),
            unassigned_variables: HeapedMap::default(),
        }
    }
}

impl PricerTrait for UpdateOnlyAssignedPricer {
    #[inline(always)]
    fn add_variable(&mut self, priority: u64) {
        let variable = self.priorities.len();
        self.priorities.push((priority, OrderedFloat(1.0 - 1e-8)));
        self.unassigned_variables
            .insert(variable, (priority, OrderedFloat(1.0 - 1e-8)));
    }

    #[inline(never)]
    fn update_activity(&mut self, conflict_literals: &LiteralSet, state: &impl AssertionState) {
        let e = 1.0 / self.time_constant;
        for order in 0..state.number_of_assertions() {
            if let Predicate::Literal(literal) = state.assertion(order) {
                debug_assert!(!conflict_literals.contains_key(!literal));
                debug_assert!(!self.unassigned_variables.contains_key(literal.index()));
                let activity = &mut self.priorities[literal.index()].1;
                if conflict_literals.contains_key(literal) {
                    *activity = f64::mul_add(1.0 - e, (*activity).into(), e).into();
                } else {
                    *activity *= 1.0 - e;
                }
            }
        }
        for literal in conflict_literals.iter() {
            if !state.literal_state(literal.into()).is_assigned() {
                let activity = &mut self.priorities[literal.index()].1;
                *activity = f64::mul_add(1.0 - e, (*activity).into(), e).into();
            }
        }
    }

    #[inline(always)]
    fn get(&self, variable: usize) -> ((u64, f64), bool) {
        (
            (
                self.priorities[variable].0,
                self.priorities[variable].1.into(),
            ),
            self.unassigned_variables.contains_key(variable),
        )
    }

    #[inline(always)]
    fn peek(&self) -> Option<(usize, (u64, f64))> {
        self.unassigned_variables
            .first()
            .map(|(&variable, &priority)| {
                debug_assert!(priority.0 == self.priorities[variable].0);
                debug_assert!(priority.1 == self.priorities[variable].1);
                (variable, (priority.0, priority.1.into()))
            })
    }

    #[inline(always)]
    fn set_priority(&mut self, variable: usize, priority: u64) {
        self.priorities[variable].0 = priority;
        if self.unassigned_variables.contains_key(variable) {
            self.unassigned_variables.insert(
                variable,
                (self.priorities[variable].0, self.priorities[variable].1),
            );
        }
    }

    #[inline(always)]
    fn assign(&mut self, variable: usize) {
        self.unassigned_variables.remove(variable);
    }

    #[inline(always)]
    fn unassign(&mut self, variable: usize) {
        self.unassigned_variables.insert(
            variable,
            (self.priorities[variable].0, self.priorities[variable].1),
        );
    }
}
