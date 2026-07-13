use sal_core::dbg::Dbg;
use crate::{Context, Eval, me};

/// Вычисляет точный период вращения (в отсчетах) через автокорреляцию.
/// Ищет максимум функции в узком окне от показаний тахометра.
pub struct OrderSpectrum<Child> {
    /// Частота дискретизации в Гц.
    sample_rate: f64,
    child: Child,
    dbg: Dbg,
}
impl<Child> OrderSpectrum<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    /// 2 * PI
    const PI2: f64 = std::f64::consts::PI * 2.0;
    ///
    /// ### Returns `OrderSpectrum` new instance
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
}
impl<Child> Eval<Context, Context> for OrderSpectrum<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    //
    fn eval(&self, ctx: Context) -> Context {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        // Do calculations, write to ctx
        // Write error if calculation faled
        // ctx.err = Some(Error::new(&self.dbg, "eval").err("Error message"))
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
