use rustfft::num_complex::Complex;
use sal_core::dbg::Dbg;
use crate::{Eval, AngularCtx};
///
/// Фиксированный ФНЧ полосы слежения (2 биквада, I/Q)
pub struct LPFTrackingBand<Child> {
    child: Child,
    dbg: Dbg,
}
//
//
impl<Child> LPFTrackingBand<Child>
where
    Child: Eval<AngularCtx, AngularCtx> + Send + 'static,
{
    /// ### Returns `LPFTrackingBand` new instance
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
//
impl<Child> Eval<AngularCtx, AngularCtx> for LPFTrackingBand<Child>
where
    Child: Eval<AngularCtx, AngularCtx> + Send + 'static,
{
    fn eval(&self, ctx: AngularCtx) -> AngularCtx {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        ctx.rpm_detection.lpf_tracking_band.filtered_signal.clear();
        for y in ctx.rpm_detection.complex_local_oscillator.complex_signal.iter() {
            let (i1, q1) = ctx.rpm_detection.lpf_tracking_band.stage1.process(y.re, y.im);
            let (i_filt, q_filt) = ctx.rpm_detection.lpf_tracking_band.stage2.process(i1, q1);
            ctx.rpm_detection.lpf_tracking_band.filtered_signal.push(Complex::new(i_filt, q_filt));
        }
        ctx
    }
    fn exit(&self) { self.child.exit(); }
}