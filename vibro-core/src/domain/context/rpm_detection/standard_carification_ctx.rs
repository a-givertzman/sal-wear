use std::f64::consts::{PI, TAU};

use rustfft::num_complex::Complex;

use crate::RpmDetectionConf;
///
/// Контекст узла уточнения частоты
#[derive(Debug, Clone)]
pub struct StandardClarificationCtx {
    /// Число оборотов вала в окне усреднения δ
    pub revs_per_window: f64,
    /// Минимальная длина окна в сэмплах (защита на высоких оборотах)
    pub min_window_samples: usize,
    /// Разомкнутая коррекция f_het включена (correction_gain > 0)
    pub apply_correction: bool,
    /// Доля δ, переносимая в f_het за одно окно (0.0..1.0)
    pub correction_gain: f64,
    /// Частота среза одного ФНЧ, Гц. Равна `B/2`, где `B` — полная полоса слежения,
    pub f_cutoff: f64,
    // --- Рабочее состояние тракта ---
    /// Непрерывный развернутый аргумент огибающей, рад
    pub arg_unwrapped: f64,
    /// Сырой аргумент предыдущего сэмпла, рад
    pub prev_arg: Option<f64>,
    /// Развернутый аргумент на начало текущего окна усреднения, рад
    pub window_start_arg: f64,
    /// Сэмплов накоплено в текущем окне
    pub window_samples: usize,
    // --- Публикуемый результат ---
    /// Текущая оценка отклонения частоты δ, Гц (обновляется раз в окно)
    pub delta_hz: f64,
    /// Уточненная частота вращения f_het + δ, Гц (None до первого завершенного окна)
    pub f_precise_hz: Option<f64>,
}
//
impl StandardClarificationCtx {
    /// Длина окна усреднения в сэмплах: целое число оборотов вала, не меньше минимума
    pub fn window_len_samples(&self, f_sample_dec: f64, f_het: f64) -> usize {
        ((self.revs_per_window * f_sample_dec / f_het).round() as usize)
            .max(self.min_window_samples)
    }
    /// Обработка одного сэмпла огибающей: развертка фазы и, при закрытии окна, оценка δ.
    /// Возвращает `Some(δ)`, Гц, если окно завершилось на этом сэмпле.
    pub fn update(&mut self, y: &Complex<f64>, window_len: usize, f_sample_dec: f64, f_het: f64) -> Option<f64> {
        let arg = y.im.atan2(y.re);
        self.arg_unwrapped = match self.prev_arg {
            Some(prev) => {
                let mut d = arg - prev;
                if d > PI { d -= TAU; } else if d < -PI { d += TAU; }
                self.arg_unwrapped + d
            }
            // Первый сэмпл после старта/сброса: только база, интервалов еще не было
            None => {
                self.window_start_arg = arg;
                self.arg_unwrapped = arg;
                self.prev_arg = Some(arg);
                return None;
            }
        };
        self.prev_arg = Some(arg);
        self.window_samples += 1;
        // Окно завершено: оценка δ по полному приращению фазы за целое число оборотов
        if self.window_samples >= window_len {
            let dt = window_len as f64 / f_sample_dec;
            let delta = (self.arg_unwrapped - self.window_start_arg) / (TAU * dt);
            self.window_start_arg = self.arg_unwrapped;
            self.window_samples = 0;
            // За пределами полосы слежения оценке доверять нельзя — ограничиваем
            self.delta_hz = delta.clamp(-self.f_cutoff, self.f_cutoff);
            self.f_precise_hz = Some(f_het + self.delta_hz);
            Some(self.delta_hz)
        } else {
            None
        }
    }
}
//
impl StandardClarificationCtx {
    /// ### Инициализация контекста из конфигурации
    /// - `conf` - Параметры определения RPM и фазы вала (`AnalysisConf.rpm_detection`)
    pub fn new(revs_per_window: f64, correction_gain: f64, min_window_samples: usize, f_cutoff: f64) -> Self {
        Self {
            revs_per_window,
            min_window_samples,
            apply_correction: correction_gain > 0.0,
            correction_gain,
            f_cutoff,
            arg_unwrapped: 0.0,
            prev_arg: None,
            window_start_arg: 0.0,
            window_samples: 0,
            delta_hz: 0.0,
            f_precise_hz: None,
        }
    }
    /// Сброс рабочего состояния при входе в режим захвата.
    /// Настройка сохраняется — сбрасываются только состояние и результат:
    /// старое окно и развертка относятся к «доаварийному» тракту и недоверяемы.
    pub fn reset(&mut self) {
        self.arg_unwrapped = 0.0;
        self.prev_arg = None;
        self.window_start_arg = 0.0;
        self.window_samples = 0;
        self.delta_hz = 0.0;
        self.f_precise_hz = None;
    }
}
//
impl Default for StandardClarificationCtx {
    fn default() -> Self {
        Self {
            arg_unwrapped: 0.0,
            prev_arg: None,
            window_start_arg: 0.0,
            window_samples: 0,
            delta_hz: 0.0,
            f_precise_hz: None,
            revs_per_window: 4.0,
            min_window_samples: 64,
            apply_correction: true,
            correction_gain: 0.25,
            f_cutoff: 3.5,
        }
    }
}