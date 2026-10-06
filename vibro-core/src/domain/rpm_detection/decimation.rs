use std::collections::VecDeque;
use sal_core::dbg::Dbg;
use crate::{AngularCtx, Eval};

/// Коэффициенты КИХ-фильтра нижних частот (ФНЧ)
/// Расчитаны для Fs=320kHz, Fc=8kHz (спад к 16kHz), 32 тапа, окно Хемминга.
const FIR_TAPS_M20_N32: [f64; 32] = [
    -0.000713, -0.001646, -0.002497, -0.002570, -0.000958,  0.003180,  0.010260,  0.020141,
     0.032213,  0.045431,  0.058371,  0.069485,  0.077366,  0.081125,  0.081125,  0.077366,
     0.069485,  0.058371,  0.045431,  0.032213,  0.020141,  0.010260,  0.003180, -0.000958,
    -0.002570, -0.002497, -0.001646, -0.000713,  0.000000,  0.000000,  0.000000,  0.000000,
];

/// Адаптивный децимирующий фильтр (Anti-Aliasing)
pub struct Decimation<Child> {
    /// Коэффициент децимации (во сколько крат проредить входной сигнал)
    factor: usize,
    /// Размер истории сэмплов для КИХ-фильтра (длина 32 .. 64)
    fir_size: usize,
    /// Частота дискретизации в Гц.
    sample_rate: f64,
    child: Child,
    dbg: Dbg,
}

impl<Child> Decimation<Child>
where
    Child: Eval<AngularCtx, AngularCtx> + Send + 'static,
{
    /// ### Returns `Decimation` new instance
    /// - `parent` - Идентификатор родительской сущности (для отладки)
    /// - `factor` - Коэффициент децимации (во сколько крат проредить входной сигнал)
    /// - `child` - Дочерний (предыдущий) шаг вычислений
    pub fn new(parent: &Dbg, factor: usize, sample_rate: f64, child: Child) -> Self { // VORZHEV Z.A.: `parent: impl Into<String>` CHANGED TO  `parent: &Dbg` cause of error
        let dbg = Dbg::new(parent, crate::me::<Self>());
        Self {
            factor,
            fir_size: 32,
            sample_rate,
            child,
            dbg,
        }
    }
}

impl<Child> Eval<AngularCtx, AngularCtx> for Decimation<Child>
where
    Child: Eval<AngularCtx, AngularCtx> + Send + 'static,
{
    #[inline]
    fn eval(&self, ctx: AngularCtx) -> AngularCtx {
        // Передаем контекст дальше по цепочке вниз
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        // Если стейт внутри контекста еще не инициализирован под размер фильтра
        if ctx.rpm_detection.decimation.dec_fir_state.len() < FIR_TAPS_M20_N32.len() {
            ctx.rpm_detection.decimation.dec_fir_state = VecDeque::from(vec![0.0; FIR_TAPS_M20_N32.len()]);
            // Выделяем емкость под новые децимированные сэмплы (примерно 512 / 20 = ~26 сэмплов)
            let expected_capacity = ctx.samples.capacity() / self.factor + 1;
            ctx.rpm_detection.decimation.decimated = Vec::with_capacity(expected_capacity);
        }
        ctx.rpm_detection.decimation.decimated.clear();
        ctx.rpm_detection.decimation.f_decimation = self.sample_rate / (self.factor as f64);
        // Горячий цикл обработки сырых данных
        for raw in ctx.samples.iter() {
            let sample = *raw as f64 - 2048.0;
            // Продвигаем кольцевой буфер КИХ-фильтра
            if ctx.rpm_detection.decimation.dec_fir_state.len() >= self.fir_size {
                ctx.rpm_detection.decimation.dec_fir_state.pop_back();
            }
            ctx.rpm_detection.decimation.dec_fir_state.push_front((sample).into());
            ctx.rpm_detection.decimation.dec_counter += 1;
            // Прореживание: считаем КИХ-фильтр только для каждого М-го сэмпла
            if ctx.rpm_detection.decimation.dec_counter >= self.factor {
                ctx.rpm_detection.decimation.dec_counter = 0;
                // Свертка КИХ-фильтра.
                let mut filtered_value = 0.0;
                for (i, &state_val) in ctx.rpm_detection.decimation.dec_fir_state.iter().enumerate() {
                    filtered_value += state_val * FIR_TAPS_M20_N32[i];
                }
                // Записываем результат децимации в контекст
                ctx.rpm_detection.decimation.decimated.push(filtered_value);
            }
        }
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
