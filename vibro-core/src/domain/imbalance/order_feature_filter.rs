use chrono::Utc;
use sal_core::dbg::Dbg;
use crate::{Eval, ImbContext, Order, Phase};

/// ### Фильтрует целевые кратности частот вращения (0.5x..3x RPM).
///
/// Выявление усточивого изменения гармоник углового домена
/// 
/// [Подробнее о выявлении дефектов](../../../design/order-feature-filter.md)
pub struct OrderFeatureFilter<Child> {
    /// Сдвиг угла между началом и концом выборок FFT в радианах.
    phase_offset: f64,
    /// Предыдущий узел конвейера вычислений (например, спектральный анализ).
    child: Child,
    dbg: Dbg,
}
impl<Child> OrderFeatureFilter<Child>
where
    Child: Eval<ImbContext, ImbContext> + Send + 'static {
    ///
    /// ### Returns `OrderFeatureFilter` new instance
    /// - `parent` - Идентификатор родительской сущности (для отладки).
    /// - `n_fft` - Размер FFT выборки спектрального анализа в угловой области.
    /// - `angular_step_rad` - Шаг угловой сетки в радианах.
    /// - `child` - Дочерний (предыдущий) расчетный шаг
    pub fn new(parent: impl Into<String>, n_fft: usize, angular_step_rad: f64, child: Child) -> Self {
        let dbg = Dbg::new(parent, crate::me::<Self>());
        Self {
            phase_offset: (n_fft as f64 - 1.0) * angular_step_rad,
            child,
            dbg,
        }
    }
    /// ### Вычисляем точный угол фазы начала БПФ-окна.
    #[inline]
    fn start_phase(&self, last_phase: Phase<f64>) -> f64 {
        last_phase.to_radians() - self.phase_offset
    }
}
impl<Child> Eval<ImbContext, ImbContext> for OrderFeatureFilter<Child>
where
    Child: Eval<ImbContext, ImbContext> + Send + 'static {
    //
    #[inline]
    fn eval(&self, ctx: ImbContext) -> ImbContext {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        let start_phase = self.start_phase(ctx.last_phase);
        for filter in ctx.filters.iter() {
            if let Some(rms) = filter.eval(&ctx.fft_window) {
                let ix = filter.order_index();
                let fft_val = ctx.fft_window[ix];
                let local_phase = f64::atan2(fft_val.im as f64, fft_val.re as f64);
                let order = filter.target_order();
                let phase = Phase(local_phase - (order.value() * start_phase)).normalize_signed();
                ctx.results.push(super::DiagResult::new(
                    Utc::now(),
                    filter.target_order(),
                    filter.order_id().to_string(),
                    rms,
                    phase,
                    ctx.rpm,
                ));
            }
        }
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
