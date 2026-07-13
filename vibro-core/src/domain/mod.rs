// Тут прикладные типы и классы алгоритмов
mod conf;
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};

pub use conf::*;
mod context;
pub use context::*;
mod angular_grid;
pub(crate) use angular_grid::*;
mod autocorrelation;
pub(crate) use autocorrelation::*;
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



pub struct Inputs {
    rpm: AtomicU64,
}
impl Inputs {
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