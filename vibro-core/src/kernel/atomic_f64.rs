use std::sync::atomic::{AtomicU64, Ordering};

/// Атомарная работа с f64 через AtomicU64 битовым копированием
pub struct AtomicF64 {
    val: AtomicU64,
}
impl AtomicF64 {
    pub fn new(v: f64) -> Self {
        Self {
            val: AtomicU64::new(v.to_bits())
        }
    }
    pub fn load(&self) -> f64 {
        f64::from_bits(self.val.load(Ordering::Relaxed))
    }
    pub fn store(&self, v: f64) {
        self.val.store(v.to_bits(), Ordering::Relaxed);
    }
}