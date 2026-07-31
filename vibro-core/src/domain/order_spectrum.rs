use std::sync::Arc;

use rustfft::{Fft, FftPlanner};
use sal_core::{dbg::Dbg, error::Error};
use crate::{Eval, ImbContext, WindowFn, me};

/// ### Спектральный анализ сигнала в угловом домене (Order Tracking) 
/// для низкочастотной диагностики роторного оборудования.
///
/// #### Назначение
/// Расчет спектра порядков (Order Spectrum) для автоматического выявления
/// макро-механических дефектов на низких кратностях оборотной частоты (1X, 2X, 3X RPM).
///
/// #### Различаемые дефекты согласно ISO 20816-1
/// * **1X RPM (Статический/динамический дисбаланс):** Рост амплитуды строго на первом порядке.
/// * **2X RPM (Несоосность валов / расцентровка муфт):** Доминирование второго порядка, сопровождаемое осевой вибрацией.
/// * **3X RPM (Механические ослабления / люфты опор):** Появление третьей гармоники и субгармоник (0.5X, 1.5X).
///
/// #### Математическое обоснование параметров
/// * **Количество точек на оборот ($N_{rev} = 64$):** Обеспечивает верхнюю границу измеряемых порядков 
///   по Найквисту $O_{max} = 64 / 2 = 32X$. С учетом переходной полосы цифрового антиалиасингового 
///   фильтра, гарантирует чистый спектр (без наложений) вплоть до $12X \dots 15X$ порядка.
/// * **Шаг разрешения ($\Delta O = 0.01$):** Требует накопления данных минимум за $K = 1 / 0.01 = 100$ 
///   полных оборотов вала. Это критически важно для селекции близких частот: разделения физического 
///   дисбаланса асинхронного двигателя ($1.0X \approx 49$ Гц) и сетевых электромагнитных наводок ($1.02X \approx 50$ Гц).
/// 
/// [Подробнее об Order Spectrum](../../../design/order-spectrum.md)
pub struct OrderSpectrum<Child> {
    /// Планировщик FFT.
    fft: Arc<dyn Fft<f32>>,
    /// Оконная функция
    window_fn: Option<WindowFn<f32>>,
    /// Предыдущий узел конвейера вычислений (например, угловой ресемплер или оконный фильтр).
    child: Child,
    dbg: Dbg,
}
impl<Child> OrderSpectrum<Child>
where
    Child: Eval<ImbContext, ImbContext> + Send + 'static {
    ///
    /// ### Returns `OrderSpectrum` new instance
    /// - `parent` - Идентификатор родительской сущности (для отладки).
    /// - `n_fft` - Размер буфера FFT (из конфига).
    /// - `window_fn` - Оконная функция для подготовки данных к FFT
    /// - `child` - Дочерний (предыдущий) расчетный шаг
    pub fn new(parent: &Dbg, n_fft: usize, window_fn: Option<WindowFn<f32>>, child: Child) -> Self {
        let dbg = Dbg::new(parent, me::<Self>());
        log::debug!("{dbg}.new | n_fft: {n_fft}");
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(n_fft);
        Self {
            fft,
            window_fn,
            child,
            dbg,
        }
    }
}
impl<Child> Eval<ImbContext, ImbContext> for OrderSpectrum<Child>
where
    Child: Eval<ImbContext, ImbContext> + Send + 'static {
    //
    #[inline]
    fn eval(&self, ctx: ImbContext) -> ImbContext {
        let mut ctx = self.child.eval(ctx);
        if ctx.is_err() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        if ctx.order_samples.len() > ctx.fft_buff.capacity() {
            let length = ctx.order_samples.len();
            let capacity = ctx.fft_buff.capacity();
            return ctx.with_err(&self.dbg, "eval",
                format!("Размер входящей выборки ({}) превышает емкость FFT буфера ({})", length, capacity));
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
