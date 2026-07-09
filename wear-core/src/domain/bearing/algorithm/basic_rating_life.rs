use sal_core::{dbg::Dbg, error::Error};
use crate::{
    Eval, 
    LIFE_EXPONENT_ROLLER, 
    domain::context::Context
};
///
/// Расчёт номинального ресурса подшипника [H]
pub struct BasicRatingLife<Child> {
    /// Динамическая грузоподъёмность подшипника
    cr: f64,
    child: Child,
    dbg: Dbg,
}
//
//
impl<Child> BasicRatingLife<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// Новый экземпляр [BasicRatingLife]
    /// * `cr` - динамическая грузоподъёмность подшипника
    pub fn new(
        cr: f64,
        parent: &Dbg, 
        child: Child
    ) -> Self {
        let dbg = Dbg::new(parent, "BasicRatingLife");
        Self {
            cr,  
            child,
            dbg,
        }
    }
}
impl<Child> Eval<Context, Context> for BasicRatingLife<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    fn eval(&self, ctx: Context) -> Context {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        if ctx.equivalent_load < 0.1 {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Bearing equivalent load is about zero"));
            return ctx;
        }
        ctx.basic_rating_life = (self.cr / ctx.equivalent_load).powf(LIFE_EXPONENT_ROLLER);
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
