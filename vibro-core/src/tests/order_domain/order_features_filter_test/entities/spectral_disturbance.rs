use crate::num_complex::Complex; // Используем комплексные числа ядра
use std::f64::consts::PI;
///
/// Форма импульса помехи
#[derive(Debug, Clone, Copy)]
pub enum ImpulseShape {
    Rectangular,
    Triangular,
    Sinusoidal,
}
///
/// Описание аддитивного возмущения (помехи) в спектре
#[derive(Debug, Clone)]
pub struct SpectralDisturbance {
    /// Номер фрейма, на котором помеха включается
    pub start_frame: u64,
    /// Длительность помехи в количестве фреймов
    pub duration_frames: u64,
    /// Амплитудный коэффициент (X в формуле X * A0)
    pub amplitude_factor: f64,
    /// Геометрическая форма импульса
    pub shape: ImpulseShape,
}

impl SpectralDisturbance {
    /// Вычисляет мгновенное значение помехи для фрейма `i`
    #[inline]
    pub fn evaluate(&self, i: u64, a0: f64) -> f64 {
        if i < self.start_frame || i >= self.start_frame + self.duration_frames {
            return 0.0;
        }
        let max_amplitude = self.amplitude_factor * a0;
        // Нормализованное время внутри импульса: [0.0; 1.0]
        let t = (i - self.start_frame) as f64 / self.duration_frames as f64;
        match self.shape {
            ImpulseShape::Rectangular => max_amplitude,
            ImpulseShape::Triangular => {
                if t <= 0.5 {
                    max_amplitude * (t * 2.0) // Подъем до пика
                } else {
                    max_amplitude * (2.0 - t * 2.0) // Спуск до нуля
                }
            }
            ImpulseShape::Sinusoidal => {
                max_amplitude * (t * PI).sin() // Полусинусоида куполом
            }
        }
    }
}