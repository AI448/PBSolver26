mod literal;
pub use literal::Literal;

mod parameter_bound;
pub use parameter_bound::{ParameterLowerBound, ParameterUpperBound};

/// (単項の)述語
#[derive(Clone, Copy, PartialEq)]
pub enum Predicate {
    Literal(Literal),
    ParameterLowerBound(ParameterLowerBound),
    ParameterUpperBound(ParameterUpperBound),
}

impl From<Literal> for Predicate {
    #[inline(always)]
    fn from(literal: Literal) -> Self {
        Self::Literal(literal)
    }
}

impl From<ParameterLowerBound> for Predicate {
    #[inline(always)]
    fn from(parameter_lower_bound: ParameterLowerBound) -> Self {
        Self::ParameterLowerBound(parameter_lower_bound)
    }
}

impl From<ParameterUpperBound> for Predicate {
    #[inline(always)]
    fn from(parameter_upper_bound: ParameterUpperBound) -> Self {
        Self::ParameterUpperBound(parameter_upper_bound)
    }
}

impl std::ops::Not for Predicate {
    type Output = Predicate;
    #[inline(always)]
    fn not(self) -> Self::Output {
        match self {
            Self::Literal(literal) => Self::Literal(!literal),
            Self::ParameterLowerBound(lower_bound) => Self::ParameterUpperBound(!lower_bound),
            Self::ParameterUpperBound(upper_bound) => Self::ParameterLowerBound(!upper_bound),
        }
    }
}

impl std::fmt::Display for Predicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Literal(literal) => literal.fmt(f),
            Self::ParameterLowerBound(lower_bound) => lower_bound.fmt(f),
            Self::ParameterUpperBound(upper_bound) => upper_bound.fmt(f),
        }
    }
}

impl std::fmt::Debug for Predicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}
