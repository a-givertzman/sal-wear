use std::{f64::consts::PI};
use rustfft::num_complex::Complex;
use sal_core::dbg::Dbg;
use crate::{AngularCtx, Biquad, Conf, Eval, FSample, LPFTrackingBand};
///
/// Заглушка для интерфейса `Child`
struct DummyChild;
impl Eval<AngularCtx, AngularCtx> for DummyChild {
    fn eval(&self, ctx: AngularCtx) -> AngularCtx { ctx }
    fn exit(&self) {}
}
const F: f64 = 320000.0;
const DECIMATION_COEFF: f64 = 20.0;
const FS: f64 = F / DECIMATION_COEFF;   // частота после децимации
const BAND_HZ: f64 = 7.0;
const F_CUTOFF: f64 = BAND_HZ / 2.0;
///
/// Инициализация фильтра ФНЧ
fn make_filter() -> LPFTrackingBand<DummyChild> {
    let dbg = Dbg::own("fixed_lpf_test");
    LPFTrackingBand::new(&dbg, DummyChild)
}
///
/// Контекст с настроенным каскадом Баттерворта 4-го порядка.
fn make_ctx() -> AngularCtx {
    let mut ctx = AngularCtx::default();
    ctx.rpm_detection.lpf_tracking_band.stage1 = Biquad::new(F_CUTOFF, FS, 0.5412);
    ctx.rpm_detection.lpf_tracking_band.stage2 = Biquad::new(F_CUTOFF, FS, 1.3066);
    ctx
}
fn prod_ctx() -> AngularCtx {
        let conf: Conf = serde_yaml::from_str(r#"
            adc:
                sample-rate-hz: 320000
                chunk-size: 512
                dc-offset: 2048
            analysis:
                order-tracking:
                    max-order: 300
                    order-resolution: 0.01
                bands:
                    low-order: 0.5..10.0
                    mid-hz: ..5000
                    high-hz: 5000..10000
                rpm-detection:
                    f-sample-dec-target-hz: 16000
                    max-freq-rate-hz-s: 2.0
                    tau-net-s: 0.5
                    t-update-s: 0.5
                    delta-f-net-hz: 3.0
                    band-margin-hz: 2.0
    "#).unwrap();
    let f_sample = FSample(conf.adc.sample_rate_hz);
    let rd = &conf.analysis.rpm_detection;
    let factor = rd.decimation_factor(f_sample);
    let f_cutoff = rd.tracking_cutoff_hz();
    assert!((f_cutoff - F_CUTOFF).abs() < 1e-12, "конфиг дал fc = {f_cutoff}, тест ждёт {F_CUTOFF}");
    AngularCtx::new(f_sample, conf.adc.chunk_size, factor, f_cutoff)
}
///
/// Симуляция гетеродированного сигнала: I/Q
fn tone(f: f64, start: usize, len: usize) -> Vec<Complex<f64>> {
    (start..start + len)
        .map(|k| {
            let phi = 2.0 * PI * f * k as f64 / FS;
            Complex::new(phi.cos(), phi.sin())
        })
        .collect()
}
///
/// Прогоняет тон чанками децимированным размером
/// и возвращает максимум амплитуды.
fn run(f: f64, seconds: f64) -> f64 {
    let filter = make_filter();
    let mut ctx = make_ctx();
    let total = (FS * seconds) as usize;
    let chunk = 13;
    let mut peak = 0.0f64;
    let mut k = 0;
    let mut n = 0usize;
    while k < total {
        ctx.rpm_detection.complex_local_oscillator.complex_signal = tone(f, k, chunk);
        ctx = filter.eval(ctx);
        if k > total * 3 / 4 {
            n += ctx.rpm_detection.lpf_tracking_band.filtered_signal.len();
            for y in &ctx.rpm_detection.lpf_tracking_band.filtered_signal {
                peak = peak.max(y.norm());
            }
        }
        k += chunk;
    }
    assert!(n > 1000, "собрано слишком мало точек: {n}");

    peak
}
fn run_with(ctx: AngularCtx, f: f64, seconds: f64) -> f64 {
    let filter = make_filter();
    let mut ctx = ctx;
    let total = (FS * seconds) as usize;
    let chunk = 13;
    let (mut peak, mut k) = (0.0f64, 0usize);
    while k < total {
        ctx.rpm_detection.complex_local_oscillator.complex_signal = tone(f, k, chunk);
        ctx = filter.eval(ctx);
        if k > total * 3 / 4 {
            for y in &ctx.rpm_detection.lpf_tracking_band.filtered_signal {
                peak = peak.max(y.norm());
            }
        }
        k += chunk;
    }
    peak
}
///
/// Подаём f=0.5 Гц — это δ, остаток после правильно сработавшего гетеродина. 
/// 0.5 лежит внутри полосы [-3.5, 3.5] Гц. 
/// Ожидание: амплитуда на выходе должна остаться около 1 (входная амплитуда тона), 
/// фильтр почти не должен её тронуть. 
/// Если тест падает — фильтр слишком сильно давит даже то, 
/// что должен пропускать (например, fc посчитана неверно или перепутана с чем-то другим).
#[test]
fn passes_low_frequency() {
    let amp = run(0.5, 20.0);
    let target_amp = 1.0; // Основное тригонометрической тождество: cos(x) + sin(x) = 1
    assert!((amp - target_amp).abs() < 0.05, "0.5 Гц должна проходить, амплитуда = {amp}");
}
///
/// Подаём f= X Гц — модель гармоники 2X, 
/// оказавшейся далеко от нуля: 2X после гетеродинирования уходит на частоту порядка f̂, 
/// далеко за пределы узкой полосы B. 
/// Отношение 50/3.5 ≈ 14, при затухании 4-го порядка 
/// амплитуда должна упасть примерно в 14⁴ ≈ 4·10⁴ раз. 
#[test]
fn rejects_far_frequency_fourth_order() {
    let f = 50.0;
    let amp = run(f, 20.0);
    let target_amp = (F_CUTOFF / f).powi(4); // ≈ 2.4e-5
    // 4-й порядок: ожидание ±3×. Один биквад дал бы ≈ 5e-3, то есть в ~200 раз больше.
    assert!(
        amp < target_amp * 3.0 && amp > target_amp / 3.0,
        "50 Гц: амплитуда {amp}, ожидалось ≈{target_amp}"
    );
}
///
/// Амплитуда (коэффициент передачи) фильтра Баттерворта 
/// на частоте среза всегда равна 1/sqrt(2) независимо от порядка фильтра [n].
#[test]
fn cutoff_is_minus_3db() {
    let amp = run(F_CUTOFF, 20.0);
    assert!((amp - 0.707).abs() < 0.03, "на fc ожидается ≈0.707, амплитуда = {amp}");
}
///
/// Тест, что фильтр симметричен относительно нуля — 
/// не важно, в какую сторону ошиблась грубая оценка RPM (частота вала выше или ниже f̂), 
/// полоса должна пропускать одинаково.
#[test]
fn negative_frequency_passes_symmetrically() {
    // δ может быть отрицательной (оценка RPM завышена): должна проходить так же
    let amp = run(-0.5, 20.0);
    assert!((amp - 1.0).abs() < 0.05, "-0.5 Гц должна проходить, амплитуда = {amp}");
}
///
/// Прогоняет один и тот же тон 3 Гц двумя способами: 
/// чанками через отдельные вызовы eval, и одним сплошным куском. 
/// Раз состояние биквада — это рекурсия (каждое y[n] зависит от предыдущих w[n-1], w[n-2]), 
/// результат должен быть побитово идентичен (с точностью до f64). 
#[test]
fn state_persists_between_frames() {
    let filter = make_filter();
    let total = 13 * 77;
    let mut ctx_a = make_ctx();
    let mut chunked: Vec<Complex<f64>> = Vec::new();
    for k in (0..total).step_by(13) {
        ctx_a.rpm_detection.complex_local_oscillator.complex_signal = tone(3.0, k, 13);
        ctx_a = filter.eval(ctx_a);
        chunked.extend(ctx_a.rpm_detection.lpf_tracking_band.filtered_signal.iter().cloned());
    }
    let mut ctx_b = make_ctx();
    ctx_b.rpm_detection.complex_local_oscillator.complex_signal = tone(3.0, 0, total);
    ctx_b = filter.eval(ctx_b);
    let whole = &ctx_b.rpm_detection.lpf_tracking_band.filtered_signal;
    assert_eq!(chunked.len(), whole.len());
    for (a, b) in chunked.iter().zip(whole.iter()) {
        assert!((a - b).norm() < 1e-12, "состояние потеряно между фреймами: {a} vs {b}");
    }
}
///
/// Подаём сигнал только в канал I (Q=0 на входе), проверяем, 
/// что на выходе Q остаётся пренебрежимо малым. 
/// Биквад — линейная система с раздельными состояниями w1_i/w2_i и w1_q/w2_q, 
/// поэтому канал I физически не должен влиять на канал Q, 
/// только на свой собственный выход. Это защита от возможной ошибки, 
/// при которой состояния случайно перепутались бы местами или использовались совместно.
#[test]
fn iq_channels_are_independent() {
    let filter = make_filter();
    let mut ctx = make_ctx();
    ctx.rpm_detection.complex_local_oscillator.complex_signal =
        (0..2000).map(|k| Complex::new((2.0 * PI * 1.0 * k as f64 / FS).cos(), 0.0)).collect();
    ctx = filter.eval(ctx);
    for y in &ctx.rpm_detection.lpf_tracking_band.filtered_signal {
        assert!(y.im.abs() < 1e-12, "Q-канал получил сигнал из I: {}", y.im);
    }
}
///
/// AngularCtx::new(f_sample, chunk) настраивает биквады на f_sample_dec = 16 кГц, а не на 320 кГц.
/// Признаки: коэффициент передачи ≈1 вблизи нуля и ≈0.707 на fc.
#[test]
fn production_ctx_uses_decimated_sample_rate() {
    let dc = run_with(prod_ctx(), 0.05, 20.0);
    assert!((dc - 1.0).abs() < 0.02, "DC-gain ≠ 1: {dc}");
    let at_fc = run_with(prod_ctx(), F_CUTOFF, 20.0);
    assert!(
        (at_fc - 0.7071).abs() < 0.03,
        "на fc ожидалось 0.707, получено {at_fc}: биквады настроены не на f_sample_dec"
    );
}
///
/// Численная устойчивость: 10 минут при 16 кГц (9.6 млн отсчётов).
/// Выход конечен, ограничен, амплитуда в начале и в конце совпадает.
#[test]
fn long_run_is_numerically_stable() {
    let filter = make_filter();
    let mut ctx = prod_ctx();
    let total = (FS * 600.0) as usize;
    let chunk = 13;
    let (mut early, mut late) = (0.0f64, 0.0f64);
    let mut k = 0usize;
    while k < total {
        ctx.rpm_detection.complex_local_oscillator.complex_signal = tone(0.5, k, chunk);
        ctx = filter.eval(ctx);
        for y in &ctx.rpm_detection.lpf_tracking_band.filtered_signal {
            let a = y.norm();
            assert!(a.is_finite() && a < 1.1, "расходимость на отсчёте {k}: {a}");
            if k > total / 10 && k < total / 5 { early = early.max(a); }
            if k > total * 9 / 10 { late = late.max(a); }
        }
        k += chunk;
    }
    assert!(early > 0.9, "сигнал не прошёл: {early}");
    assert!((early - late).abs() < 1e-6, "дрейф амплитуды: {early} → {late}");
}
///
/// Пустой complex_signal: filtered_signal пуст, паники нет, состояние не испорчено.
#[test]
fn empty_input_gives_empty_output() {
    let filter = make_filter();
    let mut ctx = prod_ctx();
    ctx.rpm_detection.complex_local_oscillator.complex_signal.clear();
    ctx = filter.eval(ctx);
    assert!(ctx.err.is_none());
    assert!(ctx.rpm_detection.lpf_tracking_band.filtered_signal.is_empty());
    // после пустого пакета фильтр продолжает работать корректно
    let amp = run_with(ctx, 0.5, 20.0);
    assert!((amp - 1.0).abs() < 0.05, "состояние испорчено пустым пакетом: {amp}");
}