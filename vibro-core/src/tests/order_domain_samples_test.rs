use debugging::session::debug_session::{DebugSession, LogLevel};
use rustfft::{FftPlanner, num_complex::Complex};
use sal_core::dbg::Dbg;
use crate::{Conf, Eval, Frame, ImbContext, OrderDomainSamples, Pass, tests::{FftBuffer, Frequency, Udp}};

///
/// ### Функциональное тестирование OrderDomainSamples (Метрология)
/// - Тест на стационарность при разгоне                    (Обязательно!)
/// - Тест фиксированного количества точек на оборот        (Обязательно!)
/// - Тест постоянного смещения (DC Offset)                 (Желательно)
#[test]
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
        (Frequency::Rpm(50.0), 200),
        (Frequency::Rpm(100.0), 250),
        (Frequency::Rpm(150.0), 150),
        // Шумы
        (Frequency::Rpm(1000.0), 100),
        (Frequency::Rpm(3000.0), 100),
        // Статический резонанс на 5 кГц с амплитудой 100
        (Frequency::Rpm(5000.0), 100),
        (Frequency::Rpm(7000.0), 100),
        (Frequency::Rpm(12000.0), 100),
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
        // Имитируем получение АЦП выборки из сети
        udp.parse(rpm, &mut samples);
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
    let result = results[0].iter().sum::<f32>() / results[0].len() as f32;
    assert!((result - 200.0).abs() < 15.0, "1x (50Hz) amplitude mismatched: {result}");
    let result = results[1].iter().sum::<f32>() / results[1].len() as f32;
    assert!((result - 250.0).abs() < 10.0, "2x (100Hz) amplitude mismatched: {result}");
    let result = results[2].iter().sum::<f32>() / results[0].len() as f32;
    assert!((result - 150.0).abs() < 20.0, "3x (150Hz) amplitude mismatched: {result}");
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
/// ### Спектральное тестирование (White Box)
/// - Тест на интерполяционный шум                          (Желательно)
/// - Контроль фазового сдвига                              (Обязательно!)
#[test]
fn spectrum_test() {
    DebugSession::new().filter(LogLevel::Debug).init();
    let dbg = Dbg::own("OrderDomainSamples-spectrum-test");
    let f_sample = 320_000; // Частота семплирования АЦП (Гц)
    todo!()
}

/// ### Стресс-тесты и обработка аномалий (Edge Cases)
/// - Пропуск тах-импульса (Missing Pulse)                  (Желательно)
/// - Двойной тах-импульс (Double Triggering)               (Желательно)
/// - Мгновенный останов (Zero Speed)                       (Обязательно!)
#[test]
fn stress_and_edge_cases_test() {
    DebugSession::new().filter(LogLevel::Debug).init();
    let dbg = Dbg::own("OrderDomainSamples-stress-and-edge-cases-test");
    let f_sample = 320_000; // Частота семплирования АЦП (Гц)
    todo!()
}

/// ### Производительность в Проде (Performance)            (Желательно)
/// - Тест производительности при максимальных оборотах
/// 
/// Частота вращения современных асинхронных двигателей при работе от преобразователя частоты
/// может регулироваться в диапазоне от 0 до 6000 об/мин (иногда до 10 000 об/мин и выше для специальных серий).
#[test]
fn performance_test() {
    DebugSession::new().filter(LogLevel::Debug).init();
    let dbg = Dbg::own("OrderDomainSamples-performance-test");
    let f_sample = 320_000; // Частота семплирования АЦП (Гц)
    let rpm = 12_000;   // Возьмем разумный предел частоты 8..12 RPM
    todo!()
}
