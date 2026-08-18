use crate::{
    Conf, Eval, ImbContext, OrderFeatureFilter, Pass, Phase, Retain,
    tests::order_domain::order_features_filter_test::entities::{
        ImpulseShape, SpectralDisturbance, SpectrumModel,
    },
};
use debugging::session::debug_session::{DebugSession, LogLevel};
use rustfft::num_complex::Complex;
use sal_core::dbg::Dbg;
use std::sync::Arc;

///
/// Функциональное тестирование [OrderFeaturesFilter]
#[test]
fn order_features_filter_stationary_test() {
    DebugSession::new().filter(LogLevel::Debug).init();
    let dbg = Dbg::own("OrderFeaturesFilter-test");
    // Экспоненциальный множитель на одну итерацию:
    // Тк Общее количество фреймов за 30 дней = 1_620_000
    // А к последней итерации конечная амплитуда = 1.5% + начальная амплитуда
    let f_sample = 320_000; // Частота семплирования АЦП (Гц)
    let conf: Conf = serde_yaml::from_str(&format!(
        r#"
        adc:
            sample-rate-hz: {f_sample}
            chunk-size: 512
        analysis:
            order-tracking:
                max-order: 100
                order-resolution: 0.01
                # samples-per-rev:
            bands:
                low-order: 0.5..5.0
                mid-hz: ..5000
                high-hz: 5000..10000
    "#
    ))
    .unwrap();
    // начальная амплитуда
    let a0 = 200.0;
    // исследуемый ордер
    let target_order = 1.0;
    // исследуемой индекс бина для исследуемого ордера
    let target_bin_idx = (target_order * conf.analysis.fft_turns() as f64).round() as usize;
    // теорема Найквиста-Котельникова
    let num_bins = conf.analysis.n_fft() / 2;
    // Модель тестового воздействия
    let mut spectrum_model = SpectrumModel::new(a0, 1_620_000, target_bin_idx, num_bins);
    // =========================================================================
    // НАСТРОЙКА И ДОБАВЛЕНИЕ ВСЕХ ПОМЕХ ПЕРЕД ЦИКЛОМ (Пункт 5.4)
    // =========================================================================
    // 1. Длинная 8-часовая прямоугольная помеха на целевом бине
    let long_target_disturbance = SpectralDisturbance {
        start_frame: 100,
        duration_frames: 18_000,
        amplitude_factor: 3.0,
        shape: ImpulseShape::Rectangular,
    };
    spectrum_model.add_target_disturbance(long_target_disturbance);
    // 2. Короткая помеха на целевом бине (проверка устойчивости к микро-импульсам)
    let short_target_disturbance = SpectralDisturbance {
        start_frame: 500_000,
        duration_frames: 2,
        amplitude_factor: 5.0,
        shape: ImpulseShape::Rectangular,
    };
    spectrum_model.add_target_disturbance(short_target_disturbance);
    // 3. Постоянная внеполосная помеха (соседний бин справа). Критерий 5.5.3 (Изоляция гармоник)
    let constant_out_of_band = SpectralDisturbance {
        start_frame: 0,
        duration_frames: 1_620_000,
        amplitude_factor: 0.5,
        shape: ImpulseShape::Rectangular,
    };
    spectrum_model.add_out_of_band_disturbance(target_bin_idx + 1, constant_out_of_band);
    // 4. Длинная 8-часовая треугольная помеха на нецелевом бине (слева от целевого)
    let long_out_of_band = SpectralDisturbance {
        start_frame: 500_000,
        duration_frames: 18_000,
        amplitude_factor: 4.0,
        shape: ImpulseShape::Triangular,
    };
    spectrum_model.add_out_of_band_disturbance(target_bin_idx - 1, long_out_of_band);
    // 5. Короткая помеха на нецелевом бине (слева от целевого)
    let short_out_of_band = SpectralDisturbance {
        start_frame: 500_000,
        duration_frames: 10,
        amplitude_factor: 5.0,
        shape: ImpulseShape::Rectangular,
    };
    spectrum_model.add_out_of_band_disturbance(target_bin_idx - 1, short_out_of_band);
    // =========================================================================
    let mut fft_window: Vec<Complex<f32>> = Vec::<Complex<f32>>::with_capacity(num_bins);
    let order_features_filter =
        OrderFeatureFilter::new(&dbg, conf.analysis.n_fft(), 0.02454, Pass::new());
    let retain = Arc::new(Retain::mock(&dbg, []));
    let mut i_ctx = ImbContext::new(
        &dbg,
        conf.analysis.samples_per_rev(),
        conf.analysis.fft_turns(),
        retain.clone(),
    );
    let n_fft = conf.analysis.n_fft();
    let mut filtered_history: Vec<(u64, f64)> = Vec::new();
    // =========================================================================
    // Симуляция 3 дня работы
    // =========================================================================
    for i in 0..1_620_000u64 {
        log::debug!("Frame: {}", i);
        spectrum_model.generate_frame(i, &mut fft_window);
        i_ctx.total_phase = Phase(i as f64 * 0.02454);
        i_ctx.fft_window = fft_window;
        i_ctx = order_features_filter.eval(i_ctx);
        fft_window = i_ctx.fft_window;
        if let Some(feat) = i_ctx.features.iter().find(|f| f.order_id == "1x") {
            filtered_history.push((i, feat.rms.value()));
        }
    }
    // ===================== PROPERTY-BASED ПРОВЕРКИ =====================
    fn nearest(history: &[(u64, f64)], target: u64) -> (u64, f64) {
        *history
            .iter()
            .min_by_key(|(i, _)| i.abs_diff(target))
            .expect("history must not be empty")
    }
    assert!(!filtered_history.is_empty(), "фильтр ни разу не вернул значение");
    // ---------- 1. ОГРАНИЧЕННОСТЬ ----------
    // x_hat не может физически превышать максимально возможный вход.
    // Максимум по амплитуде дают: базовый тренд (макс ~a0*1.015) + самая большая
    // целевая помеха (long_target ×3, short_target ×5) + вклад соседей в leakage-окне.
    // Берём заведомо широкую верхнюю границу с запасом.
    let max_possible_target_amp = a0 * 1.015 + a0 * 5.0; // тренд + самая сильная целевая помеха
    let max_possible_neighbor_amp = a0 * 5.0; // самая сильная внеполосная (short_out_of_band ×5)
    let amplitude_factor = 2f64.sqrt() / n_fft as f64;
    let sum_sq_upper = max_possible_target_amp.powi(2) + 2.0 * max_possible_neighbor_amp.powi(2);
    let upper_bound = sum_sq_upper.sqrt() * amplitude_factor;
    for &(frame, val) in &filtered_history {
        assert!(
            val >= 0.0 && val <= upper_bound,
            "x_hat вышел за физически разумные границы на кадре {frame}: {val}, upper_bound={upper_bound}"
        );
    }
    // ---------- 2. МОНОТОННОСТЬ НА ЧИСТОМ УЧАСТКЕ ТРЕНДА ----------
    // Берём заведомо "чистый" диапазон без активных помех: после длинной целевой
    // помехи (>18100+запас на восстановление) и до начала помех у 500000.
    let clean_start = 40_000u64;
    let clean_end = 480_000u64;
    let clean_slice: Vec<&(u64, f64)> = filtered_history
        .iter()
        .filter(|(f, _)| *f >= clean_start && *f <= clean_end)
        .collect();
    assert!(clean_slice.len() > 2, "недостаточно точек на чистом участке для проверки монотонности");
    let mut non_monotonic_count = 0;
    for w in clean_slice.windows(2) {
        if w[1].1 < w[0].1 {
            non_monotonic_count += 1;
        }
    }
    // Допускаем единичные микро-колебания из-за floor_noise, но не систематическую немонотонность
    let non_monotonic_ratio = non_monotonic_count as f64 / clean_slice.len() as f64;
    assert!(
        non_monotonic_ratio < 0.05,
        "x_hat немонотонен на чистом участке тренда чаще, чем в 5% случаев: {non_monotonic_ratio}"
    );
    // ---------- 3. СХОДИМОСТЬ НА ДЛИННОЙ ЦЕЛЕВОЙ ПОМЕХЕ (frame 100..18100, ×3) ----------
    let (baseline_frame, baseline_val) = nearest(&filtered_history, 50);
    let (during_long_frame, during_long_val) = nearest(&filtered_history, 17_500);
    assert!(
        during_long_val > baseline_val * 1.5,
        "фильтр не отследил длинную целевую помеху: baseline(frame {baseline_frame})={baseline_val}, \
        during(frame {during_long_frame})={during_long_val}"
    );
    // ---------- 4. ВОЗВРАТ К ТРЕНДУ ПОСЛЕ ОКОНЧАНИЯ ДЛИННОЙ ПОМЕХИ ----------
    let (after_long_frame, after_long_val) = nearest(&filtered_history, 40_000);
    let ratio_to_baseline = after_long_val / baseline_val;
    assert!(
        ratio_to_baseline < 1.2,
        "фильтр не вернулся к тренду после окончания длинной помехи: baseline={baseline_val}, \
        after(frame {after_long_frame})={after_long_val}, ratio={ratio_to_baseline}"
    );
    // ---------- 5. ПОДАВЛЕНИЕ КОРОТКОГО ИМПУЛЬСА (frame 500000, 2 кадра, ×5) ----------
    let (pre_short_frame, pre_short_val) = nearest(&filtered_history, 499_500);
    let (during_short_frame, during_short_val) = nearest(&filtered_history, 500_001);
    let long_response_ratio = during_long_val / baseline_val;      // реакция на ДОЛГУЮ помеху (×3)
    let short_response_ratio = during_short_val / pre_short_val;    // реакция на КОРОТКУЮ помеху (×5)
    // Короткая помеха физически сильнее (×5 против ×3), но длится в 9000 раз меньше.
    // Если фильтр работает как задумано, относительная реакция на короткий импульс
    // должна быть заметно СЛАБЕЕ реакции на длинную помеху, несмотря на большую амплитуду.
    assert!(
        short_response_ratio < long_response_ratio,
        "короткий импульс (×5, 2 кадра) вызвал реакцию сильнее, чем длинная помеха (×3, 18000 кадров): \
        short_ratio={short_response_ratio} (frame {during_short_frame}, pre={pre_short_val} at {pre_short_frame}), \
        long_ratio={long_response_ratio}"
    );
    // ---------- 6. ИЗОЛЯЦИЯ ОТ ВНЕПОЛОСНЫХ ПОМЕХ ----------
    // В окне 500050..518000 у нас идёт long_out_of_band (треугольная, ×4) на соседнем
    // бине, а на целевом бине в это время (после frame 500002) ничего не происходит,
    // кроме базового тренда. x_hat не должен заметно реагировать на рост соседней помехи.
    let (before_oob_frame, before_oob_val) = nearest(&filtered_history, 499_000);
    let (mid_oob_frame, mid_oob_val) = nearest(&filtered_history, 509_000); // пик триугольной помехи
    let oob_deviation = (mid_oob_val - before_oob_val).abs() / before_oob_val;
    assert!(
        oob_deviation < 0.05,
        "внеполосная помеха (на соседнем бине) повлияла на целевой ордер сильнее 5%: \
        before(frame {before_oob_frame})={before_oob_val}, mid(frame {mid_oob_frame})={mid_oob_val}, \
        deviation={oob_deviation}"
    );
    // ---------- 7. ПОСТОЯННАЯ ВНЕПОЛОСНАЯ ПОМЕХА (constant_out_of_band, весь тест, ×0.5) ----------
    // Раз она активна всё время симметрично, её эффект (если он есть из-за leakage)
    // должен быть ПОСТОЯННЫМ смещением, а не расти/падать во времени.
    // Проверяем, что относительный разброс x_hat на разных "чистых" участках стабилен
    // (уже частично покрыто пунктом 2, здесь — доп. sanity-check на двух далёких точках).
    let (early_clean_frame, early_clean_val) = nearest(&filtered_history, 45_000);
    let (late_clean_frame, late_clean_val) = nearest(&filtered_history, 470_000);
    let trend_growth = (late_clean_val - early_clean_val) / early_clean_val;
    // Ожидаемый рост тренда за этот промежуток кадров — малая доля от полных 1.5%
    let expected_growth = 1.015f64.powf((late_clean_frame - early_clean_frame) as f64 / 1_620_000.0) - 1.0;
    assert!(
        (trend_growth - expected_growth).abs() < 0.01,
        "рост тренда на чистом участке не соответствует ожидаемому: \
        actual_growth={trend_growth}, expected_growth={expected_growth}, \
        early(frame {early_clean_frame})={early_clean_val}, late(frame {late_clean_frame})={late_clean_val}"
    );
}