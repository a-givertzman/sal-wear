use sal_core::dbg::Dbg;
use crate::{Context, Eval, me};

/// Угловая сетка (фазовый профиль) для заданного окна временных отсчетов.
/// Представляет собой массив углов поворота вала, соответствующих каждому отсчету вибрации.
pub struct AngularGrid<Child> {
    child: Child,
    /// Массив углов в радианах. Размер совпадает с окном входящих данных.
    phases: Vec<f32>,
    /// Последний вычисленный угол для бесшовной передачи состояния.
    /// Хранится в f64 для предотвращения накопления ошибки округления.
    last_angle: f64,
    dbg: Dbg,
}
impl<Child> AngularGrid<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// ### Returns `Autocorrelation` new instance
    /// Вычисляет угловую сетку для новой порции данных.
    /// * `samples` - Окно отсчетов для расчета (должно вмещать минимум 2-3 оборота вала).
    /// * `rough_rpm` - Приблизительная частота вращения с тахометра (об/мин).
    /// * `sample_rate` - Частота дискретизации в Гц.
    /// * `initial_angle` - Угол $\theta$ на начало окна.
    pub fn new(parent: impl Into<String>, samples: &[f32], rough_rpm: f32, sample_rate: f32, initial_angle: f64, child: Child) -> Self {
        let dbg = Dbg::new(parent, me::<Self>());
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
        Self {
            phases,
            last_angle: current_angle,
            child,
            dbg,
        }
    }
    /// Возвращает фазовый профиль текущего окна.
    pub fn phases(&self) -> &[f32] {
        &self.phases
    }
    /// Возвращает точный угол для старта следующего окна вычислений.
    pub fn next_initial_angle(&self) -> f64 {
        self.last_angle
    }
}
impl<Child> Eval<Context, Context> for AngularGrid<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    //
    fn eval(&self, ctx: Context) -> Context {
        todo!()
    }
    //
    fn exit(&self) {
        todo!()
    }
}