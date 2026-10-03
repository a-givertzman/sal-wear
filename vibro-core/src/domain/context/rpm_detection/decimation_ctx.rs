use std::collections::VecDeque;
///
/// Результат работы адаптивного децимирующего фильтра (Anti-Aliasing)
pub struct DecimationCtx {
    // Частота децимации
    pub f_decimation: f64,
    // Прореженный входной сигнал для определения текущей RPM и фазы угла поворота вала
    pub decimated: Vec<f64>,
    /// История сэмплов для КИХ-фильтра (длина 32)
    pub dec_fir_state: VecDeque<f64>,
    /// Счетчик прореживания от 0 до 19
    pub dec_counter: usize,
}
impl Default for DecimationCtx {
    fn default() -> Self {
        Self {
            f_decimation: Default::default(),
            decimated: Default::default(),
            dec_fir_state: Default::default(),
            dec_counter: Default::default(),
        }
    }
}