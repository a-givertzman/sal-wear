use crate::{
    Conf, Eval, ImbContext, OrderFeatureFilter, Pass, Retain,
    tests::order_domain::order_features_filter_test::entities::{
        ImpulseShape, SpectralDisturbance, SpectrumModel,
    },
};
use debugging::session::debug_session::{
    DebugSession, 
    LogLevel
};
use rustfft::num_complex::Complex;
use sal_core::dbg::Dbg;
use sal_sync::services::Service;
use std::sync::Arc;
/// Шаг времени фильтра Калмана (не кадр АЦП!).
/// fft_turns = next_pow2(ceil(1 / order_resolution))
/// f_step = rpm / (60 * fft_turns)   [шагов/с]
/// T_step = 1 / f_step               [с/шаг]
fn compute_t_step(order_resolution: f64, rpm: f64) -> f64 {
    let raw = (1.0 / order_resolution).ceil() as u64;
    let fft_turns = raw.next_power_of_two() as f64;
    let f_step = rpm / (60.0 * fft_turns);
    1.0 / f_step
}

#[test]
fn order_features_filter_stationary_test() {
    DebugSession::new().filter(LogLevel::Debug).init();
    let dbg = Dbg::own("OrderFeaturesFilter-test");
    // ===================== МАСШТАБ =====================
    const ORDER_RESOLUTION: f64 = 0.01;
    const RPM: f64 = 1650.0;
    const FULL_TEST_DAYS: f64 = 30.0;
    let t_step = compute_t_step(ORDER_RESOLUTION, RPM);
    let total_frames = ((FULL_TEST_DAYS * 24.0 * 3600.0) / t_step).round() as u64;
    // ===================== ФИЗИЧЕСКИЕ ПАРАМЕТРЫ =====================
    const A0: f64 = 200.0;
    const TARGET_ORDER: f64 = 1.0;
    const TREND_GROWTH: f64 = 1.015; // ожидаемый рост тренда за весь тест
    const F_SAMPLE: u64 = 320_000;
    const WINDOW: usize = 10000;
    const Q: f64 = 1e-15;
    let trend_multiplier = TREND_GROWTH.powf(1.0 / total_frames as f64);
    let conf: Conf = serde_yaml::from_str(&format!(
        r#"
        adc:
            sample-rate-hz: {F_SAMPLE}
            chunk-size: 512
            ds-offset: 2047.5
        analysis:
            order-tracking:
                max-order: 100
                order-resolution: {ORDER_RESOLUTION}
            bands:
                low-order: 0.5..5.0
                mid-hz: ..5000
                high-hz: 5000..10000
    "#
    )).unwrap();
    let target_bin_idx = (TARGET_ORDER * conf.analysis.fft_turns() as f64).round() as usize;
    let num_bins = conf.analysis.n_fft() / 2;
    let amplitude_factor = 2f64.sqrt() / conf.analysis.n_fft() as f64;
    // ===================== КОНФИГУРАЦИЯ ПОМЕХ =====================
    let mut spectrum_model = SpectrumModel::new(A0, total_frames, target_bin_idx, num_bins, trend_multiplier, 0.0);
    // Короткий пуск на целевом бине
    spectrum_model.add_target_disturbance(SpectralDisturbance {
        start_frame: total_frames / 4,
        duration_frames: 3,
        amplitude_factor: 5.0,
        shape: ImpulseShape::Rectangular,
    });
    // Длительность 8-часовой помехи в шагах фильтра
    let hours_8_in_frames = ((8.0 * 3600.0) / t_step).round() as u64;
    // 8-часовая помеха на целевом бине
    spectrum_model.add_target_disturbance(SpectralDisturbance {
        start_frame: total_frames / 2,
        duration_frames: hours_8_in_frames,
        amplitude_factor: 2.0,
        shape: ImpulseShape::Rectangular,
    });
    // Постоянная помеха на соседнем бине
    spectrum_model.add_out_of_band_disturbance(target_bin_idx + 2, SpectralDisturbance {
        start_frame: 0,
        duration_frames: total_frames,
        amplitude_factor: 0.5,
        shape: ImpulseShape::Rectangular,
    });
    // ===================== СИМУЛЯЦИЯ =====================
    let mut fft_window: Vec<Complex<f32>> = Vec::with_capacity(num_bins);
    let order_features_filter = OrderFeatureFilter::new(&dbg, conf.analysis.n_fft(), 0.02454, Pass::new());
    let retain = Arc::new(Retain::mock(&dbg, []));
    retain.run().unwrap();
    let mut i_ctx = ImbContext::new_with_params(
        &dbg,
        conf.analysis.samples_per_rev(),
        conf.analysis.n_fft(),
        conf.adc.chunk_size,
        retain.clone(),
        WINDOW,
        Q,
    );
    // Единственный результат — все реальные срабатывания saving_threshold
    let mut result: Vec<(u64, f64)> = Vec::new();
    let mut all_order_ids: Vec<String> = Vec::new(); // для проверки 1 — собираем ВСЕ order_id, что вообще срабатывали
    for i in 0..total_frames {
        spectrum_model.generate_frame(i, &mut fft_window);
        i_ctx.fft_window = fft_window;
        i_ctx = order_features_filter.eval(i_ctx);
        fft_window = i_ctx.fft_window;
        for feat in &i_ctx.features {
            all_order_ids.push(feat.order_id.clone());
            if feat.order_id == "1x" {
                result.push((i, feat.rms.0));
            }
        }
    }
    log::debug!("Всего срабатываний на '1x': {}", result.len());
    for (frame, rms) in &result {
        log::debug!("  frame={frame}, rms={rms}");
    }
    // ===================== ASSERT 1 — срабатывал ТОЛЬКО нужный ордер =====================
    for order_id in &all_order_ids {
        assert_eq!(
            order_id, "1x",
            "Обнаружено срабатывание постороннего ордера: {order_id}"
        );
    }
    // ===================== ASSERT 2 — сработал не больше N раз =====================
    const MAX_ALLOWED_TRIGGERS: usize = 5;
    assert!(
        result.len() <= MAX_ALLOWED_TRIGGERS,
        "Слишком много срабатываний: {} (допустимо не более {MAX_ALLOWED_TRIGGERS})",
        result.len()
    );
    // ===================== ASSERT 3 — последнее значение соответствует заданному росту тренда =====================
    const SAVING_THRESHOLD: f64 = 0.01;
    let expected_trigger_frame = (
        (1.0 + SAVING_THRESHOLD).ln() / TREND_GROWTH.ln() * total_frames as f64
    ).round() as u64;
    log::debug!(
        "Ожидаемый кадр срабатывания (чистый тренд достиг +{}%): {}",
        SAVING_THRESHOLD * 100.0, expected_trigger_frame
    );
    // Допуск — сколько кадров разницы считаем приемлемым
    let tolerance_frames = total_frames / 20; // ±5% от всего теста
    let (last_frame, _) = *result.last().expect("хотя бы одно срабатывание должно быть");
    assert!(
        (last_frame as i64 - expected_trigger_frame as i64).unsigned_abs() < tolerance_frames,
        "Срабатывание произошло не в момент, когда чистый тренд достиг +{}%: \
        last_frame={last_frame}, expected≈{expected_trigger_frame} (допуск ±{tolerance_frames}). \
        Вероятно, момент спровоцирован помехой, а не честным накоплением тренда.",
        SAVING_THRESHOLD * 100.0
    );
}