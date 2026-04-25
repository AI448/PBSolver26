use index_collections::{HeapedMap, NaturalComparator, ReverseComparator, Set, ValueComparator};
use ordered_float::NotNan;

use crate::{AssertionState, Literal, PricerTrait, utility::LiteralSet};

#[derive(Clone)]
pub struct CachePricer {
    time_constant: f64,
    enable_cache: bool,
    priorities: Vec<(u64, NotNan<f64>)>,
    cache: Set,
    unassigned_variables: HeapedMap<(u64, bool, NotNan<f64>), ReverseComparator<ValueComparator<NaturalComparator>>>,
    increase_value: f64,
}


impl CachePricer {
    pub fn new(time_constant: f64) -> Self {
        Self { time_constant, enable_cache: false, priorities: Vec::default(), cache: Set::default(), unassigned_variables: HeapedMap::default(), increase_value: 1.0 }
    }
}

impl PricerTrait for CachePricer {
    fn add_variable(&mut self, priority: u64) {
        let variable = self.priorities.len();
        self.priorities.push((priority, NotNan::new(0.0).unwrap()));
        self.unassigned_variables.insert(variable, (self.priorities[variable].0, false, self.priorities[variable].1));
    }

    fn set_priority(&mut self, variable: usize, priority: u64) {
        todo!()
    }

    fn update_activity(&mut self, conflict_literals: &LiteralSet, state: &impl AssertionState) {
        self.increase_value /= 1.0 - 1.0 / self.time_constant;
        for literal in conflict_literals.iter() {
            self.priorities[literal.index()].1 += self.increase_value;
            if self.unassigned_variables.contains_key(literal.index()) {
                self.unassigned_variables.insert(
                    literal.index(),
                    (
                        self.priorities[literal.index()].0,
                        self.enable_cache,
                        self.priorities[literal.index()].1,
                    )
                );
            }
        }
        if self.increase_value > 1e10 {
            for (_, activity) in self.priorities.iter_mut() {
                *activity /= self.increase_value;
            }
            self.increase_value = 1.0;
            let unassigned_variables = Vec::from_iter(self.unassigned_variables.iter().map(|(&v, &(_, f, _))| (v, f)));
            self.unassigned_variables.clear();
            self.unassigned_variables.extend(
                unassigned_variables.into_iter().map(
                    |(variable, flag)| (
                        variable, (
                            self.priorities[variable].0,
                            flag,
                            self.priorities[variable].1
                        )
                    )
                )
            );
        }

        if self.time_constant < 1e2 {
            self.time_constant += 1e-3;
        }

        if self.enable_cache {
            for &variable in self.cache.iter() {
                if self.unassigned_variables.contains_key(variable)
                    && !conflict_literals.contains_key(Literal::new(variable, false))
                    && !conflict_literals.contains_key(Literal::new(variable, true)) {
                    debug_assert!(self.unassigned_variables.get(variable).is_some_and(|&(_, f, _)| f));
                    self.unassigned_variables.insert(variable, 
                        (
                            self.priorities[variable].0,
                            false,
                            self.priorities[variable].1
                        )
                    );
                }
            }
            self.cache.clear();
            self.cache.extend(conflict_literals.iter().map(|l|l.index()));
        }
    }

    fn get(&self, variable: usize) -> ((u64, f64), bool) {
        (
            (
                self.priorities[variable].0,
                self.priorities[variable].1.into(),
            ),
            self.unassigned_variables.contains_key(variable),
        )
    }

    fn peek(&self) -> Option<(usize, (u64, f64))> {
        self.unassigned_variables
            .first()
            .map(|(&variable, &priority)| {
                debug_assert!(priority.0 == self.priorities[variable].0);
                debug_assert!(priority.1 == self.cache.contains_key(variable));
                debug_assert!(priority.2 == self.priorities[variable].1);
                (variable, (priority.0, priority.1.into()))
            })

    }

    fn assign(&mut self, variable: usize) {
        self.unassigned_variables.remove(variable);
    }

    fn unassign(&mut self, variable: usize) {
        self.unassigned_variables.insert(
            variable,
            (self.priorities[variable].0, self.cache.contains_key(variable), self.priorities[variable].1),
        );
    }
}