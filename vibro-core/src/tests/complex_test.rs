use std::sync::Arc;
use chrono::Utc;
use debugging::session::debug_session::{DebugSession, LogLevel};
use sal_core::{dbg::Dbg, error::Error};
use sal_sync::{services::RECV_TIMEOUT, sync::channel::{self, RecvTimeoutError}, thread_pool::ThreadPool};
use crate::{AngularGrid, Autocorrelation, Conf, Context, Eval, Frame, ImbContext, ImbalanceDetector, Inputs, LowPassSignal, OrderDomainSamples, OrderSpectrum, Pass, ReadInputs, Retain, WindowFn, tests::{Frequency, Udp}};

///
/// 
#[test]
fn complex_test () {
    DebugSession::new().filter(LogLevel::Debug).init();
    let dbg = Dbg::own("complex-test");
    let tp = ThreadPool::new(&dbg, Some(8));
    let scheduler = tp.scheduler();
    let conf: Conf = serde_yaml::from_str(r#"
        hardware:
            sample-rate-hz: 320000
            chunk-size: 512
        angular:
            max-order: 100
            resolution: 0.05
        bands:
            low-order: 0.5..10.0
            mid-hz: ..5000
            high-hz: 5000..10000
    "#).unwrap();
    let inputs = Arc::new(Inputs::new());
    let mut samples = [0u16; Frame::SIZE];
    let mut ctx = Context::new();
    let angular_grid = AngularGrid::new(&dbg,
        Autocorrelation::new(&dbg,
            conf.hardware.sample_rate_hz,
            ReadInputs::new(&dbg, inputs.clone())
        ),
    );
    let window_size = OrderSpectrum::<Pass>::fft_buffer_size();
    let window_fn = WindowFn::<f32>::kaiser(&dbg, window_size, window_size, 0, 5.65).unwrap();
    let low_range = ImbalanceDetector::new(&dbg, 
        OrderSpectrum::new(&dbg,
            Some(window_fn),
            OrderDomainSamples::new(&dbg,
                conf.angular.n_rev(),
                LowPassSignal::new(&dbg,
                    conf.hardware.sample_rate_hz,
                    conf.bands.low_cutoff_order(),
                    Pass::new(),
                ),
            ),
        ),
    );
    let mut udp = Udp::new(Frame::SIZE, conf.hardware.sample_rate_hz, [
        // Статический резонанс на 5 кГц с амплитудой 100
        (Frequency::Static(5000.0), 100),
    ]);
    let retain = Arc::new(Retain::mock(&dbg, []));
    let mut low_range_ctx = ImbContext::new(&dbg, conf.angular.n_rev(), conf.angular.fft_turns(), retain);
    let (low_send, low_recv) = channel::bounded(1);
    let (mid_send, mid_recv) = channel::bounded(1);
    let (high_send, high_recv) = channel::bounded(1);
    _ = scheduler.spawn({
        move || {
        loop {
            match low_recv.recv_timeout(RECV_TIMEOUT) {
                Ok(frame) => {
                    low_range_ctx.update(frame);
                    low_range_ctx = low_range.eval(low_range_ctx);
                }
                Err(RecvTimeoutError::Timeout) => {}
                _ => break,
            }
        }
    }});
    _ = scheduler.spawn({
        move || {
        loop {
            match mid_recv.recv_timeout(RECV_TIMEOUT) {
                Ok(frame) => {
                    // mid_range_ctx.update(frame);
                    // mid_range_ctx = mid_range.eval(mid_range_ctx);
                }
                Err(RecvTimeoutError::Timeout) => {}
                _ => break,
            }
        }
    }});
    _ = scheduler.spawn({
        move || {
        loop {
            match high_recv.recv_timeout(RECV_TIMEOUT) {
                Ok(frame) => {
                    // high_range_ctx.update(frame);
                    // high_range_ctx = high_range.eval(high_range_ctx);
                }
                Err(RecvTimeoutError::Timeout) => {}
                _ => break,
            }
        }
    }});

    loop {
        // для тестирования вручную имитируем изменение rpm привода
        let rpm = 3000.0;
        // Имитируем получение АЦП выборки из сети
        udp.parse(rpm, &mut samples);
        let ts = Utc::now();
        // для тестирования вручную обновляем rpm на входе, в работе он будет приходить извне
        inputs.set_rpm(rpm);
        ctx.push_chunk(&samples);
        let phases;
        (ctx, phases) = angular_grid.eval(ctx);
        match &ctx.err {
            Some(err) => log::warn!("{}", err),
            None => {
                if ctx.ac_samples.is_full() {
                    let frame = Frame::new(samples, phases, ts);
                    _ = low_send.send(frame.clone());
                    _ = mid_send.send(frame.clone());
                    _ = high_send.send(frame.clone());
                }
            }
        }
    }
}
