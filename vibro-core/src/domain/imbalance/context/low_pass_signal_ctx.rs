use crate::Rpm;

/// Состояние биквадратного фильтра (история отсчетов).
#[derive(Copy, Clone, Default)]
pub struct BiquadState {
    pub x1: f64, pub x2: f64,
    pub y1: f64, pub y2: f64,
}
/// Коэффициенты биквадратного фильтра 2-го порядка.
#[derive(Copy, Clone, Default)]
pub struct BiquadCoeffs {
    pub b0: f64, pub b1: f64, pub b2: f64,
    pub a1: f64, pub a2: f64,
}
/// Контекст для `LowPassSignal`
pub struct LowPassSignalCtx {
    /// Текущие рассчитанные коэффициенты фильтра.
    pub coeffs: BiquadCoeffs,
    /// Последняя известная частота вала (для перерасчета коэффициентов).
    pub last_rpm: Rpm<f64>,
    /// Внутреннее состояние фильтра (история предыдущих 2 отсчетов).
    pub state: BiquadState,
}
impl LowPassSignalCtx {
    pub fn new() -> Self {
        Self {
            coeffs: BiquadCoeffs::default(),
            last_rpm: Rpm(0.0),
            state: BiquadState::default(),
        }
    }
}