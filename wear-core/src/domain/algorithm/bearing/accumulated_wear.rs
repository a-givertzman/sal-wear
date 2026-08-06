use sal_core::dbg::Dbg;
use crate::{
    Eval, 
    domain::context::Context
};
///
/// Расчёт накопленного повреждения подшипника: D [безразмерная величина]
/// См. [раздел 8.2, шаг 4](../../Operion_Diag_Вибродиагностика_и_остаточный_ресурс.pdf)
/// Формула: 
/// D = n / N
/// Где:
/// * `n` — [фактическое число оборотов](crate::domain::algorithm::bearing::actual_speed::ActualSpeed) [об]
/// * `N` — [допустимое число оборотов](crate::domain::algorithm::bearing::limiting_speed::LimitingSpeed)) (номинальный ресурс) [об]
pub struct BearingAccumulatedWear<Child> {
    child: Child,
    dbg: Dbg,
}
//
//
impl<Child> BearingAccumulatedWear<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// Новый экземпляр [BearingAccumulatedWear]
    pub fn new(
        parent: &Dbg, 
        child: Child
    ) -> Self {
        let dbg = Dbg::new(parent, "BearingAccumulatedWear");
        Self {
            child,
            dbg,
        }
    }
}
impl<Child> Eval<Context, Context> for BearingAccumulatedWear<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    fn eval(&self, ctx: Context) -> Context {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        ctx.bearing_accumulated_wear = ctx.actual_speed / ctx.limiting_speed;
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
