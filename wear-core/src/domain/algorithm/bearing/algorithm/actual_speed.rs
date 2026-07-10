use sal_core::{dbg::Dbg, error::Error};
use crate::{
    Eval, 
    domain::context::Context
};
///
/// Расчёт фактического числа оборотов подшипника [об]
pub struct ActualSpeed<Child> {
    child: Child,
    dbg: Dbg,
}
//
//
impl<Child> ActualSpeed<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// Новый экземпляр [ActualSpeed]
    pub fn new(
        parent: &Dbg, 
        child: Child
    ) -> Self {
        let dbg = Dbg::new(parent, "ActualSpeed");
        Self {
            child,
            dbg,
        }
    }
}
impl<Child> Eval<Context, Context> for ActualSpeed<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    fn eval(&self, ctx: Context) -> Context {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        let Some(motor_rpm) = &ctx.motor_rpm else {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Motor RPM isn't initialized"));
            return ctx;
        };
        if ctx.duration < 0.1 {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Duration is about zero"));
            return ctx;
        }
        if *motor_rpm < 0.1 {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Motor RPM load is about zero"));
            return ctx;
        }
        ctx.actual_speed = motor_rpm * (ctx.duration / 60.0);
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
