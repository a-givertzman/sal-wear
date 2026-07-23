use chrono::Utc;
use sal_core::dbg::Dbg;
use crate::{Eval, ImbContext, Phase};

/// ### Выявление макро-механических дефектов на низких кратностях частоты вращения (0.5x..3x RPM).
///
/// #### Назначение
/// Выявление усточивого изменения гармоник углового домена
///
/// #### Различаемые дефекты согласно ISO 20816-1
/// * **0.5X, 1.5X, 2.5x RPM (Механические ослабления / люфты опор)** 
/// * **1X RPM (Статический/динамический дисбаланс):** Рост амплитуды строго на первом порядке.
/// * **2X RPM (Несоосность валов / расцентровка муфт):** Доминирование второго порядка, сопровождаемое осевой вибрацией.
/// * **3X RPM (Механические ослабления / люфты опор):** Появление третьей гармоники и субгармоник (0.5X, 1.5X).
/// 
/// [Подробнее о выявлении дефектов](../../../design/imbalance-detector.md)
pub struct ImbalanceDetector<Child> {
    /// Шаг угловой сетки в радианах.
    angular_step_rad: f64,
    /// Предыдущий узел конвейера вычислений (например, спектральный анализ).
    child: Child,
    dbg: Dbg,
}
impl<Child> ImbalanceDetector<Child>
where
    Child: Eval<ImbContext, ImbContext> + Send + 'static {
    ///
    /// ### Returns `ImbalanceDetector` new instance
    /// - `parent` - Идентификатор родительской сущности (для отладки).
    /// - `angular_step_rad` - Шаг угловой сетки в радианах.
    /// - `child` - Дочерний (предыдущий) расчетный шаг
    pub fn new(parent: impl Into<String>, angular_step_rad: f64, child: Child) -> Self {
        let dbg = Dbg::new(parent, crate::me::<Self>());
        Self {
            angular_step_rad,
            child,
            dbg,
        }
    }
}
impl<Child> Eval<ImbContext, ImbContext> for ImbalanceDetector<Child>
where
    Child: Eval<ImbContext, ImbContext> + Send + 'static {
    //
    #[inline]
    fn eval(&self, ctx: ImbContext) -> ImbContext {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        // Вычисляем точный угол начала БПФ-окна
        let start_phase = ctx.last_phase.to_radians() - (ctx.fft_window.len() as f64 - 1.0) * self.angular_step_rad;
        for filter in ctx.filters.iter() {
            if let Some(rms) = filter.eval(&ctx.fft_window) {
                let ix = filter.order_index();
                let fft_val = ctx.fft_window[ix];
                let local_phase = f64::atan2(fft_val.im as f64, fft_val.re as f64);
                let order = filter.target_order();
                let phase = Phase(local_phase - (order * start_phase)).normalize_signed();
                ctx.results.push((
                    Utc::now(),
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
