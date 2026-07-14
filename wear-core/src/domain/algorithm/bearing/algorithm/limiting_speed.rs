use sal_core::{dbg::Dbg, error::Error};
use crate::{
    Eval, 
    domain::context::Context
};
///
/// Расчёт допустимого числа оборотов подшипника: N [об]
/// См. [раздел 8.2, шаг 2](../../Operion_Diag_Вибродиагностика_и_остаточный_ресурс.pdf)
/// Формула:
/// N = L10 * 10^6
/// Где: 
/// * `L10` — [номинальный ресурс](crate::domain::algorithm::bearing::algorithm::basic_rating_life::BasicRatingLife) [миллионы оборотов]
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
        ctx.limiting_speed = ctx.basic_rating_life * 1_000_000.0;
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
