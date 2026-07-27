use std::sync::Arc;
use chrono::{DateTime, Utc};
use crate::Phases;

/// Контейнер для раздачи имутабельных данных вычислительным потокам
pub struct Frame {
    ///  Метка времени выборки
    pub ts: DateTime<Utc>,
    /// Сырые выборки из АЦП
    /// Размер: `Frame::SIZE`
    pub samples: [f32; Self::SIZE],
    /// Угловая сетка в радианах (фазовый профиль) для заданного окна временных отсчетов.
    /// Представляет собой массив углов поворота вала (в радианах), соответствующих каждому отсчету вибрации.
    /// Размер: `Frame::SIZE`
    pub phases: Phases<f32>,
}
impl Frame {
    /// Количество сырых сэмплов в одной пачке,
    /// Которая за раз заходит на обработку (приходит из сети).
    pub const SIZE: usize = 512;
    /// ### Создает новый инстанс `Frame`.
    /// - `ts` - Метка времени выборки
    /// - `samples` - сырые выборки из АЦП. Будет автоматически удален DC (`-2048.0`)
    /// Размер: `Frame::SIZE`
    /// - `phases` - Угловая сетка в радианах (фазовый профиль) для заданного окна временных отсчетов.
    /// Представляет собой массив углов поворота вала (в радианах), соответствующих каждому отсчету вибрации.
    /// Размер: `Frame::SIZE`
    pub fn new(ts: DateTime<Utc>, samples: [u16; Self::SIZE], phases: Phases<f32>) -> Arc<Self> {
        Arc::new(Frame {
            ts,
            samples: samples.map(|v| v as f32 - 2048.0),
            phases,
        })
    }
    /// ### Создает новый инстанс `Frame` толлько с сырыми сэмплами, фазы добавим после расчета.
    /// - `ts` - Метка времени выборки
    /// - `samples` - сырые выборки из АЦП. Будет автоматически удален DC (`-2048.0`)
    /// Размер: `Frame::SIZE`
    pub fn raw(ts: DateTime<Utc>, samples: [u16; Self::SIZE]) -> Self {
        Frame {
            ts,
            samples: samples.map(|v| v as f32 - 2048.0),
            phases: Phases::new(0),
        }
    }
    /// ### Добавляет угловую сетку в радианах.
    /// - `phases` - Угловая сетка в радианах (фазовый профиль) для заданного окна временных отсчетов.
    /// Представляет собой массив углов поворота вала (в радианах), соответствующих каждому отсчету вибрации.
    /// Размер: `Frame::SIZE`
    pub fn with_phases(mut self, phases: Phases<f32>) -> Self {
        self.phases = phases;
        self
    }
}
impl Default for Frame {
    fn default() -> Self {
        Self {
            ts: Utc::now(),
            samples: [0.0; Self::SIZE],
            phases: Phases::new(Self::SIZE),
        }
    }
}