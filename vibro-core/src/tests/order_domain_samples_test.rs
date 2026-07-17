use std::sync::atomic::{AtomicU64, Ordering};

use debugging::session::debug_session::{DebugSession, LogLevel};
use rustfft::{FftPlanner, num_complex::{Complex, ComplexFloat}};
use sal_core::dbg::Dbg;
use crate::{Conf, Eval, Frame, ImbContext, OrderDomainSamples, Pass, tests::{FftBuffer, Udp}};

///
/// ### Тестируем тестовую среду
/// - **Синхронность фазы:** Сгенерируйте сигнал, где частота вибрации строго равна частоте вращения вала (1-й порядок).
///   Проверьте, что на один оборот (между двумя тах-пульсами) всегда приходится ровно один полный период синусоиды вибрации.
/// - **Проверка профиля разгона (Run-up/Coast-down):** Задайте эмулятору линейное ускорение вала (например, от 600 до 3000 об/мин).
///   Убедитесь, что временной интервал между тах-пульсами уменьшается строго по квадратичному закону, а амплитуда синуса не плывет.
#[test]
fn adc_emulator_test () {
    DebugSession::new().filter(LogLevel::Debug).init();
    let dbg = Dbg::own("adc-emulator-test");
    let f_sample = 320_000; // Частота семплирования АЦП (Гц)
    let conf: Conf = serde_yaml::from_str(&format!(r#"
        hardware:
            sample-rate-hz: {f_sample}
            chunk-size: 512
        angular:
            max-order: 100
            resolution: 0.05
            # points-per-turn: 
        bands:
            low-order: 0.5..5.0
            mid-hz: ..5000
            high-hz: 5000..10000
    "#)).unwrap();
    let mut samples = [0u16; Frame::SIZE];
    let rpm = Rpm::start_with(3000.0);
    let freqs = [
        // Шумы зависимые от RPM 
        (1000.0, 100),
        // Статический резонанс на 5 кГц с амплитудой 100
        (5000.0, 100),
    ];
    let mut udp = Udp::new(
        Frame::SIZE,    // 512
        conf.hardware.sample_rate_hz,
        freqs.clone(),
    );
    let mut results: Vec<Vec<_>> = freqs.iter().map(|_| vec![]).collect();
    let fft_size = 4096 * 4;
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);
    let mut buffer = FftBuffer::new(fft_size, 1024);
    let rpm_speed = 10.0;   // Рост скорости 10 об.мин за цикл
    for _ in 0..1000 {  // Выборок из АЦП
        rpm.update(|rpm| rpm + rpm_speed);
        // для тестирования вручную имитируем изменение rpm привода
        let rpm_amp = 0;
        // Имитируем получение АЦП выборки из сети
        udp.parse(rpm.get(), rpm_amp, &mut samples);
        let mut fft_buf = vec![Complex{re: 0.0, im: 0.0}; fft_size];
        buffer.add(samples.iter().map(|v| Complex{re: *v as f32, im: 0.0}));
        // log::debug!("{dbg} | After filter: {:?}", low_range_ctx.samples);
        if buffer.is_full() {
            buffer.copy_into(&mut fft_buf);
            fft.process(&mut fft_buf);
            let delta_f = f_sample as f64 / fft_size as f64;
            // log::debug!("{dbg} | delta_f: {}", delta_f);
            for (i, (freq, target)) in freqs.iter().enumerate() {
                let n = (freq / delta_f).round() as usize;
                let amp = |v: Complex<f32>| {
                    2.0 * v.abs() / fft_size as f32
                };
                // for offset in -4..=4 {
                //     let idx = (n as isize).checked_add(offset).unwrap_or(0) as usize;
                //     if idx < fft_buf.len() {
                //         log::debug!("{dbg} | buffer[{n}{:+}] (бин {}): {}", offset, idx, amp(fft_buf[idx]));
                //     }
                // }
                let result = (amp(fft_buf[n-1]).powi(2)
                    + amp(fft_buf[n]).powi(2)
                    + amp(fft_buf[n+1]).powi(2))
                    .sqrt();
                results[i].push(result);
                // log::debug!("{dbg} | result: {}", result);
            }
            // assert!((result - 100.0).abs() < 50.0);
            buffer.reset();
        }
    }
    for (i, r) in results.iter().enumerate() {
        let s: f32 = r.iter().sum();
        log::debug!("{dbg} | result f {}: {}", freqs[i].0, s / r.len() as f32);
    }
    // Проверяем полосу пропускания (низкие частоты 1x..3x должны пройти)
    assert!((results[0].iter().sum::<f32>() / results[0].len() as f32 - 200.0).abs() < 15.0, "1x (50Hz) attenuation is too high");
    assert!((results[1].iter().sum::<f32>() / results[1].len() as f32 - 250.0).abs() < 10.0, "2x (100Hz) amplitude mismatched");
    assert!((results[2].iter().sum::<f32>() / results[2].len() as f32 - 150.0).abs() < 20.0, "3x (150Hz) unexpected attenuation");
    // Проверяем полосу подавления (шумы выше 1000 Гц должны быть жестко зарезаны)
    let mean_1khz = results[3].iter().sum::<f32>() / results[3].len() as f32;
    let mean_3khz = results[4].iter().sum::<f32>() / results[4].len() as f32;
    let mean_5khz = results[5].iter().sum::<f32>() / results[5].len() as f32;
    let mean_7khz = results[6].iter().sum::<f32>() / results[6].len() as f32;
    let mean_12khz = results[7].iter().sum::<f32>() / results[7].len() as f32;
    assert!(mean_1khz < 10.0, "LowPassFilter failed to suppress 1 kHz noise (got {})", mean_1khz);
    assert!(mean_3khz < 2.0,  "LowPassFilter failed to suppress 3 kHz noise (got {})", mean_3khz);
    assert!(mean_5khz < 1.0,  "LowPassFilter failed to suppress 5 kHz noise (got {})", mean_5khz);
    assert!(mean_7khz < 1.0,  "LowPassFilter failed to suppress 7 kHz noise (got {})", mean_7khz);
    assert!(mean_12khz < 1.0,  "LowPassFilter failed to suppress 12 kHz noise (got {})", mean_12khz);

}
///
/// 
// #[test]
fn order_domain_smples_test () {
    DebugSession::new().filter(LogLevel::Debug).init();
    let dbg = Dbg::own("OrderDomainSamples-test");
    let f_sample = 320_000; // Частота семплирования АЦП (Гц)
    let conf: Conf = serde_yaml::from_str(&format!(r#"
        hardware:
            sample-rate-hz: {f_sample}
            chunk-size: 512
        angular:
            max-order: 100
            resolution: 0.05
            # points-per-turn: 
        bands:
            low-order: 0.5..5.0
            mid-hz: ..5000
            high-hz: 5000..10000
    "#)).unwrap();
    let mut samples = [0u16; Frame::SIZE];
    // Выполняет ресемплинг (Order Tracking) отфильтрованного сигнала во временной области в равномерную сетку угловой области.
    // Использует локальную кубическую интерполяцию Catmull-Rom для предотвращения алиасинга.
    let low_range = OrderDomainSamples::new(&dbg,
        conf.angular.points_per_turn(),
        Pass::new(),
    );
    let freqs = [
        // Полезный сигнал
        (50.0, 200),
        (100.0, 250),
        (150.0, 150),
        // Шумы
        (1000.0, 100),
        (3000.0, 100),
        // Статический резонанс на 5 кГц с амплитудой 100
        (5000.0, 100),
        (7000.0, 100),
        (12000.0, 100),
    ];
    let mut udp = Udp::new(
        Frame::SIZE,    // 512
        conf.hardware.sample_rate_hz,
        freqs.clone(),
    );
    let mut results: Vec<Vec<_>> = freqs.iter().map(|_| vec![]).collect();
    let fft_size = 4096 * 4;
    let mut ctx = ImbContext::new(conf.angular.points_per_turn(), conf.angular.fft_turns());
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);
    let mut buffer = FftBuffer::new(fft_size, 1024);
    for i in 0..1000 { // Выборок из АЦП
        // для тестирования вручную имитируем изменение rpm привода
        let rpm =  3000.0;
        let rpm_amp = 0;
        // Имитируем получение АЦП выборки из сети
        udp.parse(rpm, rpm_amp, &mut samples);
        ctx.samples = samples.map(|v| v as f32 - 2047.5);    // Пишем сырую выбору в контекст и убираем DC
        ctx.rpm = rpm;    // Имитируем чтение текущей частоты, в работе делает ReadInpurs,
        // log::debug!("{dbg} | Before filter: {:?}", low_range_ctx.frame.samples);
        ctx = low_range.eval(ctx);
        let mut fft_buf = vec![Complex{re: 0.0, im: 0.0}; fft_size];
        // buffer.add(samples.iter().map(|v| Complex{re: *v as f32, im: 0.0}));
        buffer.add(ctx.samples.iter().map(|v| Complex{re: *v, im: 0.0}));
        // log::debug!("{dbg} | After filter: {:?}", low_range_ctx.samples);
        if buffer.is_full() {
            buffer.copy_into(&mut fft_buf);
            fft.process(&mut fft_buf);
            let delta_f = f_sample as f64 / fft_size as f64;
            // log::debug!("{dbg} | delta_f: {}", delta_f);
            for (i, (freq, target)) in freqs.iter().enumerate() {
                let n = (freq / delta_f).round() as usize;
                let amp = |v: Complex<f32>| {
                    2.0 * v.abs() / fft_size as f32
                };
                // for offset in -4..=4 {
                //     let idx = (n as isize).checked_add(offset).unwrap_or(0) as usize;
                //     if idx < fft_buf.len() {
                //         log::debug!("{dbg} | buffer[{n}{:+}] (бин {}): {}", offset, idx, amp(fft_buf[idx]));
                //     }
                // }
                let result = (amp(fft_buf[n-1]).powi(2)
                    + amp(fft_buf[n]).powi(2)
                    + amp(fft_buf[n+1]).powi(2))
                    .sqrt();
                results[i].push(result);
                // log::debug!("{dbg} | result: {}", result);
            }
            // assert!((result - 100.0).abs() < 50.0);
            buffer.reset();
        }
    }
    for (i, r) in results.iter().enumerate() {
        let s: f32 = r.iter().sum();
        log::debug!("{dbg} | result f {}: {}", freqs[i].0, s / r.len() as f32);
    }
    // Проверяем полосу пропускания (низкие частоты 1x..3x должны пройти)
    assert!((results[0].iter().sum::<f32>() / results[0].len() as f32 - 200.0).abs() < 15.0, "1x (50Hz) attenuation is too high");
    assert!((results[1].iter().sum::<f32>() / results[1].len() as f32 - 250.0).abs() < 10.0, "2x (100Hz) amplitude mismatched");
    assert!((results[2].iter().sum::<f32>() / results[2].len() as f32 - 150.0).abs() < 20.0, "3x (150Hz) unexpected attenuation");
    // Проверяем полосу подавления (шумы выше 1000 Гц должны быть жестко зарезаны)
    let mean_1khz = results[3].iter().sum::<f32>() / results[3].len() as f32;
    let mean_3khz = results[4].iter().sum::<f32>() / results[4].len() as f32;
    let mean_5khz = results[5].iter().sum::<f32>() / results[5].len() as f32;
    let mean_7khz = results[6].iter().sum::<f32>() / results[6].len() as f32;
    let mean_12khz = results[7].iter().sum::<f32>() / results[7].len() as f32;
    assert!(mean_1khz < 10.0, "LowPassFilter failed to suppress 1 kHz noise (got {})", mean_1khz);
    assert!(mean_3khz < 2.0,  "LowPassFilter failed to suppress 3 kHz noise (got {})", mean_3khz);
    assert!(mean_5khz < 1.0,  "LowPassFilter failed to suppress 5 kHz noise (got {})", mean_5khz);
    assert!(mean_7khz < 1.0,  "LowPassFilter failed to suppress 7 kHz noise (got {})", mean_7khz);
    assert!(mean_12khz < 1.0,  "LowPassFilter failed to suppress 12 kHz noise (got {})", mean_12khz);

}
struct Rpm {
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
