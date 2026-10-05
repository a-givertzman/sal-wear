use std::{f64::consts::PI};
use rustfft::num_complex::Complex;
use sal_core::dbg::Dbg;
use crate::{Eval, InstantShaftPhase, RpmDetectionCtx};

struct DummyChild;
impl Eval<RpmDetectionCtx, RpmDetectionCtx> for DummyChild {
    fn eval(&self, ctx: RpmDetectionCtx) -> RpmDetectionCtx { ctx }
    fn exit(&self) {}
}
//
fn node() -> InstantShaftPhase<DummyChild> {
    InstantShaftPhase::new(&Dbg::own("shaft_phase_test"), DummyChild)
}
///
/// φ_вал = atan2(Q, I) + φ_het (mod 2π): известные числа, включая отрицательный угол и сумму > 2π.
#[test]
fn phase_is_atan2_plus_phi_het() {
    let mut ctx = RpmDetectionCtx::default();
    ctx.lpf_tracking_band_ctx.filtered_signal = [0.3f64, 1.0, -1.0]
        .iter().map(|&a| Complex::from_polar(2.0, a)).collect();
    ctx.complex_local_oscillator.phi_net = vec![5.0, 0.5, 0.2];
    ctx = node().eval(ctx);
    let out = &ctx.instant_shaft_phase.shaft_phase;
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
    let mut ctx = RpmDetectionCtx::default();
    ctx.lpf_tracking_band_ctx.filtered_signal =
        (0..100).map(|k| Complex::from_polar(1.0, k as f64 * 0.37 - 3.0)).collect();
    ctx.complex_local_oscillator.phi_net = (0..100).map(|k| k as f64 * 0.11).collect();
    ctx = node().eval(ctx);
    assert_eq!(ctx.instant_shaft_phase.shaft_phase.len(), 100);
    for &p in &ctx.instant_shaft_phase.shaft_phase {
        assert!((0.0..2.0 * PI).contains(&p), "вне диапазона: {p}");
    }
}
///
/// Выход не копится между вызовами.
#[test]
fn output_is_cleared_between_calls() {
    let n = node();
    let mut ctx = RpmDetectionCtx::default();
    for _ in 0..3 {
        ctx.lpf_tracking_band_ctx.filtered_signal = vec![Complex::new(1.0, 0.0); 13];
        ctx.complex_local_oscillator.phi_net = vec![0.0; 13];
        ctx = n.eval(ctx);
        assert_eq!(ctx.instant_shaft_phase.shaft_phase.len(), 13);
    }
}
///
/// Пустой вход: выход не должен содержать данные прошлого вызова.
#[test]
fn empty_input_leaves_empty_output() {
    let n = node();
    let mut ctx = RpmDetectionCtx::default();
    ctx.lpf_tracking_band_ctx.filtered_signal = vec![Complex::new(1.0, 0.0); 5];
    ctx.complex_local_oscillator.phi_net = vec![0.0; 5];
    ctx = n.eval(ctx);
    assert_eq!(ctx.instant_shaft_phase.shaft_phase.len(), 5);
    ctx.lpf_tracking_band_ctx.filtered_signal.clear();
    ctx.complex_local_oscillator.phi_net.clear();
    ctx = n.eval(ctx);
    assert!(ctx.instant_shaft_phase.shaft_phase.is_empty(), "остались данные прошлого вызова");
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
        let mut ctx = RpmDetectionCtx::default();
        ctx.lpf_tracking_band_ctx.filtered_signal = (0..n)
            .map(|k| Complex::from_polar(0.5, 2.0 * PI * delta * k as f64 / fs)).collect();
        ctx.complex_local_oscillator.phi_net = (0..n)
            .map(|k| (2.0 * PI * f_het * k as f64 / fs).rem_euclid(2.0 * PI)).collect();
        ctx = node().eval(ctx);
        let ph = &ctx.instant_shaft_phase.shaft_phase;
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