use rustfft::num_complex::Complex;

///
/// Результат работы комплексного гетеродина (снос 1X на нулевую частоту)
pub struct ComplexLocalOscillatorCtx {
    // Комплексный сигнал y[n]=I[n]+jQ[n], в котором компонента 1X находится на частоте δ≈0.
    pub complex_signal: Vec<Complex<f64>>,
    // Фаза гетеродина
    pub phi_net: f64,
}
impl Default for ComplexLocalOscillatorCtx {
    fn default() -> Self {
        Self {
            complex_signal: Default::default(),
            phi_net: Default::default(),
        }
    }
}