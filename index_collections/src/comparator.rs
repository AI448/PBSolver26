pub trait Comparator<ValueT> {
    fn cmp(&self, lhs: &ValueT, rhs: &ValueT) -> std::cmp::Ordering;

    /// ==
    fn eq(&self, lhs: &ValueT, rhs: &ValueT) -> bool {
        self.cmp(lhs, rhs) == std::cmp::Ordering::Equal
    }

    /// <=
    fn le(&self, lhs: &ValueT, rhs: &ValueT) -> bool {
        self.cmp(lhs, rhs) != std::cmp::Ordering::Greater
    }

    /// >=
    fn ge(&self, lhs: &ValueT, rhs: &ValueT) -> bool {
        self.cmp(lhs, rhs) != std::cmp::Ordering::Less
    }
}

#[derive(Default, Clone, Debug)]
pub struct NaturalComparator {}

impl<ValueT> Comparator<ValueT> for NaturalComparator
where
    ValueT: std::cmp::Ord,
{
    #[inline(always)]
    fn cmp(&self, lhs: &ValueT, rhs: &ValueT) -> std::cmp::Ordering {
        return lhs.cmp(rhs);
    }

    #[inline(always)]
    fn eq(&self, lhs: &ValueT, rhs: &ValueT) -> bool {
        lhs.eq(rhs)
    }

    #[inline(always)]
    fn le(&self, lhs: &ValueT, rhs: &ValueT) -> bool {
        lhs.le(rhs)
    }

    #[inline(always)]
    fn ge(&self, lhs: &ValueT, rhs: &ValueT) -> bool {
        lhs.ge(rhs)
    }
}

#[derive(Default, Clone, Debug)]
pub struct NaturalPartialComparator {}

impl<ValueT> Comparator<ValueT> for NaturalPartialComparator
where
    ValueT: std::cmp::PartialOrd,
{
    #[inline(always)]
    fn cmp(&self, lhs: &ValueT, rhs: &ValueT) -> std::cmp::Ordering {
        return lhs.partial_cmp(rhs).unwrap();
    }

    #[inline(always)]
    fn eq(&self, lhs: &ValueT, rhs: &ValueT) -> bool {
        lhs.eq(rhs)
    }

    #[inline(always)]
    fn le(&self, lhs: &ValueT, rhs: &ValueT) -> bool {
        lhs.le(rhs)
    }

    #[inline(always)]
    fn ge(&self, lhs: &ValueT, rhs: &ValueT) -> bool {
        lhs.ge(rhs)
    }
}

#[derive(Default, Clone, Debug)]
pub struct ReverseComparator<ComparatorT> {
    comparator: ComparatorT,
}

impl<ComparatorT> ReverseComparator<ComparatorT> {
    #[inline(always)]
    pub fn new(comparator: ComparatorT) -> Self {
        return Self { comparator };
    }
}

impl<ComparatorT, ValueT> Comparator<ValueT> for ReverseComparator<ComparatorT>
where
    ComparatorT: Comparator<ValueT>,
{
    #[inline(always)]
    fn cmp(&self, lhs: &ValueT, rhs: &ValueT) -> std::cmp::Ordering {
        self.comparator.cmp(rhs, lhs)
    }

    #[inline(always)]
    fn eq(&self, lhs: &ValueT, rhs: &ValueT) -> bool {
        self.comparator.eq(rhs, lhs)
    }

    #[inline(always)]
    fn le(&self, lhs: &ValueT, rhs: &ValueT) -> bool {
        self.comparator.le(rhs, lhs)
    }

    #[inline(always)]
    fn ge(&self, lhs: &ValueT, rhs: &ValueT) -> bool {
        self.comparator.ge(rhs, lhs)
    }
}

#[derive(Default, Clone, Debug)]
pub struct IndexComparator<IndexComparatorT> {
    comparator: IndexComparatorT,
}

impl<IndexComparatorT, ValueT> Comparator<(usize, ValueT)> for IndexComparator<IndexComparatorT>
where
    IndexComparatorT: Comparator<usize>,
{
    #[inline(always)]
    fn cmp(&self, (lhs, _): &(usize, ValueT), (rhs, _): &(usize, ValueT)) -> std::cmp::Ordering {
        self.comparator.cmp(lhs, rhs)
    }

    #[inline(always)]
    fn eq(&self, (lhs, _): &(usize, ValueT), (rhs, _): &(usize, ValueT)) -> bool {
        self.comparator.eq(lhs, rhs)
    }

    #[inline(always)]
    fn le(&self, (lhs, _): &(usize, ValueT), (rhs, _): &(usize, ValueT)) -> bool {
        self.comparator.le(lhs, rhs)
    }

    #[inline(always)]
    fn ge(&self, (lhs, _): &(usize, ValueT), (rhs, _): &(usize, ValueT)) -> bool {
        self.comparator.ge(lhs, rhs)
    }
}

#[derive(Default, Clone, Debug)]
pub struct ValueComparator<ValueComparatorT> {
    comparator: ValueComparatorT,
}

impl<ComparatorT, ValueT> Comparator<(usize, ValueT)> for ValueComparator<ComparatorT>
where
    ComparatorT: Comparator<ValueT>,
{
    #[inline(always)]
    fn cmp(&self, (_, lhs): &(usize, ValueT), (_, rhs): &(usize, ValueT)) -> std::cmp::Ordering {
        self.comparator.cmp(lhs, rhs)
    }

    #[inline(always)]
    fn eq(&self, (_, lhs): &(usize, ValueT), (_, rhs): &(usize, ValueT)) -> bool {
        self.comparator.eq(lhs, rhs)
    }

    #[inline(always)]
    fn le(&self, (_, lhs): &(usize, ValueT), (_, rhs): &(usize, ValueT)) -> bool {
        self.comparator.le(lhs, rhs)
    }

    #[inline(always)]
    fn ge(&self, (_, lhs): &(usize, ValueT), (_, rhs): &(usize, ValueT)) -> bool {
        self.comparator.ge(lhs, rhs)
    }
}
