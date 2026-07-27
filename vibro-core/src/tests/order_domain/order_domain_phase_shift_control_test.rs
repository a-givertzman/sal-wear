use crate::{
    AngularGrid, 
    Autocorrelation, 
    Conf, 
    Context, 
    Eval, 
    Frame, 
    ImbContext, 
    Inputs,
    OrderDomainSamples, Pass, ReadInputs,
    tests::{
        Frequency, 
        Udp
    },
};
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
        ctx = angular_grid.eval(ctx);
        let frame = Arc::new(Frame {
            samples: samples.map(|v| v as f32 - 2048.0),
            phases: ctx.phases.to_vec(),
        });
        // for i in 0..Frame::SIZE {
        //     writeln!(file_time, "{}", frame.samples[i]).unwrap();
        // }
        // println!("{:?}", frame.samples);
        for sample in frame.samples.iter() {
            if *sample == 200.0 {
                i_ctx.update(frame.clone());
                i_ctx.rpm = rpm;
                // file_time.flush().unwrap();
                // log::info!("Симуляция прервана по условию на чанке №{}", chunk_idx);
                return (ctx, i_ctx);
            }
        }
    }
    panic!("Пик не сошёлся к амплитуде");
}
///
/// Функциональное тестирование [OrderDomainSamples] на стационарность при разгоне
#[test]
fn order_domain_stationary_test() {
    DebugSession::new().filter(LogLevel::Debug).init();
    let dbg = Dbg::own("OrderDomainSamples-test");
    let f_sample = 320_000; // Частота семплирования АЦП (Гц)
    let conf: Conf = serde_yaml::from_str(&format!(
        r#"
        hardware:
            sample-rate-hz: {f_sample}
            chunk-size: 512
        angular:
            max-order: 100
            resolution: 0.05
            # points-per-turn: 
        bands:
            low-order: 0.5..5.0
            mid-hz: ..5000
            high-hz: 5000..10000
    "#
    ))
    .unwrap();
    let mut samples = [0u16; Frame::SIZE];
    let inputs = Arc::new(Inputs::new());
    let low_range = OrderDomainSamples::new(&dbg, conf.angular.points_per_turn(), Pass::new());
    let mut inputs = Arc::new(Inputs::new());
    let mut ctx = Context::new();
    let angular_grid = AngularGrid::new(
        &dbg,
        Autocorrelation::new(
            &dbg,
            conf.hardware.sample_rate_hz,
            ReadInputs::new(&dbg, inputs.clone()),
        ),
    );
    let mut i_ctx = ImbContext::new(conf.angular.points_per_turn(), conf.angular.fft_turns());
    let test_data = [
        (0, 1.57, 600.0,  [(Frequency::Rpm(1.0), 200)]),
        (1, 1.57, 1200.0, [(Frequency::Rpm(2.0), 200)]),
        (2, 1.57, 1800.0, [(Frequency::Rpm(3.0), 200)]),
    ];
    let mut found: Vec<bool> = vec![false; test_data.len()];
    for (step, angle_of_signal_peak, rpm, freqs) in test_data.iter() {
        ctx.current_theta = 0.0;
        let mut udp = Udp::new(Frame::SIZE, conf.hardware.sample_rate_hz, freqs.clone());
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
        for (i, order_sample) in i_ctx.order_samples.iter().enumerate() {
            let solution_error = (freqs[0].1 as f32 / 100.0) * 10.0;
            if (order_sample.re - freqs[0].1 as f32).abs() <= solution_error {
                if (i_ctx.order_phases[i] - angle_of_signal_peak).abs() <= 0.5 {
                    found[*step] = true;
                    break;
                }
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