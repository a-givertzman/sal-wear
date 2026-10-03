use rustfft::num_complex::Complex;
use sal_core::dbg::Dbg;
use crate::{Eval, RpmDetectionCtx};
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
    Child: Eval<RpmDetectionCtx, RpmDetectionCtx> + Send + 'static,
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
impl<Child> Eval<RpmDetectionCtx, RpmDetectionCtx> for LPFTrackingBand<Child>
where
    Child: Eval<RpmDetectionCtx, RpmDetectionCtx> + Send + 'static,
{
    fn eval(&self, ctx: RpmDetectionCtx) -> RpmDetectionCtx {
        let mut ctx = self.child.eval(ctx);
        if ctx.is_err() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        ctx.lpf_tracking_band_ctx.filtered_signal.clear();
        for y in ctx.complex_local_oscillator.complex_signal.iter() {
            let (i1, q1) = ctx.lpf_tracking_band_ctx.stage1.process(y.re, y.im);
            let (i_filt, q_filt) = ctx.lpf_tracking_band_ctx.stage2.process(i1, q1);
            ctx.lpf_tracking_band_ctx.filtered_signal.push(Complex::new(i_filt, q_filt));
        }
        ctx
    }
    fn exit(&self) { self.child.exit(); }
}