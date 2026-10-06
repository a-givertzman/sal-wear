use crate::{ComplexLocalOscillatorCtx, DecimationCtx};
///
/// Контейнер для передачи данных между вычислительными шагами
pub struct RpmDetectionCtx {
    /// Результат работы адаптивного децимирующего фильтра (Anti-Aliasing)
    pub(crate) decimation: DecimationCtx,
    /// Результат работы комплексного гетеродина (снос 1X на нулевую частоту)
    pub(crate) complex_local_oscillator: ComplexLocalOscillatorCtx,
}
impl RpmDetectionCtx {
    /// - `f_sample` - Частота дискретизации АЦП.
    pub fn new() -> Self {
        Self {
            decimation: Default::default(),
            complex_local_oscillator: Default::default(),
        }
    }
}
//
impl Default for RpmDetectionCtx {
    fn default() -> Self {
        Self {
            decimation: Default::default(),
            complex_local_oscillator: Default::default(),
        }
    }
}
