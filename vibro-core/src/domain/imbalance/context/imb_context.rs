use std::sync::Arc;
use chrono::{DateTime, Utc};
use crate::{OrderZone, Phase, Retain, Retained, Rms, Rpm, ShortSigma, num_complex::Complex};
use sal_core::error::Error;
use crate::{Frame, KalmanFilter, LowPassSignalCtx, MirroredBuffer, OrderSpectrum, Pass};

///
/// Контейнер для передачи данных между вычислительными шагами
pub struct ImbContext {
    /// Уточненная частота вращения вала, об/мин.
    pub rpm: Rpm<f64>,
    /// Сырые выборки из АЦП и угловая сетка.
    pub frame: Arc<Frame>,
    /// LowPassSinal Context.
    pub low_pass_signal: LowPassSignalCtx,
    /// Отфилтрованная выборка сырого АЦП сигнала.
    pub samples: Box<[f32; Frame::SIZE]>,

    /// Сигнал развернутый в равномерную сетку угловой области.
    /// Значения вибрации соответствуют каждому углу поворота вала механизма.
    pub order_samples: Vec<Complex<f32>>,
    /// Текущая фаза, соответствующая последнему элементу в `order_samples`
    pub last_phase: Phase<f64>,

    /// Буфер для аккумулирования выборок для FFT (OrderSpectrum)
    pub fft_buff: MirroredBuffer<Complex<f32>>,
    /// Буфер результатов FFT (OrderSpectrum).
    /// Первая его половина комплексный спектр амплитуд и фаз порядков от `0X` до `32X` с шагом ≈ 0.0078X.
    pub fft_window: Vec<Complex<f32>>,

    /// Фильтры накопления изменений гармоник исследуемых дефектов (0.5x, 1.0x, 1.5x, 2.0x, 2.5x, 3.0x)
    pub filters: Vec<KalmanFilter>,
    /// Массив результатов. Формат: (timestamp, Имя порядка, RMS, фаза, RPM).
    pub results: Vec<(DateTime<Utc>, String, Rms<f64>, Phase<f64>, Rpm<f64>)>,
    /// Текущая ошибка вычислений.
    /// Будет `Some(Error)` если шаг вычислений вернул ошибку, остальные шали эскалируют наверх.
    pub(crate) err: Option<Error>,
}
//
impl ImbContext {
    /// - `n_rev` - Плотность угловой сетки (точек на оборот) (из конфига).
    /// - `n_fft` - Размер буфера FFT (из конфига).
    pub fn new(parent: impl Into<String>, n_rev: usize, n_fft: usize, retain: Arc<Retain>) -> Self {
        let parent = parent.into();
        // TODO: Исправить размер, он должен быть равен предполагаемому количеству углов исходя из размера входной выборки и максимальных оборотов
        let capacity = n_fft;
        let filters = [0.5, 1.0, 1.5, 2.0, 2.5, 3.0].map(|order| {
            // Идентификатор зоны для хранения в retain
            let order_id = format!("{order}x");
            // Скорость старения процесса
            let q = 1e-7;
            let retained: Retained = retain.get(&order_id).unwrap_or(Retained::default());
            // Полуширина захвата в долях порядка (Для плавающих режимов ±0.05..±0.1 порядка).
            let half_width = 0.05;
            KalmanFilter::new(&parent, order_id, q, 0.01, retained, retain.clone(),
                OrderZone::new(order, half_width, 3, n_fft, n_rev),
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
            samples: Box::new([0.0; Frame::SIZE]),
            order_samples: Vec::with_capacity(capacity),
            last_phase: Phase(0.0),
            fft_buff: MirroredBuffer::new(n_fft),
            fft_window: Vec::with_capacity(n_fft),
            filters,
            results: vec![],
            err: None,
        }
    }
    /// Добавляет новый массив сэмплов из АЦП в обработку
    /// - Сбрасывает массив результатов.
    /// - Сбрасывает ошибки.
    pub fn update(&mut self, frame: Arc<Frame>) {
        self.frame = frame;
        self.results = vec![];
        self.err = None;
    }
    /// Эскалирует ошибку
    pub fn pass_err(mut self, me: impl Into<String>, area: impl Into<String>) -> ImbContext {
        self.err = match self.err {
            Some(err) => Some(Error::new(me, area).pass(err)),
            None => Some(Error::new(me, area)),
        };
        self
    }
}