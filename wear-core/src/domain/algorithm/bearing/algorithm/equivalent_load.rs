use sal_core::dbg::Dbg;
use crate::{
    Eval, 
    domain::context::Context
};
///
/// Расчёт эквивалентной нагрузки на подшипник: P [H]
/// См. [раздел 7.2.1](../../Operion_Diag_Вибродиагностика_и_остаточный_ресурс.pdf)
/// Формула:
/// P = X * F_r + Y * F_a
/// Где:
/// * `X, Y` — коэффициенты для расчёта эквивалентной нагрузки
/// * `F_r` —  [радиальная нагрузка на подшипник](crate::domain::algorithm::bearing::algorithm::radial_load::RadialLoad) [H] (если нет прямого измерения — должна быть оценена из механической модели)
/// * `F_a` — [осевая нагрузка на подшипник](crate::domain::algorithm::bearing::algorithm::axial_load::AxialLoad) [H] (если осевая нагрузка отсутствует - передавать 0)
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
