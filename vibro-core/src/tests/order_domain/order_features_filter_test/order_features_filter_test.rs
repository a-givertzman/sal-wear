use crate::{
    Conf, Eval, ImbContext, OrderFeatureFilter, Pass, Retain,
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
    const SCALE: u64 = 500000;
    let total_frames: u64 = 1_620_000_000 / SCALE;

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
                # samples-per-rev:
            bands:
                low-order: 0.5..5.0
                mid-hz: ..5000
                high-hz: 5000..10000
    "#
    ))
    .unwrap();
    // нужный ордер
    // рмр равно значению которое посчитал сумма трех 
    // дельта на которое выросло рмс на 1.5 процента
    // нужное время фильтр срабатывал
    // результаты фильтра пусто -2 +2 не влияют 
    // таблица с зависимости от роста амплитуд
    let a0 = 200.0;
    let target_order = 1.0;
    let target_bin_idx = (target_order * conf.analysis.fft_turns() as f64).round() as usize;
    let num_bins = conf.analysis.n_fft() / 2;
    let mut spectrum_model = SpectrumModel::new(a0, total_frames, target_bin_idx, num_bins);
    // 1. Длинная 8-часовая прямоугольная помеха на целевом бине
    let long_target_disturbance = SpectralDisturbance {
        start_frame: 100,
        duration_frames: 800,
        amplitude_factor: 3.0,
        shape: ImpulseShape::Rectangular,
    };
    spectrum_model.add_target_disturbance(long_target_disturbance);
    // 2. Короткая помеха на целевом бине
    let short_target_disturbance = SpectralDisturbance {
        start_frame: 500_000 / SCALE,
        duration_frames: (2_u64).max(1), // держим минимум 1-2 кадра, иначе помеха исчезнет
        amplitude_factor: 5.0,
        shape: ImpulseShape::Rectangular,
    };
    spectrum_model.add_target_disturbance(short_target_disturbance);
    // 3. Постоянная внеполосная помеха (весь тест)
    let constant_out_of_band = SpectralDisturbance {
        start_frame: 0,
        duration_frames: total_frames,
        amplitude_factor: 2.5,
        shape: ImpulseShape::Rectangular,
    };
    spectrum_model.add_out_of_band_disturbance(target_bin_idx + 1, constant_out_of_band);
    // 4. Длинная 8-часовая треугольная помеха на нецелевом бине
    let long_out_of_band = SpectralDisturbance {
        start_frame: 500_000 / SCALE,
        duration_frames: (18_000 / SCALE).max(1),
        amplitude_factor: 4.0,
        shape: ImpulseShape::Triangular,
    };
    spectrum_model.add_out_of_band_disturbance(target_bin_idx - 1, long_out_of_band);
    // 5. Короткая помеха на нецелевом бине
    let short_out_of_band = SpectralDisturbance {
        start_frame: 500_000 / SCALE,
        duration_frames: (10_u64 / SCALE).max(1),
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
        conf.analysis.n_fft(),
        conf.adc.chunk_size,
        retain.clone(),
    );
    // =========================================================================
    // Симуляция
    // =========================================================================
    for i in 0..total_frames {
        log::debug!("Frame: {}/{}", i, total_frames);
        spectrum_model.generate_frame(i, &mut fft_window);
        if fft_window[target_bin_idx].re > 800.0 {
            println!("{:?}", fft_window[target_bin_idx]);
        }
        i_ctx.fft_window = fft_window;
        i_ctx = order_features_filter.eval(i_ctx);
        fft_window = i_ctx.fft_window;
    }
    for feature in i_ctx.features.iter() {
        println!("{:?}", feature.rms);
    }
}