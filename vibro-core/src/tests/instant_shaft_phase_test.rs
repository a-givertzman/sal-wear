use std::{f64::consts::PI};
use rustfft::num_complex::Complex;
use sal_core::dbg::Dbg;
use crate::{AngularCtx, Biquad, Eval, FSampleDec, InstantShaftPhase, InstantShaftPhaseCtx, group_delay_samples};

struct DummyChild;
impl Eval<AngularCtx, AngularCtx> for DummyChild {
    fn eval(&self, ctx: AngularCtx) -> AngularCtx { ctx }
    fn exit(&self) {}
}
//
fn node() -> InstantShaftPhase<DummyChild> {
    InstantShaftPhase::new(&Dbg::own("shaft_phase_test"), DummyChild)
}
//
const FS: f64 = 16_000.0;
fn wrap(d: f64) -> f64 { (d + PI).rem_euclid(2.0 * PI) - PI }
//
fn ctx_for(d: usize, filtered: Vec<Complex<f64>>, phi: Vec<f64>) -> AngularCtx {
    let mut ctx = AngularCtx::default();
    ctx.rpm_detection.instant_shaft_phase = InstantShaftPhaseCtx::new(filtered.len(), d);
    ctx.rpm_detection.lpf_tracking_band.filtered_signal = filtered;
    ctx.rpm_detection.complex_local_oscillator.phi_het = phi;
    ctx
}
///
/// φ_вал = atan2(Q, I) + φ_het (mod 2π): известные числа, включая отрицательный угол и сумму > 2π.
#[test]
fn phase_is_atan2_plus_phi_het() {
    let mut ctx = AngularCtx::default();
    ctx.rpm_detection.lpf_tracking_band.filtered_signal = [0.3f64, 1.0, -1.0]
        .iter().map(|&a| Complex::from_polar(2.0, a)).collect();
    ctx.rpm_detection.complex_local_oscillator.phi_het = vec![5.0, 0.5, 0.2];
    ctx = node().eval(ctx);
    let out = &ctx.rpm_detection.instant_shaft_phase.shaft_phase;
    assert_eq!(out.len(), 3);
    let expected = [
        (0.3f64 + 5.0).rem_euclid(2.0 * PI),
        (1.0f64 + 0.5).rem_euclid(2.0 * PI),
        (-1.0f64 + 0.2).rem_euclid(2.0 * PI),
    ];
    for (o, e) in out.iter().zip(expected.iter()) {
        assert!((o - e).abs() < 1e-12, "получено {o}, ожидалось {e}");
    }
}
///
/// Результат всегда в [0, 2π).
#[test]
fn phase_is_in_range() {
    let mut ctx = AngularCtx::default();
    ctx.rpm_detection.lpf_tracking_band.filtered_signal =
        (0..100).map(|k| Complex::from_polar(1.0, k as f64 * 0.37 - 3.0)).collect();
    ctx.rpm_detection.complex_local_oscillator.phi_het = (0..100).map(|k| k as f64 * 0.11).collect();
    ctx = node().eval(ctx);
    assert_eq!(ctx.rpm_detection.instant_shaft_phase.shaft_phase.len(), 100);
    for &p in &ctx.rpm_detection.instant_shaft_phase.shaft_phase {
        assert!((0.0..2.0 * PI).contains(&p), "вне диапазона: {p}");
    }
}
///
/// Выход не копится между вызовами.
#[test]
fn output_is_cleared_between_calls() {
    let n = node();
    let mut ctx = AngularCtx::default();
    for _ in 0..3 {
        ctx.rpm_detection.lpf_tracking_band.filtered_signal = vec![Complex::new(1.0, 0.0); 13];
        ctx.rpm_detection.complex_local_oscillator.phi_het = vec![0.0; 13];
        ctx = n.eval(ctx);
        assert_eq!(ctx.rpm_detection.instant_shaft_phase.shaft_phase.len(), 13);
    }
}
///
/// Пустой вход: выход не должен содержать данные прошлого вызова.
#[test]
fn empty_input_leaves_empty_output() {
    let n = node();
    let mut ctx = AngularCtx::default();
    ctx.rpm_detection.lpf_tracking_band.filtered_signal = vec![Complex::new(1.0, 0.0); 5];
    ctx.rpm_detection.complex_local_oscillator.phi_het = vec![0.0; 5];
    ctx = n.eval(ctx);
    assert_eq!(ctx.rpm_detection.instant_shaft_phase.shaft_phase.len(), 5);
    ctx.rpm_detection.lpf_tracking_band.filtered_signal.clear();
    ctx.rpm_detection.complex_local_oscillator.phi_het.clear();
    ctx = n.eval(ctx);
    assert!(ctx.rpm_detection.instant_shaft_phase.shaft_phase.is_empty(), "остались данные прошлого вызова");
}
///
/// Cкорость роста фазы равна истинной частоте при любой ошибке f_het.
#[test]
fn phase_rate_equals_true_frequency_for_any_f_het() {
    let fs = 16_000.0;
    let f_true = 50.0;
    for f_het in [49.5, 50.0, 50.5] {
        let delta = f_true - f_het;
        let n = 4000usize;
        let mut ctx = AngularCtx::default();
        ctx.rpm_detection.lpf_tracking_band.filtered_signal = (0..n)
            .map(|k| Complex::from_polar(0.5, 2.0 * PI * delta * k as f64 / fs)).collect();
        ctx.rpm_detection.complex_local_oscillator.phi_het = (0..n)
            .map(|k| (2.0 * PI * f_het * k as f64 / fs).rem_euclid(2.0 * PI)).collect();
        ctx = node().eval(ctx);
        let ph = &ctx.rpm_detection.instant_shaft_phase.shaft_phase;
        assert_eq!(ph.len(), n);
        let mut unwrapped = 0.0;
        for w in ph.windows(2) {
            let mut d = w[1] - w[0];
            if d > PI { d -= 2.0 * PI; }
            if d < -PI { d += 2.0 * PI; }
            unwrapped += d;
        }
        let freq = unwrapped / (2.0 * PI * (n - 1) as f64 / fs);
        assert!((freq - f_true).abs() < 0.05, "f_het={f_het}: получено {freq}, ожидалось {f_true}");
    }
}



/// При D = 0 все флаги ready равны true. 
/// Подтверждает, что при отключённой задержке узел ведёт себя как раньше.
#[test]
fn zero_delay_everything_ready() {
    let ctx = node().eval(ctx_for(0, vec![Complex::new(1.0, 0.0); 20], vec![0.0; 20]));
    assert!(ctx.rpm_detection.instant_shaft_phase.ready.iter().all(|&r| r));
}

/// Первые D отсчётов не готовы при любой нарезке на пакеты; длины выходов равны входу.
#[test]
fn warmup_marks_not_ready_across_chunks() {
    let d = 30;
    let nd = node();
    let mut ctx = AngularCtx::default();
    ctx.rpm_detection.instant_shaft_phase = InstantShaftPhaseCtx::new(13, d);
    let mut flags = Vec::new();
    for _ in 0..6 {
        ctx.rpm_detection.lpf_tracking_band.filtered_signal = vec![Complex::new(1.0, 0.0); 13];
        ctx.rpm_detection.complex_local_oscillator.phi_het = vec![0.0; 13];
        ctx = nd.eval(ctx);
        let s = &ctx.rpm_detection.instant_shaft_phase;
        assert_eq!(s.shaft_phase.len(), 13);
        assert_eq!(s.ready.len(), 13);
        flags.extend(s.ready.iter().cloned());
    }
    assert!(flags[..d].iter().all(|&r| !r) && flags[d..].iter().all(|&r| r));
}

/// Главный тест: огибающая задержана ровно на D, фаза равна 2π·f_true·(n−D)/fs + θ0
/// при любой ошибке гетеродина, включая край полосы.
#[test]
fn delayed_identity_exact_for_any_f_het() {
    let (d, n, f_true, th0) = (100usize, 4000usize, 50.0, 0.7);
    for f_het in [47.0, 49.5, 50.0, 50.5, 53.0] {
        let delta = f_true - f_het;
        let filtered: Vec<_> = (0..n).map(|k|
            Complex::from_polar(0.5, 2.0 * PI * delta * (k as f64 - d as f64) / FS + th0)).collect();
        let phi: Vec<_> = (0..n).map(|k|
            (2.0 * PI * f_het * k as f64 / FS).rem_euclid(2.0 * PI)).collect();
        let ctx = node().eval(ctx_for(d, filtered, phi));
        let out = &ctx.rpm_detection.instant_shaft_phase;
        for k in d..n {
            let exp = 2.0 * PI * f_true * (k as f64 - d as f64) / FS + th0;
            assert!(out.ready[k]);
            assert!(wrap(out.shaft_phase[k] - exp).abs() < 1e-9, "f_het={f_het}, n={k}");
        }
    }
}

/// Контрольный тест: без задержки (D = 0) на тех же данных ошибка равна 2π·δ·D.
/// Фиксирует, ради чего сделана линия задержки.
#[test]
fn without_delay_error_is_two_pi_delta_d() {
    let (d, n, f_true, f_het) = (100usize, 4000usize, 50.0, 49.0);
    let delta = f_true - f_het;
    let filtered: Vec<_> = (0..n).map(|k|
        Complex::from_polar(0.5, 2.0 * PI * delta * (k as f64 - d as f64) / FS)).collect();
    let phi: Vec<_> = (0..n).map(|k|
        (2.0 * PI * f_het * k as f64 / FS).rem_euclid(2.0 * PI)).collect();
    let ctx = node().eval(ctx_for(0, filtered, phi));
    let k = n - 1;
    let truth_now = 2.0 * PI * f_true * k as f64 / FS;
    let err = wrap(ctx.rpm_detection.instant_shaft_phase.shaft_phase[k] - truth_now).abs();
    let expected = 2.0 * PI * delta * d as f64 / FS;
    assert!((err - expected).abs() < 1e-9, "ошибка {err}, ожидалось {expected}");
}

/// reset очищает линию задержки и развёртку, прогрев начинается заново.
#[test]
fn reset_restarts_warmup() {
    let nd = node();
    let mut ctx = nd.eval(ctx_for(5, vec![Complex::new(1.0, 0.0); 20], vec![0.0; 20]));
    ctx.rpm_detection.instant_shaft_phase.reset();
    ctx.rpm_detection.lpf_tracking_band.filtered_signal = vec![Complex::new(1.0, 0.0); 8];
    ctx.rpm_detection.complex_local_oscillator.phi_het = vec![0.0; 8];
    ctx = nd.eval(ctx);
    let r = &ctx.rpm_detection.instant_shaft_phase.ready;
    assert!(r[..5].iter().all(|&x| !x) && r[5..].iter().all(|&x| x));
}

/// Развёртка переносится между пакетами: пакетами по 13 результат такой же, как одним куском.
#[test]
fn unwrap_persists_between_chunks() {
    let truth: Vec<f64> = (0..13 * 20).map(|k| 5.0 + 0.1 * k as f64).collect();
    let nd = node();
    let mut ctx = AngularCtx::default();
    let mut unwrapped = Vec::new();
    for c in truth.chunks(13) {
        ctx.rpm_detection.lpf_tracking_band.filtered_signal =
            c.iter().map(|&p| Complex::from_polar(1.0, p)).collect();
        ctx.rpm_detection.complex_local_oscillator.phi_het = vec![0.0; c.len()];
        ctx = nd.eval(ctx);
        unwrapped.extend(ctx.rpm_detection.instant_shaft_phase.shaft_phase_unwrapped.iter().cloned());
    }
    assert_eq!(unwrapped.len(), truth.len());
    for w in unwrapped.windows(2) {
        assert!((w[1] - w[0] - 0.1).abs() < 1e-9, "скачок на стыке пакетов: {}", w[1] - w[0]);
    }
}

/// Добротность (Q) первого биквадратного звена для каскадного ФНЧ Баттерворта 4-го порядка.
const Q_FACTOR_1: f64 = 0.5412;
/// Добротность (Q) второго биквадратного звена для каскадного ФНЧ Баттерворта 4-го порядка.
/// 
/// Сочетание парных добротностей Q1 и Q2 формирует классический гладкий 
/// аналоговый прототип Баттерворта 4-го порядка с максимально плоской 
/// амплитудно-частотной характеристикой (АЧХ) в полосе пропускания.
const Q_FACTOR_2: f64 = 1.3066;
/// Реальные биквады: остаточная ошибка после выравнивания по времени мала при малой δ
/// и заметно больше на краю полосы. Допуски ориентировочные, подберите по первому прогону.
#[test]
fn real_filter_residual_error() {
    let fc = 3.5;
    let d = group_delay_samples(fc, FSampleDec(FS));
    let (f_true, th0) = (50.0, 0.7);
    let residual = |f_het: f64| -> f64 {
        let delta = f_true - f_het;
        let mut s1 = Biquad::new(fc, FS, Q_FACTOR_1);
        let mut s2 = Biquad::new(fc, FS, Q_FACTOR_2);
        let n = (FS * 15.0) as usize;
        let mut filtered = Vec::with_capacity(n);
        let mut phi = Vec::with_capacity(n);
        for k in 0..n {
            let a = 2.0 * PI * delta * k as f64 / FS + th0;     // вход ФНЧ после гетеродина
            let (i1, q1) = s1.process(a.cos(), a.sin());
            let (i, q) = s2.process(i1, q1);
            filtered.push(Complex::new(i, q));
            phi.push((2.0 * PI * f_het * k as f64 / FS).rem_euclid(2.0 * PI));
        }
        let ctx = node().eval(ctx_for(d, filtered, phi));
        let k = n - 1;
        let exp = 2.0 * PI * f_true * (k as f64 - d as f64) / FS + th0;
        wrap(ctx.rpm_detection.instant_shaft_phase.shaft_phase[k] - exp).abs()
    };
    for f_het in [49.5, 49.0, 48.0, 47.0] {
        log::debug!("f_het={f_het}, δ={}: ошибка {:.4} рад", 50.0 - f_het, residual(f_het));
    }
    log::debug!("D = {} отсчётов", group_delay_samples(3.5, FSampleDec(FS)));
    assert!(residual(49.5) < 0.005);   // измерено 0.0011
    assert!(residual(48.0) < 0.15);    // измерено 0.086
    assert!(residual(47.0) > 0.2);     // край полосы заметно хуже
}