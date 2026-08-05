// Тут прикладные типы и классы алгоритмов
mod conf;
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};

pub use conf::*;
mod context;
pub use context::*;
mod angular_grid;
pub use angular_grid::*;
mod autocorrelation;
pub use autocorrelation::*;
mod read_inputs;
pub use read_inputs::*;
mod imbalance;
pub use imbalance::*;
mod frame;
pub use frame::*;
mod pass;
pub use pass::*;
mod order_spectrum;
pub use order_spectrum::*;
mod window_fn;
use sal_sync::services::EventValueAccess;
pub use window_fn::*;
mod types;
pub use types::*;
mod retain;
pub use retain::*;
mod sql_export;
pub use sql_export::*;
mod vibro_sensor;
pub use vibro_sensor::*;
mod mid_range;
pub use mid_range::*;
mod high_range;
pub use high_range::*;

pub struct MockEventValues {
    rpm: AtomicU64,
}
impl MockEventValues {
    pub fn new() -> Self {
        Self { rpm: AtomicU64::new(f64::NAN.to_bits()) }
    }
    pub fn set_rpm(&self, val: f64) {
        self.rpm.store(val.to_bits(), Ordering::Relaxed);
    }
    pub fn rpm(&self) -> Option<f64> {
        let val = f64::from_bits(self.rpm.load(Ordering::Relaxed));
        if val.is_finite() {
            Some(val)
        } else {
            None
        }
    }
}
impl EventValueAccess<str, f64> for MockEventValues {    
    //
    fn register(&mut self, key: &str) {
        if key != "rpm" {
            panic!("MockEventValues.register | Unknown key '{key}'")
        }
    }
    //
    fn get(&self, key: &str) -> Option<f64> {
        if key != "rpm" {
            panic!("MockEventValues.get | Unknown key '{key}'")
        }
        self.rpm()
    }
    //
    fn insert(&self, key: &str, val: f64) {
        if key != "rpm" {
            panic!("MockEventValues.insert | Unknown key '{key}'")
        }
        self.rpm.store(val.to_bits(), Ordering::Relaxed);
    }
}
