use sal_core::{dbg::Dbg, error::Error};
use crate::{
    Eval, 
    domain::context::Context
};
///
/// Расчёт номинального ресурса подшипника: L10 [миллионы оборотов]
/// См. [раздел 7.2.2](../../Operion_Diag_Вибродиагностика_и_остаточный_ресурс.pdf)
/// Формула:
/// L10 = (C_r/P)^p
/// Где:
/// * `C_r`  — динамическая грузоподъемность подшипника [Н] (берётся из каталога подшипников)
/// * `P` — [эквивалентная нагрузка](crate::domain::algorithm::bearing::algorithm::equivalent_load::EquivalentLoad) [Н]
/// * `p` — показатель степени кривой усталости [безразмерная величина]:
/// 	* `p` = 3 для шарикоподшипников
/// 	* `p` = 10/3 - для роликоподшипников
pub struct BasicRatingLife<Child> {
    /// Динамическая грузоподъёмность подшипника [H]
    cr: f64,
    /// Показатель степени кривой усталости [безразмерная величина]
    p: f64,
    child: Child,
    dbg: Dbg,
}
//
//
impl<Child> BasicRatingLife<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// Новый экземпляр [BasicRatingLife]
    /// * `cr` - динамическая грузоподъёмность подшипника
    /// * `p` — показатель степени кривой усталости [безразмерная величина]
    pub fn new(
        cr: f64,
        p: f64,
        parent: &Dbg, 
        child: Child
    ) -> Self {
        let dbg = Dbg::new(parent, "BasicRatingLife");
        Self {
            cr,  
            p,
            child,
            dbg,
        }
    }
}
impl<Child> Eval<Context, Context> for BasicRatingLife<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    fn eval(&self, ctx: Context) -> Context {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        if ctx.equivalent_load < 0.1 {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Bearing equivalent load is about zero"));
            return ctx;
        }
        ctx.basic_rating_life = (self.cr / ctx.equivalent_load).powf(self.p);
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
