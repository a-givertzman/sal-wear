use sal_core::dbg::Dbg;
use crate::{
    Eval, 
    domain::context::Context
};
///
/// Расчёт накопленного повреждения подшипника с учётом температуры: D_T [об]
/// См. [раздел 8.2, шаг 6](../../Operion_Diag_Вибродиагностика_и_остаточный_ресурс.pdf)
/// Формула:
/// D_T = D * KT
/// Где:
/// * `D` — [накопленное повреждение в режиме](crate::domain::algorithm::bearing::accumulated_wear::BearingAccumulatedWear) [об]
/// * `KT` - [температурный коэффициент](crate::domain::algorithm::temp_coeff::TempCoeff) (при отсутствии температуры -  KT = 1)
pub struct BearingTempAccumulatedWear<Child> {
    child: Child,
    dbg: Dbg,
}
//
//
impl<Child> BearingTempAccumulatedWear<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// Новый экземпляр [BearingTempAccumulatedWear]
    pub fn new(
        parent: &Dbg, 
        child: Child
    ) -> Self {
        let dbg = Dbg::new(parent, "BearingTempAccumulatedWear");
        Self {
            child,
            dbg,
        }
    }
}
impl<Child> Eval<Context, Context> for BearingTempAccumulatedWear<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    fn eval(&self, ctx: Context) -> Context {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        ctx.bearing_temp_accumulated_wear = ctx.bearing_accumulated_wear * ctx.temp_coeff;
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
