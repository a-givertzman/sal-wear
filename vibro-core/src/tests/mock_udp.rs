use std::f64::consts::TAU;

pub struct Udp {
    chunk: usize,
    sample_freq: f64,
    dt: f64,
    freqs: Vec<(f64, u16)>, // Статические резонансы (частота, амплитуда)
    base_time: f64,     // Аккумулятор времени симуляции
    rpm_phase: f64,     // Аккумуляторы фаз вращения вала
    fixed_phases: Vec<f64>, // Аккумуляторы фаз для статических резонансов
}
impl Udp {
    /// Имитирует сигнал с АЦП выборками заданного размера
    /// `chunk` - размер выборок с АЦП (512)
    /// `sample_freq` - Частота дискретизации АЦП
    /// `freqs` - Массив пар (частота, амплитуда)
    pub fn new(chunk: usize, sample_rate_hz: impl Into<f64>, freqs: impl IntoIterator<Item = (f64, u16)>) -> Self {
        let sample_rate_hz = sample_rate_hz.into();
        let freqs: Vec<_> = freqs.into_iter().collect();
        let fixed_phases = vec![0.0; freqs.len()];
        Self {
            chunk,
            sample_freq: sample_rate_hz,
            dt: 1.0 / sample_rate_hz,
            freqs,
            base_time: 0.0,
            rpm_phase: 0.0,
            fixed_phases,
        }
    }
    /// Заполняет переданный буфер синтетическими данными.
    /// Выполняет сложение синусоид и смещение нулевой линии для формата u16.
    /// `rpm` - Текущая частота вращения привода в об/мин
    /// `rpm_amp` - Амплитуда 1x гармоники вала
    pub fn parse(&mut self, rpm: f64, rpm_amp: u16, samples: &mut [u16]) {
        let rpm_hz = rpm / 60.0;
        let delta_phase_rpm = TAU * rpm_hz * self.dt;
        for i in 0..samples.len() {
            self.rpm_phase += delta_phase_rpm;
            self.rpm_phase = self.rpm_phase.rem_euclid(TAU);
            let mut val = 2047.5; // Смещение нулевой линии для 12-бит АЦП
            // Добавляем динамическую 1x гармонику
            val += (rpm_amp as f64) * self.rpm_phase.sin();
            // Добавляем статические резонансы механизма
            for (j, (f, amp)) in self.freqs.iter().enumerate() {
                self.fixed_phases[j] += TAU * f * self.dt;
                self.fixed_phases[j] = self.fixed_phases[j].rem_euclid(TAU);
                val += (*amp as f64) * self.fixed_phases[j].sin();
            }
            samples[i] = val.round().clamp(0.0, 4095.0) as u16;
        }
        self.base_time += samples.len() as f64 * self.dt;
    }
}
///
/// 
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_range_and_signal_presence() {
        let mut udp = Udp::new(512, 320_000, vec![(100.0, 500)]);
        let mut samples = vec![0u16; 512];
        udp.parse(600.0, 200, &mut samples);
        // 1. Проверяем, что не вышли за границы АЦП
        assert!(samples.iter().all(|&v| v >= 0 && v <= 4095), "Samples out of ADC range");
        // 2. Проверяем, что сигнал не «зажат» в узком диапазоне (есть реальная вариация)
        let min = *samples.iter().min().unwrap() as f64;
        let max = *samples.iter().max().unwrap() as f64;
        let peak_to_peak = max - min;
        assert!(peak_to_peak > 100.0, "Signal has no meaningful variation");
        // 3. Оцениваем амплитуду по размаху и проверяем, что она в разумных пределах
        let estimated_amp = peak_to_peak / 2.0;
        // Ожидаем, что амплитуда хотя бы не меньше 150 (с учётом округления и возможного несинфаза)
        assert!(estimated_amp > 150.0, "Amplitude too low");
        // И не больше, чем сумма амплитуд плюс небольшой запас на округление
        assert!(estimated_amp < 800.0, "Amplitude unexpectedly high");
        // 4. (Опционально) Проверяем, что среднее не «уехало» совсем далеко
        let mean: f64 = samples.iter().map(|&v| v as f64).sum::<f64>() / samples.len() as f64;
        assert!((mean - 2047.5).abs() < 500.0, "Mean too far from center");
    }
}
