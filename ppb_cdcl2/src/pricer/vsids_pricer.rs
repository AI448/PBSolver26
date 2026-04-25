use index_collections::{HeapedMap, Map, NaturalComparator, ReverseComparator, ValueComparator};
use ordered_float::NotNan;

use crate::Pricer;

#[derive(Clone)]
pub struct VsidsPricer {
    initial_time_constant: f64,
    time_constant_increase: f64,
    reset_time_constant_at_restart: bool,
    time_constant: f64,
    priorities: Vec<(u64, f64)>,
    unassigned_variables:
        HeapedMap<(u64, NotNan<f64>), ReverseComparator<ValueComparator<NaturalComparator>>>,
    activity_increase: f64,
}

impl VsidsPricer {
    pub fn new(
        initial_time_constant: f64,
        time_constant_increase: f64,
        reset_time_constant_at_restart: bool,
    ) -> Self {
        Self {
            initial_time_constant,
            time_constant_increase,
            reset_time_constant_at_restart,
            time_constant: initial_time_constant,
            priorities: Vec::default(),
            unassigned_variables: HeapedMap::default(),
            activity_increase: 1.0,
        }
    }
}

impl Pricer for VsidsPricer {
    fn add_variable(&mut self, priority: u64, initial_activity: f64) {
        let variable = self.priorities.len();
        self.priorities.push((priority, initial_activity));
        self.unassigned_variables.insert(
            variable,
            (
                self.priorities[variable].0,
                NotNan::try_from(self.priorities[variable].1).unwrap(),
            ),
        );
    }

    fn set_priority(&mut self, variable: usize, priority: u64) {
        todo!()
    }

    #[inline(never)]
    fn update_activity(&mut self, shadow_price: &Map<f64>) {
        self.activity_increase /= 1.0 - 1.0 / self.time_constant;
        let mut max_activity = 0.0;
        for (&variable, &price) in shadow_price.iter() {
            debug_assert!(price >= 0.0);
            self.priorities[variable].1 =
                f64::mul_add(price, self.activity_increase, self.priorities[variable].1);
            max_activity = f64::max(self.priorities[variable].1, max_activity);
            if self.unassigned_variables.contains_key(variable) {
                self.unassigned_variables.insert(
                    variable,
                    (
                        self.priorities[variable].0,
                        self.priorities[variable].1.try_into().unwrap(),
                    ),
                );
            }
        }
        debug_assert!(max_activity < f64::INFINITY);
        self.time_constant += self.time_constant_increase;

        if self.activity_increase > 1e100 {
            for (_, activity) in self.priorities.iter_mut() {
                *activity /= self.activity_increase;
            }
            self.activity_increase = 1.0;
            let unassigned_variables =
                Vec::from_iter(self.unassigned_variables.iter().map(|(&v, _)| v));
            self.unassigned_variables.clear();
            self.unassigned_variables
                .extend(unassigned_variables.into_iter().map(|variable| {
                    (
                        variable,
                        (
                            self.priorities[variable].0,
                            self.priorities[variable].1.try_into().unwrap(),
                        ),
                    )
                }));
        }
    }

    #[inline(always)]
    fn get(&self, variable: usize) -> ((u64, f64), bool) {
        (
            (
                self.priorities[variable].0,
                self.priorities[variable].1.into(),
            ),
            self.unassigned_variables.contains_key(variable),
        )
    }

    #[inline(always)]
    fn peek(&self) -> Option<(usize, (u64, f64))> {
        self.unassigned_variables
            .first()
            .map(|(&variable, &priority)| {
                debug_assert!(priority.0 == self.priorities[variable].0);
                debug_assert!(priority.1 == self.priorities[variable].1);
                (variable, (priority.0, priority.1.into()))
            })
    }

    fn set_to_assigned(&mut self, variable: usize) {
        self.unassigned_variables.remove(variable);
    }

    fn set_to_unassigned(&mut self, variable: usize) {
        self.unassigned_variables.insert(
            variable,
            (
                self.priorities[variable].0,
                self.priorities[variable].1.try_into().unwrap(),
            ),
        );
    }
}
