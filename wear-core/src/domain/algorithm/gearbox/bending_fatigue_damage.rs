use sal_core::{
    dbg::Dbg, 
};
use crate::{
    Eval, 
    domain::context::Context
};
///
/// Расчёт доли повреждения от усталости при изгибе
/// См. [раздел 8.3, шаг 7](../../Operion_Diag_Вибродиагностика_и_остаточный_ресурс.pdf)
/// Формула: 
/// DF = n_mesh / NF
/// Где:
/// * `n_mesh` — число циклов зацепления
/// * `NF` — допустимое число циклов (S–N) для изгиба
pub struct BendingFatigueDamage<Child> {
    child: Child,
    dbg: Dbg,
}
//
//
impl<Child> BendingFatigueDamage<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// Новый экземпляр [BendingFatigueDamage]
    pub fn new(
        parent: &Dbg, 
        child: Child
    ) -> Self {
        let dbg = Dbg::new(parent, "BendingFatigueDamage");
        Self {
            child,
            dbg,
        }
    }
}
impl<Child> Eval<Context, Context> for BendingFatigueDamage<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    fn eval(&self, ctx: Context) -> Context {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }        
        ctx.bending_fatigue_damage = ctx.number_mesh_cycles / ctx.bending_num_cycles;
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
