use sal_core::{
    dbg::Dbg, 
    error::Error
};
use crate::{
    Eval, 
    domain::context::Context
};
///
/// Расчёт частоты зацепления
/// См. [раздел 8.3, шаг 2](../../Operion_Diag_Вибродиагностика_и_остаточный_ресурс.pdf)
/// Формула: 
/// f_GMF = z_p / f_rot [Гц]
/// Где:
/// * `z_p` — число зубьев ведущей шестерни
/// * `f_rot` — частота вращения зубчатой передачи
pub struct GearMeshFrequency<Child> {
    child: Child,
    dbg: Dbg,
}
//
//
impl<Child> GearMeshFrequency<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// Новый экземпляр [GearMeshFrequency]
    pub fn new(
        parent: &Dbg, 
        child: Child
    ) -> Self {
        let dbg = Dbg::new(parent, "GearMeshFrequency");
        Self {
            child,
            dbg,
        }
    }
}
impl<Child> Eval<Context, Context> for GearMeshFrequency<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    fn eval(&self, ctx: Context) -> Context {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }        
        let Some(z_p) = &ctx.z_p else {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Count pinion isn't initialized"));
            return ctx;
        };
        if *z_p < 1 {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Count pinion load is about zero"));
            return ctx;
        }
        ctx.gear_mesh_frequency = *z_p as f64 * ctx.rotational_frequency;
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
