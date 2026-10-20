use std::f64::consts::PI;
use sal_core::dbg::Dbg;
use crate::{AngularCtx, Eval, PhaseStatus, err};
use function_name::named;
/// Вычисление мгновенной фазы вала
///
/// # Входные данные
/// Комплексная огибающая `I[n] + j*Q[n]` после фильтрации ФНЧ и массив накопленной фазы
/// гетеродина `phi_het[n]`, соответствующие текущему отсчету времени.
///
/// # Математическая суть
/// Абсолютная мгновенная фаза вала восстанавливается для каждого сэмпла по тождеству:
///
///   phi_вал[n - D] = (atan2(Q[n], I[n]) + phi_het[n - D]) mod (2 * PI)
///
/// где `D` - групповая задержка ФНЧ в отсчетах. Тождество точно при любой ошибке `f_hat`
/// грубой настройки частоты: если сетевой RPM неточен, `arg(y)` дрейфует со скоростью
/// `2 * PI * delta`, и этот дрейф в сумме компенсируется фазой гетеродина.
///
/// # Оптимизация
/// В текущей версии `atan2` вычисляется на каждом сэмпле, без прореживания и экстраполяции.
/// Задание допускает вычисление раз в K = 32 сэмпла с линейной экстраполяцией по оценке
/// частоты; это отложено до профилирования. Выход в `[0, 2π)`, дополнительно ведется
/// непрерывная развернутая фаза `shaft_phase_unwrapped` (счетчик оборотов `turns`
/// переносится между пакетами).
///
/// # Компенсация задержки ФНЧ
/// Фиксированный ФНЧ имеет групповую задержку `D = (1/Q1 + 1/Q2) / (2π·fc)` (около 0.12 с
/// при `fc = 3.5 Гц`), то есть `atan2` берется от огибающей момента `n - D`. Чтобы
/// складывать фазы одного момента времени, `phi_het` пропускается через линию задержки
/// на `D` отсчетов. Следствия:
/// - `shaft_phase[n]` относится к моменту `n - D` (латентность `D / f_sample_dec`);
///   потребитель обязан синхронизировать с ней сырые отсчеты (задержка `D·M` отсчетов АЦП);
/// - первые `D` отсчетов помечаются `ready = false`;
/// - остается малая ошибка `arg H(δ) + 2π·δ·D`, так как задержка слегка зависит от частоты:
///   нулевая при `δ → 0`, растет к краю полосы. Покрывается флагом валидности.
///
///
/// # Выход данных
/// Угол поворота вала в радианах `0..2*PI` для каждого отсчета тракта плюс флаг `ready`,
/// готовый для передачи в дисковые фильтры порядкового анализа.
pub struct InstantShaftPhase<Child> {
    /// Дочерний вычислительный узел конвейера
    child: Child,
    /// Контекст отладки и логирования
    dbg: Dbg,
}
//
impl<Child> InstantShaftPhase<Child>
where
    Child: Eval<AngularCtx, AngularCtx> + Send + 'static,
{
    /// Создает новый экземпляр вычислительного узла `InstantShaftPhase`.
    /// 
    /// # Аргументы
    /// * `parent` - Ссылка на отладочный контекст родительской сущности
    /// * `child` - Предыдущий вычислительный шаг конвейера
    pub fn new(parent: &Dbg, child: Child) -> Self {
        let dbg = Dbg::new(parent, crate::me::<Self>());
        Self {
            child,
            dbg,
        }
    }
}
//
impl<Child> Eval<AngularCtx, AngularCtx> for InstantShaftPhase<Child>
where
    Child: Eval<AngularCtx, AngularCtx> + Send + 'static,
{
    /// Восстанавливает абсолютную фазу вала отсчет за отсчетом с выравниванием по времени.
    ///
    /// Фаза гетеродина кладется в линию задержки и извлекается с запаздыванием на
    /// `delay_samples`, пока линия заполняется, отсчеты помечаются `ready = false`.
    #[inline]
    #[named]
    fn eval(&self, ctx: AngularCtx) -> AngularCtx {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        let rd = &mut ctx.rpm_detection;
        let out = &mut rd.instant_shaft_phase;
        out.shaft_phase.clear();
        out.shaft_phase_unwrapped.clear();
        out.ready.clear();

        let filtered = &rd.lpf_tracking_band.filtered_signal;
        let phi_het = &rd.complex_local_oscillator.phi_het;
        if filtered.is_empty() {
            return ctx;
        }
        if filtered.len() != phi_het.len() {
            let (a, b) = (filtered.len(), phi_het.len());
            out.reset();
            ctx.err = Some(err!(self.dbg, "eval: filtered_signal ({}) != phi_het ({})", a, b));
            return ctx;
        }
        for (y, &phi_now) in filtered.iter().zip(phi_het.iter()) {
            out.phi_delay.push_back(phi_now);
            if out.phi_delay.len() <= out.delay_samples {
                // Линия задержки еще заполняется: фаза недостоверна.
                out.shaft_phase.push(crate::PhaseStatus::Invalid);
                out.shaft_phase_unwrapped.push(crate::PhaseStatus::Invalid);
                out.ready.push(false);
                continue;
            }
            let phi = out.phi_delay.pop_front().unwrap();
            let phase = (y.im.atan2(y.re) + phi).rem_euclid(2.0 * PI);
            let valid = phase.is_finite();
            if valid {
                if out.prev_phase.is_finite() {
                    // Переход через границу круга: скачок свёрнутой фазы больше π по модулю
                    // означает, что реальное вращение продолжилось в ту же сторону.
                    let d = phase - out.prev_phase;
                    if d < -PI { out.turns += 1; }
                    else if d > PI { out.turns -= 1; }
                }
                out.prev_phase = phase;
            } else if out.prev_phase.is_finite() {
                // TODO(P0): при разрыве потока/провале сигнала вал физически проворачивается
                // на N оборотов, которые мы не отследили. Свёрнутая фаза после восстановления
                // корректна, но развёрнутая получила бы скачок -N·2π, что ломает ресемплинг в
                // угловом домене (OrderDomainSamples использует phases[i+1]-phases[i]).
                // Пока сбрасываем turns И prev_phase, чтобы после восстановления развёрнутая
                // фаза стартовала от текущей свёрнутой. Решение отложено до
                // готовности [шага 3.5.](vibro-core/design/rpm-detection-v1.md)
                out.turns = 0;
                out.prev_phase = f64::NAN;
            }
            out.shaft_phase.push(PhaseStatus::from_value(phase));
            out.shaft_phase_unwrapped.push(PhaseStatus::from_value(out.turns as f64 * 2.0 * PI + phase));
            out.ready.push(true);
        }
        ctx
    }
    /// Вызывает каскадное завершение для дочерних шагов цепочки вычислений.
    fn exit(&self) {
        self.child.exit();
    }
}
