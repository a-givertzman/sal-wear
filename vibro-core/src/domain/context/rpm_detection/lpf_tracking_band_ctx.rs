use rustfft::num_complex::Complex;
use crate::Biquad;
const BAND_HZ: f64 = 7.0;  
const F_CUTOFF: f64 = 3.5;
const Q_FACTOR_1: f64 = 0.5412;
const Q_FACTOR_2: f64 = 1.3066;
///
/// Результат работы фиксированного ФНЧ полосы слежения (2 биквада, I/Q)
pub struct LPFTrackingBandCtx {
    pub stage1: Biquad, // каскадный биквад 
    pub stage2: Biquad, // каскадный биквад
    // Отфильтрованная комплексная огибающая I[n]+jQ[n] компоненты 1X
    pub filtered_signal: Vec<Complex<f64>>,
}
//
impl LPFTrackingBandCtx {
    pub fn new(
        f_sample: f64, 
        signal_capacity: usize
    ) -> Self {
        Self { 
            stage1: Biquad::new(F_CUTOFF, f_sample, Q_FACTOR_1), 
            stage2: Biquad::new(F_CUTOFF, f_sample, Q_FACTOR_2), 
            filtered_signal: Vec::with_capacity(signal_capacity),
        }
    }
}
//
impl Default for LPFTrackingBandCtx {
    fn default() -> Self {
        Self {
            stage1: Default::default(),
            stage2: Default::default(),
            filtered_signal: Default::default(),
        }
    }
}