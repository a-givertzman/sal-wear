use sal_core::{dbg::Dbg, error::Error};
use crate::{
    Eval, 
    LIFE_EXPONENT_ROLLER, 
    domain::context::Context
};
///
/// Расчёт допустимого числа оборотов подшипника [об]
pub struct LimitingSpeed<Child> {
    child: Child,
    dbg: Dbg,
}
//
//
impl<Child> LimitingSpeed<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// Новый экземпляр [LimitingSpeed]
    pub fn new(
        parent: &Dbg, 
        child: Child
    ) -> Self {
        let dbg = Dbg::new(parent, "LimitingSpeed");
        Self {
            child,
            dbg,
        }
    }
}
impl<Child> Eval<Context, Context> for LimitingSpeed<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    fn eval(&self, ctx: Context) -> Context {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        ctx.limiting_speed = ctx.basic_rating_life * 10e6;
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
