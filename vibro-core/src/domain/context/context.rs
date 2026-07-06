use sal_core::error::Error;
use crate::TimeDomainSamples;

///
/// Контейнер для передачи данных между вычислительными шагами
pub struct Context {
    pub(crate) samples: TimeDomainSamples<512>,
    /// Период вращения вала вычисленный из сырого неточного значения частоты (Inputs, Ethrnet)
    pub(crate) raw_period: f32,
    /// Уточненный период вращения вала
    pub(crate) period: f32,
    /// Текущая ошибка вычислений
    pub(crate) err: Option<Error>,
}
impl Context {
    pub fn new(samples: TimeDomainSamples<512>) -> Self {
        Self {
            samples,
            raw_period: f32::NAN,
            period: f32::NAN,
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