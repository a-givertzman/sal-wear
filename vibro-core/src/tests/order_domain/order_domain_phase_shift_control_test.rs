use crate::{
    AngularGrid, Autocorrelation, Conf, Context, Eval, Frame, ImbContext, Inputs, OrderDomainSamples, Pass, ReadInputs, Retain, Rpm, tests::{
        Frequency, 
        Udp
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
    inputs: &mut Arc<Inputs>,
    samples: &mut [u16; Frame::SIZE],
) -> (Context, ImbContext) {
    const MAX_CHUNKS: usize = 10_000; // защита от бесконечного цикла
    // let mut file_time = BufWriter::new(
    //     File::create("/home/debian/Documents/python_test/input/sim_time_signal.txt").unwrap()
    // );
    for chunk_idx in 0..MAX_CHUNKS {
        udp.parse(rpm, samples);
        inputs.set_rpm(rpm);
        ctx.push_chunk(samples);
        let phases;
        (ctx, phases) = angular_grid.eval(ctx);
        let frame = Frame::new(Utc::now(), samples.clone(), phases);
        // {
        //     samples: samples.map(|v| v as f32 - 2048.0),
        //     phases: ctx.phases.to_vec(),
        // });
        // for i in 0..Frame::SIZE {
        //     writeln!(file_time, "{}", frame.samples[i]).unwrap();
        // }
        // println!("{:?}", frame.samples);
        for sample in frame.samples.iter() {
            if *sample == 200.0 {
                i_ctx.update(frame.clone());
                i_ctx.rpm = Rpm(rpm);
                // file_time.flush().unwrap();
                // log::info!("Симуляция прервана по условию на чанке №{}", chunk_idx);
                return (ctx, i_ctx);
            }
        }
    }
    panic!("Пик не сошёлся к амплитуде");
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
                resolution: 0.05
                # points-per-turn: 
            bands:
                low-order: 0.5..5.0
                mid-hz: ..5000
                high-hz: 5000..10000
    "#)).unwrap();
    let mut samples = [0u16; Frame::SIZE];
    let inputs = Arc::new(Inputs::new());
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
    let test_data = [
        (0, 1.57, 600.0,  [(Frequency::Rpm(1.0), 200)]),
        (1, 1.57, 1200.0, [(Frequency::Rpm(2.0), 200)]),
        (2, 1.57, 1800.0, [(Frequency::Rpm(3.0), 200)]),
    ];
    let mut found: Vec<bool> = vec![false; test_data.len()];
    for (step, angle_of_signal_peak, rpm, freqs) in test_data.iter() {
        ctx.current_theta = 0.0;
        let mut udp = Udp::new(Frame::SIZE, conf.adc.sample_rate_hz, freqs.clone());
        let (new_ctx, new_i_ctx) = full_signal_simulation(
            &mut udp,
            ctx,
            i_ctx,
            &angular_grid,
            *rpm,
            &mut inputs,
            &mut samples,
        );
        ctx = new_ctx;
        i_ctx = new_i_ctx;
        i_ctx = low_range.eval(i_ctx);
        let closest = i_ctx.order_phases
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let da = (**a as f64 - angle_of_signal_peak).abs();
                let db = (**b as f64 - angle_of_signal_peak).abs();
                da.partial_cmp(&db).unwrap()
            });
        if let Some((idx, closest_phase)) = closest {
            let angle_diff = (*closest_phase as f64 - angle_of_signal_peak).abs();
            let amp_diff = (i_ctx.order_samples[idx].re - freqs[0].1 as f32).abs();
            let solution_error = (freqs[0].1 as f32 / 100.0) * 10.0;
            if angle_diff <= 0.5 && amp_diff <= solution_error {
                found[*step] = true;
            }
        }
    }
    for (i, f) in found.iter().enumerate() {
        assert!(
            f,
            "Шаг {}: пик амплитуды не найден на ожидаемом угле",
            i
        );
    }
}