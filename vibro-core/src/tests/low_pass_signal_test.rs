use std::sync::Arc;
use rustfft::{FftPlanner, num_complex::{Complex, ComplexFloat}};
use sal_core::dbg::Dbg;
use slice_ring_buffer::SliceRingBuffer;
use crate::{Conf, Eval, Frame, ImbContext, LowPassSignal, Pass, tests::Udp};

///
/// 
#[test]
fn low_pass_signal_test () {
    let dbg = Dbg::own("LowPassSignal-test");
    let f_sample = 320_000; // Частота семплирования АЦП (Гц)
    let conf: Conf = serde_yaml::from_str(&format!(r#"
        hardware:
            sample-rate-hz: {f_sample}
            chunk-size: 512
        angular:
            max-order: 100
            resolution: 0.05
        bands:
            low-order: 0.5..5.0
            mid-hz: ..5000
            high-hz: 5000..10000
    "#)).unwrap();
    let mut samples = [0u16; Frame::SIZE];
    let low_cutoff_order = conf.bands.low_cutoff_order();  // Возвращает верхнюю границу для ФНЧ в порядках (Orders), например 10X
    // Фильтр нижних частот (Баттерворт 2-го порядка) для подавления ВЧ-шумов.
    // Пропускает частоты до заданного порядка (например, 10x от текущих оборотов).
    // RPM берет из контекста ImbContext.rpm
    let low_range = LowPassSignal::new(&dbg,
        conf.hardware.sample_rate_hz,
        low_cutoff_order,
        Pass::new(),
    );
    let freqs = [
        // Полезный сигнал
        (50.0, 200),
        (100.0, 250),
        (150.0, 150),
        // Шумы
        (1000.0, 100),
        (3000.0, 100),
        // Статический резонанс на 5 кГц с амплитудой 100
        (5000.0, 100),
        (7000.0, 100),
        (12000.0, 100),
    ];
    let mut udp = Udp::new(
        Frame::SIZE,    // 512
        conf.hardware.sample_rate_hz, freqs.clone(),
    );
    let mut results: Vec<Vec<_>> = freqs.iter().map(|_| vec![]).collect();
    let fft_size = 4096 * 4;
    let mut low_range_ctx = ImbContext::new(conf.angular.points_per_turn(), conf.angular.fft_turns());
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);
    let mut buffer = FftBuffer::new(fft_size, 1024);
    for i in 0..1000 { // Выборок из АЦП
        // для тестирования вручную имитируем изменение rpm привода
        let rpm =  3000.0;
        let rpm_amp = 0;
        // Имитируем получение АЦП выборки из сети
        udp.parse(rpm, rpm_amp, &mut samples);
        let frame = Arc::new(Frame {
            samples: samples.map(|v| v as f32 - 2047.5),    // убираем DC
            phases: [0.0; Frame::SIZE],
        });
        low_range_ctx.rpm = rpm;    // Имитируем чтение текущей частоты, в работе делает ReadInpurs,
        low_range_ctx.update(frame);
        // println!("Before filter: {:?}", low_range_ctx.frame.samples);
        low_range_ctx = low_range.eval(low_range_ctx);
        let mut fft_buf = vec![Complex{re: 0.0, im: 0.0}; fft_size];
        // buffer.add(samples.iter().map(|v| Complex{re: *v as f32, im: 0.0}));
        buffer.add(low_range_ctx.samples.iter().map(|v| Complex{re: *v, im: 0.0}));
        // println!("After filter: {:?}", low_range_ctx.samples);
        if buffer.is_full() {
            buffer.copy_into(&mut fft_buf);
            fft.process(&mut fft_buf);
            let delta_f = f_sample as f64 / fft_size as f64;
            // println!("delta_f: {}", delta_f);
            for (i, (freq, target)) in freqs.iter().enumerate() {
                let n = (freq / delta_f).round() as usize;
                let amp = |v: Complex<f32>| {
                    2.0 * v.abs() / fft_size as f32
                };
                // for offset in -4..=4 {
                //     let idx = (n as isize).checked_add(offset).unwrap_or(0) as usize;
                //     if idx < fft_buf.len() {
                //         println!("buffer[{n}{:+}] (бин {}): {}", offset, idx, amp(fft_buf[idx]));
                //     }
                // }
                let result = (amp(fft_buf[n-1]).powi(2)
                    + amp(fft_buf[n]).powi(2)
                    + amp(fft_buf[n+1]).powi(2))
                    .sqrt();
                results[i].push(result);
                // println!("result: {}", result);
            }
            // assert!((result - 100.0).abs() < 50.0);
            buffer.reset();
        }
    }
    for (i, r) in results.iter().enumerate() {
        let s: f32 = r.iter().sum();
        println!("result f {}: {}", freqs[i].0, s / r.len() as f32);
    }
    // Проверяем полосу пропускания (низкие частоты 1x..3x должны пройти)
    assert!((results[0].iter().sum::<f32>() / results[0].len() as f32 - 200.0).abs() < 15.0, "1x (50Hz) attenuation is too high");
    assert!((results[1].iter().sum::<f32>() / results[1].len() as f32 - 250.0).abs() < 10.0, "2x (100Hz) amplitude mismatched");
    assert!((results[2].iter().sum::<f32>() / results[2].len() as f32 - 150.0).abs() < 20.0, "3x (150Hz) unexpected attenuation");
    // Проверяем полосу подавления (шумы выше 1000 Гц должны быть жестко зарезаны)
    let mean_1khz = results[3].iter().sum::<f32>() / results[3].len() as f32;
    let mean_3khz = results[4].iter().sum::<f32>() / results[4].len() as f32;
    let mean_5khz = results[5].iter().sum::<f32>() / results[5].len() as f32;
    let mean_7khz = results[6].iter().sum::<f32>() / results[6].len() as f32;
    let mean_12khz = results[7].iter().sum::<f32>() / results[7].len() as f32;
    assert!(mean_1khz < 10.0, "LowPassFilter failed to suppress 1 kHz noise (got {})", mean_1khz);
    assert!(mean_3khz < 2.0,  "LowPassFilter failed to suppress 3 kHz noise (got {})", mean_3khz);
    assert!(mean_5khz < 1.0,  "LowPassFilter failed to suppress 5 kHz noise (got {})", mean_5khz);
    assert!(mean_7khz < 1.0,  "LowPassFilter failed to suppress 7 kHz noise (got {})", mean_7khz);
    assert!(mean_12khz < 1.0,  "LowPassFilter failed to suppress 12 kHz noise (got {})", mean_12khz);

}
struct FftBuffer<T> {
    size: usize,
    buf: SliceRingBuffer<Complex<T>>,
    step: usize,
    new_samples: usize,
}
impl<T: Copy> FftBuffer<T> {
    pub fn new(size: usize, step: usize) -> Self {
        Self {
            size,
            buf: SliceRingBuffer::with_capacity(size),
            step,
            new_samples: 0,
        }
    }
    pub fn add(&mut self, values: impl IntoIterator<Item = Complex<T>>) {
        for v in values.into_iter() {
            if self.buf.len() >= self.size {
                self.buf.pop_front();
            }
            self.buf.push_back(v);
            self.new_samples += 1;
        }
    }
    pub fn copy_into(&self, target: &mut [Complex<T>]) {
        let src = self.buf.as_slice();
        let len = src.len().min(target.len());
        target[..len].copy_from_slice(&src[..len]);
    }
    pub fn is_full(&self) -> bool {
        self.buf.len() >= self.size && self.new_samples >= self.step
    }
    pub fn reset(&mut self) {
        self.new_samples = 0;
    }
}
