use std::todo;

use sal_core::{
    dbg::Dbg, 
    error::Error
};
use crate::{
    Eval, 
    domain::context::Context
};
///
/// Расчёт напряжения изгиба
/// См. [раздел 8.3, шаг 5](../../Operion_Diag_Вибродиагностика_и_остаточный_ресурс.pdf)
/// Формула: 
/// σF_i = KF * (Ft / (b * m)) * YF
/// Где:
/// * `KF` — коэффициент нагрузки
/// * `Ft` — окружная сила
/// * `b` — ширина зубчатого венца [м]
/// * `m` — модуль зубчатого колеса [м]
/// * `YF` — коэффициент нагрузки
pub struct BendingStresses<Child> {
    child: Child,
    dbg: Dbg,
}
//
//
impl<Child> BendingStresses<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// Новый экземпляр [BendingStresses]
    pub fn new(
        parent: &Dbg, 
        child: Child
    ) -> Self {
        let dbg = Dbg::new(parent, "BendingStresses");
        Self {
            child,
            dbg,
        }
    }
}
impl<Child> Eval<Context, Context> for BendingStresses<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    fn eval(&self, ctx: Context) -> Context {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }        
        let Some(z_p) = &ctx.z_p else {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Bending load facto isn't initialized"));
            return ctx;
        };
        if *z_p < 1 {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Bending load facto load is about zero"));
            return ctx;
        }
        let Some(kf) = &ctx.kf else {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Bending load facto isn't initialized"));
            return ctx;
        };
        if *kf < 0.1 {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Bending load facto load is about zero"));
            return ctx;
        }
        ctx.gear_mesh_frequency = todo!();
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
