use sal_core::{
    dbg::Dbg, 
};
use crate::{
    Eval, 
    domain::context::Context
};
///
/// Расчёт повреждения зуба
/// См. [раздел 8.3, шаг 7](../../Operion_Diag_Вибродиагностика_и_остаточный_ресурс.pdf)
/// Формула: 
/// D_gear = max(DF, DH)
/// Где:
/// * `DF` — доля повреждения от усталости при изгибе
/// * `DH` — доля повреждения от усталости при контакте
pub struct ToothDamage<Child> {
    child: Child,
    dbg: Dbg,
}
//
//
impl<Child> ToothDamage<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// Новый экземпляр [ToothDamage]
    pub fn new(
        parent: &Dbg, 
        child: Child
    ) -> Self {
        let dbg = Dbg::new(parent, "ToothDamage");
        Self {
            child,
            dbg,
        }
    }
}
impl<Child> Eval<Context, Context> for ToothDamage<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    fn eval(&self, ctx: Context) -> Context {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }        
        ctx.tooth_damage = ctx.bending_fatigue_damage.max(ctx.contact_fatigue_damage);
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
