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
/// * `KF` — коэффициент нагрузки изгиба
/// * `Ft` — окружная сила
/// * `b` — ширина зубчатого венца [м]
/// * `m` — модуль зубчатого колеса [м]
/// * `YF` — коэффициент геометрии формы зуба 

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
        let Some(b) = &ctx.b else {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Face width (of the gear) isn't initialized"));
            return ctx;
        };
        if *b < 0.1 {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Face width (of the gear) is about zero"));
            return ctx;
        }
        let Some(m) = &ctx.m else {
            ctx.err = Some(Error::new(&self.dbg, "eval").err(" Module (normal or transverse module) isn't initialized"));
            return ctx;
        };
        if *m < 0.1 {
            ctx.err = Some(Error::new(&self.dbg, "eval").err(" Module (normal or transverse module) load is about zero"));
            return ctx;
        }
        let Some(kf) = &ctx.kf else {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Bending load factor isn't initialized"));
            return ctx;
        };
        if *kf < 0.1 {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Bending load factor load is about zero"));
            return ctx;
        }
        let Some(yf) = &ctx.yf else {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Bending stress shape factor isn't initialized"));
            return ctx;
        };
        if *yf < 0.1 {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Bending stress shape factor load is about zero"));
            return ctx;
        }
        ctx.bending_stresses = *kf * (ctx.tangential_force / (b * m)) * yf;
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
