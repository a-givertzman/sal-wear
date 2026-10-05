use rustfft::num_complex::Complex;
use crate::Biquad;
///
/// Результат работы фиксированного ФНЧ полосы слежения (2 биквада, I/Q)
pub struct LPFTrackingBandCtx {
    pub stage1: Biquad, // каскадный биквад 
    pub stage2: Biquad, // каскадный биквад
    // Отфильтрованная комплексная огибающая I[n]+jQ[n] компоненты 1X
    pub filtered_signal: Vec<Complex<f64>>,
}
impl Default for LPFTrackingBandCtx {
    fn default() -> Self {
        Self {
            stage1: Default::default(),
            stage2: Default::default(),
            filtered_signal: Default::default(),
        }
    }
}