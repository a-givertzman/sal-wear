use std::ops::Range;

use serde::Deserialize;

/// Главная конфигурация конвейера обработки вибросигнала.
/// Инкапсулирует базовые аппаратные константы, параметры сетки и границы фильтров.
#[derive(Clone, Debug, Deserialize)]
pub struct Conf {
    /// Параметры аппаратной части (АЦП, чанки).
    pub hardware: HardwareConf,
    /// Настройки угловой сетки и буферов для Order Tracking.
    pub angular: AngularConf,
    /// Частотные диапазоны для детекторов.
    pub bands: BandsConf,
}
/// Аппаратные параметры источника данных.
#[derive(Clone, Debug, Deserialize)]
pub struct HardwareConf {
    /// Возвращает базовую частоту дискретизации в Гц.
    #[serde(alias = "sample-rate-hz")]
    pub sample_rate_hz: f32,
    /// Возвращает размер пакета данных, поступающего из сети.
    #[serde(alias = "chunk-size")]
    pub chunk_size: usize,
}
/// Настройки углового домена (Order Tracking).
#[derive(Clone, Debug, Deserialize)]
pub struct AngularConf {
    /// Плотность угловой сетки
    #[serde(alias = "resolution")]
    pub points_per_rev: usize,
    /// Размер окна для спектрального анализа
    #[serde(alias = "fft-size")]
    pub fft_size: usize,
}
impl AngularConf {
    /// Вычисляет угловой шаг в радианах.
    /// Возвращает f64 для предотвращения деградации точности при интегрировании фазы вала.
    pub fn angular_step_rad(&self) -> f64 {
        std::f64::consts::TAU / (self.points_per_rev as f64)
    }
    /// Вычисляет итоговый размер буфера для спектрального анализа.
    /// Гарантирует степень двойки для быстрого FFT, если параметры заданы корректно.
    pub fn fft_buffer_size(&self) -> usize {
        self.points_per_rev * self.fft_size
    }
}
/// Границы частотных диапазонов для фильтрации и анализа.
#[derive(Clone, Debug, Deserialize)]
pub struct BandsConf {
    /// Границы низкочастотной зоны в порядках (Orders).
    #[serde(alias = "low-order")]
    pub low_order: Range<f32>,
    /// Верхняя граница среднего диапазона в Герцах (нижняя определяется порядками).
    #[serde(alias = "mid-hz")]
    pub mid_hz: Range<f32>,
    /// Границы высокочастотной зоны в Герцах.
    #[serde(alias = "high-hz")]
    pub high_hz: Range<f32>,
}
impl BandsConf {
    /// Возвращает границы низкочастотной зоны в порядках (Orders).
    pub fn low_range_orders(&self) -> (f32, f32) {
        (self.low_order.start, self.low_order.end)
    }
    /// Возвращает границы высокочастотной зоны в Герцах.
    pub fn high_range_hz(&self) -> (f32, f32) {
        (self.high_hz.start, self.high_hz.end)
    }
    /// Возвращает верхнюю границу среднего диапазона в Герцах (нижняя определяется порядками).
    pub fn mid_range_max_hz(&self) -> f32 {
        self.mid_hz.end
    }
}