use std::cell::RefCell;

use index_collections::Set;

use crate::{AssertionState, Constraint, LiteralState};

#[derive(Default)]
pub struct CalculatePLBD {
    decision_level_set: RefCell<Set>,
}

impl Clone for CalculatePLBD {
    fn clone(&self) -> Self {
        Self {
            decision_level_set: RefCell::default(),
        }
    }
}

impl CalculatePLBD {
    #[inline(never)]
    pub fn calculate(
        &self,
        constraint: impl Constraint,
        state: &impl AssertionState,
        decision_level: usize,
    ) -> usize {
        self._calculate(
            &mut self.decision_level_set.borrow_mut(),
            constraint,
            state,
            decision_level,
        )
    }

    fn _calculate(
        &self,
        decision_level_set: &mut Set,
        constraint: impl Constraint,
        state: &impl AssertionState,
        decision_level: usize,
    ) -> usize {
        decision_level_set.clear();
        for (literal, _) in constraint.iter_terms() {
            let assertion_state = state.literal_state(literal);
            if assertion_state.is_false() {
                if let Some(level) = assertion_state.decision_level()
                    && level != 0
                    && level <= decision_level
                {
                    decision_level_set.insert(level);
                }
            }
        }
        return decision_level_set.len();
    }
}
