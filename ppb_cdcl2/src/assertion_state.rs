use std::hash::Hash;

use crate::{Literal, Predicate};

/// 表明の理由
#[derive(Clone, Copy, Debug)]
pub enum Reason<ExplainKeyT>
where
    ExplainKeyT: Copy,
{
    /// 決定
    Decision,
    /// 含意
    Implication { explain_key: ExplainKeyT },
}

impl<ExplainKeyT> Reason<ExplainKeyT>
where
    ExplainKeyT: Copy,
{
    #[inline(always)]
    pub fn is_decision(&self) -> bool {
        matches!(self, Self::Decision)
    }

    #[inline(always)]
    pub fn is_implication(&self) -> bool {
        matches!(self, Self::Implication { .. })
    }
}

pub trait AssertionState {
    type ExplainKey: Copy + Eq + Hash;
    // ブール変数の数
    fn number_of_variables(&self) -> usize;

    /// 表明された述語の数
    fn number_of_assertions(&self) -> usize;

    /// 値が割り当てられている変数の数
    fn number_of_assigned_variables(&self) -> usize;

    /// 現在の決定レベル
    fn decision_level(&self) -> usize;

    /// decision_level に対応する order の範囲
    fn order_range(&self, decision_level: usize) -> std::ops::Range<usize>;

    /// order 番目の表明
    fn assertion(&self, order: usize) -> Predicate;

    /// 表明の状態
    fn literal_state(&self, literal: Literal) -> impl LiteralState<ExplainKey = Self::ExplainKey>;

    fn parameter_lower_bound_before(&self, order: usize) -> f64;

    fn parameter_upper_bound_before(&self, order: usize) -> f64;

    fn parameter_lower_bound(&self) -> f64;

    fn parameter_upper_bound(&self) -> f64;
}

pub trait LiteralState {
    type ExplainKey: Copy;

    fn order(&self) -> Option<usize>;

    fn decision_level(&self) -> Option<usize>;

    fn is_assigned_before(&self, assertion_order: usize) -> bool;

    fn is_assigned(&self) -> bool;

    fn is_true_before(&self, assertion_order: usize) -> bool;

    fn is_true(&self) -> bool;

    fn is_false_before(&self, assertion_order: usize) -> bool;

    fn is_false(&self) -> bool;

    fn reason(&self) -> Option<Reason<Self::ExplainKey>>;
}
