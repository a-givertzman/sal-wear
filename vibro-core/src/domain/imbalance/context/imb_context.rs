use std::sync::Arc;
use crate::{DiagFeatures, DiagnosticResult, Order, OrderZone, Phase, Retain, Retained, Rpm, ShortSigma, num_complex::Complex};
use sal_core::error::Error;
use crate::{Frame, KalmanFilter, LowPassSignalCtx, MirroredBuffer};

///
/// Контейнер для передачи данных между вычислительными шагами
pub struct ImbContext {
    /// Уточненная частота вращения вала, об/мин.
    pub rpm: Rpm<f64>,
    /// Сырые выборки из АЦП и угловая сетка. Приходят из AngularGrid
    pub frame: Arc<Frame<u16, f32>>,
    /// LowPassSinal Context.
    pub low_pass_signal: LowPassSignalCtx,
    /// Отфилтрованная выборка сырого АЦП сигнала.
    /// Имеет размер `conf.adc.chunk_size`
    pub samples: Vec<f32>,

    /// Сигнал развернутый в равномерную сетку угловой области.
    /// Значения вибрации соответствуют каждому углу поворота вала механизма.
    pub order_samples: Vec<Complex<f32>>,
    /// Текущий абсолютный вычисленный угол θ поворота вала (не сбрасывается), не используется в расчетах, для отчетности.
    /// Соответствует последнему элементу в `order_samples`
    pub total_phase: Phase<f64>,


    /// Буфер для аккумулирования выборок для FFT (OrderSpectrum)
    pub fft_buff: MirroredBuffer<Complex<f32>>,
    /// Буфер результатов FFT (OrderSpectrum).
    /// Первая его половина комплексный спектр амплитуд и фаз порядков от `0X` до `32X` с шагом ≈ 0.0078X.
    pub fft_window: Vec<Complex<f32>>,

    /// Фильтры накопления изменений гармоник исследуемых дефектов (0.5x, 1.0x, 1.5x, 2.0x, 2.5x, 3.0x)
    pub filters: Vec<KalmanFilter>,
    /// Массив результатов фильтрации целевых гармоник спектрального анализа сигнала в угловом домене.
    pub features: Vec<DiagFeatures>,
    /// Результаты диагностического анализа, содержащий информацию об обнаруженных дефектах
    pub results: Vec<DiagnosticResult>,

    /// Текущая ошибка вычислений.
    /// Будет `Some(Error)` если шаг вычислений вернул ошибку, остальные шали эскалируют наверх.
    err: Option<Error>,
}
//
impl ImbContext {
    /// - `samples_per_rev` - Плотность угловой сетки (точек на оборот) (из конфига).
    /// - `n_fft` - Размер буфера FFT (из конфига).
    /// - `chunk_size` - размер пакета данных, поступающего из АЦП за один раз.
    /// - `retain` - Инструмент хранения пар Key-Value на диске.
    pub fn new(parent: impl Into<String>, samples_per_rev: usize, n_fft: usize, chunk_size: usize, retain: Arc<Retain>) -> Self {
        let parent = parent.into();
        // TODO: Исправить размер, он должен быть равен предполагаемому количеству углов исходя из размера входной выборки и максимальных оборотов
        let capacity = n_fft;
        log::debug!("{}.new | n_fft: {n_fft}", crate::me::<Self>());
        let filters = [0.5, 1.0, 1.5, 2.0, 2.5, 3.0].map(|order| {
            // Идентификатор зоны для хранения в retain
            let order_id = format!("{order}x");
            // Скорость старения процесса
            let q = 1e-7;
            let retained: Retained = retain.get(&order_id).unwrap_or(Retained::default());
            // Полуширина захвата в долях порядка (Для плавающих режимов ±0.05..±0.1 порядка).
            let half_width = 0.05;
            KalmanFilter::new(&parent, order_id.to_owned(), q, 0.01, retained, retain.clone(), // VORZHEV Z.A 07.09.2026 added `to_owned` cause of error "Expected `String` found `str`"
                OrderZone::new(Order(order), half_width, 3, n_fft, samples_per_rev),
                ShortSigma::new(
                    10, 
                    retained.x_hat,
                ),
            )
        }).into();
        Self {
            rpm: Rpm(f64::EPSILON),
            frame: Arc::new(Frame::default()),
            low_pass_signal: LowPassSignalCtx::new(),
            samples: vec![0.0; chunk_size],
            order_samples: Vec::with_capacity(capacity),
            total_phase: Phase(0.0),
            fft_buff: MirroredBuffer::new(n_fft),
            fft_window: Vec::with_capacity(n_fft),
            filters,
            features: vec![],
            results: vec![],
            err: None,
        }
    }
    /// VORZHEV Z.A. 1.09.2026 NEW CONSTRUCTOR FOR TESTING `WINDOW` AND `q` PARAMETERS
    pub fn new_with_params(parent: impl Into<String>, samples_per_rev: usize, n_fft: usize, chunk_size: usize, retain: Arc<Retain>, window: usize, q: f64) -> Self {
        let parent = parent.into();
        // TODO: Исправить размер, он должен быть равен предполагаемому количеству углов исходя из размера входной выборки и максимальных оборотов
        let capacity = n_fft;
        let filters = [0.5, 1.0, 1.5, 2.0, 2.5, 3.0].map(|order| {
            // Идентификатор зоны для хранения в retain
            let order_id = format!("{order}x");
            // Скорость старения процесса
            let retained: Retained = retain.get(&order_id).unwrap_or(Retained::default());
            // Полуширина захвата в долях порядка (Для плавающих режимов ±0.05..±0.1 порядка).
            let half_width = 0.05;
            KalmanFilter::new(&parent, order_id.to_owned(), q, 0.01, retained, retain.clone(), // VORZHEV Z.A 07.09.2026 added `to_owned` cause of error "Expected `String` found `str`"
                OrderZone::new(Order(order), half_width, 3, n_fft, samples_per_rev),
                ShortSigma::new(
                    window, 
                    retained.x_hat,
                ),
            )
        }).into();
        Self {
            rpm: Rpm(f64::EPSILON),
            frame: Arc::new(Frame::default()),
            low_pass_signal: LowPassSignalCtx::new(),
            samples: vec![0.0; chunk_size],
            order_samples: Vec::with_capacity(capacity),
            total_phase: Phase(0.0),
            fft_buff: MirroredBuffer::new(n_fft),
            fft_window: Vec::with_capacity(n_fft),
            filters,
            features: vec![],
            results: vec![],
            err: None,
        }
    }
    /// Добавляет новый массив сэмплов из АЦП в обработку
    /// - Сбрасывает массив результатов.
    /// - Сбрасывает ошибки.
    pub fn update(&mut self, frame: Arc<Frame<u16, f32>>) {
        self.frame = frame;
        self.features = vec![];
        self.err = None;
    }
    /// ### Устанавливает ошибку в контекст.
    /// 
    /// Это приведет к останову вычислений и экалации ошибки на верхний уровень.
    /// 
    /// - `me` - Имя текущего класса.
    /// - `area` - Имя текущего метода.
    /// - `err` - Ошибка.
    /// - Возвращает [ImbContext] с установленно ошибкой `err`.
    pub fn with_err(mut self, me: impl Into<String>, area: impl Into<String>, err: impl ToString) -> ImbContext {
        self.err = Some(Error::new(me, area).err(err.to_string()));
        self
    }
    /// Возвращает `true` если предыдущий шаг вернул ошибку
    pub fn is_err(&self) -> bool {
        self.err.is_some()
    }
    /// Эскалирует ошибку
    /// - `me` - Имя текущего класса
    /// - `area` - Имя текущего метода
    pub fn pass_err(mut self, me: impl Into<String>, area: impl Into<String>) -> ImbContext {
        self.err = match self.err {
            Some(err) => Some(Error::new(me, area).pass(err)),
            None => Some(Error::new(me, area)),
        };
        self
    }
    /// Возвращает ошибку вычислений, если есть
    pub fn err(&self) -> Option<Error> {
        self.err.as_ref().map(|e| e.clone())
    }
}
//
impl Default for ImbContext {
    fn default() -> Self {
        Self {
            rpm: Rpm(0.0),
            frame: Default::default(),
            low_pass_signal: Default::default(),
            samples: Default::default(),
            order_samples: Default::default(),
            total_phase: Default::default(),
            fft_buff: MirroredBuffer::new(0),
            fft_window: Default::default(),
            filters: Default::default(),
            features: Default::default(),
            results: Default::default(),
            err: Default::default(),
        }
    }
}