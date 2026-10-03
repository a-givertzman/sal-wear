use std::f64::consts::PI;
///
/// Вычисление коэффициентов для фильтра Баттерфорта 2-го порядка
pub struct Biquad {
    b0: f64, b1: f64, b2: f64,
    a1: f64, a2: f64,
    w1_i: f64, w2_i: f64,  // состояние для канала I
    w1_q: f64, w2_q: f64,  // состояние для канала Q
}

impl Default for Biquad {
    fn default() -> Self {
        Self { 
            b0: Default::default(), 
            b1: Default::default(), 
            b2: Default::default(), 
            a1: Default::default(), 
            a2: Default::default(), 
            w1_i: Default::default(), 
            w2_i: Default::default(), 
            w1_q: Default::default(), 
            w2_q: Default::default() 
        }
    }
}

impl Biquad {
    pub fn new(f_cutoff: f64, f_sample: f64, q_factor: f64) -> Self {
        let omega0 = 2.0 * PI * f_cutoff / f_sample;
        let alpha = omega0.sin() / (2.0 * q_factor);
        let cos_omega0 = omega0.cos();

        let a0 = 1.0 + alpha;
        let b0 = ((1.0 - cos_omega0) / 2.0) / a0;
        let b1 = (1.0 - cos_omega0) / a0;
        let b2 = b0;
        let a1 = (-2.0 * cos_omega0) / a0;
        let a2 = (1.0 - alpha) / a0;

        Self { b0, b1, b2, a1, a2, w1_i: 0.0, w2_i: 0.0, w1_q: 0.0, w2_q: 0.0 }
    }

    #[inline]
    fn process_one(b0: f64, b1: f64, b2: f64, a1: f64, a2: f64, x: f64, w1: &mut f64, w2: &mut f64) -> f64 {
        let w = x - a1 * (*w1) - a2 * (*w2);
        let y = b0 * w + b1 * (*w1) + b2 * (*w2);
        *w2 = *w1;
        *w1 = w;
        y
    }

    pub fn process(&mut self, i: f64, q: f64) -> (f64, f64) {
        let y_i = Self::process_one(self.b0, self.b1, self.b2, self.a1, self.a2, i, &mut self.w1_i, &mut self.w2_i);
        let y_q = Self::process_one(self.b0, self.b1, self.b2, self.a1, self.a2, q, &mut self.w1_q, &mut self.w2_q);
        (y_i, y_q)
    }
}