use sal_core::{dbg::Dbg, error::Error};
use crate::{
    Eval, 
    domain::context::Context
};
///
/// Расчёт крутящего момента на валу редуктора
pub struct MotorTorque<Child> {
    child: Child,
    dbg: Dbg,
}
//
//
impl<Child> MotorTorque<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// Новый экземпляр [MotorTorque]
    pub fn new(
        parent: &Dbg, 
        child: Child
    ) -> Self {
        let dbg = Dbg::new(parent, "MotorTorque");
        Self {
            child,
            dbg,
        }
    }
}
impl<Child> Eval<Context, Context> for MotorTorque<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    fn eval(&self, ctx: Context) -> Context {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        let Some(motor_p) = &ctx.motor_p else {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Motor P isn't initialized"));
            return ctx;
        };
        let Some(motor_rpm) = &ctx.motor_rpm else {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Motor RPM isn't initialized"));
            return ctx;
        };
        if *motor_rpm < 0.1 {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Motor RPM is about zero"));
            return ctx;
        }
        ctx.motor_torque = 9550.0 * motor_p / motor_rpm;
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
