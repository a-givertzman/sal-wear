use sal_core::{dbg::Dbg, error::Error};
use crate::{
    Eval, 
    domain::context::Context
};
///
/// Расчёт осевой нагрузки на подшипник [H]
pub struct AxialLoad<Child> {
    child: Child,
    dbg: Dbg,
}
//
//
impl<Child> AxialLoad<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// Новый экземпляр [AxialLoad]
    pub fn new(
        parent: &Dbg, 
        child: Child
    ) -> Self {
        let dbg = Dbg::new(parent, "AxialLoad");
        Self {
            child,
            dbg,
        }
    }
}
impl<Child> Eval<Context, Context> for AxialLoad<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    fn eval(&self, ctx: Context) -> Context {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        ctx.axial_load = 0.2 * ctx.radial_load;
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
