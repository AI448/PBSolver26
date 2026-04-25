use std::cmp::{Reverse, max};

use fxhash::FxHashSet;
use num::Zero;
use ordered_float::OrderedFloat;

use crate::{
    AssertionState, Constraint, ImplicationReceiver, Integer, Literal, LiteralState, Predicate,
    Propagator, PropagatorAddConstraint,
    propagator::row_storage::{RowId, RowStorage},
    utility::LiteralArray,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct LinearPropagatorExplainKey {
    row_id: RowId,
}

#[derive(Clone)]
pub struct LinearPropagator<ValueT>
where
    ValueT: Integer,
{
    rows: RowStorage<Row<ValueT>>,
    columns: LiteralArray<Column<ValueT>>,
    reduction_count: usize,
    unreflected_order: usize,
}

#[derive(Clone)]
struct Row<ValueT>
where
    ValueT: Integer,
{
    literal_terms: Vec<(Literal, ValueT)>,
    lower_bound: ValueT,
    is_learnt: bool,
    last_involved_timestamp: usize,
    activity: f64,
    sup_literal_terms: ValueT,
    max_unassigned_position: Option<usize>,
    max_unassigned_coefficient: ValueT,
}

impl<ValueT> Constraint for Row<ValueT>
where
    ValueT: Integer,
{
    type Value = ValueT;

    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone {
        self.literal_terms.iter().cloned()
    }

    fn lower_bound(&self) -> Self::Value {
        self.lower_bound
    }
}

impl<ValueT> Row<ValueT>
where
    ValueT: Integer,
{
    fn new(
        constraint: impl Constraint<Value = ValueT>,
        is_learnt: bool,
        reduction_count: usize,
        state: &impl AssertionState,
    ) -> Self {
        let mut literal_terms: Vec<_> = constraint.iter_terms().collect();
        literal_terms.sort_unstable_by_key(|&(_, c)| c);
        let mut max_unassigned_position = None;
        let mut max_unassigned_coefficient = ValueT::zero();
        for (k, &(literal, coefficient)) in literal_terms.iter().enumerate().rev() {
            if !state.literal_state(literal).is_assigned() {
                max_unassigned_position = Some(k);
                max_unassigned_coefficient = coefficient;
                break;
            }
        }
        Self {
            literal_terms,
            lower_bound: constraint.lower_bound().clone(),
            is_learnt,
            last_involved_timestamp: reduction_count,
            activity: 1.0,
            sup_literal_terms: constraint.sup_literal_terms(state),
            max_unassigned_position,
            max_unassigned_coefficient,
        }
    }
}

#[derive(Clone)]
struct Column<ValueT>
where
    ValueT: Integer,
{
    literal_terms: Vec<ColumnItem<ValueT>>,
}

#[derive(Clone)]
struct ColumnItem<ValueT>
where
    ValueT: Integer,
{
    row_id: RowId,
    position: usize,
    coefficient: ValueT,
}

impl<ValueT> Default for Column<ValueT>
where
    ValueT: Integer,
{
    fn default() -> Self {
        Self {
            literal_terms: Vec::default(),
        }
    }
}

impl<ValueT> LinearPropagator<ValueT>
where
    ValueT: Integer,
{
    pub fn new() -> Self {
        Self {
            rows: RowStorage::default(),
            columns: LiteralArray::default(),
            reduction_count: 0,
            unreflected_order: 0,
        }
    }

    #[inline(never)]
    fn reflect_literal_assertion(
        &mut self,
        literal: Literal,
        state: &impl AssertionState,
        receiver: &mut impl ImplicationReceiver<LinearPropagatorExplainKey>,
    ) {
        for &ColumnItem {
            row_id,
            position: _,
            coefficient,
        } in self.columns[!literal].literal_terms.iter()
        {
            let row = self.rows.get_mut(row_id).unwrap();
            row.sup_literal_terms -= coefficient;
            Self::confirm_propagation(row_id, row, state, receiver);
        }
    }

    #[inline(always)]
    fn confirm_propagation(
        row_id: RowId,
        row: &mut Row<ValueT>,
        state: &impl AssertionState,
        receiver: &mut impl ImplicationReceiver<LinearPropagatorExplainKey>,
    ) {
        if row.sup_literal_terms < row.lower_bound {
            receiver.receive_conflict(LinearPropagatorExplainKey { row_id });
        } else {
            // Boolean 変数への伝播を確認
            if let Some(max_unassigned_position) = row.max_unassigned_position
                && row.sup_literal_terms < row.max_unassigned_coefficient + row.lower_bound
            {
                debug_assert!(
                    row.literal_terms[max_unassigned_position].1 == row.max_unassigned_coefficient
                );
                // max_unassigned_coefficient を更新
                if state
                    .literal_state(row.literal_terms[max_unassigned_position].0)
                    .is_assigned()
                {
                    row.max_unassigned_position = None;
                    row.max_unassigned_coefficient = ValueT::zero();
                    for (k, &(literal, coefficient)) in row.literal_terms
                        [0..max_unassigned_position]
                        .iter()
                        .enumerate()
                        .rev()
                    {
                        if !state.literal_state(literal).is_assigned() {
                            row.max_unassigned_position = Some(k);
                            row.max_unassigned_coefficient = coefficient;
                            break;
                        }
                    }
                }
                // 伝播
                if let Some(max_unassigned_position) = row.max_unassigned_position
                    && row.sup_literal_terms < row.max_unassigned_coefficient + row.lower_bound
                {
                    row.activity += 1.0;
                    for &(literal, coefficient) in
                        row.literal_terms[0..=max_unassigned_position].iter().rev()
                    {
                        if row.sup_literal_terms >= coefficient + row.lower_bound {
                            break;
                        }
                        if !state.literal_state(literal).is_assigned() {
                            // eprintln!("PROPAGATE {} by {}", literal, row.dump(usize::MAX, state));
                            let normalized_slack = (row.sup_literal_terms.to_f64().unwrap()
                                - row.lower_bound.to_f64().unwrap())
                                / coefficient.to_f64().unwrap();
                            receiver.receive_literal_assertion(
                                literal,
                                LinearPropagatorExplainKey { row_id },
                                normalized_slack,
                            );
                        }
                    }
                }
            }
        }
    }

    #[inline(never)]
    fn retract_literal_assertion(&mut self, literal: Literal, _state: &impl AssertionState) {
        for &ColumnItem {
            row_id,
            position,
            coefficient,
        } in self.columns[!literal].literal_terms.iter()
        {
            let row = self.rows.get_mut(row_id).unwrap();
            row.sup_literal_terms += coefficient;
            if row.max_unassigned_position.is_none_or(|p| p < position) {
                row.max_unassigned_position = Some(position);
                row.max_unassigned_coefficient = coefficient;
            }
        }
        for &ColumnItem {
            row_id,
            position,
            coefficient,
        } in self.columns[literal].literal_terms.iter()
        {
            let row = self.rows.get_mut(row_id).unwrap();
            if row.max_unassigned_position.is_none_or(|p| p < position) {
                row.max_unassigned_position = Some(position);
                row.max_unassigned_coefficient = coefficient;
            }
        }
    }
}

impl<ValueT> Propagator for LinearPropagator<ValueT>
where
    ValueT: Integer,
{
    type ExplainKey = LinearPropagatorExplainKey;
    type ExplanationConstraint<'a>
        = impl Constraint<Value = ValueT> + 'a
    where
        Self: 'a;

    fn add_variable(&mut self) {
        self.columns.push([Column::default(), Column::default()]);
    }

    fn propagate(
        &mut self,
        state: &impl AssertionState,
        receiver: &mut impl ImplicationReceiver<Self::ExplainKey>,
    ) {
        debug_assert!(
            self.unreflected_order <= state.number_of_assertions(),
            "{} {}",
            self.unreflected_order,
            state.number_of_assertions()
        );
        debug_assert!(
            self.unreflected_order + 1 >= state.number_of_assertions(),
            "{} {}",
            self.unreflected_order,
            state.number_of_assertions()
        );
        while self.unreflected_order < state.number_of_assertions() {
            let assetion = state.assertion(self.unreflected_order);
            match assetion {
                Predicate::Literal(literal) => {
                    self.reflect_literal_assertion(literal, state, receiver);
                }
                _ => unreachable!(),
            }
            self.unreflected_order += 1;
        }
    }

    fn explain(
        &self,
        explain_key: Self::ExplainKey,
        _state: &impl AssertionState,
    ) -> Self::ExplanationConstraint<'_> {
        self.rows.get(explain_key.row_id).unwrap()
    }

    fn backjump(&mut self, backjump_level: usize, state: &impl AssertionState) {
        let backjump_order = state.order_range(backjump_level).end;
        debug_assert!(self.unreflected_order >= backjump_order);
        while self.unreflected_order > backjump_order {
            self.unreflected_order -= 1;
            let assetion = state.assertion(self.unreflected_order);
            match assetion {
                Predicate::Literal(literal) => {
                    self.retract_literal_assertion(literal, state);
                }
                _ => unreachable!(),
            }
        }
    }

    fn receive_involved_constraints(
        &mut self,
        involved_constraint: impl Iterator<Item = Self::ExplainKey>,
    ) {
        for explain_key in involved_constraint {
            let row = self.rows.get_mut(explain_key.row_id).unwrap();
            row.last_involved_timestamp = self.reduction_count;
        }
    }

    fn reduce_learnt_constraints(&mut self, state: &impl AssertionState) {
        assert!(state.decision_level() == 0);

        let mut number_of_static_constraint = 0;
        let mut number_of_learnt_constraints = 0;

        for (_, row) in self.rows.iter() {
            if row.is_learnt {
                number_of_learnt_constraints += 1;
            } else {
                number_of_static_constraint += 1;
            }
        }

        let number_of_removing_constraints = max(1000, number_of_learnt_constraints / 10);
        let min_number_of_retaining_constraints =
            max(1000, number_of_static_constraint / 10) + 10 * self.reduction_count;

        if number_of_learnt_constraints
            < number_of_removing_constraints + min_number_of_retaining_constraints
        {
            return;
        }

        let mut removing_row_ids = FxHashSet::default();
        let mut candidate_row_ids = Vec::default();
        for (row_id, row) in self.rows.iter() {
            // if row.lower_bound().calculate(state.parameter_lower_bound()).is_finite_and(|sup| row.inf_literal_terms(state) >= sup)
            if row.inf_literal_terms(state) >= row.lower_bound() {
                // 明らかに充足される制約である場合
                removing_row_ids.insert(row_id);
            } else if row.is_learnt {
                candidate_row_ids.push(row_id);
            }
        }
        if removing_row_ids.len() < number_of_removing_constraints {
            // 学習制約の生成に使用されていない期間が長い・アクティビティが小さい順にソート
            candidate_row_ids.sort_unstable_by_key(|&row_id| {
                let row = self.rows.get(row_id).unwrap();
                (
                    Reverse(max(self.reduction_count - row.last_involved_timestamp, 3)),
                    OrderedFloat(row.activity),
                )
            });
            removing_row_ids.extend(
                candidate_row_ids
                    .iter()
                    .cloned()
                    .take(number_of_removing_constraints - removing_row_ids.len()),
            );
        }
        // 削除
        for &removing_row_id in removing_row_ids.iter() {
            self.rows.deallocate(removing_row_id);
        }
        // 列方向からも削除
        for (_, column) in self.columns.iter_mut() {
            column
                .literal_terms
                .retain(|column_item| !removing_row_ids.contains(&column_item.row_id));
        }
        // activity を減衰
        for (_, row) in self.rows.iter_mut() {
            row.activity *= 0.9;
        }
        self.reduction_count += 1;
    }
}

impl<ConstraintT> PropagatorAddConstraint<ConstraintT> for LinearPropagator<ConstraintT::Value>
where
    ConstraintT: Constraint,
    ConstraintT::Value: Integer,
{
    fn add_constraint(
        &mut self,
        constraint: ConstraintT,
        is_learnt: bool,
        state: &impl AssertionState,
        receiver: &mut impl ImplicationReceiver<Self::ExplainKey>,
    ) {
        // 事前条件をチェック
        assert!(self.unreflected_order == state.number_of_assertions());

        // let constraint = constraint
        //     .into_drop_fixed_variables(state)
        //     .into_strengthen(state);

        // 常に充足される制約は無視
        //if constraint.sup_rhs_before(state.order_range(0).end, state).is_finite_and(|sup| sup <= ConstraintT::Value::zero())
        if constraint.lower_bound() <= ConstraintT::Value::zero() {
            return;
        }

        // 行を追加
        let row_id =
            self.rows
                .allocate(Row::new(constraint, is_learnt, self.reduction_count, state));
        let row = self.rows.get_mut(row_id).unwrap();

        // 列方向の項を追加
        for (position, &(literal, coefficient)) in row.literal_terms.iter().enumerate() {
            self.columns[literal].literal_terms.push(ColumnItem {
                row_id,
                position,
                coefficient,
            });
        }

        Self::confirm_propagation(row_id, row, state, receiver);
    }
}
