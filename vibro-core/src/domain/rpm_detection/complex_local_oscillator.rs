use std::f64::consts::PI;
use rustfft::num_complex::Complex;
use sal_core::dbg::Dbg;
use crate::{RpmDetectionCtx, Eval};
///
/// Комплексный гетеродин — снос 1X на нулевую частоту
pub struct ComplexLocalOscillator<Child> {
    child: Child,
    dbg: Dbg,
}
//
impl<Child> ComplexLocalOscillator<Child>
where
    Child: Eval<RpmDetectionCtx, RpmDetectionCtx> + Send + 'static,
{
    /// ### Returns `ComplexLocalOscillator` new instance
    /// - `parent` - Идентификатор родительской сущности (для отладки)
    /// - `child` - Дочерний (предыдущий) шаг вычислений
    /// - `rpm_net` - Грубая оценка оборотов вала, об/мин
    /// - `f_decimation` - Частота дискретизации входного потока, Гц
    pub fn new(parent: &Dbg, child: Child) -> Self {
        let dbg = Dbg::new(parent, crate::me::<Self>());
        Self {
            child,
            dbg,
        }
    }
}
//
impl<Child> Eval<RpmDetectionCtx, RpmDetectionCtx> for ComplexLocalOscillator<Child>
where
    Child: Eval<RpmDetectionCtx, RpmDetectionCtx> + Send + 'static,
{
    /// ### Сносит частоту 1X к нулю, обрабатывая децимированный сигнал сэмпл за сэмплом
    ///
    /// Для каждого входного отсчёта:
    /// 1. Продвигает накопленную фазу гетеродина `φ_het` на шаг,
    ///    соответствующий текущей грубой частоте `rpm_net` (с нормализацией `mod 2π`).
    /// 2. Строит опорную комплексную экспоненту `e^(jφ_het)` на этот момент.
    /// 3. Умножает входной (вещественный) отсчёт на эту экспоненту,
    ///    получая комплексный результат со сдвинутым спектром.
    ///
    /// `φ_het` сохраняется в `ctx` между вызовами — фаза накапливается
    /// непрерывно на протяжении всей работы системы, не только в пределах
    /// одного вызова `eval`.
    #[inline]
    fn eval(&self, ctx: RpmDetectionCtx) -> RpmDetectionCtx {
        // Передаем контекст дальше по цепочке вниз
        let mut ctx = self.child.eval(ctx);
        if ctx.is_err() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        // Первый вызов — инициализация буфера и точки отсчёта фазы
        if ctx.complex_local_oscillator.complex_signal.capacity() == 0 {
            ctx.complex_local_oscillator.complex_signal = Vec::with_capacity(ctx.decimation.decimated.len());
            ctx.complex_local_oscillator.phi_net = 0.0;
        }
        ctx.complex_local_oscillator.complex_signal.clear();
        let f_het = ctx.raw_rpm / 60.0; // об/мин → Гц
        let f_decimation = ctx.decimation.f_decimation;
        let f_coeff = f_het / f_decimation;
        let delta_phi = 2.0 * PI * f_coeff;
        // Извлекаем фазу во внутреннюю переменную, чтобы не перегружать обращения к ctx в цикле
        let mut phi = ctx.complex_local_oscillator.phi_net;
        for &x in ctx.decimation.decimated.iter() {
            // 1. Строим экспоненту на основе ТЕКУЩЕЙ фазы φ[n]
            let euler_phi_net = Complex::new(phi.cos(), -phi.sin());
            // 2. Умножаем вещественное число напрямую на комплексное
            let oscillated_value = euler_phi_net * (x as f64);
            ctx.complex_local_oscillator.complex_signal.push(oscillated_value);
            // 3. Продвигаем фазу для СЛЕДУЮЩЕГО шага: φ[n+1] = (φ[n] + Δφ) mod 2π
            phi = (phi + delta_phi) % (2.0 * PI);
        }
        // Сохраняем накопленную фазу обратно в контекст для следующего вызова eval
        ctx.complex_local_oscillator.phi_net = phi;
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}