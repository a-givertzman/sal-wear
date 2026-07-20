use std::sync::Arc;

use crate::Phases;

/// Контейнер для раздачи имутабельных данных вычислительным потокам
pub struct Frame {
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
    /// Создает новый инстанс `Frame`.
    /// - `samples` - сырые выборки из АЦП.
    /// Размер: `Frame::SIZE`
    /// - `phases` - Угловая сетка в радианах (фазовый профиль) для заданного окна временных отсчетов.
    /// Представляет собой массив углов поворота вала (в радианах), соответствующих каждому отсчету вибрации.
    /// Размер: `Frame::SIZE`
    pub fn new(samples: [u16; Self::SIZE], phases: Phases<f32>) -> Arc<Self> {
        Arc::new(Frame {
            samples: samples.map(|v| v as f32 - 2048.0),
            phases,
        })
    }
}
impl Default for Frame {
    fn default() -> Self {
        Self {
            samples: [0.0; Self::SIZE],
            phases: Phases::new(Self::SIZE),
        }
    }
}