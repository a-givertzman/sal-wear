/// Состояние биквадратного фильтра (история отсчетов).
#[derive(Copy, Clone, Default)]
struct BiquadState {
    x1: f64, x2: f64,
    y1: f64, y2: f64,
}
/// Коэффициенты биквадратного фильтра 2-го порядка.
#[derive(Copy, Clone, Default)]
struct BiquadCoeffs {
    b0: f64, b1: f64, b2: f64,
    a1: f64, a2: f64,
}
pub struct LowPassSignalCtx {
    /// Текущие рассчитанные коэффициенты фильтра.
    coeffs: BiquadCoeffs,
    /// Последняя известная частота вала (для перерасчета коэффициентов).
    last_rpm: f64,
    /// Внутреннее состояние фильтра (история предыдущих 2 отсчетов).
    state: BiquadState,
}
impl LowPassSignalCtx {
    pub fn new() -> Self {
        Self {
            coeffs: BiquadCoeffs::default(),
            last_rpm: 0.0,
            state: BiquadState::default(),
        }
    }
}