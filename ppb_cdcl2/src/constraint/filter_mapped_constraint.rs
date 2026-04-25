use crate::{Constraint, Integer, Literal};

pub struct FilterMappedConstraint<ValueT, ConstraintT, FunctionT>
where
    ValueT: Integer,
    ConstraintT: Constraint,
    FunctionT: Fn(Literal, ConstraintT::Value) -> Option<ValueT>,
{
    constraint: ConstraintT,
    function: FunctionT,
    lower_bound: ValueT,
}

impl<ValueT, ConstraintT, FunctionT> FilterMappedConstraint<ValueT, ConstraintT, FunctionT>
where
    ValueT: Integer,
    ConstraintT: Constraint,
    FunctionT: Fn(Literal, ConstraintT::Value) -> Option<ValueT>,
{
    #[inline(always)]
    pub fn new(constraint: ConstraintT, function: FunctionT, lower_bound: ValueT) -> Self {
        Self {
            constraint,
            function,
            lower_bound,
        }
    }
}

impl<ValueT, ConstraintT, FunctionT> Constraint
    for FilterMappedConstraint<ValueT, ConstraintT, FunctionT>
where
    ValueT: Integer,
    ConstraintT: Constraint,
    FunctionT: Fn(Literal, ConstraintT::Value) -> Option<ValueT>,
{
    type Value = ValueT;
    #[inline(always)]
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone {
        self.constraint
            .iter_terms()
            .filter_map(#[inline(always)] |(l, c)| (self.function)(l, c).map(#[inline(always)] |c| (l, c)))
    }

    #[inline(always)]
    fn lower_bound(&self) -> Self::Value {
        self.lower_bound
    }
}
