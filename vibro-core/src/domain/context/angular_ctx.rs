use sal_core::error::Error;
use crate::{Frame, MirroredBuffer};

///
/// Контейнер для передачи данных между вычислительными шагами
pub struct AngularCtx {
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
    // /// Угловая сетка в радианах (фазовый профиль) для заданного окна временных отсчетов.
    // /// Представляет собой массив углов поворота вала (в радианах), соответствующих каждому отсчету вибрации.
    // pub phases: Box<[f32; Frame::SIZE]>,
    
    /// Текущая ошибка вычислений
    /// Будет `Some(Error)` если шаг вычислений вернул ошибку, остальные шали эскалируют наверх.
    pub(crate) err: Option<Error>,
}
impl AngularCtx {
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
    pub fn push_chunk(&mut self, samples: &[u16]) {
        if samples.len() > self.ac_samples.capacity() {
            log::error!("{}.push_chunk | Размер выборки samples больше размера буфера аккумулятора сырых сэмплов", crate::me::<Self>());
            self.ac_samples.push_chunk(&samples[..self.ac_samples.capacity()]);
        } else {
            self.ac_samples.push_chunk(samples);
        }
        self.err = None;
    }
    /// Эскалирует ошибку
    pub fn pass_err(mut self, me: impl Into<String>, area: impl Into<String>) -> AngularCtx {
        self.err = match self.err {
            Some(err) => Some(Error::new(me, area).pass(err)),
            None => Some(Error::new(me, area)),
        };
        self
    }
}
//
impl Default for AngularCtx {
    fn default() -> Self {
        Self {
            ac_samples: MirroredBuffer::new(0),
            raw_rpm: Default::default(),
            raw_period: Default::default(),
            period: Default::default(),
            omega: Default::default(),
            dt: Default::default(),
            current_theta: Default::default(),
            phases_size: Default::default(),
            err: None,
        }
    }
}