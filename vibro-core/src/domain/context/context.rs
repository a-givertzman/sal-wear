use sal_core::error::Error;
use crate::TimeDomainSamples;

///
/// Контейнер для передачи данных между вычислительными шагами
pub struct Context {
    /// Сырые выборки из АЦП
    pub(crate) samples: TimeDomainSamples<512>,
    /// Аккумулятор сырых выборок для Автокорреляции
    pub(crate) ac_samples: Buffer,
    /// Приблизительная частота вращения с тахометра (об/мин).
    pub(crate) raw_rpm: f64,
    /// Примерный (грубый) период вращения (в отсчетах АЦП).
    /// Сколько «сэмплов» АЦП теоретически укладывается в один полный оборот вала.
    pub(crate) raw_period: f64,
    /// Уточненный период вращения вала (в отсчетах АЦП).
    /// Результат работы функции автокорреляции, которая ищет реальный физический пик совпадения сигнала в узком окне вокруг rough_period.
    pub(crate) period: f64,
    /// Угловая скорость ω (в радианах в секунду).
    pub(crate) omega: f64,
    /// Шаг дискретизации Δt (в секундах).
    /// Время между двумя соседними выборками из АЦП.
    pub(crate) dt: f64,
    /// Текущая ошибка вычислений
    /// Будет `Some(Error)` если шаг вычислений вернул ошибку, остальные шали эскалируют наверх.
    pub(crate) err: Option<Error>,
}
impl Context {
    pub fn new(samples: TimeDomainSamples<512>) -> Self {
        Self {
            samples,
            raw_rpm: f64::NAN,
            raw_period: f64::NAN,
            period: f64::NAN,
            omega: f64::NAN,
            dt: f64::NAN,
            err: None,
        }
    }
    pub fn pass_err(mut self, me: impl Into<String>, area: impl Into<String>) -> Context {
        self.err = match self.err {
            Some(err) => Some(Error::new(me, area).pass(err)),
            None => Some(Error::new(me, area)),
        };
        self
    }
}