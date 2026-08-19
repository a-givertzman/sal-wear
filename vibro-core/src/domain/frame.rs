use std::{marker::PhantomData, sync::Arc};
use chrono::{DateTime, Utc};
use crate::Phases;

/// Контейнер для раздачи имутабельных данных вычислительным потокам
/// - `<D>` - Тип входного значения сэмпла
/// - `<T>` - Тип выходного значения сэмпла
pub struct Frame<D, T> {
    ///  Метка времени выборки
    pub ts: DateTime<Utc>,
    /// Сырые выборки из АЦП
    /// Размер: `Frame::SIZE`
    pub samples: Vec<T>,
    /// Угловая сетка в радианах (фазовый профиль) для заданного окна временных отсчетов.
    /// Представляет собой массив углов поворота вала (в радианах), соответствующих каждому отсчету вибрации.
    /// Размер: `Frame::SIZE`
    pub phases: Phases<T>,
    _d: PhantomData<D>,
}
impl<D, T> Frame<D, T> 
where
    T: crate::num_traits::Float,
    D: crate::num_traits::PrimInt + Into<T> {
    /// Количество сырых сэмплов в одной пачке,
    /// Которая за раз заходит на обработку (приходит из сети).
    #[deprecated(note="Use `conf.adc.chunk_size` instead.")]
    pub const SIZE: usize = 512;
    /// ### Создает новый инстанс `Frame`.
    /// - `ts` - Метка времени выборки
    /// - `total_phase` - Текущий абсолютный вычисленный угол θ поворота вала (не сбрасывается), не используется в расчетах, для отчетности.
    /// - `samples` - Сырые выборки из АЦП.
    /// - `ds_offset` - Постоянная составляющая сигнала с АЦП. Будет автоматически удалена из сигнала (`xᵢ - ds_offset`).
    /// - `phases` - Угловая сетка в радианах (фазовый профиль) для заданного окна временных отсчетов.
    /// Представляет собой массив углов поворота вала (в радианах), соответствующих каждому отсчету вибрации.
    /// Размер: `Frame::SIZE`
    pub fn new(ts: DateTime<Utc>, dc_offset: T, samples: &[D], phases: Phases<T>) -> Arc<Self> {
        Arc::new(Frame {
            ts,
            samples: samples.iter().map(|v| Into::<T>::into(*v) - dc_offset).collect(),
            phases,
            _d: PhantomData,
        })
    }
}
impl<D, T: crate::num_traits::Float> Default for Frame<D, T> {
    fn default() -> Self {
        Self {
            ts: Utc::now(),
            samples: vec![],
            phases: Phases::new(0),
            _d: PhantomData,
        }
    }
}