//! Интеграционные тесты связки «децимация → комплексный гетеродин → фиксированный ФНЧ».
//!
//! ## Что проверяется
//! Шаги 3.1–3.3 алгоритма определения RPM/фазы вала (v.01):
//! 1. `Decimation` понижает частоту дискретизации с 320 кГц до 16 кГц.
//! 2. `ComplexLocalOscillator` умножает сигнал на `e^(-jφ_het)` и сносит
//!    частоту 1X к нулю: остаётся разность `δ = f_true - f_het`.
//! 3. `LPFTrackingBand` (2 биквада Баттерворта, I/Q) пропускает только
//!    узкую полосу вокруг нуля и давит остальное.
//!
//! ## Почему тест интеграционный
//! Вход гетеродина вещественный, поэтому после умножения в `y[n]` присутствуют
//! две компоненты: разностная `f_true - f_het` (нужная) и суммарная
//! `f_true + f_het` (паразитная). Скорость вращения фазы `δ` наблюдаема только
//! после ФНЧ, который убирает суммарную компоненту. Поэтому гетеродин нельзя
//! проверять отдельно по `atan2` на его сыром выходе.
//!
//! ## Источник сигнала
//! `Udp` — генератор сырых отсчётов АЦП (`u16`, DC = 2048) на частоте 320 кГц.
//! Фаза гармоник накапливается между вызовами `parse`, поэтому поток
//! непрерывен на границах пакетов по 512 отсчётов.
use std::{f64::consts::PI, sync::Arc};
use chrono::Utc;
use sal_core::dbg::Dbg;
use crate::{
    Biquad, ComplexLocalOscillator, Decimation, Eval, Frame, LPFTrackingBand,
    MockEventValues, ReadEventValues, RpmDetectionCtx,
    tests::{Frequency, Udp},
};
// ---------- КОНФИГУРАЦИЯ ТЕСТА ----------
/// Коэффициенты КИХ-фильтра нижних частот (ФНЧ)
/// Расчитаны для Fs=320kHz, Fc=8kHz (спад к 16kHz), 32 тапа, окно Хемминга.
const FIR_TAPS_M20_N32: [f64; 32] = [
    -0.000713, -0.001646, -0.002497, -0.002570, -0.000958,  0.003180,  0.010260,  0.020141,
     0.032213,  0.045431,  0.058371,  0.069485,  0.077366,  0.081125,  0.081125,  0.077366,
     0.069485,  0.058371,  0.045431,  0.032213,  0.020141,  0.010260,  0.003180, -0.000958,
    -0.002570, -0.002497, -0.001646, -0.000713,  0.000000,  0.000000,  0.000000,  0.000000,
];
/// Частота дискретизации АЦП, Гц.
const ADC_FS: f64 = 320_000.0;
/// Коэффициент децимации.
const DEC_COEFF: f64 = 20.0;
/// Частота дискретизации после децимации, Гц (`ADC_FS / DEC_COEFF` = 16 кГц).
const FS_DEC: f64 = ADC_FS / DEC_COEFF;
/// Полоса слежения ФНЧ `B`, Гц. Частота среза `fc = B/2`.
const BAND_HZ: f64 = 7.0;
/// Амплитуда сигнала в единицах АЦП. `2048 + 1000 < 4095`, клиппинга нет.
const AMP: u16 = 1000;
///
/// Прогоняет синусоиду через цепочку и измеряет результат на установившемся участке.
///
/// Цепочка (порядок выполнения изнутри наружу):
/// `ReadEventValues` (RPM в `ctx.raw_rpm`) → `Decimation` → `ComplexLocalOscillator` → `LPFTrackingBand`.
///
/// ## Параметры
/// - `f_true` - истинная частота входного сигнала, Гц.
/// - `f_het` - грубая оценка частоты вала для гетеродина, Гц (в цепочку передаётся как RPM = `f_het * 60`).
/// - `seconds` - длительность прогона. Первая половина отбрасывается как переходный процесс КИХ и ФНЧ.
///
/// ## Возвращает
/// `(δ, A)`:
/// - `δ` - остаточная частота на выходе ФНЧ, Гц. Считается как наклон развёрнутой фазы
///   (суммарный угол поворота / время), что устойчивее разности соседних точек.
/// - `A` - средний модуль выходного вектора. Для вещественного входа амплитуды `AMP`
///   ожидается `AMP / 2`: вторая половина энергии уходит в суммарную компоненту,
///   которую срезает ФНЧ.
fn run(f_true: f64, f_het: f64, seconds: f64) -> (f64, f64) {
    let dbg = Dbg::own("heterodyne_udp_test");
    let inputs = Arc::new(MockEventValues::new());
    inputs.set_rpm(f_het * 60.0);
    let chain = LPFTrackingBand::new(
        &dbg,
        ComplexLocalOscillator::new(
            &dbg,
            Decimation::new(
                &dbg, 20, ADC_FS,
                ReadEventValues::new(&dbg, ["rpm"], inputs.clone()),
            ),
        ),
    );
    let fc = BAND_HZ / 2.0;
    let mut ctx = RpmDetectionCtx::default();
    ctx.lpf_tracking_band_ctx.stage1 = Biquad::new(fc, FS_DEC, 0.5412);
    ctx.lpf_tracking_band_ctx.stage2 = Biquad::new(fc, FS_DEC, 1.3066);
    let mut udp = Udp::new(512, ADC_FS, [(Frequency::Static(f_true), AMP)]);
    let frames = (ADC_FS * seconds / 512.0) as usize;
    let total_dec = (FS_DEC * seconds) as usize;
    let warmup = total_dec / 2; // индекс децимированного отсчёта, с которого начинаем измерять
    let mut idx = 0usize;           // сквозной индекс выходных точек
    let mut prev: Option<f64> = None; // предыдущая фаза
    let mut unwrapped = 0.0;        // накопленный угол поворота (развёрнутая фаза)
    let mut n_meas = 0usize;        // число измеренных приращений
    let mut amp_sum = 0.0;          // сумма модулей для средней амплитуды
    for _ in 0..frames {
        let mut samples = [0u16; 512];
        udp.parse(0.0, &mut samples); // гармоника Static, rpm не важен
        ctx.frame = Arc::new(Frame::raw(Utc::now(), samples));
        ctx = chain.eval(ctx);
        // Защита от «пустого» теста: без этих проверок цикл ниже мог бы
        // не выполниться ни разу, и тест прошёл бы, ничего не проверив.
        assert!(!ctx.is_err(), "ошибка в цепочке");
        assert!(!ctx.decimation.decimated.is_empty(), "decimated пуст");
        assert!(!ctx.lpf_tracking_band_ctx.filtered_signal.is_empty(), "filtered_signal пуст");
        for y in ctx.lpf_tracking_band_ctx.filtered_signal.iter() {
            let phase = y.im.atan2(y.re);
            if idx >= warmup {
                if let Some(p) = prev {
                    // Приращение фазы с поправкой на скачок через границу ±π
                    let mut d = phase - p;
                    if d > PI { d -= 2.0 * PI; }
                    if d < -PI { d += 2.0 * PI; }
                    unwrapped += d;
                    n_meas += 1;
                }
                amp_sum += y.norm();
            }
            prev = Some(phase);
            idx += 1;
        }
    }
    assert!(n_meas > 1000, "мало точек для измерения: {n_meas}");
    // Частота = число оборотов вектора / время измерительного участка
    let delta = unwrapped / (2.0 * PI * n_meas as f64 / FS_DEC);
    (delta, amp_sum / n_meas as f64)
}
/// Основной случай: истинная 50 Гц, гетеродин 49.5 Гц.
/// Ожидаем остаток `δ = +0.5 Гц` и амплитуду `AMP / 2`.
#[test]
fn udp_heterodyne_shifts_1x_to_residual() {
    let (delta, amp) = run(50.0, 49.5, 4.0);
    assert!((delta - 0.5).abs() < 0.05, "ожидалась δ≈+0.5 Гц, получено {delta}");
    // Вещественный вход → амплитуда вдвое меньше: по формуле Эйлера сигнал
    // раскладывается на две компоненты по A/2, одну из них срезает ФНЧ.
    let fir_gain: f64 = FIR_TAPS_M20_N32.iter().sum();
    let expected = AMP as f64 * 0.5 * fir_gain;
    assert!((amp - expected).abs() < expected * 0.1, "ожидалась амплитуда ≈{expected}, получено {amp}");
}
/// Гетеродин завышен (50.5 Гц против 50 Гц): остаток отрицательный, `δ = -0.5 Гц`.
/// Проверяет знак экспоненты `e^(-jφ_het)`.
#[test]
fn udp_heterodyne_negative_residual() {
    let (delta, _) = run(50.0, 50.5, 4.0);
    assert!((delta + 0.5).abs() < 0.05, "ожидалась δ≈-0.5 Гц, получено {delta}");
}
/// Гармоника 2X (100 Гц) после сноса на 49.5 Гц оказывается около 50 Гц,
/// далеко за полосой ФНЧ (fc = 3.5 Гц), и должна подавляться.
/// Оценка: `(3.5/50)^4 ≈ 2.4e-5`, от амплитуды ~500 это около 0.01.
#[test]
fn udp_heterodyne_rejects_2x() {
    let (_, amp) = run(100.0, 49.5, 4.0);
    assert!(amp < 0.5, "2X должна подавляться, амплитуда = {amp}");
}