use rustfft::num_complex::Complex;
use sal_core::dbg::Dbg;
use crate::{Eval, AngularCtx, TrackState, ValidityDetector};

struct DummyChild;
impl Eval<AngularCtx, AngularCtx> for DummyChild {
    fn eval(&self, ctx: AngularCtx) -> AngularCtx { ctx }
    fn exit(&self) {}
}

const A_LOW: f64 = 0.3;
const A_HIGH: f64 = 0.4;
const SETTLE: usize = 100;

fn node() -> ValidityDetector<DummyChild> {
    ValidityDetector::new(&Dbg::own("validity_test"), DummyChild, A_LOW, A_HIGH, SETTLE)
}

fn feed(n: &ValidityDetector<DummyChild>, ctx: AngularCtx, amps: &[f64]) -> AngularCtx {
    let mut ctx = ctx;
    ctx.rpm_detection.lpf_tracking_band.filtered_signal =
        amps.iter().map(|&a| Complex::new(a, 0.0)).collect();
    n.eval(ctx)
}
/// Подаём 10 отсчётов амплитуды 1.0. 
/// Стартовое состояние невалидное, а 1.0 выше a_high, 
/// поэтому флаг включается. 
/// Проверяется три вещи: valid == true, state == Valid, 
/// valid_flags.len() == 10. Тест гарантирует, 
/// что система вообще может выйти из начального состояния.
#[test]
fn becomes_valid_above_high_threshold() {
    let ctx = feed(&node(), AngularCtx::default(), &[1.0; 10]);
    assert!(ctx.rpm_detection.validity_detector.valid);
    assert_eq!(ctx.rpm_detection.validity_detector.state, TrackState::Valid);
    assert_eq!(ctx.rpm_detection.validity_detector.valid_flags.len(), 10);
}
/// Главный тест гистерезиса, две части.
/// 
/// Сначала [1.0, 0.35, 0.35]: флаг стал валидным на 1.0, потом амплитуда 0.35 лежит между порогами. Флаг должен остаться true.
/// Потом [0.1, 0.35, 0.35]: 0.1 ниже a_low, флаг упал, и 0.35 его не возвращает. Остаётся false.
/// 
/// Если бы порог был один, обе части сломались бы. Тест ловит дребезг.
#[test]
fn hysteresis_zone_keeps_previous_state() {
    let n = node();
    // валидна → амплитуда 0.35 (между порогами) → остаётся валидной
    let ctx = feed(&n, AngularCtx::default(), &[1.0, 0.35, 0.35]);
    assert!(ctx.rpm_detection.validity_detector.valid);
    // невалидна → амплитуда 0.35 → остаётся невалидной
    let ctx = feed(&n, ctx, &[0.1, 0.35, 0.35]);
    assert!(!ctx.rpm_detection.validity_detector.valid);
}
/// Полный цикл: 1.0 (валидно) → 0.2 (ниже a_low, невалидно) 
/// → 0.45 (выше a_high, снова валидно). 
/// Заодно проверяется, что invalid_samples обнулился после восстановления.
#[test]
fn drops_below_low_then_recovers_above_high() {
    let n = node();
    let ctx = feed(&n, AngularCtx::default(), &[1.0]);
    let ctx = feed(&n, ctx, &[0.2]);
    assert!(!ctx.rpm_detection.validity_detector.valid);
    let ctx = feed(&n, ctx, &[0.45]);
    assert!(ctx.rpm_detection.validity_detector.valid);
    assert_eq!(ctx.rpm_detection.validity_detector.invalid_samples, 0);
}
/// После валидного отсчёта подаём 50 нулей. 
/// 50 меньше settle = 100, поэтому состояние Degraded, 
/// а invalid_samples == 50. Проверяет, что короткий провал не запускает захват.
#[test]
fn short_invalidity_is_degraded() {
    let n = node();
    let ctx = feed(&n, AngularCtx::default(), &[1.0]);
    let ctx = feed(&n, ctx, &[0.0; 50]);
    assert_eq!(ctx.rpm_detection.validity_detector.state, TrackState::Degraded);
    assert_eq!(ctx.rpm_detection.validity_detector.invalid_samples, 50);
}
/// Подаём SETTLE + 1 = 101 нулей. 
/// Счётчик превышает settle_samples, состояние Acquisition. 
/// Это граница между «ждём» и «ищем частоту заново».
#[test]
fn long_invalidity_enters_acquisition() {
    let n = node();
    let ctx = feed(&n, AngularCtx::default(), &[1.0]);
    let ctx = feed(&n, ctx, &[0.0; SETTLE + 1]);
    assert_eq!(ctx.rpm_detection.validity_detector.state, TrackState::Acquisition);
}
/// Четыре вызова eval по 30 нулей, всего 120. 
/// Счётчик должен накопиться между вызовами (invalid_samples == 120), 
/// и состояние стать Acquisition, 
/// хотя ни один пакет в отдельности не превышает 100.
#[test]
fn invalid_counter_accumulates_across_calls() {
    let n = node();
    let mut ctx = feed(&n, AngularCtx::default(), &[1.0]);
    for _ in 0..4 { ctx = feed(&n, ctx, &[0.0; 30]); }
    assert_eq!(ctx.rpm_detection.validity_detector.invalid_samples, 120);
    assert_eq!(ctx.rpm_detection.validity_detector.state, TrackState::Acquisition);
}
/// Вход [1.0, 1.0, 0.1, 0.1, 0.5] должен дать флаги [true, true, false, false, true].
/// Здесь виден гистерезис по отсчётам: 0.5 выше a_high, 
/// поэтому возврат в true. 
/// Потом второй вызов с двумя отсчётами даёт valid_flags.len() == 2, 
/// то есть вектор очищается, а не копится.
#[test]
fn flags_are_per_sample_and_cleared_each_call() {
    let n = node();
    let ctx = feed(&n, AngularCtx::default(), &[1.0, 1.0, 0.1, 0.1, 0.5]);
    assert_eq!(ctx.rpm_detection.validity_detector.valid_flags, vec![true, true, false, false, true]);
    let ctx = feed(&n, ctx, &[1.0, 1.0]);
    assert_eq!(ctx.rpm_detection.validity_detector.valid_flags.len(), 2);
}
/// Атрибут #[should_panic]: конструктор с a_low = 0.5, a_high = 0.4 
/// должен паниковать через assert!(a_high > a_low). 
/// Защита от ошибки конфигурации: при перепутанных порогах гистерезис работал бы наоборот.
#[test]
#[should_panic]
fn rejects_inverted_thresholds() {
    ValidityDetector::new(&Dbg::own("t"), DummyChild, 0.5, 0.4, SETTLE);
}