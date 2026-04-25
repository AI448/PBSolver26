use std::cell::{Ref, RefCell};

use crate::{
    AssertionState, CompressedConstraint, Constraint, LiteralState, analyze::round::Round,
};

#[derive(Default)]
pub struct Rescale {
    round: Round<u128>,
    output: RefCell<CompressedConstraint<u64>>,
}

impl Clone for Rescale {
    fn clone(&self) -> Self {
        Self {
            round: self.round.clone(),
            output: RefCell::default(),
        }
    }
}

impl Rescale {
    #[inline(never)]
    pub fn rescale(
        &self,
        constraint: impl Constraint<Value = u128>,
        conflict_order: usize,
        state: &impl AssertionState,
    ) -> Ref<'_, CompressedConstraint<u64>> {
        self._rescale(
            &mut self.output.borrow_mut(),
            constraint,
            conflict_order,
            state,
        );
        self.output.borrow()
    }

    fn _rescale(
        &self,
        output: &mut CompressedConstraint<u64>,
        constraint: impl Constraint<Value = u128>,
        conflict_order: usize,
        state: &impl AssertionState,
    ) {
        let sum_of_coefficients = constraint.iter_terms().map(|(_, c)| c).sum::<u128>();
        if sum_of_coefficients <= 1 << 52 {
            output.assign(constraint.convert());
        } else {
            let number_of_terms = constraint.iter_terms().count();
            let divisor = sum_of_coefficients.div_ceil((1 << 52) - number_of_terms as u128);
            output.assign(
                self.round
                    .weaken(
                        constraint,
                        divisor,
                        |l| state.literal_state(l).is_false_before(conflict_order),
                        state,
                        conflict_order,
                    )
                    .convert(),
            );
            output.strengthen2(state);
        }
    }
}
