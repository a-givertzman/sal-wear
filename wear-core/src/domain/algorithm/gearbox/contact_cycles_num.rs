use sal_core::{
    dbg::Dbg, 
    error::Error
};
use crate::{
    Eval, 
    domain::context::Context
};
///
/// Расчёт допустимого числа циклов (S–N) для контакта
/// См. [раздел 8.3, шаг 6](../../Operion_Diag_Вибродиагностика_и_остаточный_ресурс.pdf)
/// Формула: 
/// NH = NH0 * (σH_lim / σH)^mH
/// Где:
/// * `mH` — показатели степени S–N кривой для контакта 
/// * `NH0` — базовое число циклов при напряжении σ_lim для контакта
/// * `σH_lim` — предел выносливости по контакту [Па]
/// * `σH` — значения напряжения на контакте

pub struct ContactCyclesNum<Child> {
    child: Child,
    dbg: Dbg,
}
//
//
impl<Child> ContactCyclesNum<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// Новый экземпляр [ContactCyclesNum]
    pub fn new(
        parent: &Dbg, 
        child: Child
    ) -> Self {
        let dbg = Dbg::new(parent, "ContactCyclesNum");
        Self {
            child,
            dbg,
        }
    }
}
impl<Child> Eval<Context, Context> for ContactCyclesNum<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    fn eval(&self, ctx: Context) -> Context {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }        
        let Some(m_h) = &ctx.m_h else {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("S–N curve exponent for contact isn't initialized"));
            return ctx;
        };
        if *m_h < 0.1 {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("S–N curve exponent for contact is about zero"));
            return ctx;
        }
        let Some(h_lim) = &ctx.h_lim else {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Nominal Contact stress limit isn't initialized"));
            return ctx;
        };
        if *h_lim < 0.1 {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Nominal Contact stress limit load is about zero"));
            return ctx;
        }
        let Some(nh_0) = &ctx.nh_0 else {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Base number of load cycles for contact isn't initialized"));
            return ctx;
        };
        if *nh_0 < 0.1 {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Base number of load cycles for contact load is about zero"));
            return ctx;
        }
        ctx.contact_num_cycles = *nh_0 * (*h_lim / ctx.contact_stresses).powf(*m_h);
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
