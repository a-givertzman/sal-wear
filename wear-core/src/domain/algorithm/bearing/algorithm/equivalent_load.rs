use sal_core::dbg::Dbg;
use crate::{
    Eval, 
    domain::context::Context
};
///
/// Расчёт эквивалентной нагрузки на подшипник [H]
pub struct EquivalentLoad<Child> {
    /// Коэффициент радиальной нагрузки
    X: f64,
    /// Коэффициент осевой нагрузки
    Y: f64,
    child: Child,
    dbg: Dbg,
}
//
//
impl<Child> EquivalentLoad<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// Новый экземпляр [EquivalentLoad]
    /// * `X` - коэффициент радиальной нагрузки
    /// * `Y` - коэффициент осевой нагрузки 
    pub fn new(
        X: f64,
        Y: f64,
        parent: &Dbg, 
        child: Child
    ) -> Self {
        let dbg = Dbg::new(parent, "EquivalentLoad");
        Self {
            child,
            dbg,
            X,
            Y,
        }
    }
}
impl<Child> Eval<Context, Context> for EquivalentLoad<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    fn eval(&self, ctx: Context) -> Context {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        ctx.equivalent_load = self.X * ctx.radial_load + self.Y * ctx.axial_load;
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
