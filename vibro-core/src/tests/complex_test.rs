use std::sync::Arc;
use sal_core::{dbg::Dbg, error::Error};
use sal_sync::{services::RECV_TIMEOUT, sync::channel::{self, RecvTimeoutError}, thread_pool::ThreadPool};
use crate::{AngularGrid, Autocorrelation, Conf, Context, Eval, Frame, ImbContext, Inputs, LowPassSignal, OrderDomainSamples, PI2, Pass, ReadInputs};

struct Udp {
    chunk: usize,
    sample_freq: f64,
    dt: f64,
    freqs: Vec<(f64, u16)>, // Статические резонансы (частота, амплитуда)
    dynamic_phase: f64,     // Аккумулятор фазы для оборотной частоты
    fixed_phases: Vec<f64>, // Аккумуляторы фаз для статических резонансов    dt: f64,
}
impl Udp {
    /// Имитирует сигнал с АЦП выборками заданного размера
    /// `chunk` - размер выборок с АЦП (512)
    /// `sample_freq` - Частота дискретизации АЦП
    /// `freqs` - Массив пар (частота, амплитуда)
    fn new(chunk: usize, sample_rate_hz: impl Into<f64>, freqs: impl IntoIterator<Item = (f64, u16)>) -> Self {
        let sample_rate_hz = sample_rate_hz.into();
        let freqs: Vec<_> = freqs.into_iter().collect();
        let fixed_phases = vec![0.0; freqs.len()];
        Self {
            chunk,
            sample_freq: sample_rate_hz,
            dt: 1.0 / sample_rate_hz,
            freqs,
            dynamic_phase: 0.0,
            fixed_phases,
        }
    }
    /// Заполняет переданный буфер синтетическими данными.
    /// Выполняет сложение синусоид и смещение нулевой линии для формата u16.
    /// `rpm` - Текущая частота вращения привода в об/мин
    /// `rpm_amp` - Амплитуда 1x гармоники вала
    fn parse(&mut self, rpm: f64, rpm_amp: u16, samples: &mut [u16]) {
        let rpm_hz = rpm / 60.0;
        let rpm_step = PI2 * rpm_hz * self.dt;
        for i in 0..samples.len() {
            // Накапливаем фазу вращения вала
            self.dynamic_phase += rpm_step;
            if self.dynamic_phase > PI2 {
                self.dynamic_phase -= PI2;
            }
            let mut val = 2048.0; // Смещение нулевой линии для 12-бит АЦП
            // Добавляем динамическую 1x гармонику
            val += (rpm_amp as f64) * self.dynamic_phase.sin();
            // Добавляем статические резонансы механизма
            for (j, (f, amp)) in self.freqs.iter().enumerate() {
                self.fixed_phases[j] += PI2 * f * self.dt;
                if self.fixed_phases[j] > PI2 {
                    self.fixed_phases[j] -= PI2;
                }
                val += (*amp as f64) * self.fixed_phases[j].sin();
            }
            samples[i] = val.round().clamp(0.0, 4095.0) as u16;
        }
    }
}
///
/// 
#[test]
fn complex_test () {
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
    let low_range = 
    OrderDomainSamples::new(&dbg,
        conf.angular.points_per_turn(),
        LowPassSignal::new(&dbg,
            conf.hardware.sample_rate_hz,
            conf.bands.low_cutoff_order(),
            Pass::new(),
        ),
    );
    let mut udp = Udp::new(Frame::SIZE, conf.hardware.sample_rate_hz, [
        // Статический резонанс на 5 кГц с амплитудой 100
        (5000.0, 100),
    ]);
    let mut low_range_ctx = ImbContext::new(conf.angular.points_per_turn(), conf.angular.fft_turns());
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
        let rpm_amp = 2048;
        // Имитируем получение АЦП выборки из сети
        udp.parse(rpm, rpm_amp, &mut samples);
        // для тестирования вручную обновляем rpm на входе, в работе он будет приходить извне
        inputs.set_rpm(rpm);
        ctx.push_chunk(&samples);
        ctx = angular_grid.eval(ctx);
        if ctx.ac_samples.is_full() {
            let frame = Arc::new(Frame {
                samples,
                phases: ctx.phases,
            });
            _ = low_send.send(frame.clone());
            _ = mid_send.send(frame.clone());
            _ = high_send.send(frame.clone());
        }
    }
}
