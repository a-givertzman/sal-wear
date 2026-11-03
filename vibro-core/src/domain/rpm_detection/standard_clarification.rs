use std::f64::consts::{PI, TAU};
use sal_core::dbg::Dbg;
use function_name::named;
use crate::{AngularCtx, Eval, err};
///
/// Уточнение частоты по дрейфу arg(y)
///
/// Оценивает отклонение δ = (1/2π)·d/dt arg(y[n]) огибающей после ФНЧ,
/// усреднённое на окне длиной в целое число оборотов вала, и вычисляет
/// точную частоту вращения f_precise = f_het + δ.
///
/// Разомкнутая коррекция f_het (опционально): доля δ переносится в
/// `capture_mode.f_precise`, которую гетеродин читает на старте следующего
/// чанка. Обратной связи в тракт фазы нет — тождество шага 3.4 точно
/// при любом f_het, коррекция лишь удерживает δ около нуля внутри полосы B.
///
pub struct StandardClarification<Child> {
    /// Дочерний вычислительный узел конвейера
    child: Child,
    /// Контекст отладки и логирования
    dbg: Dbg,
}
//
impl<Child> StandardClarification<Child>
where
    Child: Eval<AngularCtx, AngularCtx> + Send + 'static,
{
    /// ### Returns `StandardClarification` new instance
    /// - `parent` - Идентификатор родительской сущности (для отладки)
    /// - `child` - Дочерний (предыдущий) шаг вычислений
    pub fn new(
        parent: &Dbg,
        child: Child,
    ) -> Self {
        let dbg = Dbg::new(parent, crate::me::<Self>());
        Self {
            child,
            dbg,
        }
    }
}
//
impl<Child> Eval<AngularCtx, AngularCtx> for StandardClarification<Child>
where
    Child: Eval<AngularCtx, AngularCtx> + Send + 'static,
{
    #[named]
    fn eval(&self, ctx: AngularCtx) -> AngularCtx {
        // Передаем контекст дальше по цепочке вниз (гетеродин -> ФНЧ -> валидность)
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        let f_het = *ctx.rpm_detection.capture_mode.f_precise
            .get_or_insert_with(|| ctx.raw_rpm / 60.0);
        let f_sample_dec = ctx.rpm_detection.decimation.f_sample_dec.value();
        if !f_het.is_finite() || f_het <= 0.0 || !f_sample_dec.is_finite() || f_sample_dec <= 0.0 {
            ctx.err = Some(err!(self.dbg, "f_het {} or f_sample_dec {} is not a valid number", f_het, f_sample_dec));
            return ctx;
        }
        let track_valid = ctx.rpm_detection.validity_detector.valid;
        let fr = &mut ctx.rpm_detection.standard_clarification;
        // Длина окна усреднения — целое число периодов оборота вала
        let window_len = fr.window_len_samples(f_sample_dec, f_het);
        for y in ctx.rpm_detection.lpf_tracking_band.filtered_signal.iter() {
            if let Some(delta) = fr.update(y, window_len, f_sample_dec, f_het) {
                if fr.apply_correction && track_valid {
                    ctx.rpm_detection.capture_mode.f_precise =
                        Some(f_het + fr.correction_gain * delta);
                }
            }
        }
        ctx
    }
    /// Вызывает каскадное завершение для дочерних шагов цепочки вычислений.
    fn exit(&self) {
        self.child.exit();
    }
}