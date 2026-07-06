/// Главная конфигурация конвейера обработки вибросигнала.
/// Инкапсулирует базовые аппаратные константы, параметры сетки и границы фильтров.
#[derive(Clone, Debug, Dese)]
pub struct Conf {
    hardware: HardwareConf,
    angular: AngularConf,
    bands: BandsConf,
}
impl Conf {
    /// Создает новый иммутабельный объект конфигурации.
    pub fn new(hardware: HardwareConf, angular: AngularConf, bands: BandsConf) -> Self {
        Self { hardware, angular, bands }
    }
    /// Возвращает параметры аппаратной части (АЦП, чанки).
    pub fn hardware(&self) -> &HardwareConf {
        &self.hardware
    }
    /// Возвращает настройки угловой сетки и буферов для Order Tracking.
    pub fn angular(&self) -> &AngularConf {
        &self.angular
    }
    /// Возвращает частотные диапазоны для детекторов.
    pub fn bands(&self) -> &BandsConf {
        &self.bands
    }
}
/// Аппаратные параметры источника данных.
#[derive(Clone, Debug)]
pub struct HardwareConf {
    sample_rate_hz: f32,
    chunk_size: usize,
}
impl HardwareConf {
    /// Инициализирует параметры АЦП.
    pub fn new(sample_rate_hz: f32, chunk_size: usize) -> Self {
        Self { sample_rate_hz, chunk_size }
    }
    /// Возвращает базовую частоту дискретизации в Гц.
    pub fn sample_rate_hz(&self) -> f32 {
        self.sample_rate_hz
    }
    /// Возвращает размер пакета данных, поступающего из сети.
    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }
}
/// Настройки углового домена (Order Tracking).
#[derive(Clone, Debug)]
pub struct AngularConf {
    points_per_rev: usize,
    fft_revolutions: usize,
}
impl AngularConf {
    /// Задает плотность угловой сетки и размер окна для спектрального анализа.
    pub fn new(points_per_rev: usize, fft_revolutions: usize) -> Self {
        Self { points_per_rev, fft_revolutions }
    }
    /// Вычисляет угловой шаг в радианах.
    /// Возвращает f64 для предотвращения деградации точности при интегрировании фазы вала.
    pub fn angular_step_rad(&self) -> f64 {
        std::f64::consts::TAU / (self.points_per_rev as f64)
    }
    /// Вычисляет итоговый размер буфера для спектрального анализа.
    /// Гарантирует степень двойки для быстрого FFT, если параметры заданы корректно.
    pub fn fft_buffer_size(&self) -> usize {
        self.points_per_rev * self.fft_revolutions
    }
}
/// Границы частотных диапазонов для фильтрации и анализа.
#[derive(Clone, Debug)]
pub struct BandsConf {
    low_order_min: f32,
    low_order_max: f32,
    mid_hz_max: f32,
    high_hz_min: f32,
    high_hz_max: f32,
}
impl BandsConf {
    /// Устанавливает границы для низко-, средне- и высокочастотных зон.
    pub fn new(
        low_order_min: f32, 
        low_order_max: f32, 
        mid_hz_max: f32, 
        high_hz_min: f32, 
        high_hz_max: f32
    ) -> Self {
        Self { low_order_min, low_order_max, mid_hz_max, high_hz_min, high_hz_max }
    }
    /// Возвращает границы низкочастотной зоны в порядках (Orders).
    pub fn low_range_orders(&self) -> (f32, f32) {
        (self.low_order_min, self.low_order_max)
    }
    /// Возвращает границы высокочастотной зоны в Герцах.
    pub fn high_range_hz(&self) -> (f32, f32) {
        (self.high_hz_min, self.high_hz_max)
    }
    /// Возвращает верхнюю границу среднего диапазона в Герцах (нижняя определяется порядками).
    pub fn mid_range_max_hz(&self) -> f32 {
        self.mid_hz_max
    }
}