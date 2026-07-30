use rustfft::num_complex::Complex;
use crate::tests::order_domain::order_features_filter_test::entities::spectral_disturbance::SpectralDisturbance;
///
/// Описание спетральной модели
pub struct SpectrumModel {
    /// Начальная базовая амплитуда A0
    pub a0: f64,
    /// Общее количество фреймов в тесте (1_620_000_000)
    pub total_frames: u64,
    /// Номер целевого бина в спектре (соответствует целевому ордеру)
    pub target_bin_idx: usize,
    /// Общее количество бинов в окне спектра (размер окна FFT)
    pub num_bins: usize,
    // Экспоненциальный множитель базового тренда для одной итерации: 1.015^(1 / 1.62e9)
    trend_multiplier: f64,
    // Список помех, привязанных строго к целевому бину
    target_disturbances: Vec<SpectralDisturbance>,
    // Список помех, бьющих по нецелевым (соседним) бинам
    out_of_band_disturbances: Vec<(usize, SpectralDisturbance)>,
    /// Базовый стационарный фоновый шум измерительного тракта (например, 0.005 от A0)
    pub floor_noise: f64,
}
//
impl SpectrumModel {
    pub fn new(a0: f64, total_frames: u64, target_bin_idx: usize, num_bins: usize) -> Self {
        Self {
            a0,
            total_frames,
            target_bin_idx,
            num_bins,
            trend_multiplier: 1.015_f64.powf(1.0 / total_frames as f64),
            target_disturbances: Vec::new(),
            out_of_band_disturbances: Vec::new(),
            floor_noise: 0.001 * a0, // 0.1% фонового шума по умолчанию
        }
    }
    /// Регистрация помехи на целевом бине гармоники
    pub fn add_target_disturbance(&mut self, dist: SpectralDisturbance) {
        self.target_disturbances.push(dist);
    }
    /// Регистрация внеполосной помехи на конкретном нецелевом бине `bin_idx`
    pub fn add_out_of_band_disturbance(&mut self, bin_idx: usize, dist: SpectralDisturbance) {
        assert!(bin_idx != self.target_bin_idx, "Внеполосная помеха не может накладываться на целевой бин!");
        self.out_of_band_disturbances.push((bin_idx, dist));
    }
    /// Генерирует и заполняет вектор FFT-окна для фрейма `i` без аллокаций памяти
    #[inline]
    pub fn generate_frame(&self, i: u64, bins: &mut Vec<Complex<f64>>) {
        bins.clear();
        if bins.capacity() < self.num_bins {
            bins.reserve(self.num_bins);
        }
        // 1. Математически точный расчет амплитуды ЦЕЛЕВОГО бина
        // Базовый монотонный тренд (+1.5% на финише)
        let mut target_amp = self.a0 * self.trend_multiplier.powf(i as f64);
        // Накладываем суперпозицию всех зарегистрированных целевых возмущений
        for disturbance in &self.target_disturbances {
            target_amp += disturbance.evaluate(i, self.a0);
        }
        // 2. Формируем массив бинов спектра
        for current_idx in 0..self.num_bins {
            if current_idx == self.target_bin_idx {
                // Записываем итоговую амплитуду в целевой бин
                bins.push(Complex { re: target_amp, im: 0.0 });
            } else {
                // Расчет амплитуды НЕЦЕЛЕВОГО бина
                // Базовый уровень — белый фоновый шум тракта
                let mut noise_amp = self.floor_noise;
                // Проверяем, действуют ли на данный нецелевой бин аддитивные помехи на фрейме `i`
                for (bad_bin, disturbance) in &self.out_of_band_disturbances {
                    if *bad_bin == current_idx {
                        noise_amp += disturbance.evaluate(i, self.a0);
                    }
                }
                bins.push(Complex { re: noise_amp, im: 0.0 });
            }
        }
    }
}
