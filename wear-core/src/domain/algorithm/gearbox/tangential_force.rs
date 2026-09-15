use sal_core::{
    dbg::Dbg, 
    error::Error
};
use crate::{
    Eval, 
    domain::context::Context
};
///
/// Расчёт окружной силы
/// См. [раздел 8.3, шаг 4](../../Operion_Diag_Вибродиагностика_и_остаточный_ресурс.pdf)
/// Формула: 
/// Ft = 2 * M * d_p [Н]
/// Где:
/// * `M` — крутящий момент на валу редуктора [Н·м]
/// * `d_p` — делительный диаметр шестерни [м]
pub struct TangentialForce<Child> {
    child: Child,
    dbg: Dbg,
}
//
//
impl<Child> TangentialForce<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// Новый экземпляр [TangentialForce]
    pub fn new(
        parent: &Dbg, 
        child: Child
    ) -> Self {
        let dbg = Dbg::new(parent, "TangentialForce");
        Self {
            child,
            dbg,
        }
    }
}
impl<Child> Eval<Context, Context> for TangentialForce<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    fn eval(&self, ctx: Context) -> Context {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }        
        let Some(d_p) = &ctx.d_p else {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Pitch diameter of gear isn't initialized"));
            return ctx;
        };
        if *d_p < 0.1 {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Pitch diameter of gear is about zero"));
            return ctx;
        }
        ctx.number_mesh_cycles = 2.0 *ctx.motor_torque / d_p;
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
