use crate::Comparator;

use super::heap_sort;

pub struct PriorityQueue<ValueT, ComparatorT> {
    compare: ComparatorT,
    array: Vec<ValueT>,
}

impl<ValueT, ComparatorT> Default for PriorityQueue<ValueT, ComparatorT>
where
    ComparatorT: Default,
{
    fn default() -> Self {
        Self {
            compare: ComparatorT::default(),
            array: Vec::default(),
        }
    }
}

impl<ValueT, CompareT> Clone for PriorityQueue<ValueT, CompareT>
where
    ValueT: Clone,
    CompareT: Clone,
{
    fn clone(&self) -> Self {
        Self {
            compare: self.compare.clone(),
            array: self.array.clone(),
        }
    }
}

impl<ValueT, CompareT> std::fmt::Debug for PriorityQueue<ValueT, CompareT>
where
    Vec<ValueT>: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.array.fmt(f)
    }
}

impl<ValueT, ComparatorT> PriorityQueue<ValueT, ComparatorT>
where
    ComparatorT: Comparator<ValueT>,
{
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.array.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.array.is_empty()
    }

    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = &ValueT> + Clone + '_ {
        self.array.iter()
    }

    #[inline(always)]
    pub fn peek(&self) -> Option<&ValueT> {
        self.array.first()
    }

    // pub fn reserve(&mut self, additional: usize) {
    //     self.array.reserve(additional);
    // }

    #[inline(always)]
    pub fn push(&mut self, value: ValueT) {
        let position = self.array.len();
        self.array.push(value);
        heap_sort::up_heap(&mut self.array, position, |l, r| !self.compare.ge(l, r));
    }

    pub fn extend(&mut self, values: impl Iterator<Item = ValueT>) {
        for value in values {
            self.push(value);
        }
    }

    #[inline(always)]
    pub fn pop(&mut self) -> Option<ValueT> {
        if self.array.is_empty() {
            return None;
        } else {
            let value = self.array.swap_remove(0);
            if !self.array.is_empty() {
                heap_sort::down_heap(&mut self.array, 0, |l, r| !self.compare.ge(l, r));
            }
            return Some(value);
        }
    }

    pub fn clear(&mut self) {
        self.array.clear();
    }
}
