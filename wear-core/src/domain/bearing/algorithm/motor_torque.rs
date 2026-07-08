use sal_core::dbg::Dbg;
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
        match &ctx.err {
            Some(_) => ctx.pass_err(&self.dbg, "eval"),
            None => {
                if ctx.motor_torque.is_none() {
                    match &ctx.p_motor {
                        Some(p_motor) => {
                            match &ctx.rpm {
                                Some(rpm) => {
                                    ctx.motor_torque = Some(9550.0 * p_motor / rpm);
                                }
                                None => {
                                    return ctx.pass_err(&self.dbg, "eval");
                                }
                            }
                        }
                        None => {
                            return ctx.pass_err(&self.dbg, "eval");
                        }
                    }
                }
                ctx
            }
        }
    }
    //
    fn exit(&self) {
        todo!()
    }
}
