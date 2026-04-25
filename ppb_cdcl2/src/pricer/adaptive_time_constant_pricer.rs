use index_collections::{HeapedMap, NaturalComparator, ReverseComparator, ValueComparator};
use ordered_float::OrderedFloat;

use crate::{AssertionState, PricerTrait, utility::LiteralSet};

const I: usize = 6;

#[derive(Clone)]
pub struct AdaptiveTimeConstantPricer {
    increase: [f64; 15],
    limits: [f64; 15],
    average: [f64; 15],
    priorities: Vec<Priority>,
    unassigned_variables: HeapedMap<(u64, OrderedFloat<f64>), ReverseComparator<ValueComparator<NaturalComparator>>>,
    previous_conflict_literals: LiteralSet,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Priority {
    level: u64,
    activities: [OrderedFloat<f64>; 15]
}

impl Default for Priority {
    fn default() -> Self {
        Self { level: 0, activities: [OrderedFloat(1.0); 15]}
    }
}

impl Priority {
    fn new(level: u64) -> Self {
        Self { level, ..Default::default() }
    }
}

impl AdaptiveTimeConstantPricer {
    pub fn new() -> Self {
        Self {
            increase: [1.0; 15],
            limits: [1.0; 15],
            average: [0.0; 15],
            priorities: Vec::default(),
            unassigned_variables: HeapedMap::default(),
            previous_conflict_literals: LiteralSet::default(),
        }
    }
}

impl PricerTrait for AdaptiveTimeConstantPricer {
    fn add_variable(&mut self, level: u64) {
        let index = self.priorities.len();
        self.priorities.push(Priority::new(level));
        self.unassigned_variables.insert(index, (self.priorities[index].level, self.priorities[index].activities[I]));
    }

    fn update_activity(&mut self, conflict_literals: &LiteralSet, state: &impl AssertionState) {
        let mut n = 0;
        for literal in conflict_literals.iter() {
            if self.previous_conflict_literals.contains_key(literal) {
                n += 1;
            }
        }
        eprintln!("hoge={}", n);
        let mut log_liklifhod = [0.0; 15];
        for i in 0..15 {
            let time_constant = 2f64.powi((i + 1) as i32);
            self.increase[i] /= 1.0 - 1.0 / time_constant;

            for literal in conflict_literals.iter() {
                if self.limits[i] != 0.0 {
                    log_liklifhod[i] += f64::ln(f64::from(self.priorities[literal.index()].activities[i]) / self.limits[i]);
                }
                self.priorities[literal.index()].activities[i] += self.increase[i];
            }
            self.limits[i] += self.increase[i];
            self.average[i] += self.increase[i] * conflict_literals.len() as f64 / self.priorities.len() as f64;

            if self.increase[i] > 1e10 {
                for priority in self.priorities.iter_mut() {
                    priority.activities[i] /= self.increase[i];
                }
                self.limits[i] /= self.increase[i];
                self.average[i] /= self.increase[i];
                self.increase[i] = 1.0;
            }
            if i == I {
                let unassigned_variables = Vec::from_iter(self.unassigned_variables.iter().map(|(&v, _)| v));
                self.unassigned_variables.clear();
                for v in unassigned_variables.into_iter() {
                    self.unassigned_variables.insert(v, (self.priorities[v].level, self.priorities[v].activities[I]));
                }
            }
        }
        eprintln!("n={} {:?}", conflict_literals.len(), log_liklifhod);
        self.previous_conflict_literals.clear();
        self.previous_conflict_literals.extend(conflict_literals.iter());
    }

    fn get(&self, variable: usize) -> ((u64, f64), bool) {
        (
            (self.priorities[variable].level, self.priorities[variable].activities[I].into()),
            self.unassigned_variables.contains_key(variable),
        )
    }

    fn peek(&self) -> Option<(usize, (u64, f64))> {
        self.unassigned_variables
            .first()
            .map(|(&variable, &priority)| {
                (variable, (priority.0, priority.1.into()))
            })

    }

    fn set_priority(&mut self, variable: usize, priority: u64) {
        todo!()        
    }

    fn assign(&mut self, variable: usize) {
        self.unassigned_variables.remove(variable);        
    }

    fn unassign(&mut self, variable: usize) {
        self.unassigned_variables.insert(
            variable,
            (self.priorities[variable].level, self.priorities[variable].activities[I].into()),
        );
    }
}