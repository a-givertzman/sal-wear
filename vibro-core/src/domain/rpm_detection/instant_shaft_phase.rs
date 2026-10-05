use std::f64::consts::PI;
use sal_core::dbg::Dbg;
use crate::{RpmDetectionCtx, Eval};
///
/// Вычисление мгновенной фазы вала
pub struct InstantShaftPhase<Child> {
    child: Child,
    dbg: Dbg,
}

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

impl<Child> Eval<RpmDetectionCtx, RpmDetectionCtx> for InstantShaftPhase<Child>
where
    Child: Eval<RpmDetectionCtx, RpmDetectionCtx> + Send + 'static,
{
    #[inline]
    fn eval(&self, ctx: RpmDetectionCtx) -> RpmDetectionCtx {
        // Передаем контекст дальше по цепочке вниз
        let mut ctx = self.child.eval(ctx);
        if ctx.is_err() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        ctx.instant_shaft_phase.shaft_phase.clear();
        for (i, filtered_value) in ctx.lpf_tracking_band_ctx.filtered_signal.iter().enumerate() {
            let shaft_phase = (filtered_value.im.atan2(filtered_value.re) + ctx.complex_local_oscillator.phi_net[i])
            .rem_euclid(2.0 * PI);
            ctx.instant_shaft_phase.shaft_phase.push(shaft_phase);
        }
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}