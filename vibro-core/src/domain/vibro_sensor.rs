use std::{cell::Cell, sync::Arc};
use function_name::named;
use sal_core::{dbg::Dbg, error::Error};
use sal_sync::{kernel::state::ExitNotify, services::EventValueAccess};
use crate::{AngularGrid, Autocorrelation, AngularCtx, Eval, Frame, HighRangeCtx, ImbContext, ImbalanceDetector, LowPassSignal, MidRangeCtx, OrderDomainSamples, OrderFeatureFilter, OrderSpectrum, Pass, ReadEventValuess, Retain, Severity, SqlExport, WindowFn, err_pass, me};

type LowRange<SqlBuilder> = SqlExport<SqlBuilder, ImbContext, ImbalanceDetector<OrderFeatureFilter<OrderSpectrum<OrderDomainSamples<LowPassSignal<Pass>>>>>>;
type MidRange = Pass;
type HighRange = Pass;


/// ### VibroSensor | Расчетный вибродиагностики для одного датчика
/// 
/// Алгоритмы анализа и диагностики разделены на три частотных диапазона:
///  
/// #### 1. Диагностика низкочастотных макромеханических дефектов (диапазон 0.2x .. 3.0x RPM)
/// 
///    * Вычисляет текущую частоту вращения (RPM) и фазовый угол поворота ротора по сигналу вибродатчика.
///    * Выполняет синхронное временно́е накопление и преобразование исходного сигнала из временного домена в угловой домен.
///    * Производит спектральный анализ (БПФ) полученного сигнала в угловом домене.
///    * Осуществляет цифровую фильтрацию и сглаживание кратковременных выбросов амплитуд целевых гармоник (0.5x, 1.0x, 1.5x, 2.5x, 3.0x).
///    * Реализует логику автоматической классификации дефектов на основе паттернов и порогов целевых гармоник.
///    * Регистрирует тренды целевых гармоник и фиксирует результаты диагностики дефектов в БД.
/// 
/// #### 2. Диагностика развитых (проявленных) дефектов (среднечастотный диапазон 10x RPM .. 5 кГц: BPFI, BPFO, FTF, BSF)
/// 
///    * **Функционал в разработке (не реализован)**
///    * Выполняет полосовую фильтрацию (Bandpass) исходного вибросигнала в целевом диапазоне частот.
///    * Пересчитывает отфильтрованный сигнал из временной области в угловую на основе единой сетки углов поворота вала (Angular Grid).
///    * Вычисляет упорядоченный спектр (Order Spectrum) с заданным размером окна БПФ (FFT Size).
///    * Производит автоматический поиск дефектов подшипников (BPFI, BPFO, FTF, BSF) с помощью детектора (Defect Detector) по настроенным порогам.
///    * Формирует SQL-запросы на основе результатов детекции и экспортирует данные в БД через API-клиент.
/// 
/// 
/// #### 3. Диагностика дефектов на ранней стадии зарождения (высокочастотный диапазон 5 кГц .. 15 кГц: BPFI, BPFO, FTF, BSF)
///    * **Функционал в разработке (не реализован)**
///    * Выполняет полосовую фильтрацию (Bandpass) высокочастотного вибросигнала в диапазоне 5–10 кГц для изоляции резонансов.
///    * Выделяет огибающую отфильтрованного сигнала (Signal Envelope) методом демодуляции для обнаружения повторяющихся ударных импульсов.
///    * Пересчитывает полученную огибающую из временной области в угловую на основе единой сетки углов поворота вала (Angular Grid).
///    * Вычисляет упорядоченный спектр огибающей (Order Spectrum) с заданным размером окна БПФ (FFT Size).
///    * Реализует автоматическую идентификацию зарождающихся дефектов подшипников (BPFI, BPFO, FTF, BSF) детектором по высокочастотным порогам.
///    * Формирует SQL-запросы с результатами анализа и экспортирует их в базу данных через API-клиент.
pub struct VibroSensor<T, SqlBuilder> {
    /// Угловая сетка, текущая RPM и фаза поворота вала.
    angular: AngularGrid<Autocorrelation<ReadEventValuess<T>>>,
    /// Контекст `AngularGrid`
    angular_ctx: Cell<AngularCtx>,
    /// Анализ низкочастотного диапазона.
    low_range: LowRange<SqlBuilder>,
    /// Контекст анализа низкочастотного диапазона.
    low_range_ctx: Cell<ImbContext>,
    /// Аналих среднечастотного диапазона.
    mid_range: MidRange,
    /// Контекст анализа среднечастотного диапазона.
    mid_range_ctx: MidRangeCtx,
    /// Аналих высокочастотного диапазона.
    high_range: HighRange,
    /// Контекст анализа высокочастотного диапазона.
    high_range_ctx: HighRangeCtx,
    /// Настройки цифровой обработки вибросигнала.
    conf: crate::Conf,
    /// Exit signal.
    exit: Arc<ExitNotify>,
    /// Для отладки.
    dbg: Dbg,
}
impl<T, SqlBuilder> VibroSensor<T, SqlBuilder>
where
    T: EventValueAccess<str, f64> + Send + Sync + 'static,
    SqlBuilder: Fn(&ImbContext) {
    ///
    /// ### Returns `VibroSensor` new instance
    /// - `parent` - Идентификатор родительской сущности (для отладки).
    /// - `conf` - Конфигурация цифровой обработки вибросигнала.
    /// - `event_values` - Агрегатор входных эвентов.
    /// - `retain` - Хранение пар Key-Value на диске.
    /// - `api_link` - Провайдер отправки SQL запросов.
    #[named]
    pub fn new(parent: &Dbg, conf: crate::Conf, event_values: Arc<T>, retain: Arc<Retain>, sql_builder: SqlBuilder, exit: Arc<ExitNotify>) -> Result<Self, Error> {
        let dbg = Dbg::new(parent, me::<Self>());
        let window_size = conf.analysis.n_fft();
        let window_fn = WindowFn::<f32>::kaiser(&dbg, window_size, window_size, 0, 5.65)
            .map_err(|err| err_pass!(dbg, err))?;
        let angular_ctx = Cell::new(AngularCtx::new(conf.adc.sample_rate_hz, conf.adc.chunk_size));
        let low_range_ctx = Cell::new(ImbContext::new(&dbg, conf.analysis.samples_per_rev(), conf.analysis.n_fft(), conf.adc.chunk_size, retain.clone()));
        Ok(Self {
            angular: AngularGrid::new(&dbg, conf.adc.chunk_size,
                Autocorrelation::new(&dbg,
                    conf.adc.sample_rate_hz,
                    ReadEventValuess::new(&dbg, event_values)
                ),
            ),
            angular_ctx,
            low_range: SqlExport::new(&dbg, sql_builder,
                ImbalanceDetector::new(&dbg,
                    OrderFeatureFilter::new(&dbg,
                        conf.analysis.n_fft(),
                        conf.analysis.angular_step_rad(),
                        OrderSpectrum::new(&dbg,
                            conf.analysis.n_fft(),
                            Some(window_fn),
                            OrderDomainSamples::new(&dbg,
                                conf.analysis.samples_per_rev(),
                                LowPassSignal::new(&dbg,
                                    conf.adc.sample_rate_hz,
                                    conf.analysis.bands.low_cutoff_order(),
                                    Pass::new(),
                                ),
                            ),
                        ),
                    ),
                ),
            ),
            low_range_ctx,
            mid_range: Pass::new(),
            mid_range_ctx: MidRangeCtx {  },
            high_range: Pass::new(),
            high_range_ctx: HighRangeCtx {  },
            conf,
            exit,
            dbg,
        })
    }
}
impl<T, SqlBuilder> Eval<&[u16], Result<(), Error>> for VibroSensor<T, SqlBuilder>
where
    T: EventValueAccess<str, f64>,
    SqlBuilder: Fn(&ImbContext) {
    //
    #[named]
    #[inline]
    fn eval(&self, samples: &[u16]) -> Result<(), Error> {
        let ts = chrono::Utc::now();
        let mut angular_ctx = self.angular_ctx.take();
        let mut low_range_ctx = self.low_range_ctx.take();
        angular_ctx.push_chunk(samples);
        let phases;
        (angular_ctx, phases) = self.angular.eval(angular_ctx);
        let frame = Frame::new(ts, self.conf.adc.ds_offset as f32, samples, phases);
        low_range_ctx.update(frame.clone());
        low_range_ctx = self.low_range.eval(low_range_ctx);
        if let Some(err) = low_range_ctx.err() {
            self.angular_ctx.set(angular_ctx);
            self.low_range_ctx.set(low_range_ctx);
            return Err(err_pass!(self.dbg, err));
        }
        // self.mid_range_ctx.update(frame.clone());
        // self.mid_range_ctx = self.mid_range.eval(self.mid_range_ctx);
        // self.high_range_ctx.update(frame.clone());
        // self.high_range_ctx = self.high_range.eval(self.high_range_ctx);
        self.angular_ctx.set(angular_ctx);
        self.low_range_ctx.set(low_range_ctx);
        Ok(())
    }
    //
    fn exit(&self) {
        self.exit.exit();
    }
}
