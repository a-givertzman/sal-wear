use std::sync::Arc;
use rustfft::num_complex::Complex;
use sal_core::error::Error;
use crate::{Frame, LowPassSignalCtx, MirroredBuffer, OrderSpectrum, Pass};

///
/// Контейнер для передачи данных между вычислительными шагами
pub struct ImbContext {
    /// Уточненная частота вращения вала, об/мин.
    pub rpm: f64,
    /// Сырые выборки из АЦП и угловая сетка.
    pub frame: Arc<Frame>,
    /// LowPassSinal Context.
    pub low_pass_signal: LowPassSignalCtx,
    /// Отфилтрованная выборка сырого АЦП сигнала.
    pub samples: Box<[f32; Frame::SIZE]>,
    /// Сигнал развернутый в равномерную сетку угловой области.
    /// Значения вибрации соответствуют каждому углу поворота вала механизма.
    pub order_samples: Vec<Complex<f32>>,
    /// Значения углов для каждого значения вибрации массива в угловой области `order_samples`
    pub order_phases: Vec<f32>,
    /// Буфер для аккумулирования выборок для FFT (OrderSpectrum)
    pub fft_buff: MirroredBuffer<Complex<f32>>,
    /// Буфер результатов FFT (OrderSpectrum).
    /// Первая его половина комплексный спектр амплитуд и фаз порядков от $0X$ до $32X$ с шагом $\approx 0.0078X$.
    pub fft_window: Vec<Complex<f32>>,

    /// Текущая ошибка вычислений.
    /// Будет `Some(Error)` если шаг вычислений вернул ошибку, остальные шали эскалируют наверх.
    pub(crate) err: Option<Error>,
}
impl ImbContext {
    pub fn new(points_per_turn: usize, fft_turns: usize) -> Self {
        // Для FFT размер буфера должен быть строгой степенью двойки.
        // В порядковом анализе длина буфера формируется из двух параметров:
        //      - Угловое разрешение (Сэмплов на оборот): Берем из конфига (points_per_turn).
        //        Оптимальная сетка: 256 или 512 точек на один полный оборот вала.
        //        Если берем 256 точек, то по теореме Найквиста мы будем видеть дефекты до 128-го порядка.
        //        Этого с запасом хватит для дефектов подшипников (BPFO/BPFI обычно лежат в пределах 4–15 порядков).
        //      - Длина окна (Количество оборотов): Берем из конфига (fft_turns).
        //        Если взять 32 оборота по 256 точек, получим идеальный буфер на 8192 точки для быстрого FFT.
        //        Физический смысл: Размер FFT в 8192 точки даст нам спектральное разрешение
        //        $\Delta \text{Order} = \frac{1}{32} \approx 0.03125$ порядка.
        //        Это позволит детектору легко отличить, например, дисбаланс ротора ($1.0\times$) от дефекта сепаратора FTF (обычно около $0.38\times \dots 0.42\times$).
        let capacity = fft_turns * points_per_turn;
        Self {
            rpm: f64::EPSILON,
            frame: Arc::new(Frame::default()),
            low_pass_signal: LowPassSignalCtx::new(),
            samples: Box::new([0.0; Frame::SIZE]),
            order_samples: Vec::with_capacity(capacity),
            order_phases: Vec::with_capacity(capacity),
            fft_buff: MirroredBuffer::new(OrderSpectrum::<Pass>::fft_buffer_size()),
            fft_window: Vec::with_capacity(OrderSpectrum::<Pass>::fft_buffer_size()),
            err: None,
        }
    }
    /// Добавляет новый массив сэмплов из АЦП в обработку
    pub fn update(&mut self, frame: Arc<Frame>) {
        self.frame = frame;
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