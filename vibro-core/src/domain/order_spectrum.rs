use std::sync::Arc;

use rustfft::{Fft, FftPlanner};
use sal_core::{dbg::Dbg, error::Error};
use crate::{ImbContext, Eval, me};

/// Вычисляет точный период вращения (в отсчетах) через автокорреляцию.
/// Ищет максимум функции в узком окне от показаний тахометра.
/// 
/// [Подробнее об Order Spectrum](../../../design/order-spectrum.md)
pub struct OrderSpectrum<Child> {
    /// Частота дискретизации в Гц.
    sample_rate: f64,
    child: Child,
    fft: Arc<dyn Fft<f32>>,
    dbg: Dbg,
}
impl<Child> OrderSpectrum<Child>
where
    Child: Eval<ImbContext, ImbContext> + Send + 'static {
    ///
    /// ### Returns `OrderSpectrum` new instance
    /// - `parent` - Идентификатор родительской сущности (для отладки).
    /// - `sample_rate` - Частота дискретизации в Гц.
    /// - `child` - Дочерний (предыдущий) расчетный шаг
    pub fn new(parent: &Dbg, sample_rate_hz: impl Into<f64>, child: Child) -> Self {
        let dbg = Dbg::new(parent, me::<Self>());
        let fft_size = Self::fft_buffer_size();
        log::debug!("{dbg}.new | fft_size: {fft_size}");
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(fft_size);
        Self {
            sample_rate: sample_rate_hz.into(),
            child,
            fft,
            dbg,
        }
    }
    /// ### Возвращает размер буфера FFT требуемого для `OrderSpectrum`
    /// 
    /// #### Логика выбора размера
    /// Для диагностики подшипника в низкочастотном диапазоне (1X..3X) нужно видеть спектр как минимум до 3..4 порядка вращения вала.
    /// По теореме Найквиста нужно 2 точки на самом высоком порядке (3X порядок * 2). Но на практике берут запас в 2.5–3 раза,
    /// чтобы угловой антиалиасинговый фильтр работал корректно.
    /// Таким образом Nrev = 3X * 2.56 = 7.68, округляем до ближайшей степени двойки, получаем Nrev = 8 точек на оборот.
    /// Размер FFT буфера определяет разрешение в долях порядка `resolution = Nrev / Nfft`.
    /// Или, если выразить через количество оборотов вала `K`, которые нужно накопить в буфер для одного FFT (Nfft = K * Nrev):
    /// `resolution = 1 / K`
    /// Если с большим запасом Nrev = 64 точек на оборот.
    /// Что бы получить разрешение `resolution = 0.1` порядка (чтобы отличать, например, 4.2 порядок от 4.3 порядка)
    /// нам нужно накопить в буфер 10 полных оборотов вала (`K = 10`).
    /// Тогда разбер FFT буфера Nfft = Nrev / resolution = 64 / 0.1 = 640, ближайшая большая степень двойки 1024
    pub fn fft_buffer_size() -> usize {
        let points_per_turn = 64;
        let resolution = 0.1;
        ((points_per_turn as f64 / resolution).round() as usize).next_power_of_two()
    }

}
impl<Child> Eval<ImbContext, ImbContext> for OrderSpectrum<Child>
where
    Child: Eval<ImbContext, ImbContext> + Send + 'static {
    //
    #[inline]
    fn eval(&self, ctx: ImbContext) -> ImbContext {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        if ctx.order_samples.len() <= ctx.fft_buff.capacity() {
            ctx.err = Some(Error::new(&self.dbg, "eval")
                .err(format!("Размер выборки ctx.order_samples ({}) превышает размер FFT буфера ctx.fft_buff ({})", ctx.order_samples.len(), ctx.fft_buff.capacity())));
            return ctx;
        }
        ctx.fft_buff.push_chunk(&ctx.order_samples[..]);
        if ctx.fft_buff.is_full() {
            ctx.fft_window.copy_from_slice(ctx.fft_buff.read_window());
        }
        self.fft.process(&mut ctx.fft_window);
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
