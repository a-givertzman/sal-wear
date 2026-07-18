use std::sync::Arc;
use rustfft::{Fft, FftPlanner};
use num_complex::Complex;

// Константы оптимизированы для низкочастотной диагностики (1X, 2X, 3X)
const POINTS_PER_TURN: usize = 64;   // Nrev: Оптимально для фильтрации низких порядков
const TARGET_RESOLUTION: f64 = 0.016; // Требуемое разрешение для разделения 1X и сети (~64 оборота)

pub struct OrderSpectrum<Child> {
    sample_rate: f64,
    child: Child,
    fft: Arc<dyn Fft<f32>>,
    dbg: Dbg,
    window_coefficients: Vec<f32>, // Предрассчитанные веса окна Ханнинга
}

impl<Child> OrderSpectrum<Child>
where
    Child: Eval<ImbContext, ImbContext> + Send + 'static 
{
    pub fn new(parent: &Dbg, sample_rate_hz: impl Into<f64>, child: Child) -> Self {
        let dbg = Dbg::new(parent, me::<Self>());
        let fft_size = Self::fft_buffer_size();
        log::debug!("{dbg}.new | Низкочастотный OrderSpectrum. Размер окна FFT: {fft_size}");
        
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(fft_size);
        
        // Генерация окна Ханнинга для подавления спектрального растекания на 1X..3X
        let window_coefficients = (0..fft_size)
            .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (fft_size - 1) as f32).cos()))
            .collect();

        Self {
            sample_rate: sample_rate_hz.into(),
            child,
            fft,
            dbg,
            window_coefficients,
        }
    }

    /// Расчет размера буфера. 64 / 0.016 = 4000 отсчетов -> Ближайшая степень двойки = 4096.
    /// Это дает реальное накопление 64 оборотов вала и разрешение 0.0156 порядка.
    pub fn fft_buffer_size() -> usize {
        let min_needed_samples = POINTS_PER_TURN as f64 / TARGET_RESOLUTION;
        (min_needed_samples.round() as usize).next_power_of_two()
    }
}

impl<Child> Eval<ImbContext, ImbContext> for OrderSpectrum<Child>
where
    Child: Eval<ImbContext, ImbContext> + Send + 'static 
{
    #[inline]
    fn eval(&self, ctx: ImbContext) -> ImbContext {
        // 1. Получаем угловые сэмплы из предыдущего шага ресемплера
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }

        // Проверка: поместятся ли новые данные в свободное место кольцевого буфера
        if ctx.order_samples.len() > ctx.fft_buff.free_space() {
            ctx.err = Some(Error::new(&self.dbg, "eval")
                .err(format!("Переполнение FIFO: новые угловые сэмплы ({}) не помещаются", ctx.order_samples.len())));
            return ctx;
        }

        // 2. Накапливаем данные в FIFO
        ctx.fft_buff.push_chunk(&ctx.order_samples[..]);

        // 3. Вычисляем спектр ТОЛЬКО при полном заполнении окна
        if ctx.fft_buff.is_full() {
            let real_samples = ctx.fft_buff.read_window(); 
            
            // Перенос в комплексный массив с ОДНОВРЕМЕННЫМ наложением оконной функции
            // Это критично для точного определения амплитуды дисбаланса (1X)
            for (i, &sample) in real_samples.iter().enumerate() {
                let windowed_sample = sample * self.window_coefficients[i];
                ctx.fft_window[i] = Complex::new(windowed_sample, 0.0);
            }

            // Выполнение In-place FFT (Прямое преобразование)
            self.fft.process(&mut ctx.fft_window);
            
            // Результат: первые (Nfft / 2) элементов — это порядки от 0 до 32X с шагом 0.0156X.
            // Пик дисбаланса будет строго в бине (1.0 / 0.0156) = 64-й элемент массива.
        }

        ctx
    }

    fn exit(&self) {
        self.child.exit();
    }
}
