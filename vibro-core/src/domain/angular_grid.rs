/// Угловая сетка (фазовый профиль) для заданного окна временных отсчетов.
/// Представляет собой массив углов поворота вала, соответствующих каждому отсчету вибрации.
pub struct AngularGrid {
    /// Массив углов в радианах. Размер совпадает с окном входящих данных.
    phases: Vec<f32>,
    /// Последний вычисленный угол для бесшовной передачи состояния.
    /// Хранится в f64 для предотвращения накопления ошибки округления.
    last_angle: f64,
}
impl AngularGrid {
    /// Вычисляет угловую сетку для новой порции данных.
    /// * `samples` - Окно отсчетов для расчета (должно вмещать минимум 2-3 оборота вала).
    /// * `rough_rpm` - Приблизительная частота вращения с тахометра (об/мин).
    /// * `sample_rate` - Частота дискретизации в Гц.
    /// * `initial_angle` - Угол $\theta$ на начало окна.
    pub fn new(samples: &[f32], rough_rpm: f32, sample_rate: f32, initial_angle: f64) -> Self {
        let rough_period = sample_rate * 60.0 / rough_rpm;
        let exact_period = Self::find_exact_period(samples, rough_period);
        let omega = 2.0 * std::f64::consts::PI * (sample_rate as f64) / (exact_period as f64);
        let dt = 1.0 / (sample_rate as f64);
        let mut phases = Vec::with_capacity(samples.len());
        let mut current_angle = initial_angle;
        for _ in 0..samples.len() {
            current_angle += omega * dt;
            phases.push(current_angle as f32);
        }
        Self { phases, last_angle: current_angle }
    }
    /// Возвращает фазовый профиль текущего окна.
    pub fn phases(&self) -> &[f32] {
        &self.phases
    }
    /// Возвращает точный угол для старта следующего окна вычислений.
    pub fn next_initial_angle(&self) -> f64 {
        self.last_angle
    }
    /// Вычисляет точный период вращения (в отсчетах) через автокорреляцию.
    /// Ищет максимум функции в узком окне от показаний тахометра.
    fn find_exact_period(samples: &[f32], rough_period: f32) -> f32 {
        let margin = (rough_period * 0.1) as usize;
        let center = rough_period as usize;
        let start = center.saturating_sub(margin).max(1);
        let end = (center + margin).min(samples.len() / 2);
        let mut max_corr = 0.0;
        let mut best_lag = center;
        for lag in start..=end {
            let mut corr = 0.0;
            for i in 0..(samples.len() - lag) {
                corr += samples[i] * samples[i + lag];
            }
            if corr > max_corr {
                max_corr = corr;
                best_lag = lag;
            }
        }
        best_lag as f32
    }
}