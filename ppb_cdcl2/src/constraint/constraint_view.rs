use crate::{Constraint, Integer, Literal};

#[derive(Clone)]
pub struct ConstraintView<ValueT, LiteralTermsT>
where
    ValueT: Integer,
    LiteralTermsT: Iterator<Item = (Literal, ValueT)> + Clone,
{
    literal_terms: LiteralTermsT,
    lower_bound: ValueT,
}

impl<ValueT, LiteralTermsT> ConstraintView<ValueT, LiteralTermsT>
where
    ValueT: Integer,
    LiteralTermsT: Iterator<Item = (Literal, ValueT)> + Clone,
{
    #[inline(always)]
    pub fn new(literal_terms: LiteralTermsT, lower_bound: ValueT) -> Self {
        Self {
            literal_terms,
            lower_bound,
        }
    }
}

impl<ValueT, LiteralTermsT> Constraint for ConstraintView<ValueT, LiteralTermsT>
where
    ValueT: Integer,
    LiteralTermsT: Iterator<Item = (Literal, ValueT)> + Clone,
{
    type Value = ValueT;
    #[inline(always)]
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone {
        self.literal_terms.clone()
    }

    #[inline(always)]
    fn lower_bound(&self) -> Self::Value {
        self.lower_bound
    }
}
