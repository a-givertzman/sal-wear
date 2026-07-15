use sal_core::{dbg::Dbg, error::Error};
use crate::{BiquadCoeffs, Eval, domain::imbalance::context::ImbContext, me};

/// Фильтр нижних частот (Баттерворт 2-го порядка) для подавления ВЧ-шумов.
/// Пропускает частоты до заданного порядка (например, 10x от текущих оборотов).
pub struct LowPassSignal<Child> {
    /// Частота дискретизации в Гц.
    sample_rate: f64,
    /// Множитель частоты среза (например, 10.0 для 10-го порядка).
    cutoff_order: f64,
    child: Child,
    dbg: Dbg,
}
impl<Child> LowPassSignal<Child>
where
    Child: Eval<ImbContext, ImbContext> + Send + 'static {
    /// Константа 2 * PI
    const PI2: f64 = std::f64::consts::PI * 2.0;
    /// Создает новый экземпляр фильтра LowPassSignal.
    ///
    /// - `parent` - Идентификатор родительской сущности (для отладки).
    /// - `sample_rate_hz` - Частота дискретизации АЦП в Гц.
    /// - `cutoff_order` - Порядок отсечки (граничная частота в кратностях к оборотам вала).
    /// - `child` - Дочерний (предыдущий) расчетный шаг в конвейере.
    pub fn new(parent: &Dbg, sample_rate_hz: impl Into<f64>, cutoff_order: impl Into<f64>, child: Child) -> Self {
        let dbg = Dbg::new(parent, me::<Self>());
        Self {
            sample_rate: sample_rate_hz.into(),
            cutoff_order: cutoff_order.into(),
            child,
            dbg,
        }
    }
    /// Пересчитывает коэффициенты Баттерворта 2-го порядка при изменении скорости вала.
    fn update_coeffs(&self, mut ctx: ImbContext) -> ImbContext {
        let fc = (ctx.rpm / 60.0) * self.cutoff_order;
        let omega0 = Self::PI2 * fc / self.sample_rate;
        let alpha = omega0.sin() / (2.0 * std::f64::consts::FRAC_1_SQRT_2);
        let cos_w0 = omega0.cos();
        let a0 = 1.0 + alpha;
        ctx.low_pass_signal.coeffs = BiquadCoeffs {
            b0: ((1.0 - cos_w0) / 2.0) / a0,
            b1: (1.0 - cos_w0) / a0,
            b2: ((1.0 - cos_w0) / 2.0) / a0,
            a1: (-2.0 * cos_w0) / a0,
            a2: (1.0 - alpha) / a0,
        };
        ctx.low_pass_signal.last_rpm = ctx.rpm;
        ctx
    }
}
impl<Child> Eval<ImbContext, ImbContext> for LowPassSignal<Child>
where
    Child: Eval<ImbContext, ImbContext> + Send + 'static {
    //
    fn eval(&self, ctx: ImbContext) -> ImbContext {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        // Защита от деления на ноль при старте системы
        if ctx.rpm <= 0.1 {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Low RPM"));
            return ctx
        }
        // Пересчитываем математику фильтра, только если обороты изменились более чем на 1 RPM
        if (ctx.rpm - ctx.low_pass_signal.last_rpm).abs() > 1.0 {
            ctx = self.update_coeffs(ctx);
        }
        let coeffs = ctx.low_pass_signal.coeffs;
        let mut state = ctx.low_pass_signal.state;
        // Наполняем выборку
        for (sample, target) in ctx.frame.samples.iter().zip(ctx.samples.iter_mut()) {
            let x0 = *sample as f64;
            let y0 = coeffs.b0 * x0
                + coeffs.b1 * state.x1
                + coeffs.b2 * state.x2
                - coeffs.a1 * state.y1
                - coeffs.a2 * state.y2;
            state.x2 = state.x1;
            state.x1 = x0;
            state.y2 = state.y1;
            state.y1 = y0;
            // Кастим обратно с защитой от выхода за границы типа
            // Если ctx.samples имеет тип f32/f64, clamp и cast не нужны
            *target = y0 as f32; 
        }
        ctx.low_pass_signal.state = state;
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
