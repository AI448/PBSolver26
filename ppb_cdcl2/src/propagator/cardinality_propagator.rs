use std::cmp::{Reverse, max};

use fxhash::FxHashSet;
use num::{One, ToPrimitive, Zero};
use ordered_float::OrderedFloat;

use crate::{
    AssertionState, Constraint, ImplicationReceiver, Integer, Literal, LiteralArray, LiteralState,
    Predicate, Propagator, PropagatorAddConstraint,
    propagator::row_storage::{RowId, RowStorage},
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct CardinalityPropagatorExplainKey {
    row_id: RowId,
}

#[derive(Clone)]
pub struct CardinalityPropagator<ValueT>
where
    ValueT: Integer,
{
    rows: RowStorage<Row<ValueT>>,
    columns: LiteralArray<Column>,
    reduction_count: usize,
    unrefrected_order: usize,
}

#[derive(Clone)]
struct Row<ValueT> {
    literals: Vec<Literal>,
    lower_bound: usize,
    is_learnt: bool,
    last_involved_timestamp: usize,
    activity: f64,
    _value_t: std::marker::PhantomData<ValueT>,
}

impl<ValueT> Row<ValueT>
where
    ValueT: Integer,
{
    fn new(
        literals: Vec<Literal>,
        lower_bound: usize,
        is_learnt: bool,
        reduction_count: usize,
    ) -> Self {
        Self {
            literals,
            lower_bound,
            last_involved_timestamp: reduction_count,
            is_learnt,
            activity: 1.0,
            _value_t: std::marker::PhantomData::default(),
        }
    }

    fn number_of_watching_literals(&self) -> usize {
        self.lower_bound + 1
    }
}

impl<ValueT> Constraint for Row<ValueT>
where
    ValueT: Integer,
{
    type Value = ValueT;

    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone {
        self.literals.iter().map(|&l| (l, ValueT::one()))
    }

    fn lower_bound(&self) -> Self::Value {
        ValueT::from_usize(self.lower_bound).unwrap()
    }
}

#[derive(Default, Clone)]
struct Column {
    watchers: Vec<Watcher>,
}

#[derive(Clone)]
struct Watcher {
    row_id: RowId,
    position: usize,
}

impl<ValueT> CardinalityPropagator<ValueT>
where
    ValueT: Integer,
{
    pub fn new() -> Self {
        Self {
            rows: RowStorage::default(),
            columns: LiteralArray::default(),
            reduction_count: 0,
            unrefrected_order: 0,
        }
    }

    fn reflect_literal_assertion(
        &mut self,
        literal: Literal,
        state: &impl AssertionState,
        receiver: &mut impl ImplicationReceiver<CardinalityPropagatorExplainKey>,
    ) {
        // eprintln!("{}", self.columns[!literal].watchers.len());
        // eprintln!("ASSIGNED {}", literal);
        'for_watchers: for k in (0..self.columns[!literal].watchers.len()).rev() {
            let Watcher { row_id, position } = self.columns[!literal].watchers[k];
            let row = self.rows.get_mut(row_id).unwrap();
            debug_assert!(position < row.number_of_watching_literals());
            debug_assert!(row.literals[position] == !literal);
            for p in row.number_of_watching_literals()..row.literals.len() {
                let l = row.literals[p];
                if !state.literal_state(l).is_false() {
                    row.literals.swap(position, p);
                    self.columns[!literal].watchers.swap_remove(k);
                    self.columns[l].watchers.push(Watcher { row_id, position });
                    // eprintln!("SWAP WATHING LITERALS {} {}", !literal, l);
                    // eprintln!("{}", row.dump(usize::MAX, state));
                    continue 'for_watchers;
                }
            }
            row.activity += 1.0;
            // eprintln!("{}", row.dump(usize::MAX, state));
            for &l in row.literals[..row.number_of_watching_literals()].iter() {
                debug_assert!(l == !literal || !state.literal_state(l).is_false());
                if !state.literal_state(l).is_assigned() {
                    // eprintln!("PROPAGATE {}", l);
                    receiver.receive_literal_assertion(
                        l,
                        CardinalityPropagatorExplainKey { row_id },
                        0.0,
                    );
                }
            }
        }
    }
}

impl<ValueT> Propagator for CardinalityPropagator<ValueT>
where
    ValueT: Integer,
{
    type ExplainKey = CardinalityPropagatorExplainKey;
    type ExplanationConstraint<'a>
        = impl Constraint<Value = ValueT>
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
        debug_assert!(self.unrefrected_order <= state.number_of_assertions());
        debug_assert!(self.unrefrected_order + 1 >= state.number_of_assertions(),);

        while self.unrefrected_order < state.number_of_assertions() {
            let assetion = state.assertion(self.unrefrected_order);
            match assetion {
                Predicate::Literal(literal) => {
                    self.reflect_literal_assertion(literal, state, receiver);
                }
                _ => {}
            }
            self.unrefrected_order += 1;
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
        debug_assert!(self.unrefrected_order >= backjump_order);
        self.unrefrected_order = backjump_order;
    }

    fn receive_involved_constraints(
        &mut self,
        involved_constraint: impl Iterator<Item = Self::ExplainKey> + Clone,
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
        let min_number_of_retaining_learnt_constraints =
            max(10000, number_of_static_constraint / 10) + 10 * self.reduction_count;

        if number_of_learnt_constraints
            < number_of_removing_constraints + min_number_of_retaining_learnt_constraints
        {
            return;
        }

        let mut removing_row_ids = FxHashSet::default();
        let mut candidate_row_ids = Vec::default();
        for (row_id, row) in self.rows.iter() {
            if row.inf_literal_terms(state) >= ValueT::from_usize(row.lower_bound).unwrap() {
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
                    Reverse(std::cmp::max(
                        self.reduction_count - row.last_involved_timestamp,
                        3,
                    )),
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
                .watchers
                .retain(|watcher| !removing_row_ids.contains(&watcher.row_id));
        }
        // activity を減衰
        for (_, row) in self.rows.iter_mut() {
            row.activity *= 0.9;
        }
        self.reduction_count += 1;
    }
}

impl<ConstraintT> PropagatorAddConstraint<ConstraintT> for CardinalityPropagator<ConstraintT::Value>
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
        assert!(self.unrefrected_order == state.number_of_assertions());

        assert!(
            constraint
                .iter_terms()
                .all(|(_, c)| c == <ConstraintT as Constraint>::Value::one())
        );

        // 常に充足される制約は無視
        if constraint.lower_bound() <= ConstraintT::Value::zero() {
            return;
        }

        let mut literals = Vec::from_iter(constraint.iter_terms().map(|(l, _)| l));
        literals.sort_unstable_by_key(|&l| {
            let literal_state = state.literal_state(l);
            if literal_state.is_true() {
                (0, 0)
            } else if literal_state.is_false() {
                (2, usize::MAX - literal_state.order().unwrap())
            } else {
                (1, 0)
            }
        });

        // 行を追加
        let row_id = self.rows.allocate(Row::new(
            literals,
            constraint.lower_bound().to_usize().unwrap(),
            is_learnt,
            self.reduction_count,
        ));
        let row = self.rows.get_mut(row_id).unwrap();

        // TODO: これ以降は要リファクタリング

        if row.literals.len() < row.lower_bound {
            receiver.receive_conflict(CardinalityPropagatorExplainKey { row_id });
        } else if row.literals.len() == row.lower_bound {
            debug_assert!(state.decision_level() == 0);
            for &literal in row.literals.iter() {
                if state.literal_state(literal).is_false() {
                    receiver.receive_conflict(CardinalityPropagatorExplainKey { row_id });
                } else if !state.literal_state(literal).is_assigned() {
                    receiver.receive_literal_assertion(
                        literal,
                        CardinalityPropagatorExplainKey { row_id },
                        0.0,
                    );
                }
            }
        } else {
            debug_assert!(row.number_of_watching_literals() >= 2);

            // eprintln!("{}", row.dump(usize::MAX, state));
            // eprintln!("number_of_watching_literals={}", row.number_of_watching_literals());
            // 列方向の項を追加
            for (position, &literal) in row.literals[0..row.number_of_watching_literals()]
                .iter()
                .enumerate()
            {
                self.columns[literal]
                    .watchers
                    .push(Watcher { row_id, position });
            }

            if state
                .literal_state(row.literals[row.number_of_watching_literals() - 1])
                .is_false()
            {
                if row.literals.len() == 1
                    || state
                        .literal_state(row.literals[row.number_of_watching_literals() - 2])
                        .is_false()
                {
                    // 単項の節または監視リテラルのうち 2 つ以上に false が割当たっていれば矛盾
                    receiver.receive_conflict(CardinalityPropagatorExplainKey { row_id });
                }

                // 監視リテラルのうち 1 つに false が割当たっていれば伝播
                for &literal in row.literals[0..(row.number_of_watching_literals() - 1)].iter() {
                    if !state.literal_state(literal).is_assigned() {
                        receiver.receive_literal_assertion(
                            literal,
                            CardinalityPropagatorExplainKey { row_id },
                            0.0,
                        );
                    }
                }
            }
        }
    }
}
