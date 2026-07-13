use sal_core::dbg::Dbg;
use crate::{
    Eval, 
    domain::context::Context
};
///
/// Расчёт накопленного повреждения подшипника с учётом температуры [об]
pub struct BearingTempAccumulatedWear<Child> {
    child: Child,
    dbg: Dbg,
}
//
//
impl<Child> BearingTempAccumulatedWear<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// Новый экземпляр [BearingTempAccumulatedWear]
    pub fn new(
        parent: &Dbg, 
        child: Child
    ) -> Self {
        let dbg = Dbg::new(parent, "BearingTempAccumulatedWear");
        Self {
            child,
            dbg,
        }
    }
}
impl<Child> Eval<Context, Context> for BearingTempAccumulatedWear<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    fn eval(&self, ctx: Context) -> Context {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        ctx.bearing_temp_accumulated_wear = ctx.bearing_accumulated_wear * ctx.temp_coeff;
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
