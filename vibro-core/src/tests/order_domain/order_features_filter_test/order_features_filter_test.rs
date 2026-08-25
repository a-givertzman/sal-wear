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
use std::fs::File;
use std::io::{BufWriter, Write};
// ===================== ПАРАМЕТРЫ КОНФИГУРАЦИИ =====================
struct TestConfig {
    name: &'static str,
    /// Ожидаемый рост тренда за весь тест
    trend_growth: f64,
    /// Постоянная помеха на соседнем бине (доля A0). Активна весь тест.
    const_neighbor_amp: f64,
    /// Целевая помеха на целевом бине: (старт, длительность в кадрах, амплитуда·A0).
    /// None — помеха не регистрируется.
    target_disturbance: Option<(u64, u64, f64)>,
    /// Внеполосная помеха на соседнем бине: (старт, длительность, амплитуда·A0).
    offtarget_disturbance: Option<(u64, u64, f64)>,
    criterion: &'static str,
}
// ===================== РЕЗУЛЬТАТ ТЕСТИРОВАНИЯ =====================
struct ConfigResult {
    name: String,
    failures: Vec<String>,
    history_len: usize,
    last_frame: Option<u64>,
    last_rms: Option<f64>,
}
///
/// Тест [OrderFeatureFilter]
#[test]
fn order_features_filter_stationary_test() {
    DebugSession::new().filter(LogLevel::Debug).init();
    let dbg = Dbg::own("OrderFeaturesFilter-test");
    // ===================== МАСШТАБ СИМУЛЯЦИИ =====================
    const ITER_PER_DAY: f64 = 54_000_000.0; // кол-во итерация, описывающие день работы
    const COUNT_DAYS: f64 = 0.001; // кол-во дней симуляции
    let total_frames: u64 = (ITER_PER_DAY * COUNT_DAYS).round() as u64;
    const FULL_TEST_HOURS: f64 = 30.0 * 24.0;
    let hours_8_in_frames: u64 = ((8.0 / FULL_TEST_HOURS) * total_frames as f64).round() as u64;
    const A0: f64 = 200.0;
    const TARGET_ORDER: f64 = 1.0;
    const F_SAMPLE: u64 = 320_000;
    let floor_noise = 0.0;
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
    // ===================== НАБОР КОНФИГУРАЦИЙ =====================
    let short_impulse_frames = 2; // короткая помеха (несколько мс = 1-2 кадра)
    let below_8h = hours_8_in_frames.saturating_sub(hours_8_in_frames / 10).max(1);
    let configs = vec![
        // // ---- детекция тренда ----
        // TestConfig {
        //     name: "trend_detection_baseline",
        //     trend_growth: 1.015,
        //     const_neighbor_amp: 0.0,
        //     target_disturbance: None,
        //     offtarget_disturbance: None,
        //     criterion: "2 (детекция тренда +1.5%)",
        // },
        // ---- фильтрация коротких импульсов ----
        TestConfig {
            name: "impulse_short_target",
            trend_growth: 1.015,
            const_neighbor_amp: 0.0,
            target_disturbance: Some((total_frames / 4, short_impulse_frames, 5.0)),
            offtarget_disturbance: None,
            criterion: "1 (короткий импульс не регистрируется)",
        },
        // // ---- помеха длительностью < 8ч тоже не должна регистрироваться ----
        // TestConfig {
        //     name: "impulse_below_8h",
        //     trend_growth: 1.015,
        //     const_neighbor_amp: 0.0,
        //     target_disturbance: Some((total_frames / 4, below_8h, 3.0)),
        //     offtarget_disturbance: None,
        //     criterion: "1 (возмущение <8ч не регистрируется)",
        // },
        // // ---- изоляция от постоянного внеполосного фона ----
        // TestConfig {
        //     name: "isolation_const_neighbor_0_5",
        //     trend_growth: 1.015,
        //     const_neighbor_amp: 0.5,
        //     target_disturbance: None,
        //     offtarget_disturbance: None,
        //     criterion: "3 (изоляция от постоянного соседа)",
        // },
        // // ---- изоляция от сильной, но короткой внеполосной помехи ----
        // TestConfig {
        //     name: "isolation_short_offtarget",
        //     trend_growth: 1.015,
        //     const_neighbor_amp: 0.0,
        //     target_disturbance: None,
        //     offtarget_disturbance: Some((total_frames / 4, short_impulse_frames, 5.0)),
        //     criterion: "3 (изоляция от короткого соседа)",
        // },
    ];
    let target_bin_idx = (TARGET_ORDER * conf.analysis.fft_turns() as f64).round() as usize;
    let num_bins = conf.analysis.n_fft() / 2;
    let amplitude_factor = 2f64.sqrt() / conf.analysis.n_fft() as f64;
    let csv_dir = "/home/debian/Documents/python_test";
    let mut results: Vec<ConfigResult> = Vec::new();
    for cfg in &configs {
        log::debug!("===== Конфигурация: {} (критерий {}) =====", cfg.name, cfg.criterion);
        let trend_multiplier = cfg.trend_growth.powf(1.0 / total_frames as f64);
        let mut spectrum_model =
            SpectrumModel::new(A0, total_frames, target_bin_idx, num_bins, trend_multiplier, floor_noise);
        // Целевая помеха
        if let Some((start, dur, amp)) = cfg.target_disturbance {
            spectrum_model.add_target_disturbance(SpectralDisturbance {
                start_frame: start,
                duration_frames: dur,
                amplitude_factor: amp,
                shape: ImpulseShape::Rectangular,
            });
        }
        // Постоянный внеполосный фон (правый сосед)
        if cfg.const_neighbor_amp > 0.0 {
            spectrum_model.add_out_of_band_disturbance(target_bin_idx + 2, SpectralDisturbance {
                start_frame: 0,
                duration_frames: total_frames,
                amplitude_factor: cfg.const_neighbor_amp,
                shape: ImpulseShape::Rectangular,
            });
        }
        // Эпизодическая внеполосная помеха (левый сосед)
        if let Some((start, dur, amp)) = cfg.offtarget_disturbance {
            spectrum_model.add_out_of_band_disturbance(target_bin_idx - 2, SpectralDisturbance {
                start_frame: start,
                duration_frames: dur,
                amplitude_factor: amp,
                shape: ImpulseShape::Sinusoidal,
            });
        }
        // ---- Симуляция ----
        let mut fft_window: Vec<Complex<f32>> = Vec::with_capacity(num_bins);
        let order_features_filter =
            OrderFeatureFilter::new(&dbg, conf.analysis.n_fft(), 0.02454, Pass::new());
        let retain = Arc::new(Retain::mock(&dbg, []));
        retain.run().unwrap();   // Раскоментировать если в retain уходит много изменений (>16384)
        let mut i_ctx = ImbContext::new(
            &dbg,
            conf.analysis.samples_per_rev(),
            conf.analysis.n_fft(),
            conf.adc.chunk_size,
            retain.clone(),
        );
        let mut history: Vec<(u64, f64, String)> = Vec::new();
        let mut raw_history: Vec<(u64, f64)> = Vec::new();
        // ====================== ОСНОВНОЙ ЦИКЛ СИМУЛЯЦИИ ======================
        for i in 0..total_frames {
            spectrum_model.generate_frame(i, &mut fft_window);
            raw_history.push((i, fft_window[target_bin_idx].re as f64));
            i_ctx.fft_window = fft_window;
            i_ctx = order_features_filter.eval(i_ctx);
            fft_window = i_ctx.fft_window;
            if let Some(feat) = i_ctx.features.iter().find(|f| f.order_id == "1x") {
                history.push((i, feat.rms.0, feat.order_id.clone()));
            }
        }
        // ---- Экспорт CSV (по имени конфигурации) ----
        // let mut raw_file = BufWriter::new(File::create(format!("{csv_dir}/raw_{}.csv", cfg.name)).unwrap());
        // writeln!(raw_file, "frame,value").unwrap();
        // for (i, v) in &raw_history {
        //     writeln!(raw_file, "{i},{v}").unwrap();
        // }
        // let mut filtered_file = BufWriter::new(File::create(format!("{csv_dir}/filtered_{}.csv", cfg.name)).unwrap());
        // writeln!(filtered_file, "frame,value").unwrap();
        // for (i, v, _) in &history {
        //     writeln!(filtered_file, "{i},{v}").unwrap();
        // }
        // ---- Проверки для этой конфигурации ----
        let mut failures: Vec<String> = Vec::new();
        // === Проверка A — правильный ордер (общая для всех) ===
        if history.is_empty() {
            failures.push(format!("[{}] A: history пуста — фильтр ни разу не сработал", cfg.name));
        }
        for (frame, _rms, order_id) in &history {
            if order_id != "1x" {
                failures.push(format!("[{}] A: кадр {frame}, неверный order_id={order_id}", cfg.name));
            }
        }
        // === Критерий 1 — короткое возмущение НЕ должно регистрироваться ===
        // Проверяем: в окне действия целевой помехи (если она короткая) значение x_hat
        // не должно скакнуть — фильтр обязан её подавить. Сравниваем с "чистым" трендом
        // без учёта самой помехи.
        if cfg.criterion.starts_with("1") {
            if let Some((start, dur, _amp)) = cfg.target_disturbance {
                for (frame, rms, _) in &history {
                    if *frame < start || *frame >= start + dur {
                        continue;
                    }
                    // "чистый" тренд без импульса
                    let clean_amp = A0 * trend_multiplier.powf(*frame as f64);
                    let expected_clean = clean_amp * amplitude_factor;
                    let deviation = (rms - expected_clean).abs() / expected_clean;
                    if deviation >= 0.10 {
                        failures.push(format!(
                            "[{}] Критерий 1: кадр {frame}, импульс просочился — rms={rms}, expected_clean={expected_clean}, dev={deviation}",
                            cfg.name
                        ));
                    }
                }
            }
        }
        // === Критерий 3 — внеполосная помеха НЕ должна влиять на целевой ордер ===
        if cfg.criterion.starts_with("3") {
            if let Some((start, dur, _amp)) = cfg.offtarget_disturbance {
                for (frame, rms, _) in &history {
                    if *frame < start || *frame >= start + dur {
                        continue;
                    }
                    let clean_amp = A0 * trend_multiplier.powf(*frame as f64);
                    let right_amp = cfg.const_neighbor_amp * A0;
                    let expected_clean = (clean_amp.powi(2) + right_amp.powi(2)).sqrt() * amplitude_factor;
                    let deviation = (rms - expected_clean).abs() / expected_clean;
                    if deviation >= 0.06 {
                        failures.push(format!(
                            "[{}] Критерий 3: кадр {frame}, внеполосная помеха повлияла — rms={rms}, expected_clean={expected_clean}, dev={deviation}",
                            cfg.name
                        ));
                    }
                }
            }
            // Для постоянного фона (const_neighbor) — проверяем на всём протяжении
            if cfg.const_neighbor_amp > 0.0 && cfg.offtarget_disturbance.is_none() {
                for (frame, rms, _) in &history {
                    let clean_amp = A0 * trend_multiplier.powf(*frame as f64);
                    let right_amp = cfg.const_neighbor_amp * A0;
                    let expected = (clean_amp.powi(2) + right_amp.powi(2)).sqrt() * amplitude_factor;
                    let deviation = (rms - expected).abs() / expected;
                    if deviation >= 0.06 {
                        failures.push(format!(
                            "[{}] Критерий 3 (пост. фон): кадр {frame}, rms={rms}, expected={expected}, dev={deviation}",
                            cfg.name
                        ));
                    }
                }
            }
        }
        // === Критерий 2 — тренд к концу теста должен быть зарегистрирован ===
        if cfg.criterion.starts_with("2") {
            if let Some((last_frame, last_rms, last_order_id)) = history.last() {
                if last_order_id != "1x" {
                    failures.push(format!("[{}] Критерий 2: last_order_id={last_order_id}", cfg.name));
                }
                let target_amp_final = A0 * cfg.trend_growth;
                let right_amp_final = cfg.const_neighbor_amp * A0;
                let expected_final = (target_amp_final.powi(2) + right_amp_final.powi(2)).sqrt() * amplitude_factor;
                let deviation = (last_rms - expected_final).abs() / expected_final;
                if deviation >= 0.05 {
                    failures.push(format!(
                        "[{}] Критерий 2: last_frame={last_frame}, last_rms={last_rms}, expected={expected_final}, dev={deviation}",
                        cfg.name
                    ));
                }
            }
        }
        log::debug!(
            "  history_len={}, failures={}",
            history.len(), failures.len()
        );
        results.push(ConfigResult {
            name: cfg.name.to_string(),
            failures,
            history_len: history.len(),
            last_frame: history.last().map(|(f, _, _)| *f),
            last_rms: history.last().map(|(_, r, _)| *r),
        });
    }
    // ===================== ИТОГОВЫЙ ОТЧЁТ =====================
    log::debug!("\n===== СВОДКА ПО КОНФИГУРАЦИЯМ =====");
    for r in &results {
        log::debug!(
            "  {}: срабатываний={}, last_frame={:?}, last_rms={:?}, нарушений={}",
            r.name, r.history_len, r.last_frame, r.last_rms, r.failures.len()
        );
    }
    let all_failures: Vec<String> = results.into_iter().flat_map(|r| r.failures).collect();
    assert!(
        all_failures.is_empty(),
        "\n===== ПРОВАЛЕННЫЕ ПРОВЕРКИ ({}) =====\n{}",
        all_failures.len(),
        all_failures.join("\n")
    );
}