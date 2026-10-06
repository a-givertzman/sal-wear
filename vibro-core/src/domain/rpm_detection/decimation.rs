use sal_core::dbg::Dbg;
use crate::{AngularCtx, Eval, FIR_TAPS_M20_N32, err};
use function_name::named;
/// Адаптивный децимирующий фильтр (Anti-Aliasing Filter + Downsampler).
/// 
/// Выполняет две ключевые задачи цифровой обработки сигналов (DSP):
/// 1. Низкочастотную фильтрацию с помощью КИХ-фильтра (FIR) для подавления частот выше новой частоты 
///    Найквиста, что исключает появление эффекта наложения спектров (aliasing).
/// 2. Прореживание (downsampling) дискретного сигнала во временной области в `factor` раз.
pub struct Decimation<Child> {
    /// Размер линии задержки (истории сэмплов) для КИХ-фильтра. Строго соответствует количеству тапов.
    fir_size: usize,
    /// Дочерний вычислительный узел, вызываемый перед текущим шагом децимации.
    child: Child,
    /// Контекст отладки и логирования.
    dbg: Dbg,
}
//
impl<Child> Decimation<Child>
where
    Child: Eval<AngularCtx, AngularCtx> + Send + 'static,
{
    /// Создает новый экземпляр вычислительного узла `Decimation`.
    /// 
    /// Инициализирует контекст отладки, привязывая его к имени текущего типа, и жестко 
    /// задает длину линии задержки КИХ-фильтра (по умолчанию 32 тапа).
    ///
    /// # Аргументы
    /// * `parent` - Ссылка на отладочный контекст родительской сущности для построения дерева вызовов.
    /// * `child` - Следующий (или предыдущий по цепочке выполнения) вычислительный шаг.
    pub fn new(parent: &Dbg, child: Child) -> Self {
        let dbg = Dbg::new(parent, crate::me::<Self>());
        Self {
            fir_size: 32,
            child,
            dbg,
        }
    }
}
//
impl<Child> Eval<AngularCtx, AngularCtx> for Decimation<Child>
where
    Child: Eval<AngularCtx, AngularCtx> + Send + 'static,
{
    /// Выполняет фильтрацию и прореживание входного массива отсчетов.
    /// 
    /// Метод сначала продвигает контекст по цепочке вызовов, валидирует конфигурацию 
    /// дециматора и запускает потоковую обработку каждого отсчета из `ctx.samples`.
    #[inline]
    #[named]
    fn eval(&self, ctx: AngularCtx) -> AngularCtx {
        // Передаем контекст дальше по цепочке вниз для накопления сырых данных/сэмплов
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        // Защита от некорректной инициализации контекста (например, при создании через Default)
        if ctx.rpm_detection.decimation.factor == 0 {
            ctx.err = Some(err!(self.dbg, "factor не задан (контекст создан через default)"));
            return ctx;
        }
        // Очищаем выходной буфер текущего чанка перед заполнением новыми отсчетами
        ctx.rpm_detection.decimation.decimated.clear();
        // Горячий цикл потоковой обработки сырых данных (DSP Pipeline)
        for raw in ctx.samples.iter() {
            // Центрируем сигнал: вычитаем постоянную составляющую (DC offset) для 12-битного АЦП (2048 = Vref / 2).
            // Это переводит беззнаковый сигнал АЦП в знаковый диапазон для корректной фильтрации.
            let sample = *raw as f64 - 2048.0;
            // Продвигаем линию задержки (сдвиговый регистр КИХ-фильтра):
            // Если очередь заполнена до размера тапов, удаляем самый старый отсчет с конца.
            if ctx.rpm_detection.decimation.dec_fir_state.len() >= self.fir_size {
                ctx.rpm_detection.decimation.dec_fir_state.pop_back();
            }
            // Добавляем новый отсчет в начало очереди (индекс 0 — новейший сэмпл)
            ctx.rpm_detection.decimation.dec_fir_state.push_front(sample);
            // Инкрементируем фазовый счетчик прореживания
            ctx.rpm_detection.decimation.dec_counter += 1;
            // Условие прореживания: КИХ-фильтр рассчитывается и выдает отсчет строго один раз на каждые M (factor) входных сэмплов
            if ctx.rpm_detection.decimation.dec_counter >= ctx.rpm_detection.decimation.factor {
                // Сбрасываем счетчик для отслеживания следующей группы отсчетов
                ctx.rpm_detection.decimation.dec_counter = 0;
                // Математическая свертка (convolution) КИХ-фильтра: y[n] = Σ (x[n-i] * h[i])
                let mut filtered_value = 0.0;
                for (i, &state_val) in ctx.rpm_detection.decimation.dec_fir_state.iter().enumerate() {
                    // Перемножаем элементы линии задержки на соответствующие коэффициенты фильтра Хемминга
                    filtered_value += state_val * FIR_TAPS_M20_N32[i];
                }
                // Записываем отфильтрованное и прореженное значение в выходной вектор контекста
                ctx.rpm_detection.decimation.decimated.push(filtered_value);
            }
        }
        ctx
    }
    /// Корректно завершает работу текущего узла и каскадно вызывает `exit` для дочерних элементов цепочки.
    fn exit(&self) {
        self.child.exit();
    }
}
