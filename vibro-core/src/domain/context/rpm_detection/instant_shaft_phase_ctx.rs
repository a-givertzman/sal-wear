use std::collections::VecDeque;
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PhaseStatus {
    Valid(f64),
    Invalid
}
//
impl PhaseStatus {
    #[inline]
    pub fn is_valid(&self) -> bool {
        matches!(self, PhaseStatus::Valid(_))
    }
    #[inline]
    pub fn value(self) -> Option<f64> {
        match self {
            PhaseStatus::Valid(v) => Some(v),
            PhaseStatus::Invalid => None,
        }
    }
    #[inline]
    pub fn from_value(v: f64) -> Self {
        if v.is_finite() {
            PhaseStatus::Valid(v)
        } else {
            PhaseStatus::Invalid
        }
    }
}


/// Контекст и выходные буферы модуля вычисления мгновенной фазы вала.
///
/// Структура инкапсулирует массив рассчитанных абсолютных углов поворота вала. 
/// Выделение памяти под буфер происходит один раз на этапе инициализации DSP-конвейера, 
/// что гарантирует отсутствие задержек аллокации в процессе потоковой обработки.
pub struct InstantShaftPhaseCtx {
    /// Мгновенная физическая фаза (угол поворота) вала с флагом валидности.
    ///
    /// Каждый отсчёт — `PhaseStatus::Valid(f)`, где `f` нормализована в полуоткрытый
    /// интервал `[0, 2*PI)`, либо `PhaseStatus::Invalid` (фаза недостоверна: нефинитная,
    /// а в будущем — вне полосы по амплитуде/δ). Буфер полностью обновляется
    /// (очищается через `clear()`) на каждом вызове `eval` для текущего чанка данных.
    pub shaft_phase: Vec<PhaseStatus>,
    /// Развёрнутая (непрерывная) фаза вала, рад: `turns · 2π + shaft_phase`.
    /// Параллельна `shaft_phase`, очищается на каждом `eval`.
    /// Первый отсчёт после старта равен своей свёрнутой фазе (`turns = 0`).
    pub shaft_phase_unwrapped: Vec<PhaseStatus>,
    /// Признак завершения прогрева для каждого отсчёта пакета (параллелен `shaft_phase`).
    ///
    /// `false`, пока линия задержки `phi_delay` заполняется: первые `delay_samples`
    /// отсчётов после создания контекста или после [`Self::reset`]. В этих позициях
    /// `shaft_phase` и `shaft_phase_unwrapped` содержат `PhaseStatus::Invalid`.
    pub ready: Vec<bool>,
    /// Линия задержки фазы гетеродина `phi_het` (FIFO).
    ///
    /// Нужна, чтобы складывать `atan2` огибающей с фазой гетеродина того же момента времени:
    /// выход ФНЧ запаздывает на `delay_samples`, поэтому фаза гетеродина берётся с таким же
    /// запаздыванием. Длина очереди между вызовами `eval` равна `delay_samples` (после
    /// прогрева), хранится в контексте, чтобы переживать границы пакетов.
    /// Очищается в [`Self::reset`].
    pub phi_delay: VecDeque<f64>,
    /// Задержка выравнивания в отсчётах после децимации.
    ///
    /// Равна групповой задержке каскада ФНЧ на нулевой частоте
    /// `D = (1/Q1 + 1/Q2) / (2π·fc) · f_sample_dec` (см. `group_delay_samples`),
    /// около 1901 отсчёта при `fc = 3.5 Гц` и `f_sample_dec = 16 кГц`.
    /// Определяет латентность выхода: `shaft_phase[n]` относится к моменту
    /// `n − delay_samples`, то есть опаздывает на `delay_samples / f_sample_dec` секунд.
    /// `0` отключает задержку (все отсчёты сразу готовы).
    /// TODO(P0): `shaft_phase[k]` относится к моменту `k - D` (D ≈ 1901 отсч. @16 кГц
    /// ≈ 119 мс ≈ 38 000 сырых отсчётов АЦП ≈ 6 оборотов при 50 Гц). Чтобы сопоставить
    /// θ с сырым сэмплом x[k], нужно задерживать и сырой поток. Решение отложено до
    /// готовности [шага 3.5.](vibro-core/design/rpm-detection-v1.md)
    pub delay_samples: usize,
    /// Свёрнутая фаза последнего отсчёта прошлого пакета. `NaN` до первого отсчёта.
    pub prev_phase: f64,
    /// Накопленное число полных оборотов (со знаком). Целое, чтобы не терять точность `f64`
    /// на длинных прогонах.
    pub turns: i64,
}
//
impl InstantShaftPhaseCtx {
    /// Создает новый контекст модуля вычисления фазы с предварительным резервированием памяти.
    ///
    /// Выделение емкости (`with_capacity`) для выходного вектора исключает накладные расходы 
    /// процессора на динамическую реаллокацию памяти на куче (heap allocations) во время работы 
    /// высоконагруженного DSP-конвейера.
    ///
    /// # Аргументы
    /// * `chunk_size_dec` - Ожидаемое число отсчётов децимированного сигнала в одном пакете.
    ///   в отсчетах (используется для предрасчета размера буфера `shaft_phase`).
    pub fn new(chunk_size_dec: usize, delay_samples: usize) -> Self {
        Self {
            shaft_phase: Vec::with_capacity(chunk_size_dec),
            shaft_phase_unwrapped: Vec::with_capacity(chunk_size_dec),
            ready: Vec::with_capacity(chunk_size_dec),
            phi_delay: VecDeque::with_capacity(delay_samples + chunk_size_dec + 1),
            delay_samples,
            prev_phase: f64::NAN,
            turns: 0,
        }
    }
    /// Сброс при потере сигнала / переходе в Acquisition.
    pub fn reset(&mut self) {
        self.phi_delay.clear();
        self.prev_phase = f64::NAN;
        self.turns = 0;
    }
}
//
impl Default for InstantShaftPhaseCtx {
    /// Создает пустой контекст модуля вычисления мгновенной фазы вала со значениями по умолчанию.
    /// 
    /// Выходной вектор инициализируется с нулевой емкостью без выделения памяти на куче. 
    /// Для промышленной потоковой обработки рекомендуется использовать метод `new` во избежание 
    /// динамических реаллокаций памяти в "горячем" цикле расчетов.
    fn default() -> Self { Self::new(0, 0) }
}
