use crate::{
    Conf,
    Eval,
    ImbContext,
    OrderFeatureFilter,
    Pass,
    Retain,
    tests::order_domain::order_features_filter_test::entities::{
        ImpulseShape, 
        SpectralDisturbance, 
        SpectrumModel,
    },
};
use debugging::session::debug_session::{DebugSession, LogLevel};
use rustfft::num_complex::Complex;
use sal_core::dbg::Dbg;
use sal_sync::services::Service;
use std::sync::Arc;
use std::fs::File;
use std::io::{BufWriter, Write};
///
/// Функциональное тестирование [OrderFeaturesFilter]
#[test]
fn order_features_filter_stationary_test() {
    DebugSession::new().filter(LogLevel::Debug).init();
    let dbg = Dbg::own("OrderFeaturesFilter-test");
    // ===================== МАСШТАБ СИМУЛЯЦИИ =====================
    const ITER_PER_DAY: f64 = 54_000_000.0; // количество кадров в сутки
    const COUNT_DAYS: f64 = 0.001;            // количество дней симуляции
    let total_frames: u64 = (ITER_PER_DAY * COUNT_DAYS).round() as u64; // общее количество фреймов
    // ===================== ФИЗИЧЕСКИЕ ПАРАМЕТРЫ =====================
    const A0: f64 = 200.0;                  // базовая амплитуда целевого бина
    const TARGET_ORDER: f64 = 1.0;          // исследуемый ордер (1.0x)
    const TREND_GROWTH: f64 = 1.015;        // ожидаемый рост тренда к концу теста (+1.5%)
    let trend_multiplier = TREND_GROWTH.powf(1.0 / total_frames as f64);
    const F_SAMPLE: u64 = 320_000;          // частота дискретизации АЦП, Гц
    // ===================== АМПЛИТУДЫ ПОМЕХ (во сколько раз больше A0) =====================
    const LONG_TARGET_AMP: f64 = 1.5;    // долгая помеха на целевом бине
    const SHORT_TARGET_AMP: f64 = 2.0;   // короткая помеха на целевом бине
    const CONST_NEIGHBOR_AMP: f64 = 0.0; // постоянная помеха на правом соседнем бине
    const LONG_NEIGHBOR_AMP: f64 = 2.0;  // долгая помеха на левом соседнем бине
    const SHORT_NEIGHBOR_AMP: f64 = 5.0; // короткая помеха на левом соседнем бине
    let floor_noise = 0.0;
    // ===================== ОКНА ДЕЙСТВИЯ ПОМЕХ (в кадрах, уже с учётом SCALE) =====================
    const LONG_TARGET_START: u64 = 100;
    const LONG_TARGET_DURATION: u64 = 4_000;
    const SHORT_TARGET_START: u64 = 300;
    const SHORT_TARGET_DURATION: u64 = 800;
    const LONG_OFFTARGET_START: u64 = 400;
    const LONG_OFFTARGET_DURATION: u64 = 3_000;
    const SHORT_OFFTARGET_START: u64 = 600;
    const SHORT_OFFTARGET_DURATION: u64 = 1200;
    let conf: Conf = serde_yaml::from_str(&format!(
        r#"
        adc:
            sample-rate-hz: {F_SAMPLE}
            chunk-size: 512
            ds-offset: 2047.5
        analysis:
            order-tracking:
                max-order: 100
                order-resolution: 0.01
            bands:
                low-order: 0.5..5.0
                mid-hz: ..5000
                high-hz: 5000..10000
    "#
    ))
    .unwrap();
    let target_bin_idx = (TARGET_ORDER * conf.analysis.fft_turns() as f64).round() as usize;
    let num_bins = conf.analysis.n_fft() / 2;
    let mut spectrum_model = SpectrumModel::new(A0, total_frames, target_bin_idx, num_bins, trend_multiplier, floor_noise);
    // ===================== РЕГИСТРАЦИЯ ПОМЕХ =====================
    // // 1. Долгая помеха на целевом бине
    // spectrum_model.add_target_disturbance(SpectralDisturbance {
    //     start_frame: LONG_TARGET_START,
    //     duration_frames: LONG_TARGET_DURATION,
    //     amplitude_factor: LONG_TARGET_AMP,
    //     shape: ImpulseShape::Rectangular,
    // });
    // // 2. Короткая помеха на целевом бине
    // spectrum_model.add_target_disturbance(SpectralDisturbance {
    //     start_frame: SHORT_TARGET_START,
    //     duration_frames: SHORT_TARGET_DURATION,
    //     amplitude_factor: SHORT_TARGET_AMP,
    //     shape: ImpulseShape::Rectangular,
    // });
    // // 3. Постоянная внеполосная помеха на правом соседнем бине (весь тест)
    // spectrum_model.add_out_of_band_disturbance(target_bin_idx + 1, SpectralDisturbance {
    //     start_frame: 0,
    //     duration_frames: total_frames,
    //     amplitude_factor: CONST_NEIGHBOR_AMP,
    //     shape: ImpulseShape::Rectangular,
    // });
    // // 4. Долгая помеха на левом соседнем бине
    // spectrum_model.add_out_of_band_disturbance(target_bin_idx + 1, SpectralDisturbance {
    //     start_frame: LONG_OFFTARGET_START,
    //     duration_frames: LONG_OFFTARGET_DURATION,
    //     amplitude_factor: LONG_NEIGHBOR_AMP,
    //     shape: ImpulseShape::Triangular,
    // });
    // // 5. Короткая помеха на левом соседнем бине
    // spectrum_model.add_out_of_band_disturbance(target_bin_idx - 1, SpectralDisturbance {
    //     start_frame: SHORT_OFFTARGET_START,
    //     duration_frames: SHORT_OFFTARGET_DURATION,
    //     amplitude_factor: SHORT_NEIGHBOR_AMP,
    //     shape: ImpulseShape::Sinusoidal,
    // });
    // ===================== ПОДГОТОВКА КОНВЕЙЕРА =====================
    let mut fft_window: Vec<Complex<f32>> = Vec::with_capacity(num_bins);
    let order_features_filter = OrderFeatureFilter::new(&dbg, conf.analysis.n_fft(), 0.02454, Pass::new());
    let retain = Arc::new(Retain::mock(&dbg, []));
    // retain.run().unwrap();   // Раскоментировать если в retain уходит много изменений (>16384)
    let mut i_ctx = ImbContext::new(
        &dbg,
        conf.analysis.samples_per_rev(),
        conf.analysis.n_fft(),
        conf.adc.chunk_size,
        retain.clone(),
    );
    // ===================== СИМУЛЯЦИЯ =====================
    let mut history: Vec<(u64, f64, String)> = Vec::new();
    let mut raw_history: Vec<(u64, f64)> = Vec::new();
    let mut raw_at_4100: Option<f64> = None; // ← сохраним "точку отсчёта" сырого сигнала
    for i in 0..total_frames {
        log::debug!("Frame: {i}/{total_frames}");
        spectrum_model.generate_frame(i, &mut fft_window);
        raw_history.push((i, fft_window[target_bin_idx].re as f64));
        i_ctx.fft_window = fft_window;
        i_ctx = order_features_filter.eval(i_ctx);

        fft_window = i_ctx.fft_window;
        if let Some(feat) = i_ctx.features.iter().find(|f| f.order_id == "1x") {
            history.push((i, feat.rms.0, feat.order_id.clone()));
        }
    }
    // ===================== ЭКСПОРТ В CSV ДЛЯ ГРАФИКОВ =====================
    let mut raw_file = BufWriter::new(File::create("/home/debian/Documents/python_test/raw.csv").unwrap());
    writeln!(raw_file, "frame,value").unwrap();
    for (i, v) in &raw_history {
        writeln!(raw_file, "{i},{v}").unwrap();
    }
    let mut filtered_file = BufWriter::new(File::create("/home/debian/Documents/python_test/filtered.csv").unwrap());
    writeln!(filtered_file, "frame,value").unwrap();
    for (i, v, _) in &history {
        writeln!(filtered_file, "{i},{v}").unwrap();
    }
    // ===================== Проверка A — фильтр адресован правильному ордеру =====================
    // Каждая запись в i_ctx.features с order_id == "1x" должна соответствовать order == Order(1.0).
    // Проверяем, что нет путаницы между гармониками (0.5x, 1.0x, 1.5x...).
    assert!(
        !history.is_empty(),
        "Ни разу не найдена запись с order_id='1x' — фильтр либо не сработал вообще, \
         либо порог никогда не пробивается (проверьте NaN-баг в saving_threshold)"
    );
    for (frame, _rms, order_id) in &history {
        assert_eq!(
            order_id, "1x",
            "На кадре {frame} найден неожиданный order_id: {order_id} (ожидался '1x')"
        );
    }
    // -2 randX
    // в +2 бине 50х
    // ===================== ЭТАЛОННЫЕ ЗНАЧЕНИЯ ДЛЯ ASSERT-ОВ =====================
    let amplitude_factor = 2f64.sqrt() / conf.analysis.n_fft() as f64;
    // ===================== Проверка B — на "шумных" участках фильтр не выдаёт искажённые значения =====================
    // Идея: для каждого кадра, где фильтр реально что-то вернул (есть в history),
    // проверяем — если этот кадр попадает в окно действия ЛЮБОЙ помехи на нецелевых
    // (внеполосных) бинах — значение НЕ должно заметно отличаться от "чистого" тренда,
    // посчитанного БЕЗ учёта этих внеполосных помех (сама помеха на целевом бине
    // в "чистый" расчёт как раз включена — она часть легитимного сигнала, а не шум).
    // Шаг 1 — собираем окна действия ВНЕПОЛОСНЫХ помех (влияют на соседние бины,
    // не должны влиять на "1x"). Целевые помехи (LONG_TARGET/SHORT_TARGET) сюда
    // не входят — они часть самого целевого сигнала, а не "шум для изоляции".
    let offtarget_windows: [(u64, u64); 2] = [
        (LONG_OFFTARGET_START, LONG_OFFTARGET_START + LONG_OFFTARGET_DURATION),
        (SHORT_OFFTARGET_START, SHORT_OFFTARGET_START + SHORT_OFFTARGET_DURATION),
    ];
    for (frame, rms, _order_id) in &history {
        // Шаг 2 — проверяем, попадает ли ТЕКУЩИЙ кадр в окно действия
        // хотя бы одной внеполосной помехи. Если нет — этот кадр не относится
        // к Проверке B, пропускаем.
        let in_offtarget_window = offtarget_windows
            .iter()
            .any(|(start, end)| frame >= start && frame < end);
        if !in_offtarget_window {
            continue;
        }
        // Шаг 3 — считаем "чистую" амплитуду ЦЕЛЕВОГО бина на этом кадре.
        let mut clean_target_amp = A0 * trend_multiplier.powf(*frame as f64);
        let long_target_active = *frame >= LONG_TARGET_START
            && *frame < LONG_TARGET_START + LONG_TARGET_DURATION;
        if long_target_active {
            clean_target_amp += LONG_TARGET_AMP * A0;
        }
        let short_target_active = *frame >= SHORT_TARGET_START
            && *frame < SHORT_TARGET_START + SHORT_TARGET_DURATION;
        if short_target_active {
            clean_target_amp += SHORT_TARGET_AMP * A0;
        }
        // Шаг 4 — считаем амплитуду ЛЕВОГО соседа (target_bin_idx - 1) БЕЗ внеполосных
        // помех — то есть как будто LONG_OFFTARGET/SHORT_OFFTARGET не существует.
        // Это и есть "эталон изоляции": что должно быть, если фильтр полностью
        // игнорирует внеполосный шум.
        let left_amp_clean = floor_noise; // без LONG_OFFTARGET/SHORT_OFFTARGET
        // Шаг 5 — амплитуда ПРАВОГО соседа (target_bin_idx + 1) — там всегда активна
        // CONST_NEIGHBOR_AMP (весь тест), это НЕ "шум для изоляции" в данной проверке,
        // а постоянный фон, который в любом случае присутствует.
        let right_amp = floor_noise + CONST_NEIGHBOR_AMP * A0;
        // Шаг 6 — собираем RMS сумма квадратов трёх бинов 
        // (leakage=3 → ±1 бин), корень, умножить на amplitude_factor.
        let expected_clean_rms = (clean_target_amp.powi(2)
            + left_amp_clean.powi(2)
            + right_amp.powi(2))
        .sqrt()
            * amplitude_factor;
        // Шаг 7 — сравниваем с реальным значением из history.
        let deviation = (rms - expected_clean_rms).abs() / expected_clean_rms;
        assert!(
            deviation < 0.06, // допуск — согласовать с начальником
            "На кадре {frame} (внеполосная помеха активна) значение отклоняется от \
             ожидаемого без учёта внеполосного шума: rms={rms}, expected_clean={expected_clean_rms}, \
             deviation={deviation}"
        );
    }
    // ===================== Проверка C — тренд к концу теста отследился =====================
    // Идея: берём САМОЕ ПОСЛЕДНЕЕ срабатывание фильтра (history.last()) и проверяем,
    // что оно (а) не искажено активной в этот момент помехой, (б) относится к нужному
    // ордеру, (в) численно соответствует ожидаемому росту тренда +1.5%.
    let (last_frame, last_rms, last_order_id) = history
        .last()
        .expect("к концу теста должно быть хотя бы одно срабатывание");
    // Шаг 1 — собираем ВСЕ окна помех (и целевые, и внеполосные), чтобы убедиться,
    // что last_frame не попадает ни под одну из них. Если попадает — мы не можем
    // быть уверены, что видим именно тренд, а не смесь тренда с помехой.
    let episodic_disturbance_windows: [(u64, u64); 4] = [
        (LONG_TARGET_START, LONG_TARGET_START + LONG_TARGET_DURATION),
        (SHORT_TARGET_START, SHORT_TARGET_START + SHORT_TARGET_DURATION),
        (LONG_OFFTARGET_START, LONG_OFFTARGET_START + LONG_OFFTARGET_DURATION),
        (SHORT_OFFTARGET_START, SHORT_OFFTARGET_START + SHORT_OFFTARGET_DURATION),
    ];
    let last_frame_in_noise = episodic_disturbance_windows
        .iter()
        .any(|(start, end)| last_frame >= start && last_frame < end);
    assert!(
        !last_frame_in_noise,
        "Последнее срабатывание (frame {last_frame}) попадает в окно действия эпизодической \
         помехи — результат недостоверен. Необходимо либо сдвинуть помехи раньше или увеличить total_frames \
         (сейчас LONG_TARGET/LONG_OFFTARGET занимают большую часть теста)."
    );
    // Шаг 2 — проверяем, что это действительно нужный ордер.
    assert_eq!(
        last_order_id, "1.0x",
        "Последнее срабатывание относится не к тому ордеру: {last_order_id}"
    );
    // Шаг 3 — считаем ожидаемую "чистую" амплитуду целевого бина НА ПОСЛЕДНЕМ КАДРЕ.
    // На last_frame эпизодических помех уже нет (проверили в Шаге 1), значит это
    // ровно базовый тренд в его финальной точке: A0 * TREND_GROWTH.
    let target_amp_final = A0 * TREND_GROWTH;
    // Шаг 4 — амплитуда левого соседа на last_frame: эпизодических помех там уже
    // нет (LONG_OFFTARGET/SHORT_OFFTARGET закончились), остаётся только floor_noise.
    let left_amp_final = floor_noise;
    // Шаг 5 — амплитуда правого соседа: CONST_NEIGHBOR_AMP активна всегда, включая
    // last_frame.
    let right_amp_final = floor_noise + CONST_NEIGHBOR_AMP * A0;
    // Шаг 6 — собираем ожидаемый RMS (3 бина, leakage=3).
    let expected_final_rms = (target_amp_final.powi(2)
        + left_amp_final.powi(2)
        + right_amp_final.powi(2))
    .sqrt()
        * amplitude_factor;
    // Шаг 7 — сравниваем с реальным последним значением.
    let deviation = (last_rms - expected_final_rms).abs() / expected_final_rms;
    assert!(
        deviation < 0.05,
        "Финальное значение не соответствует ожидаемому росту тренда +{TREND_GROWTH}%: \
         last_frame={last_frame}, last_rms={last_rms}, expected={expected_final_rms}, \
         deviation={deviation}"
    );
}

