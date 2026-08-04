use crate::{
    AngularGrid, Autocorrelation, Conf, AngularCtx, Eval, Frame, ImbContext, MockEventValues, OrderDomainSamples, Pass, Phases, ReadEventValuess, Retain, Rpm, tests::{
        Frequency, Udp
    }
};
use chrono::Utc;
use debugging::session::debug_session::{
    DebugSession, 
    LogLevel
};
use sal_core::dbg::Dbg;
use std::{f64::consts::{PI, TAU}, sync::Arc, time::Instant};
///
/// Симулирует сигнал, пока пик в order_samples не сойдётся
/// к ожидаемой амплитуде рассматриваемой гармоники (с допуском).
fn full_signal_simulation(
    udp: &mut Udp,
    mut ctx: AngularCtx,
    mut i_ctx: ImbContext,
    angular_grid: &impl Eval<AngularCtx, (AngularCtx, Phases<f32>)>,
    rpm: f64,
    k: f64,
    inputs: &mut Arc<MockEventValues>,
    samples: &mut [u16],
    low_range: &OrderDomainSamples<Pass>,
    chunk_size: usize,
) -> (AngularCtx, ImbContext, usize) {
    let f_sample = udp.sample_freq;
    let rpm_hz = rpm / 60.0;
    let delta_per_sample = std::f64::consts::TAU * k * rpm_hz / f_sample;
    let i_peak = (std::f64::consts::FRAC_PI_2 / delta_per_sample).round() as usize;
    let chunks_needed = (i_peak as f64 / chunk_size as f64).ceil() as usize;
    for _ in 0..chunks_needed {
        udp.parse(rpm, samples);
        inputs.set_rpm(rpm);
        ctx.push_chunk(samples);
        let phases;
        let t = Instant::now();
        (ctx, phases) = angular_grid.eval(ctx);
        log::debug!("AngularGrid<Autocorrelation> elapsed {:?}", t.elapsed());
        let frame = Frame::new(Utc::now(), 2048f32, &samples, phases);
        i_ctx.update(frame.clone());
        i_ctx.samples.clone_from_slice(&frame.samples);
        i_ctx.rpm = Rpm(rpm);
        let t = Instant::now();
        i_ctx = low_range.eval(i_ctx);
        log::debug!("OrderDomainSamples elapsed {:?}", t.elapsed());
    }
    (ctx, i_ctx, chunks_needed)
}
///
/// Функциональное тестирование [OrderDomainSamples] на корректность фазового сдвига в угловой области.
#[test]
fn order_domain_phase_shift_test() {
    DebugSession::new().filter(LogLevel::Debug).init();
    let dbg = Dbg::own("OrderDomainSamples-test");
    let f_sample = 320_000; // Частота семплирования АЦП (Гц).
    let chunk_size = 512;   // Размер пакета данных, поступающего из АЦП.
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
        (1, 1.0, 200.0, PI / 2.0, 600.0,  [(Frequency::Rpm(1.0), 200)]), // в pi (90 градусов) тк кратность 1
        (2, 2.0, 300.0, PI / 4.0, 1200.0, [(Frequency::Rpm(2.0), 300)]), // в pi/4 (45 градусов) тк кратность 2
        (3, 3.0, 400.0, PI / 6.0, 1800.0, [(Frequency::Rpm(3.0), 400)]), // в pi/6 (30 градусов) тк кратность 3
    ];
    for (step, k, target_rms, target_angle_rad, rpm, freqs) in test_data.iter() {
        log::debug!("Шаг {}: Симуляция сигнала с амплитудой {}, фазой {:.1} и частотой {} об/мин", step, target_rms, target_angle_rad.to_degrees(), rpm);
        let mut samples = vec![0u16; chunk_size];
        let low_range = OrderDomainSamples::new(&dbg, conf.analysis.samples_per_rev(), Pass::new());
        let mut inputs = Arc::new(MockEventValues::new());
        let mut ctx = AngularCtx::new(f_sample as f64, chunk_size);
        let angular_grid = AngularGrid::new(
            &dbg,
            chunk_size,
            Autocorrelation::new(
                &dbg,
                conf.adc.sample_rate_hz,
                ReadEventValuess::new(&dbg, inputs.clone()),
            ),
        );
        let retain = Arc::new(Retain::mock(&dbg, []));
        let mut i_ctx = ImbContext::new(&dbg, conf.analysis.samples_per_rev(), conf.analysis.fft_turns(), chunk_size, retain);
        ctx.current_theta = 0.0;
        let mut udp = Udp::new(chunk_size, conf.adc.sample_rate_hz, freqs.clone());
        let (new_ctx, new_i_ctx, chunks_needed) = full_signal_simulation(
            &mut udp,
            ctx,
            i_ctx,
            &angular_grid,
            *rpm,
            *k,
            &mut inputs,
            &mut samples,
            &low_range,
            chunk_size,
        );
        ctx = new_ctx;
        i_ctx = new_i_ctx;
        // Скорость вращения вала в радианах в секунду
        let rad_per_sec = (rpm / 60.0) * TAU;
        // Приращение фазы за ОДИН СЕМПЛ (один шаг дискретизации)
        let phase_step_rad = rad_per_sec * ctx.dt; 
        // Общее количество обработанных точек данных (семплов)
        let total_samples_processed = chunks_needed * chunk_size;
        // Математически идеальная накопленная фаза ВАЛА за всю симуляцию в частотном домене (временная область)
        let calculated_total_phase = total_samples_processed as f64 * phase_step_rad;   
        // Идеальное расстояние между точками в угловой области
        let delta_theta = std::f64::consts::TAU / (conf.analysis.samples_per_rev() as f64);
        log::debug!(
            "Шаг {step}: Суммарная фаза вала. Получено: {} ({}), ожидалось частотном: {} ({})",
            i_ctx.total_phase.to_degrees(), i_ctx.total_phase.to_radians(),
            calculated_total_phase.to_degrees(), calculated_total_phase,
        );
        assert!(
            (i_ctx.total_phase.to_radians() - calculated_total_phase).abs() < delta_theta,
            "Шаг {step}: Фаза отслеживания вала уплыла! Получено: {:.5} рад, ожидалось: {:.5} рад",
            i_ctx.total_phase.to_radians(), calculated_total_phase
        );
        // Индекс точки в угловой области, которая должна находится на исследуемом пике
        let delta = ((i_ctx.total_phase.to_radians() - target_angle_rad) / delta_theta).round() as usize; 
        let sample = i_ctx.order_samples[(i_ctx.order_samples.len() - 1) - delta];
        assert!(
            (sample.re - *target_rms).abs() < 10e-6,
            "Шаг {step}: Амплитуда сигнала в угловом домене не совпала с ожидаемой. Получено: {}, ожидалось: {}",
            sample.re, target_rms
        );
    }
}
