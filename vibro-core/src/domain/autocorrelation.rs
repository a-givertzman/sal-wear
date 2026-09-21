use std::f64::consts::TAU;

use sal_core::dbg::Dbg;
use crate::{AngularCtx, Eval, me};

/// Вычисляет точный период вращения (в отсчетах) через автокорреляцию.
/// Ищет максимум функции в узком окне от показаний тахометра.
pub struct Autocorrelation<Child> {
    /// Частота дискретизации в Гц.
    sample_rate: f64,
    child: Child,
    dbg: Dbg,
}
impl<Child> Autocorrelation<Child>
where
    Child: Eval<AngularCtx, AngularCtx> {
    ///
    /// ### Returns `Autocorrelation` new instance
    /// - `parent` - Идентификатор родительской сущности (для отладки).
    /// - `sample_rate` - Частота дискретизации в Гц.
    /// - `child` - Дочерний (предыдущий) расчетный шаг
    pub fn new(parent: &Dbg, sample_rate_hz: impl Into<f64>, child: Child) -> Self {
        let dbg = Dbg::new(parent, me::<Self>());
        Self {
            sample_rate: sample_rate_hz.into(),
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
                corr += samples[i] as f64 * samples[i + lag] as f64;
            }
            if corr > max_corr {
                max_corr = corr;
                best_lag = lag;
            }
        }
        best_lag as f64
    }
}
impl<Child> Eval<AngularCtx, AngularCtx> for Autocorrelation<Child>
where
    Child: Eval<AngularCtx, AngularCtx> {
    //
    #[inline]
    fn eval(&self, ctx: AngularCtx) -> AngularCtx {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        ctx.raw_period = self.sample_rate * 60.0 / ctx.raw_rpm;
        if let Some(window) = ctx.ac_samples.pop_window() {
            ctx.period = Self::find_exact_period(window, ctx.raw_period);
        } else {
            // До накопления окна работаем от грубой оценки тахометра
            ctx.period = ctx.raw_period;
        }
        ctx.omega = TAU * self.sample_rate / ctx.period;
        ctx.dt = 1.0 / self.sample_rate;
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
