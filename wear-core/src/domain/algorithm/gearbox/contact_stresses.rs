use sal_core::{
    dbg::Dbg, 
    error::Error
};
use crate::{
    Eval, 
    domain::context::Context
};
///
/// Расчёт напряжения контакта
/// См. [раздел 8.3, шаг 5](../../Operion_Diag_Вибродиагностика_и_остаточный_ресурс.pdf)
/// Формула: 
/// σH_i = KH * ZH * sqrt(Ft_i / (b * d_p))
/// Где:
/// * `KH` — коэффициент нагрузки изгиба
/// * `ZH` — коэффициент геометрии контакта
/// * `Ft` — окружная сила
/// * `b` — ширина зубчатого венца [м]
/// * `d_p` — делительный диаметр шестерни [м]
pub struct ContactStresses<Child> {
    child: Child,
    dbg: Dbg,
}
//
//
impl<Child> ContactStresses<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// Новый экземпляр [ContactStresses]
    pub fn new(
        parent: &Dbg, 
        child: Child
    ) -> Self {
        let dbg = Dbg::new(parent, "ContactStresses");
        Self {
            child,
            dbg,
        }
    }
}
impl<Child> Eval<Context, Context> for ContactStresses<Child>
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
        let Some(d_p) = &ctx.d_p else {
            ctx.err = Some(Error::new(&self.dbg, "eval").err(" Module (normal or transverse module) isn't initialized"));
            return ctx;
        };
        if *d_p < 0.1 {
            ctx.err = Some(Error::new(&self.dbg, "eval").err(" Module (normal or transverse module) load is about zero"));
            return ctx;
        }
        let Some(kh) = &ctx.kh else {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Bending load factor isn't initialized"));
            return ctx;
        };
        if *kh < 0.1 {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Bending load factor load is about zero"));
            return ctx;
        }
        let Some(zh) = &ctx.zh else {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Bending stress shape factor isn't initialized"));
            return ctx;
        };
        if *zh < 0.1 {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Bending stress shape factor load is about zero"));
            return ctx;
        }
        ctx.contact_stresses = *kh * *zh * (ctx.tangential_force / (b * d_p)).powf(0.5);
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
