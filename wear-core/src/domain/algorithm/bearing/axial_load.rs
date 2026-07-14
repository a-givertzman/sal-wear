use sal_core::dbg::Dbg;
use crate::{
    Eval, 
    domain::context::Context
};
///
/// Расчёт осевой нагрузки на подшипник: F_a [H]
/// См. [раздел 8.6](../../08_FaFr_Calculation.md)
/// Формула: 
/// F_a = fa_to_fr * F_r
/// Где:
/// * `F_r` — [радиальная нагрузка на подшипник](crate::domain::algorithm::bearing::algorithm::RadialLoad) [H]
/// * `fa_to_fr` — коэффициент осевого нагружения (отношение осевой силы к радиальной F_a/F_r) [безразмерная величина]
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
    /// Новый экземпляр [AxialLoad]
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
