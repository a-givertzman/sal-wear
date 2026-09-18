use sal_core::{
    dbg::Dbg, 
};
use crate::{
    Eval, 
    domain::context::Context
};
///
/// Расчёт повреждения зуба с учетом температуры
/// См. [раздел 8.3, шаг 8](../../Operion_Diag_Вибродиагностика_и_остаточный_ресурс.pdf)
/// Формула: 
/// D_gear_T = D_gear * KT
/// Где:
/// * `D_gear` — повреждения зуба
/// * `KT` — температурный множитель
pub struct TempToothDamage<Child> {
    child: Child,
    dbg: Dbg,
}
//
//
impl<Child> TempToothDamage<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// Новый экземпляр [TempToothDamage]
    pub fn new(
        parent: &Dbg, 
        child: Child
    ) -> Self {
        let dbg = Dbg::new(parent, "TempToothDamage");
        Self {
            child,
            dbg,
        }
    }
}
impl<Child> Eval<Context, Context> for TempToothDamage<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    fn eval(&self, ctx: Context) -> Context {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }        
        ctx.temp_tooth_damage = ctx.tooth_damage * ctx.temp_coeff;
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
