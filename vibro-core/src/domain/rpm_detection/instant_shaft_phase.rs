use std::f64::consts::PI;
use sal_core::dbg::Dbg;
use crate::{RpmDetectionCtx, Eval};
///
/// Вычисление мгновенной фазы вала.
///
/// Восстанавливает исходную фазу сигнала путем прибавления накопленной фазы
/// гетеродина к фазе отфильтрованного комплексного сигнала.
pub struct InstantShaftPhase<Child> {
    child: Child,
    dbg: Dbg,
}
//
impl<Child> InstantShaftPhase<Child>
where
    Child: Eval<RpmDetectionCtx, RpmDetectionCtx> + Send + 'static,
{
    /// ### Returns `InstantShaftPhase` new instance
    /// - `parent` - Идентификатор родительской сущности (для отладки)
    /// - `child` - Дочерний (предыдущий) шаг вычислений
    pub fn new(parent: &Dbg, child: Child) -> Self {
        let dbg = Dbg::new(parent, crate::me::<Self>());
        Self {
            child,
            dbg,
        }
    }
}
//
impl<Child> Eval<RpmDetectionCtx, RpmDetectionCtx> for InstantShaftPhase<Child>
where
    Child: Eval<RpmDetectionCtx, RpmDetectionCtx> + Send + 'static,
{
    /// ### Восстанавливает мгновенную фазу вала
    /// 
    /// Математически операция выполняет обратный сдвиг по частоте:
    /// \[\phi_{shaft} = \arg(Z_{filtered}) + \phi_{net} \pmod{2\pi}\]
    #[inline]
    fn eval(&self, ctx: RpmDetectionCtx) -> RpmDetectionCtx {
        // Передаем контекст дальше по цепочке вниз
        let mut ctx = self.child.eval(ctx);
        if ctx.is_err() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        let input_len = ctx.lpf_tracking_band_ctx.filtered_signal.len();
        if input_len == 0 {
            ctx.instant_shaft_phase.shaft_phase.clear();
            return ctx;
        }
        if ctx.instant_shaft_phase.shaft_phase.len() == 0 {
            ctx.instant_shaft_phase.shaft_phase = Vec::with_capacity(input_len);
        } else {
            ctx.instant_shaft_phase.shaft_phase.clear();
        }
        for (&filtered_value, phi_net) in ctx.lpf_tracking_band_ctx.filtered_signal.iter().zip(ctx.complex_local_oscillator.phi_net.iter()) {
            // Вычисляем аргумент комплексного числа (угол) и возвращаем снос частоты
            let phase_angle = filtered_value.im.atan2(filtered_value.re);
            // rem_euclid гарантирует результат в диапазоне [0, 2π) даже для отрицательных углов
            let shaft_phase = (phase_angle + phi_net).rem_euclid(2.0 * PI);
            ctx.instant_shaft_phase.shaft_phase.push(shaft_phase);
        }
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}