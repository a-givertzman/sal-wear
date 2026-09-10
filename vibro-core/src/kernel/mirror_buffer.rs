use std::cell::Cell;

use crate::Zero;


/// ### Зеркальный кольцевой буфер (Mirrored Ring Buffer)
///
/// Специализированная структура данных для систем цифровой обработки сигналов (DSP).
/// Обеспечивает получение непрерывного хронологического окна отсчетов длины `N`
/// за константное время `O(1)` без операции копирования и сдвига элементов (zero-copy read).
///
/// #### Принцип работы и выравнивание памяти
/// Структура выделяет непрерывный массив размером `2N` (где `N = capacity`).
/// Каждый входящий отсчет `x[i]` записывается одновременно по двум индексам:
/// `pos` и `pos + N`. 
///
/// Это гарантирует, что для любого индекса записи `write_idx` в диапазоне `[0, N-1]`
/// срез памяти `&buffer[write_idx..write_idx + N]` всегда содержит последовательность
/// из `N` последних элементов, расположенных в строгом хронологическом порядке.
///
/// ```text
/// Физическая память (размер `2N`):
/// [     Окно А (размер N)     ][     Окно Б (зеркало, размер N)     ]
/// [ x0 | x1 | x2 | ... | xN-1 ][ x0 | x1 | x2 |   ...   | xN-1 ]
///             ^                                 ^
///             |--- Текущее окно чтения (длина N) |
/// ```
///
/// #### Применение
/// Разработан для алгоритмов, критичных к задержкам (Real-Time DSP):
/// * Автокорреляционный анализ (AMDF/ASDF).
/// * Быстрое преобразование Фурье (БПФ / FFT) на скользящем окне.
/// * Цифровая фильтрация (КИХ / FIR-фильтры).
pub struct MirroredBuffer<T> {
    /// Внутренний массив размером `2 * capacity`.
    buffer: Vec<T>,
    /// Логический размер буфера `capacity`. Длина скользящего окна анализа.
    capacity: usize,
    /// Текущий индекс записи (от 0 до N-1).
    write_idx: usize,
    /// Текущее фактическое число накопленных элементов.
    len: usize,
    /// Шаг скользящего окна (Hop Size).
    /// Количество новых элементов, после поступления которых буфер вернет срез в `pop_window`.
    hop_size: usize,
    /// Количество элементов, накопленное с момента последнего среза `pop_window`.
    hop_len: Cell<usize>,
}
impl<T: Copy + Zero> MirroredBuffer<T> {
    /// ### Создает новый экземпляр буфера `MirroredBuffer` с выделением памяти на куче.
    ///
    /// **Аллокация памяти**. Выполняется строго один раз при инициализации.
    /// Выделяется массив размером `capacity * 2 * sizeof(T)`.
    ///
    /// - `capacity` - Требуемая длина окна анализа.
    /// 
    /// **Примечание**: По умолчанию размер скользящего окна `hop_size` равен `capacity`, без наложения.
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![T::zero(); capacity * 2],
            capacity,
            write_idx: 0,
            len: 0,
            hop_size: capacity,
            hop_len: Cell::new(0),
        }
    }
    /// ### Настраивает шаг скользящего окна (Hop Size).
    /// 
    /// - `hop_size` - Количество элементов, после поступления которых, буфер вернет срез в `pop_window`.
    /// 
    /// По умолчанию `hop_size = capacity`.
    /// 
    /// #### Panic
    /// Метод паникует если `hop_size` меньше `1` или больше `capacity`
    pub fn with_hop_size(mut self, hop_size: usize) -> Self {
        if hop_size < 1 || hop_size > self.capacity {
            panic!("{}.with_hop | Некорректный hop_size {}, должен быть от 1 до capacity ({})", crate::me::<Self>(), hop_size, self.capacity);
        }
        self.hop_size = hop_size.max(1);
        self
    }
    /// ### Возвращает логический размер буфера `capacity`. Длину скользящего окна.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
    /// ### Возвращает текущее фактическое число накопленных элементов
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }
    /// ### Возвращает `true`, если буфер заполнен до целевой емкости `capacity`.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.len >= self.capacity
    }
    /// ### Добавляет новую выборку в буфер.
    /// 
    /// #### Особенности реализации
    /// Функция автоматически обрабатывает переход через границу кольцевого буфера
    /// и дублирует данные во вторую (зеркальную) половину массива.
    ///
    /// #### Panic
    /// Паникует с сообщением `"Размер выборки превышает вместимость буфера"`, 
    /// если длина среза `chunk.len()` больше логической емкости `capacity`.
    ///
    /// - `chunk` - срез новых данных, длина не должна превышать capacity.
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
        self.hop_len.update(|v| v + m);
    }
    /// ### Возвращает непрерывный срез памяти длиной `capacity`.
    ///
    /// #### Хронология данных
    /// Возвращаемый срез отсортирован по времени:
    /// * `slice[0]` — самый старый отсчет в текущем окне.
    /// * `slice[N-1]` — самый свежий (последний записанный) отсчет.
    ///
    /// #### Эффективность
    /// Операция выполняется за **O(1)** (константное время). Не выполняет копирования
    /// элементов и операций сдвига памяти.
    ///
    /// #### Важно
    /// Если [`Self::is_full`] возвращает `false`, то начало среза будет содержать 
    /// нулевые значения инициализации (`T::zero()`), так как буфер еще не заполнился.
    #[inline]
    pub fn read_window(&self) -> &[T] {
        &self.buffer[self.write_idx..self.write_idx + self.capacity]
    }
    /// ### Возвращает непрерывный срез памяти длиной `capacity` если накоплено `hop_size` элементов с последнего среза.
    /// 
    /// #### Хронология данных
    /// Возвращаемый срез отсортирован по времени:
    /// * `slice[0]` — самый старый отсчет в текущем окне.
    /// * `slice[N-1]` — самый свежий (последний записанный) отсчет.
    ///
    /// #### Эффективность
    /// Операция выполняется за **O(1)** (константное время). Не выполняет копирования
    /// элементов и операций сдвига памяти.
    ///
    #[inline]
    pub fn pop_window(&self) -> Option<&[T]> {
        if self.len >= self.capacity && self.hop_len.get() >= self.hop_size {
            self.hop_len.set(0);
            Some(&self.buffer[self.write_idx..self.write_idx + self.capacity])
        } else {
            None
        }
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