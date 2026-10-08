use sal_core::error::Error;
use crate::{Frame, MirroredBuffer, RpmDetectionCtx};

///
/// Контейнер для передачи данных между вычислительными шагами
pub struct AngularCtx {
    /// Аккумулятор сырых сэмплов, окно Автокорреляции.
    /// Должен вмещать 2–3 полных оборота вала
    #[deprecated(note="To be deleted with `Autocorrelation`, replaced with `samples` field")]
    pub(crate) ac_samples: MirroredBuffer<u16>,
    /// Последнее окно сырых сэмплов
    pub(super) samples: Vec<f64>,
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
    /// Контейнер вычислительных шагов по определению текущей RPM и угла поворота вала
    pub(crate) rpm_detection: RpmDetectionCtx,
    // /// Угловая сетка в радианах (фазовый профиль) для заданного окна временных отсчетов.
    // /// Представляет собой массив углов поворота вала (в радианах), соответствующих каждому отсчету вибрации.
    // pub phases: Box<[f32; Frame::SIZE]>,
    /// Текущая ошибка вычислений
    /// Будет `Some(Error)` если шаг вычислений вернул ошибку, остальные шали эскалируют наверх.
    pub(crate) err: Option<Error>,
}
impl AngularCtx {
    /// - `f_sample` - Частота дискретизации АЦП.
    /// - `chunk_size` - Размер выборки, пакета сэмплов, поступающего из АЦП за один раз.
    pub fn new(f_sample: f64, chunk_size: usize) -> Self {
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
        let capacity = (3.0 * f_sample * 60.0 / 200.0).ceil() as usize;
        log::info!("AngularCtx.new | Буфер автокореляции выбран {capacity} сэмплов. Из расчета на 3 оборота при минимальной частоте вращения 200 RPM и частоте дискретизации {f_sample}");
        Self {
            ac_samples: MirroredBuffer::new(capacity).with_hop_size(capacity / 8),
            samples: Vec::with_capacity(chunk_size),
            raw_rpm: f64::NAN,
            raw_period: f64::NAN,
            period: f64::NAN,
            omega: f64::NAN,
            dt: f64::NAN,
            current_theta: 0.0,
            phases_size: chunk_size,
            rpm_detection: RpmDetectionCtx::new(f_sample, chunk_size, 20),
            err: None,
        }
    }
    /// Добавляет новый массив сэмплов из АЦП в обработку
    pub fn push_chunk(&mut self, frame: &Frame<f64>) {
        self.samples.clear();
        if frame.samples.len() > self.samples.capacity() {
            log::error!("{}.push_chunk | Размер выборки samples больше размера буфера аккумулятора сырых сэмплов", crate::me::<Self>());
            self.samples.extend_from_slice(&frame.samples[..self.samples.capacity()]);
        } else {
            self.samples.extend_from_slice(&frame.samples);
        }
        self.err = None;
    }
    /// Срез по последнему окну сырых сэмплов
    pub fn samples(&self) -> &[f64] {
        &self.samples
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
            samples: Default::default(),
            raw_rpm: Default::default(),
            raw_period: Default::default(),
            period: Default::default(),
            omega: Default::default(),
            dt: Default::default(),
            current_theta: Default::default(),
            phases_size: Default::default(),
            rpm_detection: Default::default(),
            err: None,
        }
    }
}
