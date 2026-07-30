use crate::{
    AngularGrid, Autocorrelation, Conf, Context, Eval, Frame, ImbContext, Inputs, OrderDomainSamples, Pass, ReadInputs, Retain, tests::{FftBuffer, Frequency, Udp, order_domain::order_features_filter_test::entities::{ImpulseShape, SpectralDisturbance, SpectrumModel}}
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
    let multiplier = 1.015_f32.powf(1.0 / 1_620_000_000.0); 
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
    // ДОБАВЛЕНИЕ ПОМЕХ ПЕРЕД ЦИКЛОМ
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
    // 3. Постоянная помеха во внеполосный шум (на соседний бин частоты)
    // Длится всю симуляцию, амплитуда 0.5 * A0. Проверяет Критерий 5.5.3 (Изоляция гармоник)
    let constant_out_of_band = SpectralDisturbance {
        start_frame: 0,
        duration_frames: 1_620_000_000,
        amplitude_factor: 0.5,
        shape: ImpulseShape::Rectangular,
    };
    spectrum_model.add_out_of_band_disturbance(target_bin_idx + 1, constant_out_of_band);
    let mut fft_window = Vec::<Complex<f64>>::with_capacity(num_bins);
    // Симуляция 30 дней работы
    for i in 0..1_620_000_000 {
        spectrum_model.generate_frame(i, &mut fft_window);
    }
}
