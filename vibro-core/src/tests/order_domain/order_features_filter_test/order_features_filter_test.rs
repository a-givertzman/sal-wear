use crate::{
    AngularGrid, Autocorrelation, Conf, Context, Eval, Frame, ImbContext, Inputs, KalmanFilter, OrderDomainSamples, OrderFeatureFilter, OrderZone, Pass, ReadInputs, Retain, Retained, Rpm, ShortSigma, tests::{FftBuffer, Frequency, Udp, order_domain::order_features_filter_test::entities::{ImpulseShape, SpectralDisturbance, SpectrumModel}}
};
use chrono::Utc;
use debugging::session::debug_session::{DebugSession, LogLevel};
use rustfft::{FftPlanner, num_complex::Complex};
use sal_core::dbg::Dbg;
use std::sync::Arc;
///
/// Функциональное тестирование [OrderFeaturesFilter] на стационарность при разгоне
#[test]
fn order_features_filter_stationary_test() {
    DebugSession::new().filter(LogLevel::Debug).init();
    let dbg = Dbg::own("OrderFeaturesFilter-test");
    // Экспоненциальный множитель на одну итерацию:
    // Тк Общее количество фреймов за 30 дней = 1_620_000_000 
    // А к последней итерации конечная амплитуда = 15% + начальная амплитуда 
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
    // исследуемый ордер
    let target_order = 1.0;
    // исследуемой индекс бина для исследуемого ордера
    let target_bin_idx = (target_order * conf.analysis.fft_turns() as f64).round() as usize;
    // теорема Найквиста-Котельникова
    let num_bins = conf.analysis.n_fft() / 2;
    // Модель тестового воздействия
    let mut spectrum_model = SpectrumModel::new(
        200.0, 
        1_620_000_000, 
        target_bin_idx, 
        num_bins
    );
    // =========================================================================
    // НАСТРОЙКА И ДОБАВЛЕНИЕ ВСЕХ ПОМЕХ ПЕРЕД ЦИКЛОМ (Пункт 5.4)
    // =========================================================================
    // 1. Длинная 8-часовая прямоугольная помеха на целевом бине
    // Включается на 100 млн фрейме, длится 18 млн фреймов, амплитуда 3 * A0
    let long_target_disturbance = SpectralDisturbance {
        start_frame: 100_000_000,
        duration_frames: 18_000_000,
        amplitude_factor: 3.0,
        shape: ImpulseShape::Rectangular,
    };
    spectrum_model.add_target_disturbance(long_target_disturbance);
    // 2. Короткая помеха на целевом бине для проверки устойчивости к микро-импульсам
    let short_target_disturbance = SpectralDisturbance {
        start_frame: 500_000_000,
        duration_frames: 2, // длительность 2 фрейма
        amplitude_factor: 5.0, // мощный всплеск в 5 раз
        shape: ImpulseShape::Rectangular,
    };
    spectrum_model.add_target_disturbance(short_target_disturbance);
    // 3. Постоянная помеха во внеполосный шум (на соседний бин частоты справа)
    // Длится всю симуляцию, амплитуда 0.5 * A0. Проверяет Критерий 5.5.3 (Изоляция гармоник)
    let constant_out_of_band = SpectralDisturbance {
        start_frame: 0,
        duration_frames: 1_620_000_000,
        amplitude_factor: 0.5,
        shape: ImpulseShape::Rectangular,
    };
    spectrum_model.add_out_of_band_disturbance(target_bin_idx + 1, constant_out_of_band);
    // 4. Длинная 8-часовая треугольная помеха на нецелевом бине (слева от целевого)
    // Включается на 500 млн фрейме, длится 18 млн фреймов, амплитуда 4 * A0
    let long_out_of_band = SpectralDisturbance {
        start_frame: 500_000_000,
        duration_frames: 18_000_000,
        amplitude_factor: 4.0,
        shape: ImpulseShape::Triangular, // Треугольный профиль удара
    };
    spectrum_model.add_out_of_band_disturbance(target_bin_idx - 1, long_out_of_band);
    // 5. Короткая помеха на нецелевом бине (ИСПРАВЛЕНО: добавлен индекс целевого бина слева)
    let short_out_of_band = SpectralDisturbance {
        start_frame: 500_000_000,
        duration_frames: 2, // длительность 2 фрейма
        amplitude_factor: 5.0, // мощный всплеск в 5 раз
        shape: ImpulseShape::Rectangular,
    };
    spectrum_model.add_out_of_band_disturbance(target_bin_idx - 1, short_out_of_band);
    // =========================================================================
    let mut fft_window: Vec<Complex<f32>> = Vec::<Complex<f32>>::with_capacity(num_bins);
    let order_features_filter = OrderFeatureFilter::new(
        &dbg, 
        conf.analysis.n_fft(), 
        0.02454, 
        Pass::new()
    );
    let retain = Arc::new(Retain::mock(&dbg, []));
    let mut i_ctx = ImbContext::new(&dbg, conf.analysis.samples_per_rev(), conf.analysis.fft_turns(), retain.clone());
    i_ctx.rpm = Rpm(600.0); 
    // Симуляция 30 дней работы
    for i in 0..1_620_000_000 {
        if i == 16378 {
            println!("Asdas");
        }
        log::debug!("Iteration {}/{}", i, 1_620_000_000);
        spectrum_model.generate_frame(i, &mut fft_window);
        i_ctx.fft_window = fft_window;
        i_ctx = order_features_filter.eval(i_ctx);
        fft_window = i_ctx.fft_window;
    }
    let final_feature = i_ctx.features.iter()
        .find(|f| (f.order.value() - target_order).abs() < 10e-6)
        .expect("Целевой диагностический признак порядка 1.0X не обнаружен в ctx.features!");
    let final_amplitude = final_feature.rms;
    assert!(
        (final_amplitude.0 - 203.0).abs() < 10e-6,
        "Отфильтрованная амплитуда {}, не сошлась с ожидаемой {}",
        final_amplitude.0, 203.0
    )
}
