use crate::{AssertionState, Constraint, Integer, Literal, LiteralState};

#[derive(Clone)]
pub struct CompressedConstraint<ValueT>
where
    ValueT: Integer,
{
    lhs: Vec<(Literal, ValueT)>,
    rhs: ValueT,
}

impl<ValueT> Default for CompressedConstraint<ValueT>
where
    ValueT: Integer,
{
    #[inline(always)]
    fn default() -> Self {
        Self {
            lhs: Vec::default(),
            rhs: ValueT::zero(),
        }
    }
}

impl<ValueT> CompressedConstraint<ValueT>
where
    ValueT: Integer,
{
    #[inline(always)]
    pub fn new(lhs: impl Iterator<Item = (Literal, ValueT)>, rhs: ValueT) -> Self {
        Self {
            lhs: lhs.filter(|(_, c)| !c.is_zero()).collect(),
            rhs,
        }
    }

    #[inline(always)]
    pub fn from_constraint<ConstraintT>(constraint: ConstraintT) -> Self
    where
        ConstraintT: Constraint<Value = ValueT>,
    {
        Self::new(constraint.iter_terms(), constraint.lower_bound().clone())
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.lhs.clear();
        self.rhs = ValueT::zero();
    }

    #[inline(never)]
    pub fn replace(&mut self, lhs: impl Iterator<Item = (Literal, ValueT)>, rhs: ValueT) {
        self.lhs.clear();
        self.lhs.extend(lhs.filter(|(_, c)| !c.is_zero()));
        self.rhs = rhs;
    }

    #[inline(always)]
    pub fn assign<ConstraintT>(&mut self, constraint: ConstraintT)
    where
        ConstraintT: Constraint<Value = ValueT>,
    {
        self.replace(constraint.iter_terms(), constraint.lower_bound().clone());
    }

    #[inline(never)]
    pub fn drop_fixed_variables2(&mut self, state: &impl AssertionState) {
        let mut sum_of_dropeds = ValueT::zero();
        let mut i = 0;
        for j in 0..self.lhs.len() {
            let (literal, coefficient) = self.lhs[j];
            let literal_state = state.literal_state(literal);
            if literal_state.decision_level() == Some(0) {
                if literal_state.is_true() {
                    sum_of_dropeds += coefficient;
                }
            } else {
                self.lhs[i] = (literal, coefficient);
                i += 1;
            }
        }
        self.lhs.truncate(i);
        self.rhs = self.lower_bound() - sum_of_dropeds;
    }

    #[inline(never)]
    pub fn strengthen2(&mut self, state: &impl AssertionState) {
        let lower_bound = self.lower_bound();
        let mut sum_of_unsaturating_coefficients = ValueT::zero();
        let mut gcd = ValueT::zero();
        for (_, coefficient) in self.lhs.iter_mut() {
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
            self.lhs.retain(|(_, c)| *c >= lower_bound);
            self.rhs = ValueT::one();
        } else if gcd > ValueT::one() {
            for (_, coefficient) in self.lhs.iter_mut() {
                debug_assert!(*coefficient % gcd == ValueT::zero());
                *coefficient = *coefficient / gcd;
            }
            self.rhs = self.lower_bound().div_ceil(&gcd);
        }
    }
}

impl<ValueT> Constraint for CompressedConstraint<ValueT>
where
    ValueT: Integer,
{
    type Value = ValueT;

    #[inline(always)]
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone {
        self.lhs.iter().cloned()
    }

    #[inline(always)]
    fn lower_bound(&self) -> Self::Value {
        self.rhs
    }
}
