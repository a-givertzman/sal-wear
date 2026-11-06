use std::f64::consts::PI;
use sal_core::dbg::Dbg;
use function_name::named;
use crate::{AngularCtx, Eval, FSampleDec, NOIZE_THRESHOLD};
///
/// Узел обработки сигналов для реализации **режима захвата (Capture Mode)**.
///
/// Этот компонент оборачивает дочерний узел (`Child`) и активирует алгоритм оценки 
/// точной частоты по кольцевому буферу истории децимированного сигнала с использованием 
/// сетки фильтров Герцеля и параболической интерполяции.
pub struct CaptureMode<Child> {
    child: Child,
    dbg: Dbg,
}

impl<Child> CaptureMode<Child>
where
    Child: Eval<AngularCtx, AngularCtx> + Send + 'static,
{
    ///
    /// Создает новый экземпляр узла [CaptureMode]
    pub fn new(parent: &Dbg, child: Child) -> Self {
        let dbg = Dbg::new(parent, crate::me::<Self>());
        Self {
            child,
            dbg,
        }
    }
    ///
    /// Классический алгоритм Герцеля для оценки амплитуды на заданной частоте f.
    pub fn goertzel(buf: &[f64], f: f64, f_sample_dec: FSampleDec<f64>) -> f64 {
        let f_sample = f_sample_dec.value();
        if buf.is_empty() || !f.is_finite() || !(f > 0.0 && f < f_sample / 2.0) || !(f_sample > 0.0) {
            return 0.0;
        }
        let w = 2.0 * PI * f / f_sample;
        let cos_w = w.cos();
        let sin_w = w.sin();
        // Коэффициент обратной связи фильтра
        let coeff = 2.0 * cos_w;
        let (mut s1, mut s2) = (0.0f64, 0.0f64);
        for &x in buf.iter() {
            let x = if x.is_finite() { x } else { 0.0 };
            let s = x + coeff * s1 - s2;
            s2 = s1;
            s1 = s;
        }
        // Проекция конечных состояний на действительную и мнимую составляющие
        let real = s1 - s2 * cos_w;
        let imag = s2 * sin_w;
        let power = real * real + imag * imag;
        let amp = 2.0 * power.max(0.0).sqrt() / buf.len() as f64;
        if amp.is_finite() { amp } else { 0.0 }
    }
    ///
    /// Вычисляет суббинное смещение вершины параболы по трем точкам.
    /// Возвращает смещение относительно центральной точки (y2) в диапазоне [-0.5, 0.5].
    pub fn parabolic_peak(y1: f64, y2: f64, y3: f64) -> f64 {
        // Знаменатель формулы: y1 - 2*y2 + y3
        let denominator = y1 - 2.0 * y2 + y3;
        // Проверяем на ноль, бесконечность, NaN или если парабола "перевернута" не туда
        // (знаменатель должен быть отрицательным для пика-максимума)
        if denominator.abs() < 1e-12 || !denominator.is_finite() {
            return 0.0;
        }
        // Вычисляем смещение: 0.5 * (y1 - y3) / (y1 - 2*y2 + y3)
        let delta = 0.5 * (y1 - y3) / denominator;
        if delta.is_finite() {
            delta.clamp(-0.5, 0.5)
        } else {
            0.0
        }
    }
    ///
    /// Выполняет поиск точной частоты в окрестности `f_center` по сетке алгоритмов Герцеля.
    /// Возвращает уточненную частоту f_precise и максимальную амплитуду max_amp.
    pub fn find_precise_frequency(
        buf: &[f64],
        f_center: f64,
        f_sample_dec: FSampleDec<f64>,
        half_window: f64, // Полуширина окна поиска вокруг f_center (например, 2.0 Гц)
        steps: usize,      // Количество шагов в ОДНУ сторону (всего точек на сетке будет: steps * 2 + 1)
    ) -> (f64, f64) {
        if steps < 1 || buf.is_empty() {
            return (f_center, 0.0);
        }
        // Шаг сетки по частоте
        let df = half_window / (steps as f64);
        // Аллоцируем массив под амплитуды на сетке
        let total_points = steps * 2 + 1;
        let mut amplitudes = vec![0.0f64; total_points];
        let mut frequencies = vec![0.0f64; total_points];
        // 1. Сканируем сетку частот вокруг f_center
        for i in 0..total_points {
            let offset_idx = (i as f64) - (steps as f64);
            let f_current = f_center + offset_idx * df;
            frequencies[i] = f_current;
            amplitudes[i] = Self::goertzel(buf, f_current, f_sample_dec);
        }
        // 2. Ищем индекс максимальной амплитуды (грубый пик)
        let mut max_idx = 0;
        let mut max_amp = amplitudes[0];
        for i in 1..total_points {
            if amplitudes[i] > max_amp {
                max_amp = amplitudes[i];
                max_idx = i;
            }
        }
        // Если максимум оказался на самом краю сетки, мы не можем построить параболу по 3 точкам.
        if max_idx == 0 || max_idx == total_points - 1 {
            return (frequencies[max_idx], amplitudes[max_idx]);
        }
        // 3. Параболическая интерполяция по трем точкам вокруг максимума
        let y1 = amplitudes[max_idx - 1];
        let y2 = amplitudes[max_idx];
        let y3 = amplitudes[max_idx + 1];
        let d = Self::parabolic_peak(y1, y2, y3);
        // Итоговая точная частота
        let f_precise = frequencies[max_idx] + d * df;
        (f_precise, max_amp)
    }
}
//
impl<Child> Eval<AngularCtx, AngularCtx> for CaptureMode<Child>
where
    Child: Eval<AngularCtx, AngularCtx> + Send + 'static,
{
    #[inline]
    #[named]
    fn eval(&self, ctx: AngularCtx) -> AngularCtx {
        // Пропускаем контекст через дочерний узел
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        { // Копим историю сигнала в кольцевой буффер
            let d = &mut ctx.rpm_detection;
            for &x in d.decimation.decimated.iter() {
                if d.capture_mode.signal_buffer.len() >= d.capture_mode.buffer_len {
                    d.capture_mode.signal_buffer.pop_front();
                }
                d.capture_mode.signal_buffer.push_back(x);
            }
        }
        if !ctx.rpm_detection.validity_detector.valid {
            // Поиск только при достаточной истории: короткий буфер даёт
            // разрешение ~1/T и бессмысленную оценку (на старте системы)
            if ctx.rpm_detection.capture_mode.signal_buffer.len() < ctx.rpm_detection.capture_mode.buffer_len / 2 {
                ctx.rpm_detection.capture_mode.is_updated = false;
                return ctx;
            }
            let cap = &mut ctx.rpm_detection.capture_mode;
            let buf = cap.signal_buffer.make_contiguous();
            // Запуск поиска по сетке на основе децимированного буфера истории сигнала
            let (f_precise, max_amp) = Self::find_precise_frequency(
                buf, 
                ctx.raw_rpm / 60.0, // Базовая частота вращения (Гц) из RPM
                ctx.rpm_detection.decimation.f_sample_dec, 
                cap.search_half_window_hz, 
                cap.search_steps,
            );
            // Обновление состояния контекста режима захвата
            if f_precise.is_finite() && f_precise > 0.0 && max_amp > NOIZE_THRESHOLD {
                ctx.rpm_detection.capture_mode.f_precise = Some(f_precise);
                ctx.rpm_detection.capture_mode.is_updated = true;
            } else {
                ctx.rpm_detection.capture_mode.is_updated = false;
            }
        } else {
            ctx.rpm_detection.capture_mode.is_updated = false;
        }
        ctx
    }
    fn exit(&self) {
        self.child.exit();
    }
}
