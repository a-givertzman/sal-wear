use chrono::Utc;
use sal_core::{dbg::Dbg, error::Error};
use crate::{Eval, ImbContext, KalmanFilter, Phase, Retained, Sender, ShortSigma, me};

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
    /// - `child` - Дочерний (предыдущий) расчетный шаг
    pub fn new(parent: impl Into<String>, retain: Sender<(String, Retained)>, child: Child) -> Self {
        let dbg = Dbg::new(parent, me::<Self>());
        Self {
            child,
            dbg,
        }
    }
    /// ### Расчет количества бинов зоны интереса для ImbalanceDetector.
    /// 
    /// Чтобы сделать систему гибкой, в конфигурацию необходимо ввести параметр `W_order` полуширины захвата порядка.
    /// - Для жестких условий (стабильные обороты) берут ±0.02 порядка.
    /// - Для плавающих режимов (пуски/выбеги) ширину увеличивают до ±0.05..±0.1 порядка.
    /// 
    /// #### Математика вычисления индексов:
    /// 
    /// Для целевого порядка Ok (например, `Ok = 1.0` или `Ok = 2.5` и настраиваемой полуширины `W_order` (например, 0.05):
    /// 
    /// - Центральный бин гармоники:
    /// 
    ///     `idx_center = Ok/ ΔO = (Ok x Nfft) / Nrev`
    /// 
    /// - Полуширина в количестве бинов (округляем вверх, чтобы гарантированно захватить края):
    /// 
    ///     `B_half = [ W_order / ΔO ] = [ (W_order x Nfft) / Nrev ]`
    /// 
    /// - Границы индексов в массиве FFT:
    /// 
    ///     `idx_start = idx_center - B_half`,
    ///     `idx_end = idx_center + B_half`.
    /// 
    /// - Общее количество бинов в зоне интегрирования всегда будет нечетным:
    /// 
    ///     `N_bins = 2 x B_half + 1`.
    pub fn target_order_bins(&self) -> usize {
        0
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
        if ctx.order_samples.len() > ctx.fft_buff.capacity() {
            ctx.err = Some(Error::new(&self.dbg, "eval")
                .err(format!("Размер входящей выборки ({}) превышает емкость FFT буфера ({})", ctx.order_samples.len(), ctx.fft_buff.capacity())));
            return ctx;
        }
        for filter in ctx.filters.iter() {
            if let Some(rms) = filter.eval(&ctx.fft_window) {
                let phase = Phase(todo!("Где взять фазу соответствующую данному результату"));
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
