use crate::{
    AngularGrid,
    Autocorrelation,
    Conf,
    Context,
    Eval,
    Frame,
    ImbContext,
    Inputs,
    OrderDomainSamples,
    Pass,
    ReadInputs,
    Retain,
    Rpm,
    tests::{
        Frequency, Udp
    }
};
use chrono::Utc;
use debugging::session::debug_session::{
    DebugSession, 
    LogLevel
};
use sal_core::dbg::Dbg;
use std::sync::Arc;
///
/// Симулирует сигнал, пока пик в order_samples не сойдётся
/// к ожидаемой амплитуде рассматриваемой гармоники (с допуском).
fn full_signal_simulation(
    udp: &mut Udp,
    mut ctx: Context,
    mut i_ctx: ImbContext,
    angular_grid: &AngularGrid<Autocorrelation<ReadInputs>>,
    rpm: f64,
    k: f64,
    inputs: &mut Arc<Inputs>,
    samples: &mut [u16; Frame::SIZE],
    low_range: &OrderDomainSamples<Pass>,
) -> (Context, ImbContext) {
    let f_sample = udp.sample_freq;
    let chunk_size = Frame::SIZE as f64;
    let rpm_hz = rpm / 60.0;
    let delta_per_sample = std::f64::consts::TAU * k * rpm_hz / f_sample;
    let i_peak = (std::f64::consts::FRAC_PI_2 / delta_per_sample).round() as usize;
    let chunks_needed = (i_peak as f64 / chunk_size).ceil() as usize;
    for _ in 0..chunks_needed {
        udp.parse(rpm, samples);
        inputs.set_rpm(rpm);
        ctx.push_chunk(samples);
        let phases;
        (ctx, phases) = angular_grid.eval(ctx);
        let frame = Frame::new(Utc::now(), samples.clone(), phases);
        i_ctx.update(frame.clone());
        i_ctx.rpm = Rpm(rpm);
        i_ctx = low_range.eval(i_ctx);
    }
    (ctx, i_ctx)
}
///
/// Функциональное тестирование [OrderDomainSamples] на корректность фазового сдвига в угловой области.
#[test]
fn order_domain_phase_shift_test() {
    DebugSession::new().filter(LogLevel::Debug).init();
    let dbg = Dbg::own("OrderDomainSamples-test");
    let f_sample = 320_000; // Частота семплирования АЦП (Гц)
    let conf: Conf = serde_yaml::from_str(&format!(r#"
        adc:
            sample-rate-hz: {f_sample}
            chunk-size: 512
        analysis:
            order-tracking:
                max-order: 100
                order-resolution: 0.05
                # points-per-turn: 
            bands:
                low-order: 0.5..5.0
                mid-hz: ..5000
                high-hz: 5000..10000
    "#)).unwrap();
    let test_data = [
        (1, 1.0, 200.0, 1.57, 600.0,  [(Frequency::Rpm(1.0), 200)]), // в pi (90 градусов) тк кратность 1
        (2, 2.0, 300.0, 0.7854, 1200.0, [(Frequency::Rpm(2.0), 300)]), // в pi/4 (45 градусов) тк кратность 2
        (3, 3.0, 400.0, 0.5236, 1800.0, [(Frequency::Rpm(3.0), 400)]), // в pi/6 (30 градусов) тк кратность 3
    ];
    for (step, k, amp_of_signal_peak, angle_of_signal_peak, rpm, freqs) in test_data.iter() {
        log::debug!("Шаг {}: Симуляция сигнала с амплитудой {}, фазой {} и частотой {} об/мин", step, amp_of_signal_peak, angle_of_signal_peak, rpm);
        let mut samples = [0u16; Frame::SIZE];
        let low_range = OrderDomainSamples::new(&dbg, conf.analysis.samples_per_rev(), Pass::new());
        let mut inputs = Arc::new(Inputs::new());
        let mut ctx = Context::new();
        let angular_grid = AngularGrid::new(
            &dbg,
            Autocorrelation::new(
                &dbg,
                conf.adc.sample_rate_hz,
                ReadInputs::new(&dbg, inputs.clone()),
            ),
        );
        let retain = Arc::new(Retain::mock(&dbg, []));
        let mut i_ctx = ImbContext::new(&dbg, conf.analysis.samples_per_rev(), conf.analysis.fft_turns(), retain);
        ctx.current_theta = 0.0;
        let mut udp = Udp::new(Frame::SIZE, conf.adc.sample_rate_hz, freqs.clone());
        let (new_ctx, new_i_ctx) = full_signal_simulation(
            &mut udp,
            ctx,
            i_ctx,
            &angular_grid,
            *rpm,
            *k,
            &mut inputs,
            &mut samples,
            &low_range,
        );
        ctx = new_ctx;
        i_ctx = new_i_ctx;
        let last_sample = i_ctx.order_samples.last().unwrap();
        assert!(
            (last_sample.re - *amp_of_signal_peak).abs() < *amp_of_signal_peak * 0.1,
            "Шаг {}: Амплитуда не совпала с ожидаемой. Получено: {}, ожидалось: {}",
            step, last_sample.re, amp_of_signal_peak
        );
        assert!(
            (i_ctx.total_phase.0 - *angle_of_signal_peak as f64).abs() < *angle_of_signal_peak as f64 * 0.13,
            "Шаг {}: Фаза не совпала с ожидаемой. Получено: {}, ожидалось: {}",
            step, i_ctx.total_phase.0, angle_of_signal_peak
        );
    }
}