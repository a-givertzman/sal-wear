use sal_core::dbg::Dbg;
use crate::{Context, Eval, me};


/// Вычисляет точный период вращения (в отсчетах) через автокорреляцию.
/// Ищет максимум функции в узком окне от показаний тахометра.
pub struct Autocorrelation<Child> {
    sample_rate: f64,
    child: Child,
    dbg: Dbg,
}
impl<Child> Autocorrelation<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// ### Returns `Autocorrelation` new instance
    pub fn new(parent: &Dbg, sample_rate: impl Into<f64>, child: Child) -> Self {
        let dbg = Dbg::new(parent, me::<Self>());
        Self {
            sample_rate: sample_rate.into(),
            child,
            dbg,
        }
    }
}
impl<Child> Eval<Context, Context> for Autocorrelation<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    //
    fn eval(&self, ctx: Context) -> Context {
        let mut ctx = self.child.eval(ctx);
        match &ctx.err {
            Some(_) => ctx.pass_err(&self.dbg, "eval"),
            None => {
                ctx.raw_period = self.sample_rate * 60.0 / ctx.raw_rpm;
                ctx.period = Self::find_exact_period(ctx.samples.as_slice(), ctx.raw_period);
                ctx.omega = Self::PI2 * self.sample_rate / ctx.period;
                ctx.dt = 1.0 / self.sample_rate;
                ctx
            }
        }
    }
    //
    fn exit(&self) {
        todo!()
    }
}
