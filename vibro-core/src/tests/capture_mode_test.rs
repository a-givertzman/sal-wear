#[cfg(test)]
mod capture_mode {
    use std::f64::consts::PI;
    use sal_core::dbg::Dbg;
    use crate::{
        AngularCtx,
        CaptureMode, 
        Eval,
        FSampleDec
    };
    // Вспомогательная структура-заглушка для Child узла
    pub struct MockChild;
    impl Eval<AngularCtx, AngularCtx> for MockChild {
        fn eval(&self, ctx: AngularCtx) -> AngularCtx { ctx }
        fn exit(&self) {}
    }
    /// Тест 1: Проверка математики параболической интерполяции (идеальный случай).
    #[test]
    fn test_parabolic_peak_exact() {
        // Парабола вида: y = -x^2 + 0.2x + 10.
        // Вершина аналитически находится в x = 0.1
        let y1 = -(-1.0f64).powi(2) + 0.2 * (-1.0) + 10.0; // x = -1
        let y2 = -(0.0f64).powi(2) + 0.2 * (0.0) + 10.0;  // x = 0
        let y3 = -(1.0f64).powi(2) + 0.2 * (1.0) + 10.0;  // x = 1

        let shift = CaptureMode::<MockChild>::parabolic_peak(y1, y2, y3);
        
        // Погрешность плавающей точки должна быть минимальной
        assert!((shift - 0.1).abs() < 1e-12);
    }
    /// Тест 2: Защита параболы от деления на ноль при плоском сигнале.
    #[test]
    fn test_parabolic_peak_flat_line() {
        let shift = CaptureMode::<MockChild>::parabolic_peak(5.0, 5.0, 5.0);
        assert_eq!(shift, 0.0);
    }
    /// Тест 3: Проверка алгоритма Герцеля на чистой синусоиде.
    #[test]
    fn test_goertzel_pure_sine() {
        let f_sample = 1000.0;
        let f_target = 50.0;
        
        // Генерируем 1 секунду синусоиды 50 Гц с амплитудой 10.0
        let mut buf = Vec::new();
        for n in 0..1000 {
            let t = n as f64 / f_sample;
            buf.push(10.0 * (2.0 * PI * f_target * t).sin());
        }

        let amp = CaptureMode::<MockChild>::goertzel(&buf, f_target, FSampleDec(f_sample));
        
        assert!(
            (amp - 10.0).abs() < 1e-9, 
            "Амплитуда ожидалась ~10.0, но алгоритм вернул: {amp}"
        );
    }
    /// Тест 4: В штатном режиме (valid == true) алгоритм поиска НЕ должен запускаться.
    #[test]
    fn test_eval_skips_when_valid() {
        let mut ctx = AngularCtx::default();
        ctx.rpm_detection.validity_detector.valid = true; // ШТАТНЫЙ РЕЖИМ
        ctx.raw_rpm = 3000.0; // 50 Гц
        ctx.rpm_detection.capture_mode.search_half_window_hz = 2.0;
        ctx.rpm_detection.capture_mode.search_steps = 5;
        
        // Наполняем буфер синусоидой, превышающей порог шума
        let f_sample = 1000.0;
        for n in 0..500 {
            ctx.rpm_detection.capture_mode.signal_buffer.push_back(20.0 * (2.0 * PI * 50.0 * (n as f64 / f_sample)).sin());
        }
        ctx.rpm_detection.decimation.f_sample_dec = FSampleDec(f_sample);

        let parent_dbg = Dbg::own("capture mode test");
        let node = CaptureMode::new(&parent_dbg, MockChild);
        
        let res_ctx = node.eval(ctx);

        // Так как valid == true, вычисления не должны изменять флаг и частоту
        assert_eq!(res_ctx.rpm_detection.capture_mode.is_updated, false);
        assert_eq!(res_ctx.rpm_detection.capture_mode.f_precise, Some(0.0));
    }
    /// Тест 5: В режиме аварии (valid == false) и сильном сигнале частота должна успешно захватываться.
    #[test]
    fn test_eval_captures_when_invalid_and_strong_signal() {
        let mut ctx = AngularCtx::default();
        ctx.rpm_detection.validity_detector.valid = false; // АВАРИЯ (РЕЖИМ ЗАХВАТА)
        ctx.raw_rpm = 3000.0; // Ожидаем 50 Гц
        ctx.rpm_detection.capture_mode.search_half_window_hz = 5.0; // Окно поиска 45-55 Гц
        ctx.rpm_detection.capture_mode.search_steps = 10;
        ctx.rpm_detection.capture_mode.buffer_len = 1000;
        // Сигнал с частотой 50.2 Гц (чуть смещен от сетки) и амплитудой 15.0 (выше NOIZE_THRESHOLD)
        let f_sample = 1000.0;
        let f_real = 50.2;
        for n in 0..1000 {
            ctx.rpm_detection.capture_mode.signal_buffer.push_back(15.0 * (2.0 * PI * f_real * (n as f64 / f_sample)).sin());
        }
        ctx.rpm_detection.decimation.f_sample_dec = FSampleDec(f_sample);

        let parent_dbg = Dbg::own("capture mode test");
        let node = CaptureMode::new(&parent_dbg, MockChild);
        
        let res_ctx = node.eval(ctx);
        // Проверяем успешность захвата
        assert_eq!(res_ctx.rpm_detection.capture_mode.is_updated, true);
        // Точность интерполяции должна найти наши 50.2 Гц с высокой точностью
        assert!((res_ctx.rpm_detection.capture_mode.f_precise.unwrap() - f_real).abs() < 0.05);
    }
    /// Тест 6: Фильтрация по NOIZE_THRESHOLD. Слабый сигнал ниже порога должен игнорироваться.
    #[test]
    fn test_eval_rejects_weak_signal_noise() {
        let mut ctx = AngularCtx::default();
        ctx.rpm_detection.validity_detector.valid = false;
        ctx.raw_rpm = 3000.0; 
        ctx.rpm_detection.capture_mode.search_half_window_hz = 2.0;
        ctx.rpm_detection.capture_mode.search_steps = 5;
        // Очень слабый сигнал с амплитудой 1.0 (заведомо ниже вашего NOIZE_THRESHOLD, который равен 5.0)
        let f_sample = 1000.0;
        for n in 0..500 {
            ctx.rpm_detection.capture_mode.signal_buffer.push_back(1.0 * (2.0 * PI * 50.0 * (n as f64 / f_sample)).sin());
        }
        ctx.rpm_detection.decimation.f_sample_dec = FSampleDec(f_sample);
        let parent_dbg = Dbg::own("capture mode test");
        let node = CaptureMode::new(&parent_dbg, MockChild);
        let res_ctx = node.eval(ctx);
        // Сигнал ниже порога — захват должен отклониться
        assert_eq!(res_ctx.rpm_detection.capture_mode.is_updated, false);
    }
}
