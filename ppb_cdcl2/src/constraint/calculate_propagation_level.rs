use std::cell::RefCell;

use index_collections::SortedIndexMap;

use crate::{AssertionState, Constraint, Integer, LiteralState};

#[derive(Clone, Copy)]
pub enum CalculatePropagationLevelOutput {
    /// 伝播・矛盾が発生しない
    None,
    /// decision_level において伝播が発生する
    Propagate { decision_level: usize },
    /// decision_level において矛盾している
    Conflict { decision_level: usize },
}

pub struct CalculatePropagationLevel<ValueT>
where
    ValueT: Integer,
{
    work: RefCell<Work<ValueT>>,
}

struct Work<ValueT>
where
    ValueT: Integer,
{
    decision_level_to_state: SortedIndexMap<DecisionLevelState<ValueT>>,
}

impl<ValueT> Default for Work<ValueT>
where
    ValueT: Integer,
{
    fn default() -> Self {
        Self {
            decision_level_to_state: SortedIndexMap::default(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct DecisionLevelState<ValueT>
where
    ValueT: Integer,
{
    /// その決定レベルでのリテラル項の上界の減少量
    decrease_sup_lhs: ValueT,
    /// その決定レベルで割り当てられたリテラルの係数の最大値
    max_literal_coefficient_assigned_in_this_level: ValueT,
    /// その決定レベルの完了時点でのリテラル項の上界
    sup_lhs: ValueT,
    /// その決定レベルの完了時点で未割り当てのリテラルの係数の最大値
    max_unassigned_coefficient: ValueT,
}

impl<ValueT> Default for DecisionLevelState<ValueT>
where
    ValueT: Integer,
{
    fn default() -> Self {
        Self {
            decrease_sup_lhs: ValueT::zero(),
            max_literal_coefficient_assigned_in_this_level: ValueT::zero(),
            sup_lhs: ValueT::zero(),
            max_unassigned_coefficient: ValueT::zero(),
        }
    }
}

impl<ValueT> Default for CalculatePropagationLevel<ValueT>
where
    ValueT: Integer,
{
    fn default() -> Self {
        Self {
            work: RefCell::default(),
        }
    }
}

impl<ValueT> Clone for CalculatePropagationLevel<ValueT>
where
    ValueT: Integer,
{
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl<ValueT> CalculatePropagationLevel<ValueT>
where
    ValueT: Integer,
{
    pub fn calculate(
        &self,
        constraint: &impl Constraint<Value = ValueT>,
        decision_stack: &impl AssertionState,
        include_nonfalsified_literals: bool,
    ) -> CalculatePropagationLevelOutput {
        self._calculate(
            &mut self.work.borrow_mut(),
            constraint,
            decision_stack,
            include_nonfalsified_literals,
        )
    }

    #[inline(never)]
    fn _calculate(
        &self,
        work: &mut Work<ValueT>,
        constraint: &impl Constraint<Value = ValueT>,
        state: &impl AssertionState,
        include_nonfalsified_literals: bool,
    ) -> CalculatePropagationLevelOutput {
        work.decision_level_to_state.clear();
        // 決定レベル 0 に対応する要素
        work.decision_level_to_state
            .insert(0, DecisionLevelState::default());
        // 各決定レベルでの slack の減少量と，割り当てられた変数の係数の最大値を算出
        for (litera, coefficient) in constraint.iter_terms() {
            let state = state.literal_state(litera);
            if state.is_false() {
                let decision_level = unsafe {
                    debug_assert!(state.decision_level().is_some());
                    state.decision_level().unwrap_unchecked()
                };
                if let Some(decision_level_state) =
                    work.decision_level_to_state.get_mut(decision_level)
                {
                    decision_level_state.decrease_sup_lhs += coefficient;
                    decision_level_state.max_literal_coefficient_assigned_in_this_level =
                        decision_level_state
                            .max_literal_coefficient_assigned_in_this_level
                            .max(coefficient);
                } else {
                    work.decision_level_to_state.insert(
                        decision_level,
                        DecisionLevelState {
                            decrease_sup_lhs: coefficient,
                            max_literal_coefficient_assigned_in_this_level: coefficient,
                            ..Default::default()
                        },
                    );
                }
            } else if include_nonfalsified_literals && state.is_true() {
                let decision_level = unsafe {
                    debug_assert!(state.decision_level().is_some());
                    state.decision_level().unwrap_unchecked()
                };
                if let Some(decision_level_state) =
                    work.decision_level_to_state.get_mut(decision_level)
                {
                    decision_level_state.max_literal_coefficient_assigned_in_this_level =
                        decision_level_state
                            .max_literal_coefficient_assigned_in_this_level
                            .max(coefficient);
                } else {
                    work.decision_level_to_state.insert(
                        decision_level,
                        DecisionLevelState {
                            max_literal_coefficient_assigned_in_this_level: coefficient,
                            ..Default::default()
                        },
                    );
                }
            }
        }

        // 各決定レベルの完了時点でのリテラル項の上界と未割り当てリテラル項の係数の最大値を計算
        {
            let mut sup_literal_term = ValueT::zero();
            let mut max_unassigned_literal_coefficient = ValueT::zero();
            // 現時点でのリテラル項の上界と未割り当てリテラルの係数の最大値を算出
            for (literal, coefficient) in constraint.iter_terms() {
                if !state.literal_state(literal).is_false() {
                    sup_literal_term += coefficient;
                    if !state.literal_state(literal).is_assigned() {
                        max_unassigned_literal_coefficient =
                            std::cmp::max(max_unassigned_literal_coefficient, coefficient);
                    }
                }
            }
            // 決定レベルの降順に差分から計算
            for (_, decision_level_state) in work.decision_level_to_state.iter_mut().rev() {
                decision_level_state.sup_lhs = sup_literal_term;
                decision_level_state.max_unassigned_coefficient =
                    max_unassigned_literal_coefficient;
                sup_literal_term += decision_level_state.decrease_sup_lhs;
                max_unassigned_literal_coefficient = std::cmp::max(
                    decision_level_state.max_literal_coefficient_assigned_in_this_level,
                    max_unassigned_literal_coefficient,
                );
            }
        }

        // 伝播が発生する決定レベルを特定
        // NOTE: 伝播が発生する決定レベルは複数存在し得るが，最も小さいものを返す
        let lower_bound = constraint.lower_bound();
        for (&decision_level, decision_level_state) in work.decision_level_to_state.iter() {
            if decision_level_state.sup_lhs < lower_bound {
                return CalculatePropagationLevelOutput::Conflict { decision_level };
            }
            if decision_level_state.sup_lhs - decision_level_state.max_unassigned_coefficient
                < lower_bound
            {
                return CalculatePropagationLevelOutput::Propagate { decision_level };
            }
        }

        CalculatePropagationLevelOutput::None
    }
}
