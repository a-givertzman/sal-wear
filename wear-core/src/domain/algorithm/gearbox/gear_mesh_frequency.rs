use sal_core::{
    dbg::Dbg, 
    error::Error
};
use crate::{
    Eval, 
    domain::context::Context
};
///
/// Расчёт частоты вращения зубчатой передачи
/// См. [раздел 8.3, шаг 1](../../Operion_Diag_Вибродиагностика_и_остаточный_ресурс.pdf)
/// Формула: 
/// f_rot = rpm / 60 [Гц]
/// Где:
/// * `rpm` — скорость вращения вала в оборотах в минуту (об/мин) (обязательно больше нуля)
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
        let Some(motor_rpm) = &ctx.motor_rpm else {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Motor RPM isn't initialized"));
            return ctx;
        };
        if *motor_rpm < 0.1 {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Motor RPM load is about zero"));
            return ctx;
        }
        ctx.gear_mesh_frequency = motor_rpm / 60.0;
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
