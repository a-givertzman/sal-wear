use sal_core::{
    dbg::Dbg, 
    error::Error
};
use crate::{
    Eval, 
    domain::context::Context
};
///
/// Расчёт допустимого числа циклов (S–N) для изгиба
/// См. [раздел 8.3, шаг 6](../../Operion_Diag_Вибродиагностика_и_остаточный_ресурс.pdf)
/// Формула: 
/// NF = NF0 * (σF_lim / σF)^mF
/// Где:
/// * `mF` — показатели степени S–N кривой для изгиба 
/// * `NF0` — базовое число циклов при напряжении σ_lim для изгиба
/// * `σF_lim` — предел выносливости по изгибу [Па]
/// * `σF` — значения напряжения на изгибе

pub struct BendingCyclesNum<Child> {
    child: Child,
    dbg: Dbg,
}
//
//
impl<Child> BendingCyclesNum<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// Новый экземпляр [BendingCyclesNum]
    pub fn new(
        parent: &Dbg, 
        child: Child
    ) -> Self {
        let dbg = Dbg::new(parent, "BendingCyclesNum");
        Self {
            child,
            dbg,
        }
    }
}
impl<Child> Eval<Context, Context> for BendingCyclesNum<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    fn eval(&self, ctx: Context) -> Context {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }        
        let Some(m_f) = &ctx.m_f else {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("S–N curve exponent for bending isn't initialized"));
            return ctx;
        };
        if *m_f < 0.1 {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("S–N curve exponent for bending is about zero"));
            return ctx;
        }
        let Some(f_lim) = &ctx.f_lim else {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Nominal bending stress limit isn't initialized"));
            return ctx;
        };
        if *f_lim < 0.1 {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Nominal bending stress limit load is about zero"));
            return ctx;
        }
        let Some(nf_0) = &ctx.nf_0 else {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Base number of load cycles for contact isn't initialized"));
            return ctx;
        };
        if *nf_0 < 0.1 {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Base number of load cycles for contact load is about zero"));
            return ctx;
        }
        ctx.bending_num_cycles = *nf_0 * (*f_lim / ctx.bending_stresses).powf(*m_f);
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
