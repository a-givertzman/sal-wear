use std::f64::consts::{PI, TAU};
use rustfft::num_complex::Complex;
use sal_core::dbg::Dbg;
use function_name::named;
use crate::{AngularCtx, Eval, err};
///
/// Комплексный гетеродин (снос 1X на нулевую частоту)
/// Входные данные: Прореженный сигнал на частоте 16 кГц и грубое значение частоты f_hat = RPM_net / 60.
/// Математическая суть: Вместо перестраиваемого полосового фильтра, центрируемого по сетевому RPM, 
/// выполняется комплексное гетеродинирование — умножение сигнала на комплексную экспоненту с накопленной фазой:
///     
/// y[n] = x[n] * e^(-j * phi_het[n])
/// phi_het[n+1] = (phi_het[n] + 2 * pi * f_hat / F_sample_dec) mod (2 * pi)
/// 
/// Компонента 1X оказывается на частоте delta = f_1X - f_hat, близкой к нулю. Все прочие компоненты 
/// (шум редуктора, гармоники 2X, 3X, высокочастотные дефекты) уходят на частоты f_hat и выше — 
/// за пределы полосы слежения следующего шага.
/// 
/// Критические требования реализации:
///   1. Фаза гетеродина phi_het — НАКАПЛИВАЕМАЯ величина (интеграл скорости), а не произведение f_hat * t. 
///      При смене сетевого RPM меняется только скорость накопления, фаза вала остается непрерывной. 
///      Реализация через f_hat * t даст скачок фазы 2 * pi * delta_f * t, растущий со временем работы, — запрещено.
///   2. phi_het нормируется по модулю 2 * pi на каждом сэмпле — аргументы sin/cos никогда не выходят за 2 * pi, 
///      численный дрейф исключен.
///   3. Сетевое значение f_hat применяется с учетом возраста (штампа времени получения); 
///      правило устаревания — см. раздел 4, п. 3.
/// 
/// Зачем это нужно: Гетеродинирование устраняет саму причину адаптивности фильтра: полоса слежения становится 
/// фиксированной, пересчета коэффициентов «на лету» не существует, рассогласование состояния фильтра с полосой 
/// невозможно по построению. Ошибка сетевого RPM не накапливается в фазе вала: она проявляется только как 
/// положение delta внутри полосы ФНЧ и в точности компенсируется тождеством шага 3.4.
/// 
/// Выход данных: Комплексный сигнал y[n] = I[n] + jQ[n], в котором компонента 1X находится на частоте delta ≈ 0.
pub struct ComplexLocalOscillator<Child> {
    child: Child,
    dbg: Dbg,
}
//
impl<Child> ComplexLocalOscillator<Child>
where
    Child: Eval<AngularCtx, AngularCtx> + Send + 'static,
{
    /// ### Returns `ComplexLocalOscillator` new instance
    /// - `parent` - Идентификатор родительской сущности (для отладки)
    /// - `child` - Дочерний (предыдущий) шаг вычислений
    pub fn new(parent: &Dbg, child: Child) -> Self {
        let dbg = Dbg::new(parent, crate::me::<Self>());
        Self {
            child,
            dbg,
        }
    }
}
//
impl<Child> Eval<AngularCtx, AngularCtx> for ComplexLocalOscillator<Child>
where
    Child: Eval<AngularCtx, AngularCtx> + Send + 'static,
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
    #[named]
    fn eval(&self, ctx: AngularCtx) -> AngularCtx {
        // Передаем контекст дальше по цепочке вниз для предварительной обработки
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        // Очищаем буферы комплексного локального осциллятора перед новым циклом расчета
        ctx.rpm_detection.complex_local_oscillator.complex_signal.clear();
        ctx.rpm_detection.complex_local_oscillator.phi_net.clear();
        // Переводим грубую частоту вращения из об/мин в Гц (f_het)
        let f_het = ctx.raw_rpm / 60.0; 
        let f_sample_dec = ctx.rpm_detection.decimation.f_sample_dec;
        // Валидация входных физических параметров на корректность и бесконечность
        if !ctx.raw_rpm.is_finite() || !f_sample_dec.is_finite() || f_sample_dec <= 0.0 {
            ctx.err = Some(err!(self.dbg, "ctx.raw_rpm {} or f_sample_dec {f_sample_dec} is not a valid number", ctx.raw_rpm));
            return ctx;
        }
        // Расчет нормированного шага частоты и соответствующего приращения фазы на один сэмпл (delta_phi)
        let f_coeff = f_het / f_sample_dec;
        let delta_phi = 2.0 * PI * f_coeff;
        // Восстанавливаем сохраненное значение фазы с предыдущего шага/блока данных
        let mut phi = ctx.rpm_detection.complex_local_oscillator.phi_last;
        // Основной цикл гетеродинирования прореженного сигнала
        for &x in ctx.rpm_detection.decimation.decimated.iter() {
            // Сохраняем текущую накопленную фазу для истории/диагностики
            ctx.rpm_detection.complex_local_oscillator.phi_net.push(phi);
            // Формируем сопряженную комплексную экспоненту по формуле Эйлера: e^(-j*phi) = cos(phi) - j*sin(phi)
            let e = Complex::new(phi.cos(), -phi.sin());
            // Сдвигаем спектр вещественного отсчета x[n] на величину e^(-j*phi), приводя компоненту 1X к нулевой частоте
            ctx.rpm_detection.complex_local_oscillator.complex_signal.push(e * (x as f64));
            // Накапливаем фазу (интегрируем скорость). Изменение RPM влияет только на delta_phi, сохраняя непрерывность фазы вала.
            phi += delta_phi;
            // Жесткое ограничение аргумента по модулю [0, 2π) для предотвращения численного дрейфа и потери точности тригонометрии
            if phi >= TAU {
                phi -= TAU;
            } else if phi < 0.0 {
                phi += TAU;
            }
        }
        // Сохраняем конечную фазу текущего блока как начальную для следующего блока (обеспечение непрерывности)
        ctx.rpm_detection.complex_local_oscillator.phi_last = phi;
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}