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
mod test {
    use crate::AtomicF64;
    #[test]
    fn test_nan() {
        let v = AtomicF64::new(f64::NAN);
        assert!(v.load().is_nan(), "Внутреннее значение должно быть NAN")
    }
    #[test]
    fn test_store_load() {
        let test_series = [
            (f64::MIN, f64::MIN),
            (-0.1, -0.1),
            (-f64::EPSILON, -f64::EPSILON),
            (-0.0, 0.0),
            (f64::EPSILON, f64::EPSILON),
            (0.1, 0.1),
            (f64::MAX, f64::MAX),
        ];
        let v = AtomicF64::new(f64::NAN);
        for (val, target) in test_series {
            v.store(val);
            assert!(v.load() == target, "Внутреннее значение должно быть {target}")
        }
    }
}