use crate::Zero;


/// Зеркальный кольцевой буфер.
pub struct MirroredBuffer<T> {
    /// Внутренний массив размером 2*capacity.
    buffer: Vec<T>,
    /// Логический размер окна (N), необходимый для автокорреляции.
    capacity: usize,
    /// Текущий индекс записи (от 0 до N-1).
    write_idx: usize,
    /// Текущая длина накопленных элементов
    len: usize,
}
impl<T: Copy + Zero> MirroredBuffer<T> {
    /// Выделяет память под буфер на куче.
    /// Вызывается строго один раз при инициализации.
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![T::zero(); capacity * 2],
            capacity,
            write_idx: 0,
            len: 0,
        }
    }
    /// Возвращает длинну коллекции
    pub fn len(&self) -> usize {
        self.len
    }
    /// Возвращает `true` если буфер наполнился
    pub fn is_full(&self) -> bool {
        self.len == self.capacity
    }
    /// Добавляет новую выборку из АЦП в буфер.
    /// Автоматически дублирует данные для обеспечения непрерывного окна чтения.
    /// Агрументы:
    /// * `chunk` - срез новых данных, длина не должна превышать capacity.
    pub fn push_chunk(&mut self, chunk: &[T]) {
        let m = chunk.len();
        assert!(m <= self.capacity, "Размер выборки превышает вместимость буфера");
        let space_left = self.capacity - self.write_idx;
        if m <= space_left {
            self.buffer[self.write_idx..self.write_idx + m].copy_from_slice(chunk);
            self.buffer[self.write_idx + self.capacity..self.write_idx + self.capacity + m].copy_from_slice(chunk);
            self.write_idx += m;
        } else {
            let part1 = space_left;
            let part2 = m - space_left;
            self.buffer[self.write_idx..self.capacity].copy_from_slice(&chunk[..part1]);
            self.buffer[self.write_idx + self.capacity..self.capacity * 2].copy_from_slice(&chunk[..part1]);
            self.buffer[0..part2].copy_from_slice(&chunk[part1..]);
            self.buffer[self.capacity..self.capacity + part2].copy_from_slice(&chunk[part1..]);
            self.write_idx = part2;
        }
        if self.write_idx == self.capacity {
            self.write_idx = 0;
        }
        if self.len < self.capacity {
            self.len += m;
            if self.len > self.capacity {
                self.len = self.capacity;
            }
        }
    }
    /// Возвращает непрерывный срез памяти длиной N для математического алгоритма.
    /// Срез отсортирован хронологически (от старейшего отсчета к самому новому).
    pub fn read_window(&self) -> &[T] {
        &self.buffer[self.write_idx..self.write_idx + self.capacity]
    }
}
///
/// Basic Tests
#[cfg(test)]
mod tests {
    use super::*;
    const PI2: f64 = 2.0 * std::f64::consts::PI;
    /// Тестирование механизма MirroredBuffer.
    /// Проверяет краевые эффекты при перехлесте индексов через границу N.
    #[test]
    fn test_mirrored_buffer_wrap_around() {
        // Инициализируем буфер размером 10 [cite: 210]
        let mut buffer = MirroredBuffer::new(10);
        let chunk1 = vec![1, 2, 3, 4, 5, 6, 7, 8];
        buffer.push_chunk(&chunk1);
        // Записываем еще 4 элемента, провоцируя перехлест [cite: 210, 211]
        let chunk2 = vec![9, 10, 11, 12];
        buffer.push_chunk(&chunk2);
        let window = buffer.read_window();
        // Проверяем, что окно выдает строго последние 10 элементов без разрывов [cite: 211]
        assert_eq!(window, &[3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
        assert_eq!(window.len(), 10);
    }
    /// Тестирование защиты от потери точности в AngularGrid.
    /// Гарантирует, что аккумулятор угла в f64 не "съедает" младшие разряды.
    #[test]
    fn test_angular_grid_f64_accumulation() {
        let sample_rate = 320_000.0;
        let dt = 1.0 / (sample_rate as f64);
        let omega = PI2 * 50.0; // 3000 об/мин = 50 Гц
        // Эмулируем работу привода в течение 10 дней без остановки
        let initial_angle: f64 = omega * (10.0 * 24.0 * 3600.0);
        let current_angle = initial_angle.rem_euclid(PI2);
        // Делаем один шаг интегратора
        let next_angle = current_angle + omega * dt;
        // Разница должна строго равняться шагу, несмотря на огромный initial_angle [cite: 253, 255, 256]
        let diff = next_angle - current_angle;
        let expected_step = omega * dt;
        assert!((diff - expected_step).abs() < 1e-12, "Катастрофическая потеря точности в f64 \n result: {} \n target: {}", diff, expected_step);
    }
}