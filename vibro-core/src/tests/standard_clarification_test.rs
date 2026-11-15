#[cfg(test)]
mod standard_clarification {
    use std::f64::consts::TAU;
    use sal_core::dbg::Dbg;
    use rustfft::num_complex::Complex; // поправьте под ваш импорт, как в LPFTrackingBand
    use crate::{
        AngularCtx,
        StandardClarification,
        StandardClarificationCtx,
        Eval,
        FSampleDec
    };
    pub struct MockChild;
    impl Eval<AngularCtx, AngularCtx> for MockChild {
        fn eval(&self, ctx: AngularCtx) -> AngularCtx { ctx }
        fn exit(&self) {}
    }
    const FS: f64 = 16000.0; // f_sample_dec
    /// Добавляет n сэмплов огибающей, вращающейся с частотой delta, Гц.
    /// k — сквозной номер сэмпла (сохраняет непрерывность фазы между чанками).
    fn push_samples(ctx: &mut AngularCtx, k: &mut usize, n: usize, delta: f64) {
        for _ in 0..n {
            let ph = TAU * delta * (*k as f64) / FS;
            *k += 1;
            ctx.rpm_detection.lpf_tracking_band.filtered_signal
                .push(Complex::new(ph.cos(), ph.sin()));
        }
    }
    /// Базовый контекст: сеть сообщает 50 Гц (3000 RPM), тракт валиден
    fn make_ctx() -> AngularCtx {
        let mut ctx = AngularCtx::default();
        ctx.raw_rpm = 3000.0;
        ctx.rpm_detection.capture_mode.f_precise = Some(50.0); // f_het
        ctx.rpm_detection.decimation.f_sample_dec = FSampleDec(FS);
        ctx.rpm_detection.validity_detector.valid = true;
        ctx
    }
    /// Тест 1: До закрытия первого окна (1280 сэмплов на 50 Гц) результат не публикуется.
    #[test]
    fn test_no_publication_before_first_window() {
        let mut ctx = make_ctx();
        let mut k = 0;
        push_samples(&mut ctx, &mut k, 26, 0.5);
        let parent_dbg = Dbg::own("standard clarification test");
        let node = StandardClarification::new(&parent_dbg, MockChild);
        let res_ctx = node.eval(ctx);
        assert!(res_ctx.rpm_detection.standard_clarification.f_precise_hz.is_none());
        assert_eq!(res_ctx.rpm_detection.standard_clarification.window_samples, 25); // 26 сэмплов минус базовый
    }
    /// Тест 2: На чистой огибающей δ измеряется точно, f_precise = f_het + δ.
    #[test]
    fn test_delta_measured_on_clean_signal() {
        let mut ctx = make_ctx();
        let mut k = 0;
        push_samples(&mut ctx, &mut k, 1300, 0.5); // чуть больше окна 1280 — одна публикация
        let parent_dbg = Dbg::own("standard clarification test");
        let node = StandardClarification::new(&parent_dbg, MockChild);
        let res_ctx = node.eval(ctx);
        let fr = &res_ctx.rpm_detection.standard_clarification;
        assert!((fr.delta_hz - 0.5).abs() < 1e-9, "δ = {}, истина 0.5", fr.delta_hz);
        assert!((fr.f_precise_hz.unwrap() - 50.5).abs() < 1e-9);
    }
    /// Тест 3: Оценка за пределами полосы слежения обрезается f_cutoff.
    #[test]
    fn test_delta_clamped_to_f_cutoff() {
        let mut ctx = make_ctx();
        let mut k = 0;
        push_samples(&mut ctx, &mut k, 1300, 10.0); // δ = 10 Гц при f_cutoff = 3.5
        let parent_dbg = Dbg::own("standard clarification test");
        let node = StandardClarification::new(&parent_dbg, MockChild);
        let res_ctx = node.eval(ctx);
        let fr = &res_ctx.rpm_detection.standard_clarification;
        assert_eq!(fr.delta_hz, 3.5);
        assert_eq!(fr.f_precise_hz.unwrap(), 53.5);
    }
    /// Тест 4: В штатном режиме (valid == true) гетеродин корректируется на gain·δ.
    #[test]
    fn test_correction_applied_when_valid() {
        let mut ctx = make_ctx(); // gain = 0.25 из Default
        let mut k = 0;
        push_samples(&mut ctx, &mut k, 1300, 0.5);
        let parent_dbg = Dbg::own("standard clarification test");
        let node = StandardClarification::new(&parent_dbg, MockChild);
        let res_ctx = node.eval(ctx);
        // f_het = 50.0 + 0.25 * 0.5 = 50.125
        let f_new = res_ctx.rpm_detection.capture_mode.f_precise.unwrap();
        assert!((f_new - 50.125).abs() < 1e-9, "f_het после коррекции: {}", f_new);
    }
    /// Тест 5: В режиме невалидности (valid == false) коррекция НЕ выполняется, измерение идет.
    #[test]
    fn test_no_correction_when_invalid() {
        let mut ctx = make_ctx();
        ctx.rpm_detection.validity_detector.valid = false; // АВАРИЯ
        let mut k = 0;
        push_samples(&mut ctx, &mut k, 1300, 0.5);
        let parent_dbg = Dbg::own("standard clarification test");
        let node = StandardClarification::new(&parent_dbg, MockChild);
        let res_ctx = node.eval(ctx);
        // Измерение опубликовано, но гетеродин не тронут
        assert!(res_ctx.rpm_detection.standard_clarification.f_precise_hz.is_some());
        assert_eq!(res_ctx.rpm_detection.capture_mode.f_precise, Some(50.0));
    }
    /// Тест 6: Окно, растянутое на много чанков, закрывается ровно один раз с той же точностью.
    #[test]
    fn test_window_spanning_multiple_chunks() {
        let mut ctx = make_ctx();
        let parent_dbg = Dbg::own("standard clarification test");
        let node = StandardClarification::new(&parent_dbg, MockChild);
        let mut k = 0;
        // 50 чанков по 26 сэмплов = 1300 — окно 1280 закроется один раз на 50-м чанке
        for _ in 0..50 {
            let mut chunk = AngularCtx::default();
            chunk.raw_rpm = 3000.0;
            chunk.rpm_detection.capture_mode.f_precise = ctx.rpm_detection.capture_mode.f_precise;
            chunk.rpm_detection.decimation.f_sample_dec = FSampleDec(FS);
            chunk.rpm_detection.validity_detector.valid = true;
            // Переносим накопленное состояние между чанками
            chunk.rpm_detection.standard_clarification = ctx.rpm_detection.standard_clarification.clone();
            push_samples(&mut chunk, &mut k, 26, 0.5);
            ctx = node.eval(chunk);
        }
        let fr = &ctx.rpm_detection.standard_clarification;
        assert!((fr.delta_hz - 0.5).abs() < 1e-9, "δ через границы чанков: {}", fr.delta_hz);
        assert!((fr.f_precise_hz.unwrap() - 50.5).abs() < 1e-9);
    }
    /// Тест 7: reset() очищает состояние и результат, но сохраняет настройку.
    #[test]
    fn test_reset_preserves_settings() {
        let mut ctx = StandardClarificationCtx::new(8.0, 0.3, 128, 2.5);
        ctx.arg_unwrapped = 12.3;
        ctx.prev_arg = Some(1.0);
        ctx.window_samples = 500;
        ctx.delta_hz = 0.7;
        ctx.f_precise_hz = Some(50.7);
        ctx.reset();
        assert_eq!(ctx.arg_unwrapped, 0.0);
        assert_eq!(ctx.prev_arg, None);
        assert_eq!(ctx.window_samples, 0);
        assert_eq!(ctx.delta_hz, 0.0);
        assert_eq!(ctx.f_precise_hz, None);
        // Настройка на месте — регрессия против "*self = default()"
        assert_eq!(ctx.revs_per_window, 8.0);
        assert_eq!(ctx.correction_gain, 0.3);
        assert_eq!(ctx.min_window_samples, 128);
        assert_eq!(ctx.f_cutoff, 2.5);
    }
}