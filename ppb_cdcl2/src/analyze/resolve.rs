use std::cell::{Ref, RefCell};

use num::Integer;

use crate::{AssertionState, Constraint, LiteralState, RandomConstraint, analyze::round::Round};

#[derive(Default)]
pub struct Resolve {
    tighten: Round<u64>,
    output: RefCell<RandomConstraint<u128>>,
}

impl Clone for Resolve {
    #[inline(never)]
    fn clone(&self) -> Self {
        Self {
            tighten: self.tighten.clone(),
            output: RefCell::default(),
        }
    }
}

impl Resolve {
    #[inline(never)]
    pub fn resolve(
        &self,
        constraint0: impl Constraint<Value = u64>,
        constraint1: impl Constraint<Value = u64>,
        target_variable: usize,
        state: &impl AssertionState,
    ) -> Ref<'_, RandomConstraint<u128>> {
        self._resolve(
            &mut self.output.borrow_mut(),
            constraint0,
            constraint1,
            target_variable,
            state,
        );
        self.output.borrow()
    }

    fn _resolve(
        &self,
        output: &mut RandomConstraint<u128>,
        constraint0: impl Constraint<Value = u64>,
        constraint1: impl Constraint<Value = u64>,
        target_variable: usize,
        state: &impl AssertionState,
    ) {
        let target_literal = constraint0
            .iter_terms()
            .find(|(l, _)| l.index() == target_variable)
            .unwrap()
            .0;

        let target_order = state
            .literal_state(target_literal)
            .order()
            .unwrap_or(usize::MAX);

        let coefficient0 = constraint0.get_coefficient(target_literal).unwrap();
        let coefficient1 = constraint1.get_coefficient(!target_literal).unwrap();

        let slack0 = constraint0.sup_literal_terms_before(target_order, state) as i128
            - constraint0.lower_bound() as i128;
        let slack1 = constraint1.sup_literal_terms_before(target_order, state) as i128
            - constraint1.lower_bound() as i128;
        // NOTE: 以下の条件が成立することにより，各制約を節化して slack が 0 である妥当不等式が得られることを保証できる
        assert!(slack0 < coefficient0 as i128);
        assert!(slack1 < coefficient1 as i128);

        if (slack0 * coefficient1 as i128 + slack1 * coefficient0 as i128)
            < (coefficient0 as i128 * coefficient1 as i128)
        {
            // slack0 / coefficient0 + slack1 / coefficient1 < 1 ならそのまま足す
            let gcd = u64::gcd(&coefficient0, &coefficient1);
            output.assign(constraint0.convert().mul((coefficient1 / gcd) as u128));
            output.add_assign(constraint1.convert().mul((coefficient0 / gcd) as u128));
        } else if slack0 * coefficient1 as i128 > slack1 * coefficient0 as i128 {
            // slack0 / coefficient0 > slack1 / coefficient1 なら constraint0 を weaken して足す
            let weakened_constraint0 = self.tighten.weaken(
                &constraint0,
                coefficient0,
                |l| l == target_literal || state.literal_state(l).is_false_before(target_order),
                state,
                target_order,
            );
            // スラックが 0 以下になっているはず
            debug_assert!(
                weakened_constraint0.sup_literal_terms_before(target_order, state)
                    <= weakened_constraint0.lower_bound()
            );
            // 係数が 1 になっているはず
            debug_assert!(weakened_constraint0.get_coefficient(target_literal) == Some(1));
            output.assign(weakened_constraint0.convert().mul(coefficient1 as u128));
            output.add_assign(constraint1.convert());
        } else {
            // slack0 / coefficient0 <= slack1 / coefficient1 なら constraint1 を weaken して足す
            let weakened_constraint1 = self.tighten.weaken(
                &constraint1,
                coefficient1,
                |l| l == !target_literal || state.literal_state(l).is_false_before(target_order),
                state,
                target_order,
            );
            // スラックが 0 以下になっているはず
            debug_assert!(
                weakened_constraint1.sup_literal_terms_before(target_order, state)
                    <= weakened_constraint1.lower_bound()
            );
            // 係数が 1 になっているはず
            debug_assert!(weakened_constraint1.get_coefficient(!target_literal) == Some(1));
            output.assign(constraint0.convert());
            output.add_assign(weakened_constraint1.convert().mul(coefficient0 as u128));
        }
        debug_assert!(
            output.sup_literal_terms_before(target_order, state) < output.lower_bound(),
            "{}\n{}\n{}",
            constraint0.dump(target_order, state),
            constraint1.dump(target_order, state),
            output.dump(target_order, state)
        );

        output.strengthen2(state);
        debug_assert!(output.get_coefficient(target_literal.into()).is_none());
        debug_assert!(output.get_coefficient((!target_literal).into()).is_none());
    }
}
