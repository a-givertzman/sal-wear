use sal_core::{dbg::Dbg, error::Error};
use crate::{Eval, ImbContext, KalmanFilter, Retained, Sender, ShortSigma, me};

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
    /// Фильтры накопления изменений искомых гармоник
    filters: Vec<KalmanFilter>,
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
        let filters = [0.5, 1.0, 1.5, 2.0, 2.5, 3.0].map(|order| {
            // Идентификатор зоны для хранения в retain
            let zone_id = format!("{order}x");
            // Скорость старения процесса
            let q = 1e-7;
            let retained = todo!();
            KalmanFilter::new(&dbg, zone_id, q, 0.01, retained, retain, 
                ShortSigma::new(
                    10, retained.x_hat
                ),
            )
        }).into();
        Self {
            filters,
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
        if ctx.order_samples.len() > ctx.fft_buff.capacity() {
            ctx.err = Some(Error::new(&self.dbg, "eval")
                .err(format!("Размер входящей выборки ({}) превышает емкость FFT буфера ({})", ctx.order_samples.len(), ctx.fft_buff.capacity())));
            return ctx;
        }
        ctx.fft_buff.push_chunk(&ctx.order_samples[..]);
        if ctx.fft_buff.is_full() {
            ctx.fft_window.copy_from_slice(ctx.fft_buff.read_window());
            if let Some(window_fn) = &self.window_fn {
                if let Err(err) = window_fn.eval(&mut ctx.fft_window) {
                    log::warn!("{}.eval | {}", self.dbg, err);
                }
            }
            self.fft.process(&mut ctx.fft_window);
        }
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
