//! # Комплексные тесты полного вычислительного конвейера определения RPM и фазы вала
//!
//! ## Что здесь тестируется
//!
//! В отличие от юнит-тестов отдельных узлов, здесь собирается **вся цепочка обработки**,
//! от сырого «сетевого пакета АЦП» до уточнённой частоты вращения:
//!
//! ```text
//!   Udp (эмулятор АЦП)
//!        │  сырые u16-отсчёты, 512 шт @ 20 кГц
//!        ▼
//!   Decimation             — КИХ-дециматор M=20 → 1000 Гц внутри тракта
//!        ▼
//!   ComplexLocalOscillator — комплексный гетеродин: перенос 1X к нулевой частоте
//!        ▼
//!   LPFTrackingBand        — каскад 2 биквадов (ФНЧ 4-го порядка, полоса слежения ±3.5 Гц)
//!        ▼
//!   InstantShaftPhase      — мгновенная фаза вала: atan2(Q,I) + φ_het, с линией задержки
//!        ▼
//!   ValidityDetector       — двухпороговый гистерезис по огибающей → Valid/Degraded/Acquisition
//!        ▼
//!   CaptureMode            — режим захвата: сетка Герцеля по буферу истории + парабола
//!        ▼
//!   StandardClarification  — штатное уточнение: δ по дрейфу arg(y) + разомкнутая коррекция
//! ```
//!
//! ## Принципы стенда
//!
//! 1. **Один генератор на тест, фаза непрерывна.** `Udp` копит фазы гармоник внутри себя
//!    посэмплово. Амплитуда меняется не новым генератором, а множителем `scale` при
//!    заполнении чанка — поэтому фаза 1X непрерывна через все фазы теста
//!    (слабый/сильный/тишина), как у реального вала: вал физически не может
//!    «перескочить» по фазе при изменении амплитуды вибрации.
//!
//! 2. **Сеть врёт.** `raw_rpm = 3000`, истина — `RPM_TRUE = 3018` (50.3 Гц).
//!    Система обязана найти истину сама: захватом по буферу и уточнением по дрейфу.
//!
//! 3. **Бюджет амплитуды посчитан заранее** (см. `A_WEAK`): амплитуды фаз выбраны так,
//!    чтобы слабый сигнал держал систему строго в Acquisition (Герцель видит сигнал,
//!    но детектор валидности ему не доверяет), а сильный — строго в Valid.
//!
//! 4. **Один тест — один фактор.** Амплитуда постоянна внутри теста динамики (тест 3),
//!    частота постоянна внутри тестов амплитуды (1 и 2): любое изменение состояния
//!    можно однозначно приписать испытуемому фактору.
#[cfg(test)]
mod full_pipeline {
    use std::sync::Arc;

    use chrono::Utc;
    use sal_core::dbg::Dbg;
    use crate::{
        AngularCtx, CaptureMode, ComplexLocalOscillator, Decimation, Eval, FSample, Frame, InstantShaftPhase, LPFTrackingBand, Phases, StandardClarification, TrackState, ValidityDetector, tests::{Frequency, Udp}
    };

    /// Заглушка — «дно» конвейера.
    ///
    /// Каждый узел-декоратор сначала вызывает `child.eval(ctx)`, и в самом низу цепочки
    /// нужен узел, который просто вернёт контекст обратно. Сырые сэмплы уже лежат
    /// в `ctx` (положены через `push_chunk`), поэтому заглушке делать нечего.
    pub struct MockChild;
    impl Eval<AngularCtx, AngularCtx> for MockChild {
        fn eval(&self, ctx: AngularCtx) -> AngularCtx { ctx }
        fn exit(&self) {}
    }

    // =========================================================================
    // Параметры стенда
    // =========================================================================

    /// Частота дискретизации «АЦП», Гц.
    const F_SAMPLE: f64 = 20000.0;
    /// Коэффициент децимации. КИХ-коэффициенты рассчитаны строго под M = 20
    /// (`DecimationCtx::new` содержит assert), поэтому менять нельзя.
    const M: usize = 20;
    /// Размер «сетевого пакета» в сырых отсчётах.
    /// После децимации: ~25 отсчётов за чанк; длительность чанка 25.6 мс.
    const CHUNK_RAW: usize = 512;
    /// Полоса слежения ФНЧ (полу-): ±3.5 Гц вокруг частоты гетеродина.
    const F_CUTOFF: f64 = 3.5;
    /// Истинная скорость вала: 3018 RPM = 50.3 Гц.
    const RPM_TRUE: f64 = 3018.0;
    /// Истинная частота вращения, Гц. Сеть при этом «врёт»: raw_rpm = 3000 (50.0 Гц).
    const F_TRUE: f64 = RPM_TRUE / 60.0;
    /// Нулевая линия 12-битного АЦП, которую эмулирует `Udp` (2048 = 4096/2).
    const ADC_ZERO: f64 = 2048.0;

    /// Амплитуда 1X в «слабой» фазе — рабочая точка режима захвата, выбрана расчётом:
    ///
    /// ```text
    ///   10 (АЦП) × 0.778 (усиление КИХ) ≈ 7.78  → амплитуда в decimated-буфере,
    ///                                             её видит Герцель: 7.78 > NOIZE_THRESHOLD = 5
    ///                                             → захвату есть что искать
    ///   7.78 / 2 ≈ 3.89 (половина ушла в зеркальную компоненту при гетеродинировании)
    ///                                             → огибающая после ФНЧ: 3.89 < a_high = 5
    ///                                             → детектор валидности НЕ доверяет сигналу
    /// ```
    ///
    /// Итог: сигнал «есть, но слабый» — система сидит строго в Acquisition.
    /// Сильный сигнал = scale ×2 → амплитуда 20 → огибающая ~7.8 > a_high → Valid.
    /// Числа подтверждены измерением в прогонах (amp_max: 3.89–3.91 слабый, 7.75–8.19 сильный).
    const A_WEAK: u16 = 10;

    // =========================================================================
    // Инфраструктура стенда
    // =========================================================================

    /// Эмулятор АЦП: полезный сигнал 1X заданной амплитуды + реалистичные помехи.
    ///
    /// Состав спектра:
    /// - `Rpm(1.0)` — оборотная частота (полезный сигнал);
    /// - `Rpm(2.0)` — вторая гармоника вала (после гетеродина сидит на +f_1X, вне полосы);
    /// - `Static(37.7)` — межгармоника «редуктора», не кратная оборотам
    ///   (проверка, что захват не перепутает её с 1X);
    /// - `Static(4300.0)` — ВЧ-мусор выше среза дециматора (проверка антиалиасинга).
    fn make_udp(ampl_1x: u16) -> Udp {
        Udp::new(CHUNK_RAW, F_SAMPLE, vec![
            (Frequency::Rpm(1.0), ampl_1x),             // 1X - полезный сигнал
            (Frequency::Rpm(2.0), ampl_1x * 3 / 10),    // 2X - гармоника вала
            (Frequency::Static(37.7), 3),               // межгармоника "редуктора"
            (Frequency::Static(4300.0), 2),             // ВЧ мусор выше среза дециматора
        ])
    }

    /// Заполняет контекст одним «сетевым пакетом» от эмулятора АЦП.
    ///
    /// - `rpm` — текущая истинная скорость вала. Передаётся в генератор каждый чанк;
    ///   фазы гармоник внутри `Udp` накапливаются посэмплово, поэтому смена `rpm`
    ///   между чанками даёт фазонепрерывный свип (как у реального привода).
    /// - `scale` — множитель амплитуды. Позволяет менять силу сигнала, НЕ трогая
    ///   генератор: фаза 1X остаётся непрерывной даже через полную тишину (scale = 0),
    ///   потому что `parse` крутит фазы всегда.
    fn fill_chunk(ctx: &mut AngularCtx, udp: &mut Udp, rpm: f64, scale: f64) {
        let mut buf = vec![0u16; CHUNK_RAW];
        udp.parse(rpm, &mut buf); // фаза копится всегда: непрерывна при смене rpm и при scale = 0
        let samples: Vec<f64> = buf.iter()
            .map(|&v| (v as f64 - ADC_ZERO) * scale)
            .collect();
        let frame = Frame { ts: Utc::now(), samples: Arc::new(samples), phases: Phases::new(CHUNK_RAW) };
        ctx.push_chunk(&frame);
    }

    /// Собирает вычислительный контекст.
    ///
    /// Ключевые настройки:
    /// - `revs_per_window = 4.0` — окно уточнения δ равно 4 оборотам вала;
    /// - `correction_gain = 0.25` — разомкнутая коррекция переносит в f_het
    ///   четверть измеренной δ за окно (медленная петля);
    /// - `min_window_samples = 64` — защита окна от схлопывания на высоких оборотах;
    /// - `capture_history_s = 1.0` — буфер истории захвата: 1000 отсчётов при 1000 Гц;
    /// - `search_steps = 10` — сетка Герцеля: 10 шагов в каждую сторону от центра.
    ///
    /// После конструктора сеть «врёт»: `raw_rpm = 3000` при истине 3018,
    /// а полуокно поиска захвата — ±5 Гц вокруг сетевого значения.
    fn make_ctx() -> AngularCtx {
        let mut ctx = AngularCtx::new(
            FSample(F_SAMPLE),
            CHUNK_RAW,
            M,
            F_CUTOFF,
            4.0,   // revs_per_window
            0.25,  // correction_gain
            64,    // min_window_samples
            1.0,   // capture_history_s: 1 с истории при 1000 Гц -> buffer 1000
            10,    // search_steps
        );
        ctx.raw_rpm = 3000.0;
        ctx.rpm_detection.capture_mode.search_half_window_hz = 5.0;
        ctx
    }

    /// Собирает полную цепочку узлов.
    ///
    /// Порядок вложения читается изнутри наружу по потоку данных:
    /// Децимация → Гетеродин → ФНЧ → Фаза → Валидность → Захват → Уточнение.
    /// Пороги детектора валидности: `a_low = 2.0`, `a_high = 5.0`,
    /// `settle_samples = 40` (после 40 подряд невалидных отсчётов — Acquisition).
    fn make_node(dbg: &Dbg) -> StandardClarification<CaptureMode<ValidityDetector<InstantShaftPhase<LPFTrackingBand<ComplexLocalOscillator<Decimation<MockChild>>>>>>> {
        StandardClarification::new(dbg,
            CaptureMode::new(dbg,
                ValidityDetector::new(dbg,
                    InstantShaftPhase::new(dbg,
                        LPFTrackingBand::new(dbg,
                            ComplexLocalOscillator::new(dbg,
                                Decimation::new(dbg, MockChild)))),
                    2.0, 5.0, 40))) // a_low, a_high, settle_samples
    }

    /// Один такт стенда: принять пакет от «АЦП» → прогнать всю цепочку → залогировать.
    ///
    /// В лог идут ключевые телеметрические величины: флаг валидности, частота
    /// гетеродина, измеренная δ, уточнённая частота и максимум огибающей за чанк.
    ///
    /// Встроенный инвариант: любая ошибка тракта (`ctx.err`) — немедленный провал теста.
    fn run_chunk(node: &impl Eval<AngularCtx, AngularCtx>, mut ctx: AngularCtx, udp: &mut Udp, rpm: f64, scale: f64) -> AngularCtx {
        fill_chunk(&mut ctx, udp, rpm, scale);
        let ctx = node.eval(ctx);
        let d = &ctx.rpm_detection;
        log::debug!("valid {} | f_het {:8.4} | δ {:+7.4} | f_prec {:?} | amp_max {:6.2}",
            d.validity_detector.valid,
            d.capture_mode.f_precise.unwrap_or(f64::NAN),
            d.standard_clarification.delta_hz,
            d.standard_clarification.f_precise_hz,
            d.lpf_tracking_band.amplitude.iter().cloned().fold(0.0, f64::max),
        );
        assert!(ctx.err.is_none(), "ошибка тракта: {:?}", ctx.err);
        ctx
    }

    // =========================================================================
    // Тесты
    // =========================================================================

    /// ## Тест 1: Полный жизненный цикл — захват → штатное слежение → уточнение и коррекция.
    ///
    /// Сценарий: система стартует на слабом сигнале (сеть врёт на 0.3 Гц),
    /// находит истину захватом, затем сигнал усиливается — система переходит
    /// в штатное слежение и стягивает гетеродин к истине разомкнутой коррекцией.
    #[test]
    fn test_full_chain_acquisition_then_tracking() {
        let parent_dbg = Dbg::own("full pipeline test");
        let node = make_node(&parent_dbg);
        let mut ctx = make_ctx();

        // === Фаза 1: старт, слабый сигнал (scale 1.0) ===
        // 40 чанков ≈ 1 с — буфер истории захвата успевает наполниться наполовину
        // (минимум для поиска), огибающая ~3.9 держит детектор в Acquisition.
        let mut udp = make_udp(A_WEAK);
        for _ in 0..40 {
            ctx = run_chunk(&node, ctx, &mut udp, RPM_TRUE, 1.0);
        }
        // Детектор валидности: огибающая 3.9 никогда не пробила a_high = 5,
        // поэтому valid = false, а за 1000 невалидных отсчётов (> settle = 40)
        // состояние — Acquisition
        assert!(!ctx.rpm_detection.validity_detector.valid);
        assert_eq!(ctx.rpm_detection.validity_detector.state, TrackState::Acquisition);
        // Захват сработал и нашёл истину вопреки вранью сети (50.0 вместо 50.3):
        // сетка Герцеля ±5 Гц вокруг сетевого значения накрывает истину
        assert!(ctx.rpm_detection.capture_mode.is_updated, "Захват не сработал");
        let f_captured = ctx.rpm_detection.capture_mode.f_precise.unwrap();
        assert!((f_captured - F_TRUE).abs() < 0.1,
            "Захват: {}, истина {}", f_captured, F_TRUE);
        // `Штатное уточнение` меряет ВСЕГДА, даже в аварийном режиме: f_het + δ ≈ истина
        // уже на слабом сигнале (коррекция при этом заблокирована — и правильно)
        let f_prec = ctx.rpm_detection.standard_clarification.f_precise_hz.unwrap();
        assert!((f_prec - F_TRUE).abs() < 0.05,
            "уточнение в фазе захвата: {}, истина {}", f_prec, F_TRUE);

        // === Фаза 2: сильный сигнал (scale ×2 → амплитуда 20, огибающая ~7.8) ===
        // Генератор тот же — фаза 1X непрерывна, меняется только громкость
        for _ in 0..30 {
            ctx = run_chunk(&node, ctx, &mut udp, RPM_TRUE, 2.0);
        }
        // Огибающая пробила a_high = 5 → детектор перешёл в доверенный режим
        assert!(ctx.rpm_detection.validity_detector.valid);
        assert_eq!(ctx.rpm_detection.validity_detector.state, TrackState::Valid);
        // В штатном режиме захват МОЛЧИТ: он гейтится состоянием Acquisition,
        // а не флагом valid — краткие провалы не должны перезаписывать f_precise
        assert!(!ctx.rpm_detection.capture_mode.is_updated, "Захват должен молчать в штатном режиме");
        // Уточнённая частота в допуске ±0.02 Гц
        let f_prec = ctx.rpm_detection.standard_clarification.f_precise_hz.unwrap();
        assert!((f_prec - F_TRUE).abs() < 0.02,
            "f_precise = {}, истина {}", f_prec, F_TRUE);
        // Разомкнутая коррекция (gain = 0.25 за окно) стянула сам гетеродин
        // к истине, а δ → 0: вектор огибающей почти не вращается — петля сошлась
        let f_het = ctx.rpm_detection.capture_mode.f_precise.unwrap();
        assert!((f_het - F_TRUE).abs() < 0.02,
            "гетеродин после коррекции: {}, истина {}", f_het, F_TRUE);
        assert!(ctx.rpm_detection.standard_clarification.delta_hz.abs() < 0.02);
    }

    /// ## Тест 2: Потеря сигнала → Acquisition → самовосстановление.
    ///
    /// Сценарий: система на слежении, затем сигнал 1X полностью пропадает
    /// (45 чанков — дольше окна истории захвата: буфер гарантированно вымывается),
    /// затем возвращается. Система обязана: пометить невалидность, НЕ испортить
    /// настройку гетеродина и самостоятельно восстановиться по возвращении сигнала.
    #[test]
    fn test_full_chain_signal_loss_and_recovery() {
        let parent_dbg = Dbg::own("full pipeline test");
        let node = make_node(&parent_dbg);
        let mut ctx = make_ctx();

        // === Выходим на слежение: 40 слабых чанков (захват) + 10 сильных (Valid) ===
        // Генератор один на весь тест: фаза 1X непрерывна через все фазы, как в железе
        let mut udp = make_udp(A_WEAK);
        for _ in 0..40 {
            ctx = run_chunk(&node, ctx, &mut udp, RPM_TRUE, 1.0);
        }
        for _ in 0..10 {
            ctx = run_chunk(&node, ctx, &mut udp, RPM_TRUE, 2.0);
        }
        assert!(ctx.rpm_detection.validity_detector.valid);
        // Запоминаем настройку гетеродина — эталон для проверки «не испорчена»
        let f_het_before = ctx.rpm_detection.capture_mode.f_precise.unwrap();

        // === Долгий провал 1X: 45 чанков тишины ===
        // 45 > 40 (окно истории): буфер захвата полностью вымывается от сигнала.
        // scale = 0: на выходе нули, но генератор продолжает копить фазу —
        // вал «крутится молча», восстановление будет фазонепрерывным
        for _ in 0..45 {
            ctx = run_chunk(&node, ctx, &mut udp, RPM_TRUE, 0.0);
        }
        // Детектор честно увидел провал: огибающая упала ниже a_low = 2.0,
        // невалидность длиннее settle → Acquisition
        assert!(!ctx.rpm_detection.validity_detector.valid);
        assert_eq!(ctx.rpm_detection.validity_detector.state, TrackState::Acquisition);
        // Захват РАБОТАЛ каждый чанк тишины (state == Acquisition), но молчал:
        // гейт amp_now требует подтверждения пика текущим чанком, а там нули.
        // Без этого гейта захват «находил» бы затухающий хвост в буфере истории
        // и перезаписывал f_precise мусором — тогда восстановление было бы
        // невозможно (истина оказалась бы вне полосы ФНЧ вокруг мусорного f_het)
        assert!(!ctx.rpm_detection.capture_mode.is_updated);
        // Частота гетеродина НЕ испорчена. Допуск ±0.1, а не бит-в-бит:
        // первые ~3 чанка провала valid ещё true (огибающая ФНЧ затухает медленно —
        // у фильтра есть память) и петля коррекции легитимно дорабатывает свои
        // последние честные шаги. «Не испорчен» ≠ «заморожен»
        let f_after = ctx.rpm_detection.capture_mode.f_precise.unwrap();
        assert!((f_after - f_het_before).abs() < 0.1,
            "f_precise испорчен на провале: {f_after}, был {f_het_before}");

        // === Сигнал вернулся (scale ×2) — тракт восстанавливается сам ===
        // Гетеродин всю тишину крутился на сохранённой ~50.30: сигнал вернулся
        // почти точно в полосу → огибающая нарастает за время установления ФНЧ
        // (~17 чанков) → valid возвращается → уточнение снова меряет истину
        for _ in 0..40 {
            ctx = run_chunk(&node, ctx, &mut udp, RPM_TRUE, 2.0);
        }
        assert!(ctx.rpm_detection.validity_detector.valid);
        assert_eq!(ctx.rpm_detection.validity_detector.state, TrackState::Valid);
        let f_prec = ctx.rpm_detection.standard_clarification.f_precise_hz.unwrap();
        assert!((f_prec - F_TRUE).abs() < 0.05,
            "после восстановления: {}, истина {}", f_prec, F_TRUE);
    }

    /// ## Тест 3: Форсаж 600 → 6000 RPM за 0.5 с (180 Гц/с).
    ///
    /// При выходе ошибки частоты за полосу слежения система не
    /// «срывается», а плавно деградирует (флаг невалидности), после чего
    /// выполняется повторный захват и слежение восстанавливается.
    ///
    /// Физика сценария: гетеродин берёт частоту из `capture_mode.f_precise`,
    /// а не из сетевого RPM напрямую, поэтому на разгоне 180 Гц/с истина уходит
    /// из полосы ±3.5 Гц за ~20 мс (быстрее чанка). Огибающая обречена рухнуть —
    /// это не баг, а следствие фиксированной полосы (которая сознательно не
    /// перестраивается ради отказобезопасности). Во время свипа спектр в буфере
    /// размазан, захват молчит; повторный захват возможен, когда обороты
    /// стабилизируются — буфер наполнится стационарным сигналом.
    ///
    /// Сеть в сценарии следует за разгоном с лагом в 1 чанк и погрешностью −1%.
    ///
    ///
    /// Амплитуда постоянна на всём тесте (scale 2.0): изолируем фактор динамики —
    /// любое изменение valid вызвано уходом частоты из полосы, а не амплитудой.
    #[test]
    fn test_full_chain_runup_600_to_6000() {
        let parent_dbg = Dbg::own("full pipeline test");
        let node = make_node(&parent_dbg);
        let mut ctx = make_ctx();

        const RPM_START: f64 = 600.0;   // 10 Гц — нижняя граница диапазона
        const RPM_END: f64 = 6000.0;    // 100 Гц — верхняя граница диапазона
        const RAMP_CHUNKS: usize = 20;  // 20 × 25.6 мс ≈ 0.5 с разгона
        const NET_ERR: f64 = 0.99;      // погрешность сети −1%

        // Сеть стартует с текущего значения (с погрешностью)
        ctx.raw_rpm = RPM_START * NET_ERR;
        let mut rpm_net = RPM_START;
        let mut udp = make_udp(A_WEAK);

        // === Прогрев на 600 RPM: захват и слежение на нижней частоте диапазона ===
        // Проверяем, что тракт работает и на 10 Гц: окно уточнения здесь длинное
        // (4 оборота = 400 отсчётов), Герцель по 1 с истории — всего 10 циклов
        let mut rpm_true = RPM_START;
        for _ in 0..40 {
            ctx = run_chunk(&node, ctx, &mut udp, rpm_true, 2.0);
        }
        assert!(ctx.rpm_detection.validity_detector.valid, "не вышли на слежение при 600 RPM");
        let f0 = ctx.rpm_detection.standard_clarification.f_precise_hz.unwrap();
        assert!((f0 - RPM_START / 60.0).abs() < 0.15,
            "старт: {f0}, истина {}", RPM_START / 60.0);

        // === Форсаж 0.5 с: истина разгоняется линейно, сеть следует с лагом в 1 чанк ===
        // Генератор получает новый rpm каждый чанк, но фаза внутри копится
        // посэмплово — свип фазонепрерывен, как у реального привода
        let mut degraded = false;
        for i in 1..=RAMP_CHUNKS {
            rpm_true = RPM_START + (RPM_END - RPM_START) * (i as f64 / RAMP_CHUNKS as f64);
            ctx = run_chunk(&node, ctx, &mut udp, rpm_true, 2.0);
            if !ctx.rpm_detection.validity_detector.valid {
                degraded = true; // плавная деградация: флаг невалидности выставлен
            }
            ctx.raw_rpm = rpm_net * NET_ERR; // сеть отдаёт значение с лагом в 1 чанк
            rpm_net = rpm_true;
        }
        // Система ОБЯЗАНА пометить деградацию: молчаливая выдача фазы, вычисленной
        // из шума (сигнал ушёл из полосы!), — худший режим отказа для мониторинга.
        // Отсутствие флага означало бы сломанный детектор валидности.
        assert!(degraded, "система обязана честно пометить деградацию на форсаже");

        // === Стабилизация на 6000 RPM: повторный захват и сходимость ===
        // 100 чанков ≈ 2.5 с: буфер истории заполняется стационарными 100 Гц,
        // захват находит пик вокруг сетевого значения (99 Гц ± 5 Гц), огибающая
        // набирается, коррекция дотягивает остаток (на 100 Гц окно — минимальные
        // 64 отсчёта, коррекция быстрая)
        ctx.raw_rpm = RPM_END * NET_ERR;
        for _ in 0..100 {
            ctx = run_chunk(&node, ctx, &mut udp, RPM_END, 2.0);
        }
        assert!(ctx.rpm_detection.validity_detector.valid, "не восстановился после форсажа");
        assert_eq!(ctx.rpm_detection.validity_detector.state, TrackState::Valid);
        // Уточнённая частота сошлась к истине на верхней границе диапазона
        let f1 = ctx.rpm_detection.standard_clarification.f_precise_hz.unwrap();
        assert!((f1 - RPM_END / 60.0).abs() < 0.1,
            "после форсажа: {f1}, истина {}", RPM_END / 60.0);
        // И сам гетеродин перезахвачен на истину (коррекция убрала остаток δ)
        let f_het = ctx.rpm_detection.capture_mode.f_precise.unwrap();
        assert!((f_het - RPM_END / 60.0).abs() < 0.1,
            "гетеродин после форсажа: {f_het}, истина {}", RPM_END / 60.0);
    }

    // =========================================================================
    // Тест 4: шумовой стенд
    // =========================================================================

    /// Стандартное отклонение гауссова шума для сценария «сигнал −15 дБ».
    const NOISE_STD_15DB: f64 = 79.53;

    /// Детерминированный ГПСЧ (xorshift64*) + преобразование Бокса–Мюллера.
    struct XorShift64(u64);
    impl XorShift64 {
        fn next_u64(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            x.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
        /// Равномерно распределённая величина на [0, 1)
        fn uniform(&mut self) -> f64 {
            (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
        }
        /// Стандартная нормальная величина N(0, 1), преобразование Бокса–Мюллера
        fn gauss(&mut self) -> f64 {
            let u1 = self.uniform() + 1e-12; // защита от ln(0)
            let u2 = self.uniform();
            (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
        }
    }

    /// То же, что `fill_chunk`, но добавляет к отсчётам гауссов шум заданной σ.
    ///
    /// Шум вводится ПОСЛЕ снятия нулевой линии АЦП и масштабирования: моделируем
    /// шум вибрации на входе тракта, а не шум квантования АЦП.
    fn fill_chunk_noisy(ctx: &mut AngularCtx, udp: &mut Udp, rpm: f64, scale: f64, rng: &mut XorShift64, noise_std: f64) {
        let mut buf = vec![0u16; CHUNK_RAW];
        udp.parse(rpm, &mut buf);
        let samples: Vec<f64> = buf.iter()
            .map(|&v| (v as f64 - ADC_ZERO) * scale + noise_std * rng.gauss())
            .collect();
        let frame = Frame { ts: Utc::now(), samples: Arc::new(samples), phases: Phases::new(CHUNK_RAW) };
        ctx.push_chunk(&frame);
    }

    /// ## Тест 4: Выделение 1X на шуме −15 дБ.
    ///
    /// ТЗ: стабильное выделение оборотной гармоники при полезном сигнале −15 дБ
    /// относительно общей мощности вибрации; ожидаемая точность фазы σ_φ ≈ 5–10°
    /// на отсчёт («уточняется при настройке»).
    ///
    /// Сценарий: система сначала выходит на слежение на ЧИСТОМ сильном сигнале
    /// (захвату и детектору даём честные условия старта), затем тот же сигнал
    /// 60 чанков (~1.5 с) тонет в гауссовом шуме σ ≈ 79.5 (в 4 раза «громче»
    /// полезной гармоники). Фаза 1X в генераторе непрерывна — шум не трогает вал,
    /// он лишь залит поверх измерения, как в реальности.
    ///
    /// Метрика качества — σ остатков развёрнутой фазы относительно линейного
    /// тренда (МНК-прямая): на постоянных оборотах истинная фаза — строго прямая,
    /// поэтому всё отклонение от неё есть фазовый шум тракта. Развёрнутая фаза
    /// (а не циклическая) выбрана, чтобы прыжки ±2π не ломали регрессию.
    #[test]
    fn test_full_chain_noise_minus_15db() {
        let parent_dbg = Dbg::own("noise test");
        let node = make_node(&parent_dbg);
        let mut ctx = make_ctx();
        let mut udp = make_udp(A_WEAK);
        let mut rng = XorShift64(0x9E37_79B9_7F4A_7C15); // фиксированный сид: тест детерминирован

        // === Прогрев: 40 чанков чистого сильного сигнала → устойчивое слежение ===
        for _ in 0..40 {
            ctx = run_chunk(&node, ctx, &mut udp, RPM_TRUE, 2.0);
        }
        assert!(ctx.rpm_detection.validity_detector.valid, "не вышли на слежение до шума");

        // === Шумовая фаза: 60 чанков ≈ 1.5 с, сигнал −15 дБ ===
        // Собираем развёрнутую фазу по валидным отсчётам вместе с абсолютным
        // номером отсчёта (1000 Гц после децимации) — это ось времени для МНК.
        // Невалидные отсчёты из регрессии исключаются, но часы (n) идут по всем —
        // ось времени остаётся равномерной.
        const NOISE_CHUNKS: usize = 60;
        let mut samples: Vec<(f64, f64)> = Vec::new(); // (n, phi)
        let mut n: f64 = 0.0;
        let mut valid_chunks = 0usize;
        for _ in 0..NOISE_CHUNKS {
            fill_chunk_noisy(&mut ctx, &mut udp, RPM_TRUE, 2.0, &mut rng, NOISE_STD_15DB);
            ctx = node.eval(ctx);
            assert!(ctx.err.is_none(), "ошибка тракта: {:?}", ctx.err);
            if ctx.rpm_detection.validity_detector.valid {
                valid_chunks += 1;
            }
            for ph in ctx.rpm_detection.instant_shaft_phase.shaft_phase_unwrapped.iter() {
                if let crate::PhaseStatus::Valid(phi) = ph {
                    samples.push((n, *phi));
                }
                n += 1.0;
            }
        }

        // Тракт обязан держать слежение почти всё шумовое окно: гистерезис и
        // инерция огибающей сглаживают краткие просадки, но систематический
        // срыв в Acquisition означал бы, что сигнал потерян под шумом
        assert!(valid_chunks * 100 >= 95 * NOISE_CHUNKS,
            "валидных чанков {valid_chunks} из {NOISE_CHUNKS} (< 95%)");

        // МНК-прямая phi ≈ a + b·n: коэффициенты по нормальным уравнениям,
        // σ остатков — фазовый шум тракта на отсчёт
        let m = samples.len() as f64;
        assert!(m > 100.0, "слишком мало валидных отсчётов для статистики: {m}");
        let (sn, sp, snn, snp) = samples.iter().fold((0.0, 0.0, 0.0, 0.0),
            |(sn, sp, snn, snp), &(x, p)| (sn + x, sp + p, snn + x * x, snp + x * p));
        let b = (m * snp - sn * sp) / (m * snn - sn * sn);
        let a = (sp - b * sn) / m;
        let sigma_rad = (samples.iter()
            .map(|&(x, p)| (p - a - b * x).powi(2))
            .sum::<f64>() / m).sqrt();
        let sigma_deg = sigma_rad.to_degrees();
        log::debug!("шум −15 дБ: σ_φ = {sigma_deg:.2}° на отсчёт, валидных чанков {valid_chunks}/{NOISE_CHUNKS}");
        // Допуск: σ_φ ≈ 5–10° на отсчёт. Измерено на этом стенде: 5.00°
        // валидность 60/60 чанков — тракт под шумом не срывается
        assert!(sigma_deg < 10.0,
            "фазовый шум σ_φ = {sigma_deg:.2}° превышает допуск 10° (ТЗ: 5–10°)");
    }
}
