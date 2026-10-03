use std::sync::Arc;
use sal_core::error::Error;
use crate::{ComplexLocalOscillatorCtx, DecimationCtx, Frame};
///
/// Контейнер для передачи данных между вычислительными шагами
pub struct RpmDetectionCtx {
    /// Сырые выборки из АЦП и угловая сетка. Приходят из AngularGrid
    pub frame: Arc<Frame<u16, f32>>,
    /// Приблизительная частота вращения с тахометра (об/мин).
    pub(crate) raw_rpm: f64,
    /// Результат работы адаптивного децимирующего фильтра (Anti-Aliasing)
    pub(crate) decimation: DecimationCtx,
    /// Результат работы комплексного гетеродина (снос 1X на нулевую частоту)
    pub(crate) complex_local_oscillator: ComplexLocalOscillatorCtx,

    /// Текущая ошибка вычислений
    /// Будет `Some(Error)` если шаг вычислений вернул ошибку, остальные шаги эскалируют наверх.
    pub(crate) err: Option<Error>,
}
impl RpmDetectionCtx {
    /// - `f_sample` - Частота дискретизации АЦП.
    pub fn new() -> Self {
        Self {
            frame: Arc::new(Frame::default()),
            raw_rpm: f64::NAN,
            decimation: Default::default(),
            complex_local_oscillator: Default::default(),
            err: None,
        }
    }
    /// ### Устанавливает ошибку в контекст.
    /// 
    /// Это приведет к останову вычислений и экалации ошибки на верхний уровень.
    /// 
    /// - `me` - Имя текущего класса.
    /// - `area` - Имя текущего метода.
    /// - `err` - Ошибка.
    /// - Возвращает [RpmDetectionCtx] с установленной ошибкой `err`.
    pub fn with_err(mut self, me: impl Into<String>, area: impl Into<String>, err: impl ToString) -> RpmDetectionCtx {
        self.err = Some(Error::new(me, area).err(err.to_string()));
        self
    }
    /// Возвращает `true` если предыдущий шаг вернул ошибку
    pub fn is_err(&self) -> bool {
        self.err.is_some()
    }
    /// Эскалирует ошибку
    pub fn pass_err(mut self, me: impl Into<String>, area: impl Into<String>) -> RpmDetectionCtx {
        self.err = match self.err {
            Some(err) => Some(Error::new(me, area).pass(err)),
            None => Some(Error::new(me, area)),
        };
        self
    }
}
//
impl Default for RpmDetectionCtx {
    fn default() -> Self {
        Self {
            frame: Default::default(),
            raw_rpm: Default::default(),
            decimation: Default::default(),
            complex_local_oscillator: Default::default(),
            err: None,
        }
    }
}
