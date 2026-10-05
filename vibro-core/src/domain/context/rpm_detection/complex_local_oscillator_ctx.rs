use rustfft::num_complex::Complex;
///
/// Результат работы комплексного гетеродина (снос 1X на нулевую частоту)
pub struct ComplexLocalOscillatorCtx {
    // Комплексный сигнал y[n]=I[n]+jQ[n], в котором компонента 1X находится на частоте δ≈0.
    pub complex_signal: Vec<Complex<f64>>,
    // Массив накопленных фаз гетеродина за последний чанк
    pub phi_net: Vec<f64>,
    // Значение последней накопленной фазы гетеродина в последнем чанке
    pub phi_last: f64,
}
impl Default for ComplexLocalOscillatorCtx {
    fn default() -> Self {
        Self {
            complex_signal: Default::default(),
            phi_net: Default::default(),
            phi_last: Default::default()
        }
    }
}