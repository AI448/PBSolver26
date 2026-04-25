mod rescale;
mod resolve;
mod round;

use fxhash::FxHashSet;
use index_collections::Map;
use rescale::Rescale;
use resolve::Resolve;
use std::{
    cell::{Ref, RefCell},
    hash::Hash,
};

use crate::{
    AssertionState, CalculatePropagationLevel, CalculatePropagationLevelOutput,
    CompressedConstraint, Constraint, Literal, LiteralState, Reason,
};

pub enum PpbAnalyzeOutput<'a, ExplainKeyT>
where
    ExplainKeyT: Copy + Eq + Hash,
{
    Unsatisfiable,
    Learnt {
        /// バックジャンプすべき決定レベル
        backjump_level: usize,
        /// 学習制約
        learnt_constraint: Ref<'a, CompressedConstraint<u64>>,
        /// 割り当ての矛盾への寄与
        shadow_price: Ref<'a, Map<f64>>,
        involved_constraints: FxHashSet<ExplainKeyT>,
    },
}

#[derive(Clone)]
pub struct Analyze {
    calculate_propagation_level: CalculatePropagationLevel<u64>,
    resolve: Resolve,
    rescale: Rescale,
    conflict_constraint: RefCell<CompressedConstraint<u64>>,
    reason_constraint: RefCell<CompressedConstraint<u64>>,
    shadow_price: RefCell<Map<f64>>,
}

enum Status {
    Unsatisfiable,
    Learnt { backjump_level: usize },
}

impl Analyze {
    #[inline(never)]
    pub fn new() -> Self {
        Self {
            calculate_propagation_level: CalculatePropagationLevel::default(),
            resolve: Resolve::default(),
            rescale: Rescale::default(),
            conflict_constraint: RefCell::default(),
            reason_constraint: RefCell::default(),
            shadow_price: RefCell::default(),
        }
    }

    #[inline(never)]
    pub fn analyze_conflict_constraint<StateT, ExplainT, ExplanationConstraintT>(
        &self,
        conflict_explain_key: StateT::ExplainKey,
        state: &StateT,
        explain: ExplainT,
    ) -> PpbAnalyzeOutput<'_, StateT::ExplainKey>
    where
        StateT: AssertionState,
        ExplainT: Fn(StateT::ExplainKey) -> ExplanationConstraintT,
        ExplanationConstraintT: Constraint<Value = u64>,
    {
        let mut involved_constraints = FxHashSet::default();
        let status = self._analyze_conflict_constraint(
            &mut self.conflict_constraint.borrow_mut(),
            &mut self.reason_constraint.borrow_mut(),
            &mut self.shadow_price.borrow_mut(),
            &mut involved_constraints,
            conflict_explain_key,
            state,
            explain,
        );
        match status {
            Status::Unsatisfiable => PpbAnalyzeOutput::Unsatisfiable,
            Status::Learnt { backjump_level } => PpbAnalyzeOutput::Learnt {
                backjump_level,
                learnt_constraint: self.conflict_constraint.borrow(),
                shadow_price: self.shadow_price.borrow(),
                involved_constraints,
            },
        }
    }

    #[inline(never)]
    pub fn analyze_conflict_implications<StateT, ExplainT, ExplanationConstraintT>(
        &self,
        variable: usize,
        explain_keys: [StateT::ExplainKey; 2],
        state: &StateT,
        explain: ExplainT,
    ) -> PpbAnalyzeOutput<'_, StateT::ExplainKey>
    where
        StateT: AssertionState,
        ExplainT: Fn(StateT::ExplainKey) -> ExplanationConstraintT,
        ExplanationConstraintT: Constraint<Value = u64>,
    {
        let mut involved_constraints = FxHashSet::default();
        let status = self._analyze_conflict_implications(
            &mut self.conflict_constraint.borrow_mut(),
            &mut self.reason_constraint.borrow_mut(),
            &mut self.shadow_price.borrow_mut(),
            &mut involved_constraints,
            variable,
            explain_keys,
            state,
            explain,
        );
        match status {
            Status::Unsatisfiable => PpbAnalyzeOutput::Unsatisfiable,
            Status::Learnt { backjump_level } => PpbAnalyzeOutput::Learnt {
                backjump_level,
                learnt_constraint: self.conflict_constraint.borrow(),
                shadow_price: self.shadow_price.borrow(),
                involved_constraints,
            },
        }
    }

    #[inline(always)]
    fn _analyze_conflict_implications<StateT, ExplainT, ExplanationConstraintT>(
        &self,
        conflict_constraint: &mut CompressedConstraint<u64>,
        reason_constraint: &mut CompressedConstraint<u64>,
        shadow_price: &mut Map<f64>,
        involved_constraints: &mut FxHashSet<StateT::ExplainKey>,
        variable: usize,
        explain_keys: [StateT::ExplainKey; 2],
        state: &StateT,
        explain: ExplainT,
    ) -> Status
    where
        StateT: AssertionState,
        ExplainT: Fn(StateT::ExplainKey) -> ExplanationConstraintT,
        ExplanationConstraintT: Constraint<Value = u64>,
    {
        conflict_constraint.assign(explain(explain_keys[0]));
        conflict_constraint.drop_fixed_variables2(state);
        conflict_constraint.strengthen2(state);
        let conflict_coefficient = conflict_constraint
            .get_coefficient(Literal::new(variable, false))
            .unwrap();
        reason_constraint.assign(explain(explain_keys[1]));
        reason_constraint.drop_fixed_variables2(state);
        reason_constraint.strengthen2(state);
        let reason_coefficient = reason_constraint
            .get_coefficient(Literal::new(variable, true))
            .unwrap();

        shadow_price.clear();
        shadow_price.insert(variable, 1.0);
        for (literal, coefficient) in conflict_constraint.iter_terms() {
            if state.literal_state(literal).is_false() {
                shadow_price.insert(
                    literal.index(),
                    coefficient as f64 / conflict_coefficient as f64,
                );
            }
        }
        involved_constraints.insert(explain_keys[0]);
        involved_constraints.insert(explain_keys[1]);
        for (literal, coefficient) in reason_constraint.iter_terms() {
            if state.literal_state(literal).is_false() {
                if let Some(p) = shadow_price.get_mut(literal.index()) {
                    *p += coefficient as f64 / reason_coefficient as f64;
                } else {
                    shadow_price.insert(
                        literal.index(),
                        coefficient as f64 / reason_coefficient as f64,
                    );
                }
            }
        }

        let resolved_constraint =
            self.resolve
                .resolve(&conflict_constraint, &reason_constraint, variable, state);
        debug_assert!(
            resolved_constraint.sup_literal_terms(state) < resolved_constraint.lower_bound()
        );
        let resolved_constraint = resolved_constraint.into_strengthen(state);
        debug_assert!(
            resolved_constraint.sup_literal_terms(state) < resolved_constraint.lower_bound()
        );
        let rescaled_constraint = self.rescale.rescale(resolved_constraint, usize::MAX, state);
        let rescaled_constraint = rescaled_constraint.into_strengthen(state);
        conflict_constraint.assign(rescaled_constraint);
        self._analyze(
            conflict_constraint,
            reason_constraint,
            shadow_price,
            involved_constraints,
            state,
            explain,
        )
    }

    #[inline(always)]
    fn _analyze_conflict_constraint<StateT, ExplainT, ExplanationConstraintT>(
        &self,
        conflict_constraint: &mut CompressedConstraint<u64>,
        reason_constraint: &mut CompressedConstraint<u64>,
        shadow_price: &mut Map<f64>,
        involved_constraints: &mut FxHashSet<StateT::ExplainKey>,
        conflict_explain_key: StateT::ExplainKey,
        state: &StateT,
        explain: ExplainT,
    ) -> Status
    where
        StateT: AssertionState,
        ExplainT: Fn(StateT::ExplainKey) -> ExplanationConstraintT,
        ExplanationConstraintT: Constraint<Value = u64>,
    {
        conflict_constraint.assign(explain(conflict_explain_key));
        conflict_constraint.drop_fixed_variables2(state);
        conflict_constraint.strengthen2(state);
        let violation =
            conflict_constraint.lower_bound() - conflict_constraint.sup_literal_terms(state);

        shadow_price.clear();
        for (literal, coefficient) in conflict_constraint.iter_terms() {
            if state.literal_state(literal).is_false() {
                shadow_price.insert(literal.index(), coefficient as f64 / violation as f64);
            }
        }
        involved_constraints.insert(conflict_explain_key);
        self._analyze(
            conflict_constraint,
            reason_constraint,
            shadow_price,
            involved_constraints,
            state,
            explain,
        )
    }

    #[inline(always)]
    fn _analyze<StateT, ExplainT, ExplanationConstraintT>(
        &self,
        conflict_constraint: &mut CompressedConstraint<u64>,
        reason_constraint: &mut CompressedConstraint<u64>,
        shadow_price: &mut Map<f64>,
        involved_constraints: &mut FxHashSet<StateT::ExplainKey>,
        state: &StateT,
        explain: ExplainT,
    ) -> Status
    where
        StateT: AssertionState,
        ExplainT: Fn(StateT::ExplainKey) -> ExplanationConstraintT,
        ExplanationConstraintT: Constraint<Value = u64>,
    {
        // conflict_assertions.clear();
        let mut conflict_order = usize::MAX;
        loop {
            // eprintln!("conflict_constraint = {}", conflict_constraint.dump(conflict_order, state));
            // eprintln!("sup_literal_terms0={}", conflict_constraint.sup_literal_terms_before(state.order_range(0).end, state));
            // eprintln!("inf_lower_bound0={}, parameter_upper_bound0={}", conflict_constraint.inf_rhs_before(state.order_range(0).end, state), state.parameter_upper_bound_before(state.order_range(0).end));
            assert!(
                conflict_constraint.sup_literal_terms_before(conflict_order, state)
                    < conflict_constraint.lower_bound()
            );

            // バックジャンプ可能かを確認
            match self
                .calculate_propagation_level
                .calculate(&conflict_constraint, state, false)
            {
                CalculatePropagationLevelOutput::Propagate { .. } => {
                    // バックジャンプ後の伝播が可能な学習制約が得られた
                    // バックジャンプすべき決定レベルを算出
                    let backjump_level = match self.calculate_propagation_level.calculate(
                        &conflict_constraint,
                        state,
                        true,
                    ) {
                        CalculatePropagationLevelOutput::Propagate { decision_level } => {
                            decision_level
                        }
                        CalculatePropagationLevelOutput::Conflict { decision_level } => {
                            decision_level
                        }
                        CalculatePropagationLevelOutput::None => unreachable!(),
                    };
                    return Status::Learnt { backjump_level };
                }
                CalculatePropagationLevelOutput::Conflict { decision_level } => {
                    if decision_level == 0 {
                        // 決定レベル 0 で矛盾する制約が得られた場合には充足不可能
                        return Status::Unsatisfiable;
                    }
                }
                CalculatePropagationLevelOutput::None => unreachable!(),
            }

            let (target_literal, reason_explain_key, next_conflict_order) = {
                let mut target_literal = None;
                let mut reason_explain_key = None;
                let mut next_conflict_order = None;
                for (literal, _) in conflict_constraint.iter_terms() {
                    let literal_state = state.literal_state(literal);
                    if literal_state.is_false_before(conflict_order) {
                        let r = unsafe {
                            debug_assert!(literal_state.reason().is_some());
                            literal_state.reason().unwrap_unchecked()
                        };
                        if let Reason::Implication { explain_key } = r {
                            let order = unsafe {
                                debug_assert!(literal_state.reason().is_some());
                                literal_state.order().unwrap_unchecked()
                            };
                            if next_conflict_order.is_none_or(|o| order > o) {
                                target_literal.replace(!literal);
                                reason_explain_key.replace(explain_key);
                                next_conflict_order.replace(order);
                            }
                        }
                    }
                }
                (
                    target_literal.unwrap(),
                    reason_explain_key.unwrap(),
                    next_conflict_order.unwrap(),
                )
            };
            conflict_order = next_conflict_order;

            // target_literal に true を割り当てる原因となった制約を取得
            let reason_constraint_ = explain(reason_explain_key);
            // eprintln!("reason_constraint = {}", reason_constraint.dump(conflict_order, state));
            // eprintln!("REASON CONSTRAINT");
            // eprintln!("{}", reason_constraint.dump(conflict_order, state));

            reason_constraint.assign(reason_constraint_);
            reason_constraint.drop_fixed_variables2(state);
            reason_constraint.strengthen2(state);
            let reason_coefficient = reason_constraint.get_coefficient(target_literal).unwrap();
            let &target_shadow_price = shadow_price.get(target_literal.index()).unwrap();

            // 矛盾に関わったリテラルを記録
            for (literal, coefficient) in reason_constraint.iter_terms() {
                if state.literal_state(literal).is_false() {
                    if let Some(p) = shadow_price.get_mut(literal.index()) {
                        *p += coefficient as f64 / reason_coefficient as f64 * target_shadow_price;
                    } else {
                        shadow_price.insert(
                            literal.index(),
                            coefficient as f64 / reason_coefficient as f64 * target_shadow_price,
                        );
                    }
                }
            }
            involved_constraints.insert(reason_explain_key);
            // 融合
            let resolved_constraint = self.resolve.resolve(
                &conflict_constraint,
                &reason_constraint,
                target_literal.index(),
                state,
            );
            debug_assert!(
                resolved_constraint.sup_literal_terms_before(conflict_order, state)
                    < resolved_constraint.lower_bound()
            );

            // スケールを調整
            let rescaled_constraint =
                self.rescale
                    .rescale(resolved_constraint, conflict_order, state);

            // 融合されたものを新たな conflict_constraint に
            conflict_constraint.assign(rescaled_constraint);
        }
    }
}
