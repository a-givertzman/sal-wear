#[cfg(test)]
mod frequency_refinement {
    use std::f64::consts::PI;
    use sal_core::dbg::Dbg;
    use crate::{
        AngularCtx, Eval, FSampleDec, TrackState,
        ComplexLocalOscillator, LPFTrackingBand, InstantShaftPhase,
        ValidityDetector, CaptureMode, StandardClarification,
        LPFTrackingBandCtx,
    };

    pub struct MockChild;
    impl Eval<AngularCtx, AngularCtx> for MockChild {
        fn eval(&self, ctx: AngularCtx) -> AngularCtx { ctx }
        fn exit(&self) {}
    }

    const FS: f64 = 1000.0;      // децимированная частота
    const F_TRUE: f64 = 50.3;    // истинная частота вала (сеть врет: 3000 RPM = 50.0)
    const F_CUTOFF: f64 = 3.5;   // полоса слежения B/2
    const CHUNK: usize = 200;    // 10 периодов 1X на чанк
    const A_WEAK: f64 = 6.0;     // огибающая 3.0 < a_high: валидности нет, захват возможен
    const A_STRONG: f64 = 15.0;  // огибающая 7.5 > a_high: штатное слежение

    /// Чанк вибросигнала: 1X + вторая гармоника + мусор "редуктора".
    /// k — сквозной номер сэмпла (непрерывность фазы между чанками).
    fn push_chunk(ctx: &mut AngularCtx, k: &mut usize, ampl: f64) {
        ctx.rpm_detection.decimation.decimated.clear(); // дециматор выдает свежий чанк
        for _ in 0..CHUNK {
            let t = *k as f64 / FS;
            *k += 1;
            let x = ampl * (2.0 * PI * F_TRUE * t).cos()
                + 0.3 * ampl * (2.0 * PI * 2.0 * F_TRUE * t).sin()
                + 3.0 * (2.0 * PI * 37.7 * t).sin();
            ctx.rpm_detection.decimation.decimated.push(x);
        }
    }

    fn make_ctx() -> AngularCtx {
        let mut ctx = AngularCtx::default();
        ctx.raw_rpm = 3000.0; // сеть: 50.0 Гц, истина 50.3
        ctx.rpm_detection.decimation.f_sample_dec = FSampleDec(FS);
        // f_precise = None — гетеродин стартует от сетевого RPM
        ctx.rpm_detection.capture_mode.f_precise = None;
        ctx.rpm_detection.capture_mode.search_half_window_hz = 5.0;
        ctx.rpm_detection.capture_mode.search_steps = 10;
        ctx.rpm_detection.lpf_tracking_band =
            LPFTrackingBandCtx::new(FSampleDec(FS), CHUNK + 1, F_CUTOFF);
        ctx
    }

    /// Тест 1: Полный жизненный цикл — захват частоты, затем штатное слежение и уточнение.
    #[test]
    fn test_acquisition_then_tracking_lifecycle() {
        let parent_dbg = Dbg::own("full pipeline test");
        // Цепочка: гетеродин -> ФНЧ -> фаза -> валидность -> захват -> уточнение
        let node = StandardClarification::new(&parent_dbg,
            CaptureMode::new(&parent_dbg,
                ValidityDetector::new(&parent_dbg,
                    InstantShaftPhase::new(&parent_dbg,
                        LPFTrackingBand::new(&parent_dbg,
                            ComplexLocalOscillator::new(&parent_dbg, MockChild))),
                    2.0, 4.0, 40))); // a_low, a_high, settle_samples

        let mut ctx = make_ctx();
        let mut k = 0;

        // === Фаза 1: старт системы, слабый сигнал — должен сработать ЗАХВАТ ===
        push_chunk(&mut ctx, &mut k, A_WEAK);
        ctx = node.eval(ctx);

        let cap = &ctx.rpm_detection.capture_mode;
        assert!(cap.is_updated, "Захват не сработал на слабом сигнале");
        let f_captured = cap.f_precise.unwrap();
        assert!((f_captured - F_TRUE).abs() < 0.3, // T = 200 сэмплов / 1000 Гц = 0.2 с  →  разрешение ~5 Гц на бин, тк один чанк
            "Захват: {}, истина {}", f_captured, F_TRUE);

        // === Фаза 2: сигнал усилился — штатное слежение, уточнение и коррекция ===
        for _ in 0..30 {
            push_chunk(&mut ctx, &mut k, A_STRONG);
            ctx = node.eval(ctx);
        }

        // Тракт валиден
        assert!(ctx.rpm_detection.validity_detector.valid);
        assert_eq!(ctx.rpm_detection.validity_detector.state, TrackState::Valid);
        // Уточнитель измерил частоту точнее, чем дала сеть
        let f_prec = ctx.rpm_detection.standard_clarification.f_precise_hz.unwrap();
        assert!((f_prec - F_TRUE).abs() < 1e-4,
            "f_precise = {}, истина {}", f_prec, F_TRUE);
        // Разомкнутая коррекция стянула гетеродин к истине: δ -> 0
        let f_het = ctx.rpm_detection.capture_mode.f_precise.unwrap();
        assert!((f_het - F_TRUE).abs() < 1e-4,
            "гетеродин после коррекции: {}, истина {}", f_het, F_TRUE);
        assert!(ctx.rpm_detection.standard_clarification.delta_hz.abs() < 0.05);
    }

    /// Тест 2: Потеря сигнала в штатном режиме -> DEGRADED/ACQUISITION -> восстановление.
    #[test]
    fn test_signal_loss_and_recovery() {
        let parent_dbg = Dbg::own("full pipeline test");
        let node = StandardClarification::new(&parent_dbg,
            CaptureMode::new(&parent_dbg,
                ValidityDetector::new(&parent_dbg,
                    InstantShaftPhase::new(&parent_dbg,
                        LPFTrackingBand::new(&parent_dbg,
                            ComplexLocalOscillator::new(&parent_dbg, MockChild))),
                    2.0, 4.0, 40)));

        let mut ctx = make_ctx();
        let mut k = 0;

        // === Выходим на слежение ===
        push_chunk(&mut ctx, &mut k, A_WEAK);
        ctx = node.eval(ctx);
        for _ in 0..10 {
            push_chunk(&mut ctx, &mut k, A_STRONG);
            ctx = node.eval(ctx);
        }
        assert!(ctx.rpm_detection.validity_detector.valid);

        // === Пропал сигнал: помеха задавила 1X ===
        for _ in 0..3 {
            push_chunk(&mut ctx, &mut k, 0.0);
            ctx = node.eval(ctx);
        }
        // Тракт невалиден; settle_samples = 40 < 3*200 — должны уйти в захват
        assert!(!ctx.rpm_detection.validity_detector.valid);
        assert_eq!(ctx.rpm_detection.validity_detector.state, TrackState::Acquisition);

        // === Сигнал вернулся — восстановление без потери нити фазы ===
        for _ in 0..20 {
            push_chunk(&mut ctx, &mut k, A_STRONG);
            ctx = node.eval(ctx);
        }
        assert!(ctx.rpm_detection.validity_detector.valid);
        let f_prec = ctx.rpm_detection.standard_clarification.f_precise_hz.unwrap();
        assert!((f_prec - F_TRUE).abs() < 0.05,
            "после восстановления: {}, истина {}", f_prec, F_TRUE);
    }
}