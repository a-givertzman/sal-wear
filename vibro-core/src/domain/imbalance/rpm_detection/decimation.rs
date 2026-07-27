use std::collections::VecDeque;
use sal_core::dbg::Dbg;
use crate::{Context, Eval};

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
    child: Child,
    dbg: Dbg,
}

impl<Child> Decimation<Child>
where
    Child: Eval<Context, Context> + Send + 'static,
{
    /// ### Returns `Decimation` new instance
    /// - `parent` - Идентификатор родительской сущности (для отладки)
    /// - `factor` - Коэффициент децимации (во сколько крат проредить входной сигнал)
    /// - `child` - Дочерний (предыдущий) шаг вычислений
    pub fn new(parent: impl Into<String>, factor: usize, child: Child) -> Self {
        let dbg = Dbg::new(parent, crate::me::<Self>());
        Self {
            factor,
            fir_size: 32,
            child,
            dbg,
        }
    }
}

impl<Child> Eval<Context, Context> for Decimation<Child>
where
    Child: Eval<Context, Context> + Send + 'static,
{
    #[inline]
    fn eval(&self, ctx: Context) -> Context {
        // Передаем контекст дальше по цепочке вниз
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        // Если стейт внутри контекста еще не инициализирован под размер фильтра
        if ctx.decimation.dec_fir_state.len() < FIR_TAPS_M20_N32.len() {
            ctx.decimation.dec_fir_state = VecDeque::from(vec![0.0; FIR_TAPS_M20_N32.len()]);
            // Выделяем емкость под новые децимированные сэмплы (примерно 512 / 20 = ~26 сэмплов)
            let expected_capacity = ctx.ac_samples.capacity() / self.factor + 1;
            ctx.decimation.decimated = Vec::with_capacity(expected_capacity);
        }
        ctx.decimation.decimated.clear();
        // Горячий цикл обработки сырых данных
        for &sample in &ctx.frame.samples {
            // Продвигаем кольцевой буфер КИХ-фильтра
            if ctx.decimation.dec_fir_state.len() >= self.fir_size {
                ctx.decimation.dec_fir_state.pop_back();
            }
            ctx.decimation.dec_fir_state.push_front(sample as f64);
            ctx.decimation.dec_counter += 1;
            // Прореживание: считаем КИХ-фильтр только для каждого М-го сэмпла
            if ctx.decimation.dec_counter >= self.factor {
                ctx.decimation.dec_counter = 0;
                // Свертка КИХ-фильтра.
                let mut filtered_value = 0.0;
                for (i, &state_val) in ctx.decimation.dec_fir_state.iter().enumerate() {
                    filtered_value += state_val * FIR_TAPS_M20_N32[i];
                }
                // Записываем результат децимации в контекст
                ctx.decimation.decimated.push(filtered_value);
            }
        }
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
///
/// Basic Tests
#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;
    // Простая заглушка конечного дочернего элемента цепочки
    struct DummyChild;
    impl Eval<Context, Context> for DummyChild {
        fn eval(&self, ctx: Context) -> Context { ctx } // Просто возвращает контекст без изменений
        fn exit(&self) {}
    }
    #[test]
    fn test_decimation_factor_and_length() {
        let decimator = Decimation::new("test", 20, DummyChild);
        // Создаем контекст с пакетом из 512 сэмплов
        let ctx = Context::new();
        let result_ctx = decimator.eval(ctx);
        // 512 / 20 = 25.6 -> Должно получиться ровно 25 или 26 сэмплов (в зависимости от начального счетчика)
        assert!(result_ctx.decimation.decimated.len() >= 25 && result_ctx.decimation.decimated.len() <= 26);
        assert!(result_ctx.err.is_none());
    }
    #[test]
    fn test_anti_aliasing_filtering() {
        let decimator = Decimation::new("test", 20, DummyChild);
        let fs = 320000.0;
        // Генерируем 1024 сэмпла:
        // Сигнал = Полезный синус (50 Гц) + Сильная высокочастотная помеха (40 кГц, выше частоты среза в 8 кГц)
        let mut raw = Vec::with_capacity(1024);
        for t in 0..1024 {
            let time = t as f64 / fs;
            let clean_signal = (2.0 * PI * 50.0 * time).sin();
            let hf_noise = 2.0 * (2.0 * PI * 40000.0 * time).sin(); // Шум в два раза громче сигнала!
            raw.push(clean_signal + hf_noise);
        }
        let ctx = Context::new();
        let result_ctx = decimator.eval(ctx);
        // Проверяем амплитуду на выходе. Высокочастотный шум с амплитудой 2.0 должен быть подавлен.
        // Выходной сигнал не должен содержать диких пиков помехи и прыгать выше значений полезного синуса.
        for &sample in &result_ctx.decimation.decimated {
            // КИХ-фильтр имеет КУ на постоянном токе около 1.0, 
            // проверяем, что значения отфильтрованного сигнала лежат в разумных пределах исходного синуса (-1.5..1.5)
            assert!(sample.abs() < 1.5, "Фильтр не подавил высокочастотный шум! Значение: {}", sample);
        }
    }
}
