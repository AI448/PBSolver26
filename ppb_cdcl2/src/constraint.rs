use num::{Integer as NumInteger, One, Zero};
use std::{cell::Ref, fmt::Display, ops::Deref, usize};

use either::Either;

mod calculate_propagation_level;
pub use calculate_propagation_level::{CalculatePropagationLevel, CalculatePropagationLevelOutput};

mod calculate_plbd;
pub use calculate_plbd::CalculatePLBD;

mod filter_mapped_constraint;
pub use filter_mapped_constraint::FilterMappedConstraint;

mod compressed_constraint;
pub use compressed_constraint::CompressedConstraint;

mod random_constraint;
pub use random_constraint::RandomConstraint;

mod constraint_view;
pub use constraint_view::ConstraintView;

use crate::{
    Literal,
    assertion_state::{AssertionState, LiteralState},
};

pub trait Integer:
    num::Integer
    + num::PrimInt
    + num::FromPrimitive
    + std::ops::AddAssign
    + std::ops::SubAssign
    + std::iter::Sum
    + Display
{
}

impl<T> Integer for T where
    T: num::Integer
        + num::PrimInt
        + num::FromPrimitive
        + std::ops::AddAssign
        + std::ops::SubAssign
        + std::iter::Sum
        + Display
{
}

pub trait Constraint: Sized {
    type Value: Integer;
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone;
    fn lower_bound(&self) -> Self::Value;

    /// デフォルト実装は線形時間であることに注意
    #[inline(never)]
    fn get_coefficient(&self, literal: Literal) -> Option<Self::Value> {
        self.iter_terms()
            .find(|&(l, _)| l == literal)
            .map(|(_, c)| c)
    }

    #[inline(never)]
    fn sup_literal_terms_before(&self, order: usize, state: &impl AssertionState) -> Self::Value {
        let mut sup = Self::Value::zero();
        for (literal, coefficient) in self.iter_terms() {
            if !state.literal_state(literal).is_false_before(order) {
                sup += coefficient;
            }
        }
        sup
    }

    #[inline(always)]
    fn sup_literal_terms(&self, state: &impl AssertionState) -> Self::Value {
        self.sup_literal_terms_before(usize::MAX, state)
    }

    #[inline(never)]
    fn inf_literal_terms_before(&self, order: usize, state: &impl AssertionState) -> Self::Value {
        let mut inf = Self::Value::zero();
        for (literal, coefficient) in self.iter_terms() {
            if state.literal_state(literal).is_true_before(order) {
                inf += coefficient;
            }
        }
        inf
    }

    #[inline(always)]
    fn inf_literal_terms(&self, state: &impl AssertionState) -> Self::Value {
        self.inf_literal_terms_before(usize::MAX, state)
    }

    #[inline(never)]
    fn max_unassigned_coefficient_before(
        &self,
        order: usize,
        state: &impl AssertionState,
    ) -> Self::Value {
        let mut max = Self::Value::zero();
        for (literal, coefficient) in self.iter_terms() {
            if !state.literal_state(literal).is_assigned_before(order) {
                max = std::cmp::max(max, coefficient);
            }
        }
        max
    }

    #[inline(always)]
    fn max_unassigned_coefficient(&self, state: &impl AssertionState) -> Self::Value {
        self.max_unassigned_coefficient_before(usize::MAX, state)
    }

    #[inline(always)]
    fn convert<T>(&self) -> impl Constraint<Value = T>
    where
        T: Integer,
    {
        ConstraintView::new(
            self.iter_terms().map(|(l, c)| (l, T::from(c).unwrap())),
            T::from(self.lower_bound()).unwrap(),
        )
    }

    #[inline(always)]
    fn mul(&self, value: Self::Value) -> impl Constraint<Value = Self::Value> {
        ConstraintView::new(
            self.iter_terms().map(move |(l, c)| (l, c * value)),
            self.lower_bound() * value,
        )
        // FilterMappedConstraint::new(
        //     self,
        //     move |_, c| Some(c * value),
        //     self.lower_bound().mul_constant(value),
        // )
    }

    #[inline(never)]
    fn into_drop_fixed_variables(
        self,
        state: &impl AssertionState,
    ) -> impl Constraint<Value = Self::Value> {
        let mut sum_of_dropeds = Self::Value::zero();
        for (literal, coefficient) in self.iter_terms() {
            let literal_state = state.literal_state(literal);
            if literal_state.decision_level() == Some(0) {
                if literal_state.is_true() {
                    sum_of_dropeds += coefficient;
                }
            }
        }
        let lower_bound = self.lower_bound() - sum_of_dropeds;
        FilterMappedConstraint::new(
            self,
            #[inline(always)] |l, c| {
                if state.literal_state(l).decision_level() != Some(0) {
                    Some(c)
                } else {
                    None
                }
            },
            lower_bound,
        )
    }

    #[inline(always)]
    fn into_strengthen(self, _state: &impl AssertionState) -> impl Constraint<Value = Self::Value> {
        let lower_bound = self.lower_bound();
        let mut sum_of_unsaturating_coefficients = Self::Value::zero();
        let mut gcd = Self::Value::zero();
        for (_, coefficient) in self.iter_terms() {
            let coefficient = std::cmp::min(coefficient, lower_bound);
            if coefficient < lower_bound {
                sum_of_unsaturating_coefficients += coefficient;
            }            
            if gcd != Self::Value::one() {
                gcd = Self::Value::gcd(&gcd, &coefficient);
            }
        }
        if gcd == Self::Value::zero() {
            Either::Left(self)
        } else if sum_of_unsaturating_coefficients < lower_bound {
            Either::Right(Either::Left(
                    FilterMappedConstraint::new(
                        self,
                        #[inline(always)] move |_, coefficient| {
                            let coefficient = std::cmp::min(coefficient, lower_bound);
                            if coefficient == lower_bound {
                                Some(Self::Value::one())
                            } else {
                                None
                            }
                        },
                        Self::Value::one(),
                    )
                )
            )
        } else {
            let new_lower_bound = self.lower_bound().div_ceil(&gcd);
            Either::Right(Either::Right(
                FilterMappedConstraint::new(
                    self,
                    #[inline(always)] move |_, coefficient| {
                        let coefficient = std::cmp::min(coefficient, lower_bound);
                        debug_assert!(coefficient % gcd == Self::Value::zero());
                        if coefficient != Self::Value::zero() {
                            Some(coefficient / gcd)
                        } else {
                            None
                        }
                    },
                    new_lower_bound,
                )
            ))
            // (self.lower_bound().div_ceil(&gcd), gcd)
        }
    }

    #[inline(always)]
    fn strengthen(&self, state: &impl AssertionState) -> impl Constraint<Value = Self::Value> {
        self.into_strengthen(state)
    }

    fn dump<'a>(
        &'a self,
        order: usize,
        state: &'a impl AssertionState,
    ) -> impl std::fmt::Display + 'a {
        Dump {
            constraint: self,
            state,
            order,
        }
    }
}

impl<T> Constraint for &T
where
    T: Constraint,
{
    type Value = T::Value;

    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone {
        (*self).iter_terms()
    }

    fn lower_bound(&self) -> Self::Value {
        (*self).lower_bound()
    }
}

impl<T> Constraint for &mut T
where
    T: Constraint,
{
    type Value = T::Value;

    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone {
        self.deref().iter_terms()
    }

    fn lower_bound(&self) -> Self::Value {
        self.deref().lower_bound()
    }
}

impl<'a, T> Constraint for Ref<'a, T>
where
    T: Constraint,
{
    type Value = T::Value;

    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone {
        self.deref().iter_terms()
    }

    fn lower_bound(&self) -> Self::Value {
        self.deref().lower_bound()
    }
}

impl<T, U> Constraint for Either<T, U>
where
    T: Constraint,
    U: Constraint<Value = T::Value>,
{
    type Value = T::Value;

    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone {
        match self {
            Either::Left(left) => Either::Left(left.iter_terms()),
            Either::Right(right) => Either::Right(right.iter_terms()),
        }
    }

    fn lower_bound(&self) -> Self::Value {
        match self {
            Either::Left(left) => left.lower_bound(),
            Either::Right(right) => right.lower_bound(),
        }
    }
}

pub struct Dump<'a, ConstraintT, StateT> {
    constraint: &'a ConstraintT,
    state: &'a StateT,
    order: usize,
}

impl<'a, ConstraintT, StateT> std::fmt::Display for Dump<'a, ConstraintT, StateT>
where
    ConstraintT: Constraint,
    StateT: AssertionState,
    ConstraintT::Value: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (literal, coefficient) in self.constraint.iter_terms() {
            let (l, u) = if self.state.literal_state(literal).is_true_before(self.order) {
                (1, 1)
            } else if self
                .state
                .literal_state(literal)
                .is_false_before(self.order)
            {
                (0, 0)
            } else {
                (0, 1)
            };
            write!(f, "+ {} {}[{}:{}] ", coefficient, literal, l, u)?;
        }
        let sup_literal_terms = self
            .constraint
            .sup_literal_terms_before(self.order, self.state);
        writeln!(f, ">= {}", self.constraint.lower_bound())?;
        write!(
            f,
            "sup_literal_terms={}",
            sup_literal_terms,
            // sup_literal_terms.to_i128().unwrap() - l.to_i128().unwrap()
        )
    }
}
