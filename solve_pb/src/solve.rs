use ppb_cdcl2::{AssertionState as PpbAssetionState, Literal, LiteralState as PpbLiteralState};
// use rand::{RngExt, SeedableRng};

use crate::{parse_opb, plbd_average::PlbdScoreAverage, plbd_normalizer::PlbdNormalizer};

pub fn solve(objective: &Option<parse_opb::Objective>, constraints: &Vec<parse_opb::Constraint>) {
    assert!(objective.is_none());
    // let start_time = std::time::Instant::now();

    // let mut rng = rand::rngs::SmallRng::seed_from_u64(42);

    type Engine =
        ppb_cdcl2::Engine<ppb_cdcl2::CompositeLinearPropagator<u64>, (), ppb_cdcl2::VsidsPricer>;
    type ExplainKey = <Engine as ppb_cdcl2::AssertionState>::ExplainKey;

    let mut ppb_engine: Engine = ppb_cdcl2::Engine::new(
        ppb_cdcl2::CompositeLinearPropagator::new(),
        ppb_cdcl2::VsidsPricer::new(2.5, 0.0, false),
    );

    let analyze = ppb_cdcl2::Analyze::new();

    let calculate_plbd = ppb_cdcl2::CalculatePLBD::default();
    let mut plbd_normalizer = PlbdNormalizer::default();
    let mut plbd_average = PlbdScoreAverage::new(10);

    // NOTE: 変数名が 1 始まりで抜けがないことを前提にしている
    // 実験コードとしては問題ないが，移植に際しては要注意
    {
        let mut frequency = Vec::default();
        for constraint in constraints.iter() {
            for weighted_term in constraint.sum.iter() {
                if weighted_term.term.index > frequency.len() {
                    frequency.resize(weighted_term.term.index, (0, 0));
                }
                let f = if matches!(
                    constraint.relational_operator,
                    parse_opb::RelationalOperator::Equal
                ) {
                    (1, 1)
                } else if weighted_term.weight > 0 {
                    (0, 1)
                } else {
                    (1, 0)
                };
                frequency[weighted_term.term.index - 1].0 += f.0;
                frequency[weighted_term.term.index - 1].1 += f.1;
            }
        }
        for &(false_frequency, true_frequency) in frequency.iter() {
            let initial_value = false_frequency < true_frequency;
            let initial_activity =
                ((false_frequency + true_frequency) as f64) / (constraints.len() as f64 + 1.0);
            ppb_engine.add_variable(initial_value, initial_activity);
        }
    }

    // 制約を追加
    for constraint in constraints.iter() {
        if let Some(ppb_constraint) = make_constraint(
            constraint
                .sum
                .iter()
                .map(|term| (term.term.index - 1, term.weight)),
            constraint.rhs,
        ) {
            ppb_engine.add_constraint(ppb_constraint, false);
            if ppb_engine.status().is_conflict() {
                println!("s UNSATISFIABLE");
                return;
            }
        }
        if constraint.relational_operator == parse_opb::RelationalOperator::Equal {
            if let Some(ppb_constraint) = make_constraint(
                constraint
                    .sum
                    .iter()
                    .map(|term| (term.term.index - 1, -term.weight)),
                -constraint.rhs,
            ) {
                ppb_engine.add_constraint(ppb_constraint, false);
                if ppb_engine.status().is_conflict() {
                    println!("s UNSATISFIABLE");
                    return;
                }
            }
        }
    }

    // let mut conflict_count = 0usize;
    // let mut restart_count = 0usize;
    let mut conflict_count_after_restarting = 0usize;

    // eprintln!(
    //     "CONFLICT_COUNT\tRESTART_COUNT\tDECISION_LEVEL\tNUMBER_OF_ASSERTIONS\tPLBD\tLEARNT_CONSTRAINT_SIZE\tLEARNT_CONSTRAINT_MAX_COEFFICIENT\tLEARNT_CONSTRAINT_MIN_COEFFICIENT\tLEARNT_CONSTRAINT_SLACK"
    // );

    let mut best_solution = Vec::default();
    loop {
        // if start_time.elapsed().as_secs_f64() > 60.0 {
        //     println!("s UNKNOWN");
        //     return;
        // }

        match ppb_engine.status() {
            ppb_cdcl2::Status::Conflict(conflict_status) => {
                let conflict_level = ppb_engine.decision_level();
                if conflict_level == 0 {
                    // eprintln!("UNSATSFIABLE (CONFLICT AT DECISION LEVEL 0)");
                    println!("s UNSATISFIABLE");
                    return;
                }

                // conflict_count += 1;
                conflict_count_after_restarting += 1;

                let analyzer_output = {
                    let explain = |composite_explain_key| {
                        let ExplainKey::Propagator(propagator_explain_key) = composite_explain_key
                        else {
                            unreachable!()
                        };
                        ppb_engine.explain(propagator_explain_key)
                    };

                    match conflict_status {
                        ppb_cdcl2::ConflictStatus::Constraint { explain_key } => {
                            // eprintln!("CONFLICT CONSTRAINT");
                            analyze.analyze_conflict_constraint(
                                ExplainKey::Propagator(explain_key),
                                &ppb_engine,
                                explain,
                            )
                        }
                        ppb_cdcl2::ConflictStatus::Literals {
                            variable,
                            explain_keys,
                        } => {
                            // eprintln!("CONFLICT IMPLICATIONS");
                            analyze.analyze_conflict_implications(
                                variable,
                                explain_keys.map(|k| ExplainKey::Propagator(k)),
                                &ppb_engine,
                                explain,
                            )
                        }
                    }
                };
                match analyzer_output {
                    ppb_cdcl2::PpbAnalyzeOutput::Unsatisfiable => {
                        // eprintln!("UNSATSFIABLE ANALYZER OUTPUT");
                        println!("s UNSATISFIABLE");
                        return;
                    }
                    ppb_cdcl2::PpbAnalyzeOutput::Learnt {
                        backjump_level,
                        learnt_constraint,
                        shadow_price,
                        involved_constraints,
                    } => {
                        let plbd = calculate_plbd.calculate(
                            &learnt_constraint,
                            &ppb_engine,
                            backjump_level,
                        );

                        let plbd_score = plbd_normalizer.observe(plbd);
                        plbd_average.add_score(plbd_score);

                        ppb_engine
                            .update_activity(&shadow_price, involved_constraints.iter().cloned());
                        ppb_engine.backjump(backjump_level);

                        // eprintln!(
                        //     "{}\t{}\t{}\t{}\t{}\t{}",
                        //     conflict_count,
                        //     restart_count,
                        //     conflict_level,
                        //     ppb_engine.number_of_assertions(),
                        //     plbd,
                        //     backjump_level,
                        // );

                        ppb_engine.add_constraint(&learnt_constraint, true);
                    }
                }
            }
            ppb_cdcl2::Status::Noconflict => {
                if ppb_engine.number_of_assigned_variables() == ppb_engine.number_of_variables() {
                    // 実行可能解が得られた場合
                    #[cfg(debug_assertions)]
                    for constraint in constraints.iter() {
                        let mut lhs = 0;
                        for wt in constraint.sum.iter() {
                            if ppb_engine
                                .literal_state(
                                    ppb_cdcl2::Literal::new(wt.term.index - 1, true).into(),
                                )
                                .is_true()
                            {
                                lhs += wt.weight;
                            }
                        }
                        assert!(lhs >= constraint.rhs);
                        if matches!(
                            constraint.relational_operator,
                            parse_opb::RelationalOperator::Equal
                        ) {
                            assert!(lhs <= constraint.rhs);
                        }
                    }
                    store_solution(&mut best_solution, &ppb_engine);
                    println!("s SATISFIABLE");
                    put_solution(&best_solution);
                    return;
                }

                if ppb_engine.decision_level() != 0
                    && plbd_average.short_term_average()
                        - 2.0 / (conflict_count_after_restarting as f64 + 1e-8).sqrt()
                        > 0.00
                {
                    // restart_count += 1;
                    conflict_count_after_restarting = 0;
                    plbd_average.reset();
                    ppb_engine.backjump(0);
                } else {
                    let decision = ppb_cdcl2::Predicate::Literal(
                        ppb_engine.top_priority_unassigned_literal().unwrap(),
                    );
                    // eprintln!("DECISION {}", decision);
                    ppb_engine.decision(decision);
                }
            }
        }
    }
}

fn make_constraint(
    terms: impl Iterator<Item = (usize, i64)> + Clone,
    rhs: i64,
) -> Option<impl ppb_cdcl2::Constraint<Value = u64>> {
    let lower = {
        let sum_of_negative_coefficients: i128 = terms
            .clone()
            .filter(|&(_, coefficient)| coefficient < 0)
            .map(|(_, coefficient)| coefficient as i128)
            .sum();

        let lower_i128 = rhs as i128 - sum_of_negative_coefficients;
        if lower_i128 <= 0 {
            return None;
        }
        assert!(lower_i128 < u64::MAX as i128);
        lower_i128 as u64
    };

    Some(ppb_cdcl2::ConstraintView::new(
        terms.filter_map(move |(index, coefficient)| {
            if coefficient == 0 {
                None
            } else if coefficient > 0 {
                Some((
                    ppb_cdcl2::Literal::new(index, true).into(),
                    std::cmp::min(coefficient as u64, lower),
                ))
            } else {
                Some((
                    ppb_cdcl2::Literal::new(index, false).into(),
                    std::cmp::min(-coefficient as u64, lower),
                ))
            }
        }),
        lower,
    ))
}

fn store_solution(solution: &mut Vec<bool>, assertion_state: &impl PpbAssetionState) {
    solution.clear();
    for variable in 0..assertion_state.number_of_variables() {
        let literal_state = assertion_state.literal_state(Literal::new(variable, true));
        debug_assert!(literal_state.is_assigned());
        solution.push(literal_state.is_true());
    }
}

fn put_solution(solution: &Vec<bool>) {
    print!("v");
    for (variable, &value) in solution.iter().enumerate() {
        if value == true {
            print!(" x{}", variable + 1);
        } else {
            print!(" -x{}", variable + 1);
        }
    }
    println!("");
}
