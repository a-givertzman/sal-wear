use crate::{ComplexLocalOscillatorCtx, DecimationCtx, FSample, InstantShaftPhaseCtx, LPFTrackingBandCtx, group_delay_samples};

///
/// Агрегирующий контекст конвейера обработки сигналов (DSP Pipeline) для оценки частоты вращения (RPM).
///
/// Является структурой-контейнером, которая аккумулирует промежуточные состояния, 
/// конфигурационные параметры и буферы данных, передаваемые между последовательными 
/// этапами цифровой обработки сигналов: от децимации до гетеродинирования.
pub struct RpmDetectionCtx {
    /// Состояние и выходные буферы адаптивного децимирующего фильтра.
    /// 
    /// Отвечает за предварительную низкочастотную фильтрацию (Anti-Aliasing) 
    /// и понижение частоты дискретизации (прореживание) входного вещественного сигнала.
    pub(crate) decimation: DecimationCtx,
    /// Состояние и выходные буферы комплексного локального осциллятора (гетеродина).
    /// 
    /// Отвечает за перенос спектра первой оборотной гармоники (1X) на нулевую частоту 
    /// путем комплексного умножения и непрерывного интегрирования фазы вала.
    pub(crate) complex_local_oscillator: ComplexLocalOscillatorCtx,
    /// Состояние и выходные буферы фиксированного ФНЧ полосы слежения.
    /// 
    /// Отвечает за каскадную фильтрацию 4-го порядка комплексного I/Q сигнала. 
    /// Вырезает все высокочастотные компоненты, гармоники редуктора и шумы, оставляя 
    /// идеально чистую комплексную огибающую первой оборотной гармоники (1X).
    pub(crate) lpf_tracking_band: LPFTrackingBandCtx,
    /// Состояние и выходные буферы модуля восстановления мгновенной фазы вала.
    /// 
    /// Отвечает за компенсацию частотного сдвига гетеродина и расчет абсолютного 
    /// физического угла поворота вала в диапазоне [0, 2*PI) для каждого отсчета чанка. 
    /// Полученные данные полностью подготовлены для ордерного (порядкового) анализа.
    pub(crate) instant_shaft_phase: InstantShaftPhaseCtx,
    /// Состояние и выходные буферы амплитудного детектора валидности с гистерезисом.
    /// 
    /// Реализует конечный автомат, который непрерывно оценивает амплитуду комплексной огибающей 1X 
    /// и переключает глобальные режимы работы тракта слежения (Valid, Degraded, Acquisition). 
    /// Защищает систему от фазового дребезга и автоматически инициирует повторный захват частоты 
    /// при длительной просадке сигнала.
    pub(crate) validity_detector: ValidityDetectorCtx,
}
//
impl RpmDetectionCtx {
    /// Создает и инициализирует единый вычислительный контекст конвейера с глубоким 
    /// резервированием памяти под внутренние структуры.
    ///
    /// Конструктор гарантирует, что все вложенные векторы и сдвиговые регистры будут 
    /// проаллоцированы один раз при запуске системы, предотвращая задержки (jitter) 
    /// из-за выделения памяти на куче (heap allocations) во время обработки потока данных.
    ///
    /// # Аргументы
    /// * `f_sample` - Частота дискретизации АЦП.
    /// * `chunk_size` - Размер пакета АЦП в отсчётах.
    /// * `factor` - Коэффициент децимации `M`.
    pub fn new(f_sample: FSample<f64>, chunk_size: usize, factor: usize, f_cutoff: f64) -> Self {
        let f_sample_dec = f_sample.decimated(factor);
        let chunk_dec = chunk_size / factor + 1; // а не 512: после децимации ≈26
        let delay = group_delay_samples(f_cutoff, f_sample_dec);
        Self {
            decimation: DecimationCtx::new(f_sample, chunk_size, factor),
            complex_local_oscillator: ComplexLocalOscillatorCtx::new(chunk_dec),
            lpf_tracking_band: LPFTrackingBandCtx::new(f_sample_dec, chunk_dec, f_cutoff),
            instant_shaft_phase: InstantShaftPhaseCtx::new(chunk_dec, delay),
        }
    }
}
//
impl Default for RpmDetectionCtx {
    /// Создает пустой контекст конвейера со значениями вложенных структур по умолчанию.
    /// 
    /// Векторы инициализируются с нулевой емкостью. Для боевого использования в DSP-цепочках 
    /// настоятельно рекомендуется использовать метод `new` во избежание динамических реаллокаций памяти.
    fn default() -> Self {
        Self {
            decimation: Default::default(),
            complex_local_oscillator: Default::default(),
            lpf_tracking_band: Default::default(),
            instant_shaft_phase: Default::default(),
            validity_detector: Default::default(),
        }
    }
}
