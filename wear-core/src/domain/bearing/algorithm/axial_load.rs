use sal_core::{dbg::Dbg, error::Error};
use crate::{
    Eval, 
    domain::context::Context
};
///
/// Расчёт осевой нагрузки на подшипник [H]
pub struct AxialLoad<Child> {
    /// Предельное значение отношения радиальной нагрузки к осевой нагрузке
    fa_to_fr: f64,
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
    /// * `fa_to_fr` - Предельное значение отношения радиальной нагрузки к осевой нагрузке
    pub fn new(
        fa_to_fr: f64,  
        parent: &Dbg, 
        child: Child
    ) -> Self {
        let dbg = Dbg::new(parent, "AxialLoad");
        Self {
            fa_to_fr,
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
        ctx.axial_load = self.fa_to_fr * ctx.radial_load;
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
