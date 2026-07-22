use std::sync::Arc;
use debugging::session::debug_session::{DebugSession, LogLevel};
use rustfft::{FftPlanner, num_complex::Complex};
use sal_core::dbg::Dbg;
use std::io::{Write, BufWriter};
use crate::{AngularGrid, Autocorrelation, Conf, Context, Eval, Frame, ImbContext, Inputs, OrderDomainSamples, Pass, ReadInputs, tests::{FftBuffer, Frequency, Udp}};
use std::fs::File;

/// Функциональное тестирование [OrderDomainSamples] на стационарность при разгоне
#[test]
fn order_domain_stationary_test() {
    DebugSession::new().filter(LogLevel::Debug).init();
    let dbg = Dbg::own("OrderDomainSamples-test");
    let f_sample = 320_000; // Частота семплирования АЦП (Гц)
    let conf: Conf = serde_yaml::from_str(&format!(r#"
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
    "#)).unwrap();
    let mut samples = [0u16; Frame::SIZE];
    let low_range = OrderDomainSamples::new(
        &dbg,
        conf.angular.points_per_turn(),
        Pass::new(),
    );

    // В соответствии с ТЗ: постоянная вибрация только на 1-м порядке
    let freqs = [
        (Frequency::Rpm(1.0), 200),
    ];
    
    let inputs = Arc::new(Inputs::new());
    let mut ctx = Context::new();
    ctx.dt = 1.0 / f_sample as f64; 
    let angular_grid = AngularGrid::new(
        &dbg,
        Autocorrelation::new(
            &dbg,
            conf.hardware.sample_rate_hz,
            ReadInputs::new(&dbg, inputs.clone())
        ),
    );

    let mut udp = Udp::new(Frame::SIZE, conf.hardware.sample_rate_hz, freqs.clone());
    let mut results: Vec<Vec<f64>> = freqs.iter().map(|_| vec![]).collect();
    let mut i_ctx = ImbContext::new(conf.angular.points_per_turn(), conf.angular.fft_turns());

    // Увеличиваем размер FFT для обеспечения максимальной узкости пика
    let fft_size = 16384; 
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);
    let mut buffer = FftBuffer::new(fft_size, 1024);

    let rpm_start = 5000.0;
    let rpm_end = 6000.0;
    let n_iters = 2000; // Увеличиваем плавность разгона (больше итераций — меньше шаг RPM)

    let mut file_raw = BufWriter::new(File::create("/home/debian/Documents/python_test/input/raw_time_signal.txt").unwrap());
    let mut file_angular = BufWriter::new(File::create("/home/debian/Documents/python_test/input/raw_angular_signal.txt").unwrap());
    
    let mut last_fft_spectrum: Vec<f64> = Vec::new();

    for iter in 0..n_iters {
        let progress = iter as f64 / n_iters as f64;
        let rpm = rpm_start + (rpm_end - rpm_start) * progress;
        
        // Эмулятор генерирует чанк данных для текущего мгновенного RPM
        udp.parse(rpm, &mut samples);
        
        // Запись сырого временного сигнала (сжатие синусоиды во времени)
        for sample in samples.iter() {
            writeln!(file_raw, "{}", sample).unwrap();
        }

        inputs.set_rpm(rpm);
        ctx.push_chunk(&samples);
        ctx = angular_grid.eval(ctx);
        println!("{:?}", ctx.phases);
        let frame = Arc::new(Frame {
            samples: samples.map(|v| v as f32 - 2048.0),
            phases: ctx.phases.to_vec(),
        });

        i_ctx.update(frame.clone());
        i_ctx.rpm = rpm;
        i_ctx = low_range.eval(i_ctx);

        // Запись углового сигнала. Ожидаем строго стабильный период!
        for complex in i_ctx.order_samples.iter() {
            writeln!(file_angular, "{}\t{}", complex.re, complex.im).unwrap();
        }

        buffer.add(i_ctx.order_samples.iter().map(|c| Complex { re: c.re as f64, im: c.im as f64 }));
        
        if buffer.is_full() {
            let mut fft_buf = vec![Complex { re: 0.0, im: 0.0 }; fft_size];
            buffer.copy_into(&mut fft_buf);
            
            // ПРИМЕНЕНИЕ ОКНА ХЕММИНГА
            let n_len = fft_buf.len();
            for i in 0..n_len {
                let angle = (2.0 * std::f64::consts::PI * i as f64) / (n_len - 1) as f64;
                let w = 0.54 - 0.46 * angle.cos();
                fft_buf[i].re *= w;
                fft_buf[i].im *= w;
            }
            
            fft.process(&mut fft_buf);
            
            let amp = |v: Complex<f64>| {
                let norm = (v.re * v.re + v.im * v.im).sqrt();
                let coherent_gain = 0.54;
                (2.0 * norm) / (fft_size as f64 * coherent_gain)
            };

            last_fft_spectrum.clear();
            for i in 0..(fft_size / 2) {
                last_fft_spectrum.push(amp(fft_buf[i]));
            }

            let points_per_turn = conf.angular.points_per_turn() as f64;
            let delta_order = points_per_turn / fft_size as f64;
            
            // Перезаписываем спектр. Самый последний буфер запишется в конце
            let mut file_fft = File::create("/home/debian/Documents/python_test/input/fft_spectrum.txt").unwrap();
            for (bin, amplitude) in last_fft_spectrum.iter().enumerate() {
                let order = bin as f64 * delta_order;
                writeln!(file_fft, "{}\t{}", order, amplitude).unwrap();
            }
            println!("frame ready");
            file_raw.flush().unwrap();
            file_angular.flush().unwrap();
            // Поиск строго по ордерной сетке
            for (i, (freq, _)) in freqs.iter().enumerate() {
                let target_order = match freq {
                    Frequency::Rpm(k) => *k,
                    Frequency::Static(f_hz) => f_hz / (rpm / 60.0),
                };
                
                let n = (target_order / delta_order).round() as usize;
                
                if n > 0 && n < fft_buf.len() - 1 {
                    // Энергетическая сумма трех бинов для компенсации микро-размытия окна Хемминга
                    let result = (amp(fft_buf[n-1]).powi(2)
                        + amp(fft_buf[n]).powi(2)
                        + amp(fft_buf[n+1]).powi(2))
                        .sqrt();
                    results[i].push(result);
                }
            }
            buffer.reset();
        }
    }    
    println!("Results (1X): {:?}", results);
    let result = results[0].iter().sum::<f64>() / results[0].len() as f64;
    
    // Проверка ТЗ: Амплитуда на 1-м ордере должна быть строго около 200
    assert!((result - 200.0).abs() < 10.0, "1x amplitude mismatched: {result}");
}
