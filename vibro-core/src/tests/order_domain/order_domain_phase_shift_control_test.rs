use crate::{
    AngularGrid, Autocorrelation, Conf, Context, Eval, Frame, ImbContext, Inputs,
    OrderDomainSamples, Pass, ReadInputs,
    tests::{Frequency, Udp},
};
use debugging::session::debug_session::{DebugSession, LogLevel};
use sal_core::dbg::Dbg;
use std::{fs::File, io::{BufWriter, Write}, sync::Arc};
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
    let mut file_time = BufWriter::new(
        File::create("/home/debian/Documents/python_test/input/sim_time_signal.txt").unwrap()
    );
    for chunk_idx in 0..MAX_CHUNKS {
        udp.parse(rpm, samples);
        inputs.set_rpm(rpm);
        ctx.push_chunk(samples);
        ctx = angular_grid.eval(ctx);
        let frame = Arc::new(Frame {
            samples: samples.map(|v| v as f32 - 2048.0),
            phases: ctx.phases.to_vec(),
        });
        for i in 0..Frame::SIZE {
            writeln!(file_time, "{}", frame.samples[i]).unwrap();
        }
        println!("{:?}", frame.samples);
        for sample in frame.samples.iter() {
            if *sample == 200.0 {
                i_ctx.update(frame.clone());
                i_ctx.rpm = rpm;
                file_time.flush().unwrap();
                log::info!("Симуляция прервана по условию на чанке №{}", chunk_idx);
                return (ctx, i_ctx);
            }
        }
    }
    panic!("Пик не сошёлся к амплитуде");
}
/// Переводит индекс k в order_samples в реальный угол (градусы),
/// повторяя логику TargetAngles::new из прод-кода OrderDomainSamples.
fn order_sample_index_to_angle_deg(
    phases: &[f32],
    points_per_turn: usize,
    k: usize,
) -> f64 {
    let delta_theta = std::f64::consts::TAU / points_per_turn as f64;
    let theta1 = phases[0] as f64;

    let start_idx = (theta1 / delta_theta).ceil() as usize;
    let wrapped_start_idx = start_idx % points_per_turn;

    let step_deg = 360.0 / points_per_turn as f64;
    let theta_start_deg = wrapped_start_idx as f64 * step_deg;

    (theta_start_deg + k as f64 * step_deg) % 360.0
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
        (1, 600.0,  [(Frequency::Rpm(1.0), 200)]),
        (2, 1200.0, [(Frequency::Rpm(2.0), 200)]),
        (3, 1800.0, [(Frequency::Rpm(3.0), 200)]),
    ];
    for (step, rpm, freqs) in test_data.iter() {
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
    }
}