use std::sync::Arc;
use sal_core::error::Error;
use crate::{Frame, domain::imbalance::context::low_pass_signal_ctx::LowPassSignalCtx};

///
/// Контейнер для передачи данных между вычислительными шагами
pub struct ImbContext {
    /// Уточненная частота вращения вала, об/мин.
    pub rpm: f64,
    /// Сырые выборки из АЦП и угловая сетка
    pub frame: Arc<Frame>,
    /// LowPassSinal Context
    pub low_pass_signal: LowPassSignalCtx,
    /// Отфилтрованная выборка сырого АЦП сигнала
    pub samples: [u16; Frame::SIZE],

    /// Текущая ошибка вычислений
    /// Будет `Some(Error)` если шаг вычислений вернул ошибку, остальные шали эскалируют наверх.
    pub(crate) err: Option<Error>,
}
impl ImbContext {
    pub fn new() -> Self {
        Self {
            rpm: f64::EPSILON,
            frame: Arc::new(Frame::default()),
            low_pass_signal: LowPassSignalCtx::new(),
            samples: [0; Frame::SIZE],
            err: None,
        }
    }
    /// Добавляет новый массив сэмплов из АЦП в обработку
    pub fn update(&mut self, frame: Arc<Frame>) {
        self.frame = frame;
        self.err = None;
    }
    /// Эскалирует ошибку
    pub fn pass_err(mut self, me: impl Into<String>, area: impl Into<String>) -> ImbContext {
        self.err = match self.err {
            Some(err) => Some(Error::new(me, area).pass(err)),
            None => Some(Error::new(me, area)),
        };
        self
    }
}