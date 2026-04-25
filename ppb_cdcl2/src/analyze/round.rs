use std::{
    cell::{Ref, RefCell},
    cmp::Reverse,
};

use crate::{AssertionState, CompressedConstraint, Constraint, Integer, Literal};

pub struct Round<ValueT>
where
    ValueT: Integer,
{
    work: RefCell<Work<ValueT>>,
    output: RefCell<CompressedConstraint<ValueT>>,
}

impl<ValueT> Default for Round<ValueT>
where
    ValueT: Integer,
{
    fn default() -> Self {
        Self {
            work: RefCell::default(),
            output: RefCell::default(),
        }
    }
}

impl<ValueT> Clone for Round<ValueT>
where
    ValueT: Integer,
{
    fn clone(&self) -> Self {
        Self::default()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Rounding {
    Unnecessary,
    Up,
    Down,
}

#[derive(Clone)]
struct Term<ValueT> {
    literal: Literal,
    coefficient: ValueT,
    rounding: Rounding,
    rounding_length: ValueT,
}

struct Work<ValueT> {
    terms: Vec<Term<ValueT>>,
}

impl<ValueT> Default for Work<ValueT> {
    fn default() -> Self {
        Self {
            terms: Vec::default(),
        }
    }
}

impl<ValueT> Round<ValueT>
where
    ValueT: Integer,
{
    #[inline(never)]
    pub fn weaken(
        &self,
        constraint: impl Constraint<Value = ValueT>,
        divisor: ValueT,
        is_core: impl Fn(Literal) -> bool,
        state: &impl AssertionState,
        order: usize,
    ) -> Ref<'_, CompressedConstraint<ValueT>> {
        self._weaken(
            &mut self.work.borrow_mut(),
            &mut self.output.borrow_mut(),
            constraint,
            divisor,
            is_core,
            state,
            order,
        );
        self.output.borrow()
    }

    fn _weaken(
        &self,
        work: &mut Work<ValueT>,
        output: &mut CompressedConstraint<ValueT>,
        constraint: impl Constraint<Value = ValueT>,
        divisor: ValueT,
        is_core: impl Fn(Literal) -> bool,
        _state: &impl AssertionState,
        _order: usize,
    ) {
        let inf_rhs = constraint.lower_bound();

        // 丸め後の右辺値
        let mut rhs_value = inf_rhs;
        // 初期の丸め方向と丸め量を算出し，右辺値を更新
        work.terms.clear();
        for (literal, coefficient) in constraint.iter_terms() {
            if coefficient == ValueT::zero() {
                continue;
            }
            let rounding;
            let rounding_length;
            if coefficient % divisor == ValueT::zero() {
                // NOTE: literal == target_literal の場合にはここを通る
                rounding = Rounding::Unnecessary;
                rounding_length = ValueT::zero();
            } else if is_core(literal) {
                rounding = Rounding::Up;
                rounding_length = coefficient.div_ceil(&divisor) * divisor - coefficient;
            } else {
                rounding = Rounding::Down;
                rounding_length = coefficient - coefficient.div_floor(&divisor) * divisor;
                // 係数を切り下げたぶん右辺値を減少
                rhs_value -= rounding_length;
            }
            debug_assert!(rounding_length < divisor);
            work.terms.push(Term {
                literal,
                coefficient,
                rounding,
                rounding_length,
            });
        }

        // rounding_length の降順にソート
        // TODO: ソートをサボれることがある
        // TODO: 基本的に一部の要素しか切り替えられないのでヒープソートしたほうがいいかも
        work.terms
            .sort_unstable_by_key(|term| Reverse(term.rounding_length));

        // 丸め方向の切り替え
        for term in work.terms.iter_mut() {
            debug_assert!(term.rounding_length < divisor);
            match term.rounding {
                Rounding::Up => {
                    // 丸め方向を切り替えた場合の右辺値の減少量
                    let decrease = divisor - term.rounding_length;
                    if rhs_value > decrease
                        && (rhs_value - decrease).div_ceil(&divisor) == rhs_value.div_ceil(&divisor)
                    {
                        term.rounding = Rounding::Down;
                        rhs_value -= decrease;
                    }
                }
                Rounding::Down => {
                    // 丸め方向を切り替えた場合の右辺値の増加量
                    let increase = term.rounding_length;
                    if (rhs_value + increase).div_ceil(&divisor)
                        == rhs_value.div_ceil(&divisor) + ValueT::one()
                    {
                        term.rounding = Rounding::Up;
                        rhs_value += increase;
                    }
                }
                Rounding::Unnecessary => {}
            }
        }
        debug_assert!(rhs_value <= inf_rhs);
        debug_assert!(rhs_value > ValueT::zero());

        // 新たな右辺値
        let lower_bound = (constraint.lower_bound() - (inf_rhs - rhs_value)).div_ceil(&divisor);

        // 決定レベル 0 の時点での右辺値の上界
        let sup0_lower_bound = lower_bound;

        // divisor で割って丸めた制約条件を構築
        output.replace(
            work.terms.iter().filter_map(|term| {
                let coefficient = std::cmp::min(
                    match term.rounding {
                        Rounding::Unnecessary => term.coefficient / divisor,
                        Rounding::Up => term.coefficient.div_ceil(&divisor),
                        Rounding::Down => term.coefficient.div_floor(&divisor),
                    },
                    sup0_lower_bound,
                );
                if coefficient != ValueT::zero() {
                    Some((term.literal, coefficient))
                } else {
                    None
                }
            }),
            lower_bound,
        );
    }
}
