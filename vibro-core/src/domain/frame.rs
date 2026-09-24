use std::{marker::PhantomData, sync::Arc};
use chrono::{DateTime, Utc};
use crate::Phases;


pub const FRAME_SIZE: usize = 512; // VORZHEV Z.A.: 

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
    pub const SIZE: usize = FRAME_SIZE;
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


    // 24/09/2026 VOZHEV Z.A. Ошибка использования generic-тип: внутри impl используется [u16; Self::SIZE] как тип параметра функции, 
    // компилятор компилирует размер массива как «анонимную константу» — а такие анонимные константы 
    // не наследуют generic-параметры (D, T) окружающего impl-блока

    // /// ### Создает новый инстанс `Frame` толлько с сырыми сэмплами, фазы добавим после расчета.
    // /// - `ts` - Метка времени выборки
    // /// - `samples` - сырые выборки из АЦП. Будет автоматически удален DC (`-2048.0`)
    // /// Размер: `Frame::SIZE`
    // pub fn raw(ts: DateTime<Utc>, samples: [u16; Self::SIZE]) -> Self {
    //     Frame {
    //         ts,
    //         samples: samples.map(|v| v as f32 - 2048.0),
    //         phases: Phases::new(0),
    //     }
    // }

    // 24/09/2026 VOZHEV Z.A. Ошибка: поле `phases` требует Phases<T> а параметр = Phases<f32>

    // /// ### Добавляет угловую сетку в радианах.
    // /// - `phases` - Угловая сетка в радианах (фазовый профиль) для заданного окна временных отсчетов.
    // /// Представляет собой массив углов поворота вала (в радианах), соответствующих каждому отсчету вибрации.
    // /// Размер: `Frame::SIZE`
    // pub fn with_phases(mut self, phases: Phases<f32>) -> Self {
    //     self.phases = phases;
    //     self
    // }
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

impl<D> Frame<D, f32>
where
    D: crate::num_traits::PrimInt + Into<f32>,
{
    /// ### Создает новый инстанс `Frame` только с сырыми сэмплами, фазы добавим после расчета.
    /// - `ts` - Метка времени выборки
    /// - `samples` - сырые выборки из АЦП. Будет автоматически удален DC (`-2048.0`)
    /// Размер: `FRAME_SIZE`
    pub fn raw(ts: DateTime<Utc>, samples: [u16; FRAME_SIZE]) -> Self {
        Frame {
            ts,
            samples: samples.map(|v| v as f32 - 2048.0).to_vec(),
            phases: Phases::new(0),
            _d: PhantomData,
        }
    }
    /// ### Добавляет угловую сетку в радианах.
    /// - `phases` - Угловая сетка в радианах (фазовый профиль) для заданного окна временных отсчетов.
    /// Представляет собой массив углов поворота вала (в радианах), соответствующих каждому отсчету вибрации.
    /// Размер: `FRAME_SIZE`
    pub fn with_phases(mut self, phases: Phases<f32>) -> Self {
        self.phases = phases;
        self
    }
}