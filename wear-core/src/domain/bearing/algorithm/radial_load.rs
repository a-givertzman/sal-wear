use sal_core::{dbg::Dbg, error::Error};
use crate::{
    Eval, 
    domain::context::Context
};
///
/// Расчёт радиальной нагрузки на подшипник [H]
pub struct RadialLoad<Child> {
    /// Диаметр вала двигателя [м]
    motor_d: f64,
    child: Child,
    dbg: Dbg,
}
//
//
impl<Child> RadialLoad<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// Новый экземпляр [RadialLoad]
    /// * `motor_d` - диаметр вала двигателя [м]
    pub fn new(
        motor_d: f64,
        parent: &Dbg, 
        child: Child
    ) -> Self {
        let dbg = Dbg::new(parent, "RadialLoad");
        Self {
            motor_d,
            child,
            dbg,
        }
    }
}
impl<Child> Eval<Context, Context> for RadialLoad<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    fn eval(&self, ctx: Context) -> Context {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        if self.motor_d < 0.1 {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Motor diameter is about zero"));
            return ctx;
        }
        ctx.radial_load = 2.0 * ctx.motor_torque / self.motor_d;
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
