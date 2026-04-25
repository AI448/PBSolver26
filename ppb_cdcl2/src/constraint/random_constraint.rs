use index_collections::Map;

use crate::{AssertionState, Constraint, Integer, Literal};

pub struct RandomConstraint<ValueT>
where
    ValueT: Integer,
{
    literal_terms: Map<(bool, ValueT)>,
    lower_bound: ValueT,
}

impl<ValueT> Default for RandomConstraint<ValueT>
where
    ValueT: Integer,
{
    fn default() -> Self {
        Self {
            literal_terms: Map::default(),
            lower_bound: ValueT::zero(),
        }
    }
}

impl<ValueT> Constraint for RandomConstraint<ValueT>
where
    ValueT: Integer,
{
    type Value = ValueT;

    #[inline(always)]
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone {
        self.literal_terms
            .iter()
            .map(|(&i, &(v, c))| (Literal::new(i, v), c))
    }

    #[inline(always)]
    fn lower_bound(&self) -> Self::Value {
        self.lower_bound
    }

    #[inline(always)]
    fn get_coefficient(&self, literal: Literal) -> Option<Self::Value> {
        self.literal_terms
            .get(literal.index())
            .filter(|&&(v, _)| v == literal.value())
            .map(|&(_, c)| c)
    }
}

impl<ValueT> RandomConstraint<ValueT>
where
    ValueT: Integer,
{
    #[inline(never)]
    pub fn clear(&mut self) {
        self.literal_terms.clear();
        self.lower_bound = ValueT::zero();
    }

    #[inline(never)]
    pub fn assign(&mut self, constraint: impl Constraint<Value = ValueT>) {
        self.literal_terms.clear();
        for (literal, coefficient) in constraint.iter_terms() {
            debug_assert!(!self.literal_terms.contains_key(literal.index()));
            if coefficient > ValueT::zero() {
                self.literal_terms
                    .insert(literal.index(), (literal.value(), coefficient));
            }
        }
        self.lower_bound = constraint.lower_bound();
    }

    #[inline(never)]
    pub fn add_assign(&mut self, constraint: impl Constraint<Value = ValueT>) {
        let mut lower_bound_decrease = ValueT::zero();
        for (rhs_literal, rhs_coefficient) in constraint.iter_terms() {
            if rhs_coefficient == ValueT::zero() {
                continue;
            }
            if let Some((lhs_value, lhs_coefficient)) =
                self.literal_terms.get_mut(rhs_literal.index())
            {
                if *lhs_value == rhs_literal.value() {
                    *lhs_coefficient += rhs_coefficient;
                } else {
                    if *lhs_coefficient > rhs_coefficient {
                        lower_bound_decrease += rhs_coefficient;
                        *lhs_coefficient -= rhs_coefficient;
                    } else if *lhs_coefficient < rhs_coefficient {
                        lower_bound_decrease += *lhs_coefficient;
                        *lhs_value = rhs_literal.value();
                        *lhs_coefficient = rhs_coefficient - *lhs_coefficient;
                    } else {
                        lower_bound_decrease += rhs_coefficient;
                        self.literal_terms.remove(rhs_literal.index());
                    }
                }
            } else {
                self.literal_terms
                    .insert(rhs_literal.index(), (rhs_literal.value(), rhs_coefficient));
            }
        }

        self.lower_bound = self.lower_bound + constraint.lower_bound() - lower_bound_decrease;
    }

    #[inline(never)]
    pub fn strengthen2(&mut self, state: &impl AssertionState) {
        let lower_bound = self.lower_bound();
        let mut sum_of_unsaturating_coefficients = ValueT::zero();
        let mut gcd = ValueT::zero();
        for (_, (_, coefficient)) in self.literal_terms.iter_mut() {
            if *coefficient < lower_bound {
                sum_of_unsaturating_coefficients += *coefficient;
            } else if *coefficient > lower_bound {
                *coefficient = lower_bound;
            }
            if gcd != ValueT::one() {
                gcd = ValueT::gcd(&gcd, &coefficient);
            }
        }
        if gcd == ValueT::zero() {
            return;
        }
        if sum_of_unsaturating_coefficients < lower_bound {
            self.literal_terms.retain(|_, (_, c)| *c >= lower_bound);
            for (_, (_, coefficient)) in self.literal_terms.iter_mut() {
                *coefficient = ValueT::one();
            }
            self.lower_bound = ValueT::one();
        } else if gcd > ValueT::one() {
            for (_, (_, coefficient)) in self.literal_terms.iter_mut() {
                debug_assert!(*coefficient % gcd == ValueT::zero());
                *coefficient = *coefficient / gcd;
            }
            self.lower_bound = self.lower_bound.div_ceil(&gcd);
        }
    }
}
