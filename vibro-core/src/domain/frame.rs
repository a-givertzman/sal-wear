use std::{sync::Arc};
use chrono::{DateTime, Utc};
use crate::Phases;


pub const FRAME_SIZE: usize = 512; // VORZHEV Z.A.: 

/// Контейнер для раздачи имутабельных данных вычислительным потокам
/// - `<D>` - Тип входного значения сэмпла
/// - `<T>` - Тип выходного значения сэмпла
#[derive(Debug, Clone)]
pub struct Frame<T> {
    ///  Метка времени выборки
    pub ts: DateTime<Utc>,
    /// Сырые выборки из АЦП
    /// Размер: `Frame::SIZE`
    pub samples: Arc<Vec<T>>,
    /// Угловая сетка в радианах (фазовый профиль) для заданного окна временных отсчетов.
    /// Представляет собой массив углов поворота вала (в радианах), соответствующих каждому отсчету вибрации.
    /// Размер: `Frame::SIZE`
    pub phases: Phases<T>,
}
impl<T> Frame<T> 
where
    T: crate::num_traits::Float {
    /// Количество сырых сэмплов в одной пачке,
    /// Которая за раз заходит на обработку (приходит из сети).
    #[deprecated(note="Use `conf.adc.chunk_size` instead.")]
    pub const SIZE: usize = FRAME_SIZE;
    /// ### Создает новый инстанс `Frame`.
    /// - `ts` - Метка времени выборки
    /// - `total_phase` - Текущий абсолютный вычисленный угол θ поворота вала (не сбрасывается), не используется в расчетах, для отчетности.
    /// - `samples` - Сырые выборки из АЦП.
    /// - `ds_offset` - Постоянная составляющая сигнала с АЦП. Будет автоматически удалена из сигнала (`xᵢ - ds_offset`).
    /// - `phases` - Угловая сетка в радианах (фазовый профиль) для заданного окна временных отсчетов.
    /// Представляет собой массив углов поворота вала (в радианах), соответствующих каждому отсчету вибрации.
    /// Размер: `Frame::SIZE`
    pub fn new<D: crate::num_traits::PrimInt + Into<T>>(ts: DateTime<Utc>, dc_offset: T, samples: &[D], phases: Phases<T>) -> Arc<Self> {
        Arc::new(Frame {
            ts,
            samples: Arc::new(samples.iter().map(|v| Into::<T>::into(*v) - dc_offset).collect()),
            phases,
        })
    }
    /// ### Создает новый инстанс `Frame` толлько с сырыми сэмплами, фазы добавим после расчета.
    /// - `ts` - Метка времени выборки
    /// - `samples` - сырые выборки из АЦП. Будет автоматически удален DC (`-2048.0`)
    /// Размер: `Frame::SIZE`
    pub fn raw<D: crate::num_traits::PrimInt + Into<T>>(ts: DateTime<Utc>, dc_offset: T, samples: &[D]) -> Self {
        Frame {
            ts,
            samples: Arc::new(samples.iter().map(|v| Into::<T>::into(*v) - dc_offset).collect()),
            phases: Phases::new(0),
        }
    }
    /// ### Добавляет угловую сетку в радианах.
    /// - `phases` - Угловая сетка в радианах (фазовый профиль) для заданного окна временных отсчетов.
    /// Представляет собой массив углов поворота вала (в радианах), соответствующих каждому отсчету вибрации.
    /// Размер: `Frame::SIZE`
    pub fn with_phases(mut self, phases: Phases<T>) -> Self {
        self.phases = phases;
        self
    }
}
impl<T: crate::num_traits::Float> Default for Frame<T> {
    fn default() -> Self {
        Self {
            ts: Utc::now(),
            samples: Arc::new(vec![]),
            phases: Phases::new(0),
        }
    }
}
