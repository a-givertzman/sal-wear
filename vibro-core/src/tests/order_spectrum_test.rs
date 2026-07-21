use std::sync::Arc;

use debugging::session::debug_session::{DebugSession, LogLevel};
use rustfft::{FftPlanner, num_complex::Complex};
use sal_core::dbg::Dbg;
use crate::{Conf, Eval, Frame, ImbContext, OrderDomainSamples, OrderSpectrum, Pass, Retain, WindowFn, tests::{FftBuffer, Frequency, Udp}};

/// ### Функциональное тестирование OrderDomainSamples (Метрология)
/// 
/// * Локализация чистого дисбаланса (1.0X)
///    * Вход: Идеальный синусоидальный сигнал с частотой строго 1 цикл на оборот вала (1.0X), амплитуда 1.0. Вектор подается без шума.
///    * Ожидаемый результат: После выполнения FFT пик спектра находится строго в бине index = 1.0 / (64 / 8192) = 128. Амплитуда пика равна 1.0 (с учетом нормировки FFT на размер окна). Соседние бины имеют нулевые значения.
/// * Разделение дисбаланса (1.0X) и сетевой наводки (1.02X)
///    * Вход: Суперпозиция двух синусоид: механический дисбаланс ротора (1.0X) и электрический фон сети (1.02X).
///    * Ожидаемый результат: На спектре отчетливо видны два раздельных пика в бинах 128 (1.0X) и 131 (1.023X). Они не сливаются в один купол. Это подтверждает, что разрешение Δ O ≤ 0.01 успешно реализовано.
/// * Идентификация несоосности (2.0X) и зазоров (3.0X)
///    * Вход: Сигнал, содержащий три гармоники: 1.0X (амплитуда 0.5), 2.0X (амплитуда 1.2), 3.0X (амплитуда 0.3).
///    * Ожидаемый результат: FFT корректно распределяет энергию по трем пикам (бины 128, 256 и 384 соответственно). Соотношение амплитуд строго сохраняется.#[test]
fn order_spectrum_test () {
    DebugSession::new().filter(LogLevel::Debug).init();
    let dbg = Dbg::own("OrderSpectrum-test");
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
    let window_size = OrderSpectrum::<Pass>::fft_buffer_size();
    let window_fn = WindowFn::<f32>::kaiser(&dbg, window_size, window_size, 0, 5.65).unwrap();
    // Выполняет Спектральный анализ сигнала в угловом домене (Order Tracking).
    let low_range = OrderSpectrum::new(&dbg,
        // Кайзер с умеренным beta 5.65 — отличная альтернатива Ханну:
        // Он дает такую же острую вершину (1.25 бина), но сужает основание на уровне -40 дБ до 3.75 бина (против 5.50 у Ханна).
        // Это дает даже лучшую селективность между 1X и 2X.
        Some(window_fn),
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
    let retain = Arc::new(Retain::new(&dbg));
    let mut ctx = ImbContext::new(&dbg, conf.angular.n_rev(), conf.angular.fft_turns(), retain);
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);
    let mut buffer = FftBuffer::new(fft_size, 1024);
    for i in 0..1000 { // Выборок из АЦП
        // для тестирования вручную имитируем изменение rpm привода
        let rpm =  crate::Rpm(3000.0);
        // Имитируем получение АЦП выборки из сети
        udp.parse(rpm.value(), &mut samples);
        *ctx.samples = samples.map(|v| v as f32 - 2047.5);    // Пишем сырую выбору в контекст и убираем DC
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
/// * Пропуск вычислений при недозаполнении буфера
///    * Вход: В конвейер передается пачка из order_samples длиной, меньшей чем размер окна (например, 256 сэмплов при размере FFT 8192). Капасити буфера fft_buff не заполнено.
///    * Ожидаемый результат: Метод eval успешно отрабатывает, данные добавляются в FIFO, но self.fft.process не вызывается. Массив ctx.fft_window остается неизменным/пустым.
/// * Триггер вычисления по заполнению FIFO
///    * Вход: Буфер fft_buff содержит 8128 точек. На вход eval приходит пачка из 64 точек (итого 8192 точки).
///    * Ожидаемый результат: Буфер переходит в состояние is_full(), данные копируются в fft_window, и метод process выполняется ровно один раз.
/// * Скользящее окно (Overlap) и очистка буфера
///    * Вход: Последующий вызов eval после того, как буфер уже был полностью заполнен и обработан.
///    * Ожидаемый результат: Поведение FIFO-буфера соответствует выбранной стратегии (либо полный сброс и накопление с нуля, либо сдвиг окна на заданный шаг/overlap). Сигнал не затирается некорректно.
#[test]
fn spectrum_test() {
    DebugSession::new().filter(LogLevel::Debug).init();
    let dbg = Dbg::own("OrderSpectrum-spectrum-test");
    let f_sample = 320_000; // Частота семплирования АЦП (Гц)
    todo!()
}

/// ### Стресс-тесты и обработка аномалий (Edge Cases)
/// * Переполнение FIFO-буфера (Защита от паники)
///    * Вход: Размер входящего среза ctx.order_samples превышает доступное свободное место в ctx.fft_buff.
///    * Ожидаемый результат: Конвейер аварийно прерывается, метод возвращает ctx с заполненным полем ctx.err, содержащим понятный текст ошибки переполнения. Код не падает в панику (panic!).
/// * Проброс ошибки от дочерних шагов (Short-Circuit)
///    * Вход: Предыдущий шаг (child.eval) возвращает ctx со значением ctx.err = Some(...) (например, сбой ресемплера или потеря сигнала тахометра).
///    * Ожидаемый результат: `OrderSpectrum` мгновенно делает return ctx.pass_err(...), не пытаясь писать в FIFO и не запуская FFT.
#[test]
fn stress_and_edge_cases_test() {
    DebugSession::new().filter(LogLevel::Debug).init();
    let dbg = Dbg::own("OrderSpectrum-stress-and-edge-cases-test");
    let f_sample = 320_000; // Частота семплирования АЦП (Гц)
    todo!()
}

/// ### Производительность в Проде (Performance)            (Желательно)
/// * Аллокация памяти в горячем цикле (Zero-Allocation)
///    * Вход: Циклический вызов метода eval в течение 100 000 итераций.
///    * Ожидаемый результат: Профайлер (например, DHAT или Valgrind) фиксирует ноль динамических аллокаций в куче (heap allocations) во время работы eval. Память под ctx.fft_window и ctx.fft_buff должна использоваться повторно.
/// * Потокобезопасность и параллельное исполнение (Send/Sync)
///    * Вход: Пул потоков (Rayon или tokio), обрабатывающий одновременно спектры для 128 разных подшипников (каждый в своем инстансе пайплайна).
///    * Ожидаемый результат: Отсутствие взаимных блокировок (deadlocks) и состояний гонки (race conditions). Архитектурный Arc<dyn Fft<f32>> эффективно шарится между потоками без копирования тяжелых таблиц планировщика FFT.
#[test]
fn performance_test() {
    DebugSession::new().filter(LogLevel::Debug).init();
    let dbg = Dbg::own("OrderSpectrum-performance-test");
    let f_sample = 320_000; // Частота семплирования АЦП (Гц)
    let rpm = 12_000;   // Возьмем разумный предел частоты 8..12 RPM
    todo!()
}
