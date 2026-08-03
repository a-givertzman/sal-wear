use std::{fmt::Write, sync::Arc};
use chrono::Utc;
use debugging::session::debug_session::{DebugSession, LogLevel};
use sal_core::dbg::Dbg;
use sal_sync::{services::RECV_TIMEOUT, sync::channel::{self, RecvTimeoutError}, thread_pool::ThreadPool};
use crate::{AngularGrid, Autocorrelation, Conf, Context, Eval, Frame, ImbContext, ImbalanceDetector, MockEventValues, LowPassSignal, OrderDomainSamples, OrderFeatureFilter, OrderSpectrum, Pass, ReadEventValuess, Retain, Severity, SqlExport, WindowFn, tests::{Frequency, Udp}};

///
/// 
#[test]
#[ignore = "Manual Test"]
fn complex_test () {
    DebugSession::new().filter(LogLevel::Debug).init();
    let dbg = Dbg::own("complex-test");
    let tp = ThreadPool::new(&dbg, Some(8));
    let scheduler = tp.scheduler();
    let conf: Conf = serde_yaml::from_str(r#"
        adc:
            sample-rate-hz: 320000
            chunk-size: 512
        analysis:
            order-tracking:
                max-order: 300              # Максимальный порядок (кратность частоты вращения), до которого производится спектральный анализ.
                order-resolution: 0.01      # Требуемая спектральное разрешение в угловом домене.
                # samples-per-rev: 256      # Плотность угловой дискретизации (сэмплов на оборот).
            bands:
                low-order: 0.5..10.0
                mid-hz: ..5000
                high-hz: 5000..10000
    "#).unwrap();
    let inputs = Arc::new(MockEventValues::new());
    let mut samples = [0u16; Frame::SIZE];
    let mut ctx = Context::new();
    let angular_grid = AngularGrid::new(&dbg,
        Autocorrelation::new(&dbg,
            conf.adc.sample_rate_hz,
            ReadEventValuess::new(&dbg, inputs.clone())
        ),
    );
    let window_size = conf.analysis.n_fft();
    let window_fn = WindowFn::<f32>::kaiser(&dbg, window_size, window_size, 0, 5.65).unwrap();
    let (api_link, api_recv) = crate::channel_unbounded();
    let equipment_id = 1212;
    let low_range = SqlExport::new(&dbg, move |ctx| {
            if ctx.is_err() { return; }
            let mut sql = String::with_capacity(ctx.results.len() * 120 + 150);
            let mut results = ctx.results.iter().filter(|r| r.severity != Severity::Green).peekable();
            if results.peek().is_some() {
                sql.push_str("INSERT INTO vibration_faults (timestamp, equipment_id, fault_kind, score, severity, rpm) VALUES ");
                for (i, r) in results.enumerate() {
                    if i > 0 { sql.push_str(", "); }
                    _ = write!(     // use std::fmt::Write - Required
                        sql,
                        "('{}', {}, '{}', {}, '{}', {})",
                        r.ts.to_rfc3339(),
                        equipment_id,
                        r.fault,
                        r.score,
                        r.severity,
                        r.rpm.value()
                    );
                }
                sql.push_str(" ON CONFLICT (equipment_id, fault_kind) DO UPDATE SET ");
                sql.push_str("timestamp = EXCLUDED.timestamp, score = EXCLUDED.score, severity = EXCLUDED.severity, rpm = EXCLUDED.rpm;");
                _ = api_link.send(sql)
            }
            if !ctx.features.is_empty() {
                let mut sql = String::with_capacity(ctx.features.len() * 120 + 150);
                sql.push_str("INSERT INTO order_vibration_trends (timestamp, equipment_id, order_id, rms_value, phase, rpm) VALUES ");
                for (i, r) in ctx.features.iter().enumerate() {
                    if i > 0 { sql.push_str(", "); }
                    _ = write!(
                        sql,
                        "('{}', {}, '{}', {}, {}, {})",
                        r.ts.to_rfc3339(),
                        equipment_id,
                        r.order_id,
                        r.rms.value(),
                        r.phase.to_degrees(),
                        r.rpm.value()
                    );
                }
                sql.push_str(" ON CONFLICT (timestamp, equipment_id, order_id) DO NOTHING;");
                _ = api_link.send(sql)
            }
        },
        ImbalanceDetector::new(&dbg,
            OrderFeatureFilter::new(&dbg,
                conf.analysis.n_fft(),
                conf.analysis.angular_step_rad(),
                OrderSpectrum::new(&dbg,
                    conf.analysis.n_fft(),
                    Some(window_fn),
                    OrderDomainSamples::new(&dbg,
                        conf.analysis.samples_per_rev(),
                        LowPassSignal::new(&dbg,
                            conf.adc.sample_rate_hz,
                            conf.analysis.bands.low_cutoff_order(),
                            Pass::new(),
                        ),
                    ),
                ),
            ),
        ),
    );
    let mut udp = Udp::new(Frame::SIZE, conf.adc.sample_rate_hz, [
        // Статический резонанс на 5 кГц с амплитудой 100
        (Frequency::Static(5000.0), 100),
    ]);
    let retain = Arc::new(Retain::mock(&dbg, []));
    let mut low_range_ctx = ImbContext::new(&dbg, conf.analysis.samples_per_rev(), conf.analysis.n_fft(), retain);
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
                    let frame = Frame::new(ts, &samples, phases);
                    _ = low_send.send(frame.clone());
                    _ = mid_send.send(frame.clone());
                    _ = high_send.send(frame.clone());
                }
            }
        }
    }
    let expected_results = todo!();
    let actual_results = api_recv.len();
    assert!(actual_results == expected_results, "{dbg} | Total number of sql's is {}, expected {}", actual_results, expected_results);
    let expected_sqls = vec![];
    for (sql, expected_sql) in api_recv.zip(expected_sqls) {
        assert!(sql == expected_sql, "{dbg} | \n Actual sql: {}, \n expected sql: {}", sql, expected_sql);
    }
}
