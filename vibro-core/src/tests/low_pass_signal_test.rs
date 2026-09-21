use std::sync::Arc;
use chrono::Utc;
use debugging::session::debug_session::{DebugSession, LogLevel};
use rustfft::{FftPlanner, num_complex::{Complex, ComplexFloat}};
use sal_core::dbg::Dbg;
use crate::{Conf, Eval, Frame, ImbContext, LowPassSignal, Pass, Phases, Retain, Rpm, tests::{FftBuffer, Frequency, Udp}};

///
/// 
#[test]
fn low_pass_signal_test () {
    DebugSession::new().filter(LogLevel::Debug).init();
    let dbg = Dbg::own("LowPassSignal-test");
    let f_sample = 320_000; // Частота семплирования АЦП (Гц)
    let conf: Conf = serde_yaml::from_str(&format!(r#"
        adc:
            sample-rate-hz: {f_sample}
            chunk-size: 512
            ds-offset: 2048
        analysis:
            order-tracking:
                max-order: 100
                order-resolution: 0.05
            bands:
                low-order: 0.5..5.0
                mid-hz: ..5000
                high-hz: 5000..10000
    "#)).unwrap();
    let mut samples = vec![0u16; conf.adc.chunk_size];
    let low_cutoff_order = conf.analysis.bands.low_cutoff_order();  // Возвращает верхнюю границу для ФНЧ в порядках (Orders), например 10X
    // Фильтр нижних частот (Баттерворт 2-го порядка) для подавления ВЧ-шумов.
    // Пропускает частоты до заданного порядка (например, 10x от текущих оборотов).
    // RPM берет из контекста ImbContext.rpm
    let low_range = LowPassSignal::new(&dbg,
        conf.adc.sample_rate_hz,
        low_cutoff_order,
        Pass::new(),
    );
    let freqs = [
        // Полезный сигнал
        (Frequency::Rpm(1.0), 200),
        (Frequency::Rpm(2.0), 250),
        (Frequency::Rpm(3.0), 150),
        // Шумы
        (Frequency::Rpm(20.0), 100),
        (Frequency::Rpm(60.0), 100),
        // Статический резонанс на 5 кГц с амплитудой 100
        (Frequency::Static(5000.0), 100),
        (Frequency::Static(7000.0), 100),
        (Frequency::Static(12000.0), 100),
    ];
    let mut udp = Udp::new(
        conf.adc.chunk_size,    // 512
        conf.adc.sample_rate_hz, freqs.clone(),
    );
    let mut results: Vec<Vec<_>> = freqs.iter().map(|_| vec![]).collect();
    let n_fft = 4096 * 4;
    let retain = Arc::new(Retain::mock(&dbg, []));
    let mut ctx = ImbContext::new(&dbg, conf.analysis.samples_per_rev(), conf.analysis.fft_turns(), conf.adc.chunk_size, retain);
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n_fft);
    let mut buffer = FftBuffer::new(n_fft, 1024);
    for i in 0..1000 { // Выборок из АЦП
        // для тестирования вручную имитируем изменение rpm привода
        let rpm =  Rpm(3000.0);
        let ts = Utc::now();
        // Имитируем получение АЦП выборки из сети
        udp.parse(rpm.value(), &mut samples);
        let frame = Frame::new(ts, 2048f32, &samples, Phases::new(conf.adc.chunk_size));
        ctx.rpm = rpm;    // Имитируем чтение текущей частоты, в работе делает ReadInpurs,
        ctx.update(frame);
        // log::debug!("{dbg} | Before filter: {:?}", low_range_ctx.frame.samples);
        ctx = low_range.eval(ctx);
        let mut fft_buf = vec![Complex{re: 0.0, im: 0.0}; n_fft];
        // buffer.add(samples.iter().map(|v| Complex{re: *v as f32, im: 0.0}));
        buffer.add(ctx.samples.iter().map(|v| Complex{re: *v, im: 0.0}));
        // log::debug!("{dbg} | After filter: {:?}", low_range_ctx.samples);
        if buffer.is_full() {
            buffer.copy_into(&mut fft_buf);
            fft.process(&mut fft_buf);
            let delta_f = f_sample as f64 / n_fft as f64;
            // log::debug!("{dbg} | delta_f: {}", delta_f);
            for (i, (freq, _)) in freqs.iter().enumerate() {
                let freq = match freq {
                    Frequency::Rpm(k) => *k * rpm.to_hz(),
                    Frequency::Static(f) => *f,
                };
                let n = (freq / delta_f).round() as usize;
                let amp = |v: Complex<f32>| {
                    2.0 * v.abs() / n_fft as f32
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
        log::debug!("{dbg} | result f {:?}: {}", freqs[i].0, s / r.len() as f32);
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
