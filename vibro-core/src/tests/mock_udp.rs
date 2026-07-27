use std::{f64::consts::TAU, sync::atomic::{AtomicU64, Ordering}};

/// Определяет частоту сигнала
/// - `Rpm(k)` - Динамический сигнал с кратностью `k` от частоты вращения вала (RPM) .
/// - `Static(f)` - Статический сигнал частотой `f` (Гц), не зависит от частоты впащенеи.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Frequency {
    /// Кратность частоты вращения (1X, 2X, ... 256X)
    Rpm(f64),
    /// Сигнал остается стабильным и не зависит от частоты вращения.
    Static(f64),
}

/// Генератор эталонных сигналов (из АЦП)
/// Для тестирования алгоритмов виброаналитики
pub struct Udp {
    pub chunk: usize,   // Используется справочно
    pub sample_freq: f64,   // Используется справочно
    dt: f64,
    freqs: Vec<(Frequency, u16)>, // Статические резонансы (частота, амплитуда)
    pub time: f64,     // Аккумулятор времени симуляции, используется справочно
    phases: Vec<f64>,     // Аккумуляторы фаз гамоник сигнала
}
impl Udp {
    /// Имитирует сигнал с АЦП выборками заданного размера
    /// `chunk` - размер выборок с АЦП (512)
    /// `sample_freq` - Частота дискретизации АЦП
    /// `freqs` - Массив пар (частота, амплитуда)
    pub fn new(chunk: usize, sample_rate_hz: impl Into<f64>, freqs: impl IntoIterator<Item = (Frequency, u16)>) -> Self {
        let sample_rate_hz = sample_rate_hz.into();
        let freqs: Vec<_> = freqs.into_iter().collect();
        let phases = vec![0.0; freqs.len()];
        Self {
            chunk,
            sample_freq: sample_rate_hz,
            dt: 1.0 / sample_rate_hz,
            freqs,
            time: 0.0,
            phases,
        }
    }
    /// Заполняет переданный буфер синтетическими данными.
    /// Выполняет сложение синусоид и смещение нулевой линии для формата u16.
    /// `rpm` - Текущая частота вращения привода в об/мин
    pub fn parse(&mut self, rpm: f64, samples: &mut [u16]) {
        let rpm_hz = rpm / 60.0;
        let rpm_delta_phase = TAU * rpm_hz * self.dt;
        for i in 0..samples.len() {
            let mut val = 2048.0; // Смещение нулевой линии для 12-бит АЦП
            for (j, (freq, amp)) in self.freqs.iter().enumerate() {
                val += (*amp as f64) * self.phases[j].sin();
                let delta = match freq {
                    // Добавляем динамические сигналы (зависят от частоты вращения вала)
                    Frequency::Rpm(k) => *k * rpm_delta_phase,
                    // Добавляем статические резонансы механизма
                    Frequency::Static(f) => TAU * *f * self.dt,
                };
                self.phases[j] += delta;
                self.phases[j] = self.phases[j].rem_euclid(TAU);
            }
            samples[i] = val.round().clamp(0.0, 4095.0) as u16;
        }
        self.time += samples.len() as f64 * self.dt;
    }
}
/// Регулятор RPM
pub struct Rpm {
    val: AtomicU64,
}
impl Rpm {
    /// Returns RPM starts from 0.0
    pub fn new() -> Self {
        Self { val: AtomicU64::new(0.0f64.to_bits()) }
    }
    /// Returns RPM starts from `start`
    pub fn start_with(rpm: f64) -> Self {
        Self { val: AtomicU64::new(rpm.to_bits()) }
    }
    /// Updates current value with predicate
    pub fn update(&self, f: impl Fn(f64) -> f64) {
        let val = f(f64::from_bits(self.val.load(Ordering::Relaxed)));
        self.val.store(val.to_bits(), Ordering::Relaxed);
    }
    /// Returns current rpm
    pub fn get(&self) -> f64 {
        f64::from_bits(self.val.load(Ordering::Relaxed))
    }
}

///
/// Basic Tests
#[cfg(test)]
mod tests {
    use super::*;
    use debugging::session::debug_session::{DebugSession, LogLevel};
    use sal_core::dbg::Dbg;
    /// Вспомогательная функция для вычисления амплитуды конкретной частоты в сигнале (ДПФ для одной точки)
    fn get_amplitude_at_freq(samples: &[u16], target_freq: f64, sample_rate: f64) -> f64 {
        let n = samples.len() as f64;
        let mut real = 0.0;
        let mut imag = 0.0;
        for (i, &val) in samples.iter().enumerate() {
            // Убираем постоянную составляющую АЦП (2047.5), чтобы она не размывалась на низкие частоты
            let signal = (val as f64) - 2047.5;
            let angle = 2.0 * std::f64::consts::PI * target_freq * (i as f64) / sample_rate;
            real += signal * angle.cos();
            imag += signal * angle.sin();
        }
        // Амплитуда спектральной составляющей
        2.0 * (real.powi(2) + imag.powi(2)).sqrt() / n
    }
    #[test]
    fn test_generator_full_validation() {
        let sample_rate = 320_000.0;
        let chunk_size = 512;
        let mut udp = Udp::new(chunk_size, sample_rate, vec![
            (Frequency::Rpm(1.0), 200),     // При 600 RPM это 10 Гц
            (Frequency::Static(100.0), 500), // Стационарный резонанс 100 Гц
        ]);
        // Для точного частотного анализа 10 Гц нам нужно собрать побольше данных (например, 1 секунду)
        let total_samples = 320_000;
        let mut all_samples = vec![0u16; total_samples];
        // Заполняем буфер порциями, имитируя реальную работу генератора по чанкам
        let rpm = 600.0; // 600 RPM / 60 = 10 Гц
        for chunk in all_samples.chunks_mut(chunk_size) {
            udp.parse(rpm, chunk);
        }
        // --- 1. ПРОВЕРКА ДИАПАЗОНА АЦП И СРЕДНЕЙ ЛИНИИ ---
        assert!(all_samples.iter().all(|&v| v <= 4095), "Samples out of ADC upper range");
        let mean: f64 = all_samples.iter().map(|&v| v as f64).sum::<f64>() / total_samples as f64;
        assert!((mean - 2047.5).abs() < 5.0, "Mean too far from center 2047.5 (DC bias error)");
        let min = *all_samples.iter().min().unwrap() as f64;
        let max = *all_samples.iter().max().unwrap() as f64;
        let peak_to_peak = max - min;
        // Теоретический максимум размаха: 2047.5 +/- (200 + 500) -> размах около 1400
        assert!(peak_to_peak > 1300.0 && peak_to_peak < 1450.0, "Signal peak-to-peak is wrong");
        // --- 2. ЧАСТОТНЫЙ АНАЛИЗ (ПРОВЕРКА МАТЕМАТИКИ СИГНАЛА) ---
        // Проверяем амплитуду Rpm(1.0) гармоники (должна быть на частоте 10 Гц)
        let amp_at_10hz = get_amplitude_at_freq(&all_samples, 10.0, sample_rate);
        // Проверяем амплитуду Static(100.0) гармоники (должна быть на частоте 100 Гц)
        let amp_at_100hz = get_amplitude_at_freq(&all_samples, 100.0, sample_rate);
        // Проверяем "пустую" частоту, которой быть не должно (например, 50 Гц)
        let amp_at_50hz = get_amplitude_at_freq(&all_samples, 50.0, sample_rate);
        // Ожидаем исходные амплитуды с небольшой погрешностью из-за дискретизации и округления до u16
        assert!((amp_at_10hz - 200.0).abs() < 2.0, "RPM harmonic (10 Hz) amplitude error: expected 200, got {amp_at_10hz}");
        assert!((amp_at_100hz - 500.0).abs() < 2.0, "Static harmonic (100 Hz) amplitude error: expected 500, got {amp_at_100hz}");
        // На частоте 50 Гц должен быть практически чистый ноль
        assert!(amp_at_50hz < 1.0, "Ghost signal detected at 50 Hz: amplitude {amp_at_50hz}");
        // --- 3. ПРОВЕРКА НЕПРЕРЫВНОСТИ ВРЕМЕНИ И ФАЗЫ ---
        // Проверяем, что внутренний счетчик времени udp.time зафиксировал ровно 1.0 секунду
        assert!((udp.time - 1.0).abs() < 1e-5, "Simulation time tracking error");
    }
    ///
    /// ### Синхронность фазы
    /// 
    /// Сигнал, где частота вибрации строго равна частоте вращения вала (1-й порядок).
    /// Тогда на один оборот (между двумя тах-пульсами) всегда приходится ровно один полный период синусоиды вибрации.
    /// 
    /// - **Идеальное совпадение начальной и конечной фазы:** Первая точка следующего оборота (6401-я точка) должна 
    ///   иметь в точности то же значение фазы и амплитуды, что и самая первая точка (1-я точка первого оборота).
    /// 
    /// - **Отсутствие смещения частоты:** Внутри выборки из 6400 точек синусоида пересекает среднюю линию (2047.5) ровно два раза,
    ///   а экстремумы (максимум и минимум) четко соответствуют заданной амплитуде 100 (2147.5 и 1947.5).
    #[test]
    fn sync_phase_test () {
        DebugSession::new().filter(LogLevel::Debug).init();
        let dbg = Dbg::own("adc-emulator-test");
        let sample_rate_hz = 320_000; // Частота семплирования АЦП (Гц)
        const FRAME: usize = 512;    // Размер эдногого набора сэмплов из АЦП
        let mut samples = [0u16; FRAME];
        let rpm = Rpm::start_with(3000.0);
        let freqs = [
            // Шумы зависимые от RPM 
            (Frequency::Rpm(1.0), 100),
            // // Статический резонанс на 5 кГц с амплитудой 100
            // (Frequency::Static(5000.0), 100),
        ];
        let mut udp = Udp::new(
            FRAME,
            sample_rate_hz,
            freqs.clone(),
        );
        // Период вращения вала (Tприв):
        //      При 3000 об/мин вал делает 3000 / 60 = 50 оборотов в секунду (50 Гц).
        //      Tприв = 1 / 50 = 0.02 сек.
        // Период дискретизации АЦП (Tацп):
        //      1 / 320 000 = 3.125 * 10^{-6} сек (или 3.125 мкс).
        // Количество точек на один оборот:
        //      0.02 / (3.125 * 10^{-6}) = 6400 точек.
        // 13 выборок из АЦП (13 * 512 = 6656) содержат 6400 из них - один период
        let n = 6400;
        let size = 512 * 26;
        let mut buffer = Vec::with_capacity(size);
        for _ in 0..26 {
            // Имитируем получение АЦП выборки из сети
            udp.parse(rpm.get(), &mut samples);
            buffer.extend(samples);
        }
        // for i in 0..16 {
        // // for i in (n/4-10)..=(n/4+10) {
        //     log::debug!("{dbg} | period 1 [{i}]: {} period 2 [{i}]: {}", buffer[i], buffer[n + i]);
        // }
        // for i in (n-8)..(n+8) {
        //     log::debug!("{dbg} | period 1 [{i}]: {} period 2 [{i}]: {}", buffer[i], buffer[n + i]);
        // }
        for i in 0..n {
            assert!(buffer[i] == buffer[n + i], "\n period 1 [{i}]: {} \n period 2 [{}]: {}", buffer[i], n + i, buffer[n + i]);
        }
        let max = buffer[..n].iter().max().unwrap();
        log::debug!("{dbg} | period 1 max: {:?}", max);
        let min = buffer[..n].iter().min().unwrap();
        log::debug!("{dbg} | period 1 min: {:?}", min);
        let max = buffer[n..(n * 2)].iter().max().unwrap();
        log::debug!("{dbg} | period 2 max: {:?}", max);
        let min = buffer[n..(n * 2)].iter().min().unwrap();
        log::debug!("{dbg} | period 2 min: {:?}", min);
        // Проверяем, пики сигналов амплитуды +/- 100
        // Максимум: 2048 + 100 = 2148
        // Минимум: 2048 - 100 = 1948
        assert!(*max == 2148, "Неверный максимум: {max}");
        assert!(*min == 1948, "Неверный минимум: {min}");
        // ТОЧКИ ПЕРЕСЕЧЕНИЯ НУЛЯ И ПИКИ (ГЕОМЕТРИЯ) ---
        // На старте (i = 0) sin(0) = 0 -> ожидаем чистый offset 2048.0 (округляется до 2048)
        assert_eq!(buffer[0], 2048, "Неверная начальная точка периода (должна быть на нулевой линии) {}", buffer[0]);
        // Ровно на 1/4 периода (i = 1600) sin(pi/2) = 1 -> пик вверх (2048.0 + 100 = 2148.0 -> 2148)
        assert_eq!(buffer[n / 4], 2148, "Положительный пик смещен или имеет неверную амплитуду {}", buffer[n / 4]);
        // Ровно на 1/2 периода (i = 3200) sin(pi) = 0 -> переход через ноль (2048.0 -> 2048)
        assert_eq!(buffer[n / 2], 2048, "Точка перехода через ноль (середина периода) смещена {}", buffer[n / 2]);
        // Ровно на 3/4 периода (i = 4800) sin(3pi/2) = -1 -> пик вниз (2048.0 - 100 = 1948.0 -> 1948)
        assert_eq!(buffer[3 * n / 4], 1948, "Отрицательный пик смещен или имеет неверную амплитуду {}", buffer[3 * n / 4]);
        // ПРОВЕРКА АНТИСИММЕТРИИ (ФОРМА СИНУСОИДЫ) ---
        // Проверяем, что первая половина волны зеркально противоположна второй половине
        for i in 0..(n / 2) {
            let first_half_centered = buffer[i] as f64 - 2048.0;
            let second_half_centered = buffer[i + n / 2] as f64 - 2048.0;
            // Сумма отклонений противоположных точек должна быть практически равна нулю.
            // Из-за целочисленного округляющего квантования допускаем микроскопическую погрешность в 1 АЦП-знак.
            assert!(
                (first_half_centered + second_half_centered).abs() <= 0.1,
                "Искажение формы синусоиды: нарушена антисимметрия полупериодов в точке {i}"
            );
        }
    }
    /// ### Проверка профиля разгона (Run-up/Coast-down)
    /// 
    /// Задайте эмулятору линейное ускорение вала (например, от 600 до 3000 об/мин).
    /// Убедитесь, что временной интервал между тах-пульсами уменьшается строго по квадратичному закону,
    /// а амплитуда синуса не плывет.
    #[test]
    fn lenear_grow_test () {
        DebugSession::new().filter(LogLevel::Debug).init();
        let dbg = Dbg::own("adc-emulator-test");
        let sample_rate_hz = 320_000; // Частота семплирования АЦП (Гц)
        const FRAME: usize = 512;    // Размер эдногого набора сэмплов из АЦП
        let mut samples = [0u16; FRAME];
        let rpm = Rpm::start_with(3000.0);
        let freqs = [
            // Шумы зависимые от RPM 
            (Frequency::Rpm(1.0), 100),
            // // Статический резонанс на 5 кГц с амплитудой 100
            // (Frequency::Static(5000.0), 100),
        ];
        let mut udp = Udp::new(
            FRAME,    // 512
            sample_rate_hz,
            freqs.clone(),
        );
        let start_rpm = 600.0;
        let end_rpm = 3000.0;
        // Период вращения вала (Tприв):
        //      При 3000 об/мин вал делает 3000 / 60 = 50 оборотов в секунду (50 Гц).
        //      Tприв = 1 / 50 = 0.02 сек.
        // Период дискретизации АЦП (Tацп):
        //      1 / 320 000 = 3.125 * 10^{-6} сек (или 3.125 мкс).
        // Количество точек на один оборот:
        //      0.02 / (3.125 * 10^{-6}) = 6400 точек.
        // 13 выборок из АЦП (13 * 512 = 6656) содержат 6400 из них - один период
        // Симулируем 0.5 секунды разгона
        let total_samples = (sample_rate_hz as f64 * 0.5) as usize;
        let total_frames = total_samples / FRAME;
        let mut buffer = Vec::with_capacity(total_frames * FRAME);
        for frame_idx in 0..total_frames {
            // Линейно интерполируем RPM от времени
            let progress = frame_idx as f64 / total_frames as f64;
            let rpm = start_rpm + (end_rpm - start_rpm) * progress;
            // Имитируем получение АЦП выборки из сети
            udp.parse(rpm, &mut samples);
            buffer.extend(samples);
        }
        // for i in 0..16 {
        // // for i in (n/4-10)..=(n/4+10) {
        //     log::debug!("{dbg} | period 1 [{i}]: {} period 2 [{i}]: {}", buffer[i], buffer[n + i]);
        // }
        // for i in (n-8)..(n+8) {
        //     log::debug!("{dbg} | period 1 [{i}]: {} period 2 [{i}]: {}", buffer[i], buffer[n + i]);
        // }
        // 2. АНАЛИЗ РЕЗУЛЬТАТА: Поиск точек перехода через ноль (снизу вверх)
        let mut zero_crossings = Vec::new();
        let offset = 2048;
        for i in 0..(buffer.len() - 1) {
            // Ищем строго момент пересечения средней линии снизу вверх
            if buffer[i] <= offset && buffer[i + 1] > offset {
                zero_crossings.push(i);
            }
        }
        // 3. Расчет периодов между пересечениями нуля
        let mut distances = Vec::new();
        for window in zero_crossings.windows(2) {
            distances.push(window[1] - window[0]);
        }
        log::debug!("Реальные периоды волны (в сэмплах) при разгоне: {:?}", distances);
        // 4. ПРОВЕРКА ТРЕНДА: Частота растет -> период строго уменьшается
        // Из-за дискретизации соседние периоды могут быть равны, но следующий не должен быть БОЛЬШЕ предыдущего.
        for i in 1..distances.len() {
            assert!(
                distances[i] <= distances[i - 1],
                "Ошибка разгона на шаге {}! Период увеличился: {} -> {}",
                i, distances[i - 1], distances[i]
            );
        }
        // 5. ПРОВЕРКА ГРАНИЦ (для 320кГц и 600->3000 RPM)
        // (с учетом динамического ускорения за 0.5 сек)
        // Первая волна сжимается из-за мгновенного старта разгона (~24.5k вместо 32k)
        assert!(
            distances[0] >= 24000 && distances[0] <= 25000, 
            "Начальный период вне ожидаемого диапазона разгона: {}", distances[0]
        );
        // Конечный период на 50 Гц стремится к 6400 сэмплов
        assert!(
            *distances.last().unwrap() < 6800, 
            "Конечный период слишком велик: {}", distances.last().unwrap()
        );
    }
}
