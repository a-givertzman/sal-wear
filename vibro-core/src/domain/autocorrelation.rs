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
    /// 2 * PI
    const PI2: f64 = std::f64::consts::PI * 2.0;
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
    /// Вычисляет точный период вращения (в отсчетах) через автокорреляцию.
    /// Ищет максимум функции в узком окне от показаний тахометра.
    #[inline]
    fn find_exact_period(samples: &[u16], raw_period: f64) -> f64 {
        let margin = (raw_period * 0.1) as usize;
        let center = raw_period as usize;
        let start = center.saturating_sub(margin).max(1);
        let end = (center + margin).min(samples.len() / 2);
        let mut max_corr = 0.0;
        let mut best_lag = center;
        for lag in start..=end {
            let mut corr = 0.0;
            for i in 0..(samples.len() - lag) {
                corr += samples[i] * samples[i + lag];
            }
            if corr > max_corr {
                max_corr = corr;
                best_lag = lag;
            }
        }
        best_lag as f64
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
