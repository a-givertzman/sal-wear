use std::sync::Arc;

use sal_core::dbg::Dbg;

use crate::{Context, Eval, me};


pub struct Autocorrelation<Child> {
    child: Child,
    dbg: Dbg,
}
impl<Child> Autocorrelation<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// ### Returns `Autocorrelation` new instance
    pub fn new(parent: &Dbg, child: Child) -> Self {
        let dbg = Dbg::new(parent, me::<Self>());
        Self {
            child,
            dbg,
        }
    }
    /// Вычисляет точный период вращения (в отсчетах) через автокорреляцию.
    /// Ищет максимум функции в узком окне от показаний тахометра.
    #[inline]
    fn find_exact_period(samples: &Arc<[u16]>, raw_period: f32) -> f32 {
        let margin = (raw_period * 0.1) as usize;
        let center = raw_period as usize;
        let start = center.saturating_sub(margin).max(1);
        let end = (center + margin).min(samples.len() / 2);
        let mut max_corr = 0;
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
        best_lag as f32
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
                ctx.period = Self::find_exact_period(&ctx.samples, ctx.raw_period);
                ctx
            }
        }
    }
    //
    fn exit(&self) {
        todo!()
    }
}
