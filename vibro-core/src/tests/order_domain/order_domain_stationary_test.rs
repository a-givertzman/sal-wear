use crate::{
    AngularGrid, Autocorrelation, Conf, Context, Eval, Frame, ImbContext, Inputs,
    OrderDomainSamples, Pass, ReadInputs,
    tests::{FftBuffer, Frequency, Udp},
};
use debugging::session::debug_session::{DebugSession, LogLevel};
use rustfft::{FftPlanner, num_complex::Complex};
use sal_core::dbg::Dbg;
use std::sync::Arc;
///
/// Функциональное тестирование [OrderDomainSamples] на стационарность при разгоне
#[test]
fn order_domain_stationary_test() {
    DebugSession::new().filter(LogLevel::Debug).init();
    let dbg = Dbg::own("OrderDomainSamples-test");
    let f_sample = 320_000; // Частота семплирования АЦП (Гц)
    let conf: Conf = serde_yaml::from_str(&format!(
        r#"
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
    "#
    ))
    .unwrap();
    let mut samples = [0u16; Frame::SIZE];
    let low_range = OrderDomainSamples::new(&dbg, conf.angular.points_per_turn(), Pass::new());
    let freqs = [(Frequency::Rpm(1.0), 200)];
    let inputs = Arc::new(Inputs::new());
    let mut ctx = Context::new();
    let angular_grid = AngularGrid::new(
        &dbg,
        Autocorrelation::new(
            &dbg,
            conf.hardware.sample_rate_hz,
            ReadInputs::new(&dbg, inputs.clone()),
        ),
    );
    let mut udp = Udp::new(Frame::SIZE, conf.hardware.sample_rate_hz, freqs.clone());
    let mut results: Vec<Vec<f64>> = freqs.iter().map(|_| vec![]).collect();
    let mut i_ctx = ImbContext::new(conf.angular.points_per_turn(), conf.angular.fft_turns());
    let fft_size = 1024;
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);
    let mut buffer = FftBuffer::new(fft_size, 1024);
    let rpm_start = 600.0;
    let rpm_end = 6000.0;
    let n_iters = 40;
    let mut last_fft_spectrum: Vec<f64> = Vec::new();
    for iter in 0..n_iters {
        log::debug!("Iteration: {}/{}", iter, n_iters);
        let progress = iter as f64 / n_iters as f64;
        let progress_sin = (progress * std::f64::consts::PI / 2.0).sin();
        let rpm = rpm_start + (rpm_end - rpm_start) * progress_sin;
        udp.parse(rpm, &mut samples);
        inputs.set_rpm(rpm);
        ctx.push_chunk(&samples);
        ctx = angular_grid.eval(ctx);
        let frame = Arc::new(Frame {
            samples: samples.map(|v| v as f32 - 2048.0),
            phases: ctx.phases.to_vec(),
        });
        i_ctx.update(frame.clone());
        i_ctx.rpm = rpm;
        i_ctx = low_range.eval(i_ctx);
        buffer.add(i_ctx.order_samples.iter().map(|c| Complex {
            re: c.re as f64,
            im: c.im as f64,
        }));
        if buffer.is_full() {
            let mut fft_buf = vec![Complex { re: 0.0, im: 0.0 }; fft_size];
            buffer.copy_into(&mut fft_buf);
            // Применяем окно Хемминга для уменьшения утечек спектра
            let n_len = fft_buf.len();
            for i in 0..n_len {
                let angle = (2.0 * std::f64::consts::PI * i as f64) / (n_len - 1) as f64;
                let w = 0.54 - 0.46 * angle.cos();
                fft_buf[i].re *= w;
                fft_buf[i].im *= w;
            }
            fft.process(&mut fft_buf);
            // Вычисляем амплитуду спектра и нормируем на размер FFT и коэффициент окна
            let amp = |v: Complex<f64>| {
                let norm = (v.re * v.re + v.im * v.im).sqrt();
                let coherent_gain = 0.54; // усредненный коэффицент размытия Хемминга
                (2.0 * norm) / (fft_size as f64 * coherent_gain)
            };
            last_fft_spectrum.clear();
            for i in 0..(fft_size / 2) {
                last_fft_spectrum.push(amp(fft_buf[i]));
            }
            let points_per_turn = conf.angular.points_per_turn() as f64;
            let delta_order = points_per_turn / fft_size as f64;
            for (i, (freq, _)) in freqs.iter().enumerate() {
                let target_order = match freq {
                    Frequency::Rpm(k) => *k,
                    Frequency::Static(f_hz) => f_hz / (rpm / 60.0),
                };
                let n = (target_order / delta_order).round() as usize;
                if n > 0 && n < fft_buf.len() - 1 {
                    let result = (amp(fft_buf[n - 1]).powi(2)
                        + amp(fft_buf[n]).powi(2)
                        + amp(fft_buf[n + 1]).powi(2))
                    .sqrt();
                    results[i].push(result);
                }
            }
            buffer.reset();
        }
    }
    for (i, result) in results.iter().enumerate() {
        for order_peak in result {
            assert!(
                (order_peak - freqs[i].1 as f64).abs() < 40.0,
                "1x amplitude mismatched: {order_peak}"
            );
        }
    }
}
