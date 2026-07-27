use std::sync::Arc;

use sal_core::error::Error;
use crate::{DecimationCtx, Frame, MirroredBuffer};

///
/// Контейнер для передачи данных между вычислительными шагами
pub struct Context {
    /// Сырые выборки из АЦП и угловая сетка.
    pub frame: Arc<Frame>,
    /// Аккумулятор сырых сэмплов, окно Автокорреляции.
    /// Должен вмещать 2–3 полных оборота вала
    pub(crate) ac_samples: MirroredBuffer<u16>,
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
    /// Текущий вычисленный угол θ поворота вала (между выборками может быть сброшен кратно 2π)
    pub current_theta: f64,
    /// Размер угловой сетки
    pub phases_size: usize,
    
    /// Прореживание сырого входного сигнала
    pub decimation: DecimationCtx,

    /// Текущая ошибка вычислений
    /// Будет `Some(Error)` если шаг вычислений вернул ошибку, остальные шали эскалируют наверх.
    pub(crate) err: Option<Error>,
}
impl Context {
    pub fn new() -> Self {
        // Формула вычисления размера выборки `capacity`
        // Чтобы автокорреляция надежно зацепилась за оборотную частоту,
        // в буфере должно лежать минимум 2–3 полных оборота вала.
        // Пропорция вычисляется от минимально возможных оборотов привода в твоей системе,
        // так как на низких оборотах один цикл занимает больше всего времени.
        // Формула выглядит так:
        //      capacity = K * Fs * 60 / RPMmin
        // Где:
        //      K — количество оборотов для уверенного захвата (берем 3).
        //      Fs — частота дискретизации АЦП (320 000 Гц).
        //      RPMmin — минимальная скорость вращения вала, при которой мы ведем анализ.
        // Для привода 1500: `3 * 320 000 * 60 / 300 => 192 000`
        // Для привода 1500: `3 * 320 000 * 60 / 600 => 96 000`
        let capacity = 96_000;
        Self {
            frame: Arc::new(Frame::default()),
            decimation: DecimationCtx::default(),
            ac_samples: MirroredBuffer::new(capacity),
            raw_rpm: f64::NAN,
            raw_period: f64::NAN,
            period: f64::NAN,
            omega: f64::NAN,
            dt: f64::NAN,
            current_theta: 0.0,
            phases_size: Frame::SIZE,
            err: None,
        }
    }
    /// Добавляет новый массив сэмплов из АЦП в обработку
    pub fn push_frame(&mut self, frame: &Arc<Frame>) {
        self.frame = frame.clone();
        self.err = None;
    }
    /// Добавляет новый массив сэмплов из АЦП в обработку
    pub fn push_chunk(&mut self, samples: &[u16; Frame::SIZE]) {
        self.ac_samples.push_chunk(samples);
        self.err = None;
    }
    /// Эскалирует ошибку
    pub fn pass_err(mut self, me: impl Into<String>, area: impl Into<String>) -> Context {
        self.err = match self.err {
            Some(err) => Some(Error::new(me, area).pass(err)),
            None => Some(Error::new(me, area)),
        };
        self
    }
}