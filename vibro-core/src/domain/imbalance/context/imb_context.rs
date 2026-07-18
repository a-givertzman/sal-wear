use std::sync::Arc;
use sal_core::error::Error;
use crate::{Frame, LowPassSignalCtx, MirroredBuffer};

///
/// Контейнер для передачи данных между вычислительными шагами
pub struct ImbContext {
    /// Уточненная частота вращения вала, об/мин.
    pub rpm: f64,
    /// Сырые выборки из АЦП и угловая сетка
    pub frame: Arc<Frame>,
    /// LowPassSinal Context
    pub low_pass_signal: LowPassSignalCtx,
    /// Отфилтрованная выборка сырого АЦП сигнала
    pub samples: [f32; Frame::SIZE],
    /// Сигнал развернутый в равномерную сетку угловой области
    /// Значения вибрации соответствуют каждому углу поворота вала механизма
    pub order_samples: Vec<f64>,

    /// Буфер для аккумулирования выборок для FFT (OrderSpectrum)
    pub fft_buff: MirroredBuffer<f32>,
    /// Буфер для результатов FFT (OrderSpectrum)
    pub fft_out: Vec<f32>,

    /// Текущая ошибка вычислений
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
            samples: [0.0; Frame::SIZE],
            order_samples: Vec::with_capacity(capacity),
            fft_buff: MirroredBuffer::new(todo!()),
            fft_out: Vec::with_capacity(todo!()),
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