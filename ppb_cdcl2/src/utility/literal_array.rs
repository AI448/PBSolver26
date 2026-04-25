use std::ops::{Index, IndexMut};

use crate::Literal;

#[derive(Default, Clone, Debug)]
pub struct LiteralArray<ValueT> {
    array: Vec<ValueT>,
}

impl<ValueT> LiteralArray<ValueT> {
    #[must_use]
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.array.len()
    }

    #[inline(always)]
    pub fn push(&mut self, values: [ValueT; 2]) {
        self.array.extend(values.into_iter());
    }

    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = (Literal, &ValueT)> + Clone {
        self.array
            .iter()
            .enumerate()
            .map(|(bits, value)| (Literal::from_bits(bits), value))
    }

    #[inline(always)]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Literal, &mut ValueT)> {
        self.array
            .iter_mut()
            .enumerate()
            .map(|(bits, value)| (Literal::from_bits(bits), value))
    }
}

impl<ValueT> Index<Literal> for LiteralArray<ValueT> {
    type Output = ValueT;
    #[inline(always)]
    fn index(&self, literal: Literal) -> &Self::Output {
        &self.array[literal.bits()]
    }
}

impl<ValueT> IndexMut<Literal> for LiteralArray<ValueT> {
    #[inline(always)]
    fn index_mut(&mut self, literal: Literal) -> &mut Self::Output {
        &mut self.array[literal.bits()]
    }
}
