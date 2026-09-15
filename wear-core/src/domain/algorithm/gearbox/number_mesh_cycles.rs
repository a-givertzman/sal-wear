use sal_core::{
    dbg::Dbg, 
    error::Error
};
use crate::{
    Eval, 
    domain::context::Context
};
///
/// Расчёт числа циклов зацепления
/// См. [раздел 8.3, шаг 3](../../Operion_Diag_Вибродиагностика_и_остаточный_ресурс.pdf)
/// Формула: 
/// n_mesh = f_GMF * duration [Гц]
/// Где:
/// * `f_GMF` — частота зацепления [Гц]
/// * `duration` — длительность данного устойчивого режима [с] (обязательно больше нуля)
pub struct NumberOfMeshCycles<Child> {
    child: Child,
    dbg: Dbg,
}
//
//
impl<Child> NumberOfMeshCycles<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// Новый экземпляр [NumberOfMeshCycles]
    pub fn new(
        parent: &Dbg, 
        child: Child
    ) -> Self {
        let dbg = Dbg::new(parent, "NumberOfMeshCycles");
        Self {
            child,
            dbg,
        }
    }
}
impl<Child> Eval<Context, Context> for NumberOfMeshCycles<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    fn eval(&self, ctx: Context) -> Context {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }        
        if ctx.duration < 0.1 {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Duration is about zero"));
            return ctx;
        }
        ctx.number_mesh_cycles = ctx.gear_mesh_frequency * ctx.duration;
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
