use crate::{Integer, Literal};

pub(crate) trait ReduceCoefficientAndDropIfZero<ValueT> {
    type Output;
    fn reduce_coefficient_and_drop_if_zero(self, sup_lower_bound: ValueT) -> Self::Output;
}

impl<IteratorT, ValueT> ReduceCoefficientAndDropIfZero<ValueT> for IteratorT
where
    IteratorT: Iterator<Item = (Literal, ValueT)>,
    ValueT: Integer,
{
    type Output =
        std::iter::FilterMap<Self, impl FnMut(IteratorT::Item) -> Option<IteratorT::Item> + Clone>;
    fn reduce_coefficient_and_drop_if_zero(self, sup_lower_bound: ValueT) -> Self::Output {
        let sup_lower_bound = std::cmp::max(sup_lower_bound, ValueT::zero());
        self.filter_map(move |(a, c)| {
            debug_assert!(c >= ValueT::zero());
            let c = std::cmp::min(c, sup_lower_bound);
            if c > ValueT::zero() {
                Some((a, c))
            } else {
                None
            }
        })
    }
}
