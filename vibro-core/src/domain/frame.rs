/// Контейнер для раздачи имутабельных данных вычислительным потокам
pub struct Frame {
    /// Сырые выборки из АЦП
    pub samples: [u16; Self::SIZE],
    /// Угловая сетка в радианах (фазовый профиль) для заданного окна временных отсчетов.
    /// Представляет собой массив углов поворота вала (в радианах), соответствующих каждому отсчету вибрации.
    pub phases: [f32; Self::SIZE],
}
impl Frame {
    pub const SIZE: usize = 512;
}
impl Default for Frame {
    fn default() -> Self {
        Self {
            samples: [0; Self::SIZE],
            phases: [0.0; Self::SIZE],
        }
    }
}