use index_collections::{HeapedMap, Map, NaturalComparator, ReverseComparator, ValueComparator};
use ordered_float::NotNan;

use crate::Pricer;

#[derive(Clone)]
pub struct AdapticeVsidsPricer {
    time_constant: f64,
    activities: Vec<[f64; 3]>,
    unassigned_variables:
        HeapedMap<(u64, NotNan<f64>), ReverseComparator<ValueComparator<NaturalComparator>>>,
    activity_increase: [f64; 3],
}

impl AdapticeVsidsPricer {
    pub fn new(time_constant: f64) -> Self {
        Self {
            time_constant,
            activities: Vec::default(),
            unassigned_variables: HeapedMap::default(),
            activity_increase: [1.0; 3],
        }
    }
}

impl Pricer for AdapticeVsidsPricer {
    fn add_variable(&mut self, priority: u64) {
        let variable = self.activities.len();
        self.activities.push([0.0; 3]);
        self.unassigned_variables.insert(
            variable,
            (0, NotNan::try_from(self.activities[variable][1]).unwrap()),
        );
    }

    fn set_priority(&mut self, variable: usize, priority: u64) {
        todo!()
    }

    #[inline(never)]
    fn update_activity(&mut self, shadow_price: &Map<f64>) {
        self.activity_increase[0] /= 1.0 - 1.0 / (0.5 * self.time_constant);
        self.activity_increase[1] /= 1.0 - 1.0 / self.time_constant;
        self.activity_increase[2] /= 1.0 - 1.0 / (2.0 * self.time_constant);

        let mut diff = [0.0; 3];
        let mut max_price = 0.0;
        let mut max_activity = 0.0;
        for (&variable, &price) in shadow_price.iter() {
            debug_assert!(price >= 0.0);
            max_price = f64::max(max_price, price);
            for i in [0, 1, 2].into_iter() {
                diff[i] += (self.activities[variable][i]
                    / self.activity_increase[i]
                    / ((0.5 + 0.5 * i as f64) * self.time_constant - 1.0)
                    - price)
                    .powi(2);
                self.activities[variable][i] = f64::mul_add(
                    price,
                    self.activity_increase[i],
                    self.activities[variable][i],
                );
                max_activity = f64::max(self.activities[variable][i], max_activity);
            }

            if self.unassigned_variables.contains_key(variable) {
                self.unassigned_variables.insert(
                    variable,
                    (0, self.activities[variable][1].try_into().unwrap()),
                );
            }
        }
        debug_assert!(max_activity < f64::INFINITY);
        eprintln!(
            "max_price={}, {} {} {}",
            max_price,
            diff[0].sqrt(),
            diff[1].sqrt(),
            diff[2].sqrt()
        );

        if max_activity > 1e100 {
            for i in [0, 1, 2].into_iter() {
                for activity in self.activities.iter_mut() {
                    activity[i] /= self.activity_increase[i];
                }
                self.activity_increase[i] = 1.0;
            }
            let unassigned_variables =
                Vec::from_iter(self.unassigned_variables.iter().map(|(&v, _)| v));
            self.unassigned_variables.clear();
            self.unassigned_variables
                .extend(unassigned_variables.into_iter().map(|variable| {
                    (
                        variable,
                        (0, self.activities[variable][1].try_into().unwrap()),
                    )
                }));
        }
    }

    #[inline(always)]
    fn get(&self, variable: usize) -> ((u64, f64), bool) {
        (
            (0, self.activities[variable][1].into()),
            self.unassigned_variables.contains_key(variable),
        )
    }

    #[inline(always)]
    fn peek(&self) -> Option<(usize, (u64, f64))> {
        self.unassigned_variables
            .first()
            .map(|(&variable, &priority)| {
                debug_assert!(priority.0 == 0);
                debug_assert!(priority.1 == self.activities[variable][1]);
                (variable, (priority.0, priority.1.into()))
            })
    }

    fn set_to_assigned(&mut self, variable: usize) {
        self.unassigned_variables.remove(variable);
    }

    fn set_to_unassigned(&mut self, variable: usize) {
        self.unassigned_variables.insert(
            variable,
            (0, self.activities[variable][1].try_into().unwrap()),
        );
    }

    fn restart(&mut self) {}
}
