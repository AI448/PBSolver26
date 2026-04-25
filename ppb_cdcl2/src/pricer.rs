// mod adaprive_vsids_pricer;
mod vsids_pricer;

// pub use adaprive_vsids_pricer::AdapticeVsidsPricer;
use index_collections::Map;
pub use vsids_pricer::VsidsPricer;

pub trait Pricer {
    fn add_variable(&mut self, priority: u64, initial_activity: f64);

    fn update_activity(&mut self, weight: &Map<f64>);

    fn get(&self, variable: usize) -> ((u64, f64), bool);

    fn peek(&self) -> Option<(usize, (u64, f64))>;

    fn set_priority(&mut self, variable: usize, priority: u64);

    fn set_to_assigned(&mut self, variable: usize);

    fn set_to_unassigned(&mut self, variable: usize);
}
