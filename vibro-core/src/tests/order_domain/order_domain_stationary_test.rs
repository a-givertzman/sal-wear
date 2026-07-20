use debugging::session::debug_session::{DebugSession, LogLevel};
use rustfft::{FftPlanner, num_complex::Complex};
use sal_core::dbg::Dbg;

use crate::{Conf, Eval, Frame, ImbContext, OrderDomainSamples, Pass, tests::{FftBuffer, Frequency, Udp}};

///
/// Функциональное тестирование [OrderDomainSamples] на стационарность при разгоне
#[test]
fn order_domain_stationary_test() {
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
        bands:
            low-order: 0.5..5.0
            mid-hz: ..5000
            high-hz: 5000..10000
    "#)).unwrap();
    let mut samples = [0u16; Frame::SIZE];
    let low_range = OrderDomainSamples::new(&dbg,
        conf.angular.points_per_turn(),
        Pass::new(),
    );
    // Полезный сигнал — заданный порядками (кратностями к обороту)
    let freqs = [
        (Frequency::Rpm(1.0), 200),   // 1x
        (Frequency::Rpm(2.0), 250),   // 2x
        (Frequency::Rpm(3.0), 150),   // 3x
    ];
    let mut udp = Udp::new(Frame::SIZE, conf.hardware.sample_rate_hz, freqs.clone());
    let mut results: Vec<Vec<f32>> = freqs.iter().map(|_| vec![]).collect();
    let fft_size = 4096 * 4;
    let mut ctx = ImbContext::new(conf.angular.points_per_turn(), conf.angular.fft_turns());
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);
    let mut buffer = FftBuffer::new(fft_size, 1024);
    // Профиль разгона: rpm растёт линейно от 500 до 3000 за все итерации
    let rpm_start = 500.0;
    let rpm_end = 3000.0;
    let n_iters = 1000;
    for iter in 0..n_iters {
        // Линейный разгон ротора — частота вибрации 1-го порядка растёт синхронно
        let progress = iter as f64 / n_iters as f64;
        let rpm = rpm_start + (rpm_end - rpm_start) * progress;
        udp.parse(rpm, &mut samples);
        *ctx.samples = samples.map(|v| v as f32 - 2047.5);
        ctx.rpm = rpm; // Передаём текущую частоту вращения в контекст
        ctx = low_range.eval(ctx); // в ctx.order_samples теперь лежит сигнал в угловой области
        let mut fft_buf = vec![Complex { re: 0.0, im: 0.0 }; fft_size];
        buffer.add(ctx.samples.iter().map(|v| Complex { re: *v, im: 0.0 })); // Добавляем сэмплы угловой области в буфер для FFT
        if buffer.is_full() {
            buffer.copy_into(&mut fft_buf);
            fft.process(&mut fft_buf); // Выполняем FFT на сигнале в угловой области
            let points_per_turn = conf.angular.points_per_turn() as f64;
            let delta_order = points_per_turn / fft_size as f64;
            for (j, (freq, _amp)) in freqs.iter().enumerate() {
                let target_order = match freq {
                    Frequency::Rpm(k) => *k,
                    // Статические резонансы после ресемплинга "плывут" по порядку
                    // вместе с текущими оборотами — переводим в порядок здесь же
                    Frequency::Static(f_hz) => f_hz / (ctx.rpm / 60.0),
                };
                let bin = (target_order / delta_order).round() as usize;
                let amplitude = fft_buf.get(bin).map(|c| c.norm()).unwrap_or(0.0);
                // Нормировка амплитуды FFT (для комплексного forward FFT без окна)
                let normalized = amplitude / (fft_size as f32 / 2.0);
                results[j].push(normalized);
            }
            buffer.reset();
        }
    }
    for (j, r) in results.iter().enumerate() {
        if r.is_empty() {
            continue;
        }
        let s: f32 = r.iter().sum();
        log::debug!("{dbg} | result {:?}: {}", freqs[j].0, s / r.len() as f32);
    }
    // Проверка полосы пропускания: 1x/2x/3x должны пройти с сохранённой амплитудой
    // на протяжении ВСЕГО разгона — это и есть проверка корректности order tracking
    let result = results[0].iter().sum::<f32>() / results[0].len() as f32;
    assert!((result - 200.0).abs() < 15.0, "1x amplitude mismatched: {result}");
    let result = results[1].iter().sum::<f32>() / results[1].len() as f32;
    assert!((result - 250.0).abs() < 10.0, "2x amplitude mismatched: {result}");
    let result = results[2].iter().sum::<f32>() / results[2].len() as f32;
    assert!((result - 150.0).abs() < 20.0, "3x amplitude mismatched: {result}");
}