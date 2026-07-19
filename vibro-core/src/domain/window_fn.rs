use rustfft::num_complex::Complex;
use sal_core::{dbg::Dbg, error::Error};
use crate::me;

/// ### Оконная функция для подготовки данных к FFT
///
/// **Назначение**
/// Используется для сгаживания разрывов на краях выборки перед выполнением FFT.
/// Без такой подготовки возникает эффект растекания спектра (spectral leakage).
/// Из-за которого ложные частотные составляющие искажают картину получаемого спектра.
/// 
/// **Варианты оконных функций**
/// - Прямоугольное окно (Rectengular).
/// - Окно ханна (Hann/Hanning).
/// - Окно Хэминга (Hamming).
/// - Окно Блэкмана (Blackman).
/// - Окно Кайзера (Kaiser).
pub struct WindowFn<T> {
    /// Массив коэффициентов оконной функции.
    lookup: Vec<T>,
    dbg: Dbg,
}
impl<T: rustfft::num_traits::Float + rustfft::num_traits::FloatConst> WindowFn<T> {
    /// ### Оконная функция для подготовки данных к FFT.
    /// 
    /// Прямоугольное окно (Rectengular) обрезает сигнал без сглаживания краев.
    /// **Плюсы**: Максимальное частотное разрешение (самый узкий главный лепесток).
    /// **Минусы**: Худшее подавление боковых лепестков (всего -13 дБ). Высокий уровень растекания спектра.
    /// **Применение**: Анализ переходных процессов, синусоид с близкими частотами и равными амплитудами.
    /// 
    /// - `size` - Размер исходной выбороки.
    /// - `width` - Размер прямоугольного окна (активной зоны), не должен превышать размер `size`.
    /// - `offset` - Сдвиг окна `width` внутри выбоки `size`, не должен превышать размер `size - width`.
    pub fn rect(parent: &Dbg, size: usize, width: usize, offset: usize) -> Result<Self, Error> {
        let dbg = Dbg::new(parent, me::<Self>());
        if width > size {
            return Err(Error::new(&dbg, "rect")
                .err(format!("Размер окна width ({width}) превысил размер выборки size ({size})")));
        }
        let end = offset.checked_add(width).ok_or_else(|| {
            Error::new(&dbg, "rect").err("Переполнение (overflow) при вычислении границ окна")
        })?;
        if end > size {
            return Err(Error::new(&dbg, "rect")
                .err(format!("Сдвиг окна offset ({offset}) с учетом ширины ({width}) выходит за пределы выборки size ({size})")));
        }
        let lookup = (0..size).map(|i| {
            if i >= offset && i < end {
                T::one()
            } else {
                T::zero()
            }
        }).collect();
        Ok(Self {
            lookup,
            dbg,
        })        
    }
    /// ### Оконная функция для подготовки данных к FFT.
    /// 
    /// Окно Ханна (Hann/Hanning) - Форма приподнятого косинуса, плавно спадающая до нуля на краях.
    /// **Плюсы**: Хорошее подавление боковых лепестков (всего -32 дБ), быстрое затухание спектральных хвостов.
    /// **Минусы**: Главный лепесток в два раза шире по сравнению с прямоугольным окном.
    /// **Применение**: Универсальное окно общего назначения для непрерывных сигналов и случайных шумов.
    /// 
    /// - `size` - Размер исходной выбороки.
    /// - `width` - Размер активной зоны окна Ханна, не должен превышать размер `size`.
    /// - `offset` - Сдвиг окна `width` внутри выборки `size`, не должен превышать размер `size - width`.
    pub fn hann(parent: &Dbg, size: usize, width: usize, offset: usize) -> Result<Self, Error> {
        let dbg = Dbg::new(parent, me::<Self>());
        if width > size {
            return Err(Error::new(&dbg, "rect")
                .err(format!("Размер окна width ({width}) превысил размер выборки size ({size})")));
        }
        let end = offset.checked_add(width).ok_or_else(|| {
            Error::new(&dbg, "hann").err("Переполнение (overflow) при вычислении границ окна")
        })?;
        if end > size {
            return Err(Error::new(&dbg, "hann").err(format!(
                "Сдвиг окна offset ({offset}) с учетом ширины ({width}) выходит за пределы выборки size ({size})"
            )));
        }
        // 2. Подготовка констант для формулы косинуса внутри активной зоны
        // Чтобы избежать деления на ноль при width <= 1, делаем безопасный знаменатель (M - 1)
        let denom = if width > 1 {
            T::from(width - 1).unwrap_or_else(T::one)
        } else {
            T::one()
        };
        let two = T::from(2.0).unwrap();
        let zero_five = T::from(0.5).unwrap();
        let tau = T::PI() * two; // 2 * PI
        let lookup = (0..size).map(|i| {
            if i >= offset && i < end {
                // Локальный индекс сэмпла внутри самого окна (от 0 до width - 1)
                let k = T::from(i - offset).unwrap();
                // Формула: 0.5 * (1.0 - cos(2*PI * k / (width - 1)))
                zero_five * (T::one() - (tau * k / denom).cos())
            } else {
                T::zero()
            }
        }).collect();
        Ok(Self {
            lookup,
            dbg,
        })        
    }
    /// ### Оконная функция для подготовки данных к FFT.
    /// 
    /// Окно Хэминга (Hamming) - Форма приподнятого косинуса, плавно спадающая на краях, но не до нуля, имеет небольшую ступеньку (около 8%).
    /// **Плюсы**: Минимизирует уровень самого первого (ближайшего) бокового лепестка (до -43 дБ)
    /// **Минусы**: Дальние боковые лепестки затухают медленнее чем у окна Ханна.
    /// **Применение**: Оптимально для узкополосных сигналов, когда нужно разделить две близкие по частоте гармоники с сильно отличающимися амплитудами.
    /// 
    /// - `size` - Размер исходной выбороки.
    /// - `width` - Размер активной зоны окна Хэмминга, не должен превышать размер `size`.
    /// - `offset` - Сдвиг окна `width` внутри выборки `size`, не должен превышать размер `size - width`.
    pub fn hamming(parent: &Dbg, size: usize, width: usize, offset: usize) -> Result<Self, Error> {
        let dbg = Dbg::new(parent, me::<Self>());
        if width > size {
            return Err(Error::new(&dbg, "rect")
                .err(format!("Размер окна width ({width}) превысил размер выборки size ({size})")));
        }
        let end = offset.checked_add(width).ok_or_else(|| {
            Error::new(&dbg, "hann").err("Переполнение (overflow) при вычислении границ окна")
        })?;
        if end > size {
            return Err(Error::new(&dbg, "hann").err(format!(
                "Сдвиг окна offset ({offset}) с учетом ширины ({width}) выходит за пределы выборки size ({size})"
            )));
        }
        // 2. Подготовка констант для формулы Хэмминга
        let denom = if width > 1 {
            T::from(width - 1).unwrap_or_else(T::one)
        } else {
            T::one()
        };
        // Коэффициенты окна Хэмминга: alpha = 0.54, beta = 0.46
        let alpha = T::from(0.54).unwrap();
        let beta = T::from(0.46).unwrap();
        let tau = T::PI() * T::from(2.0).unwrap(); // 2 * PI
        // 3. Быстрое заполнение lookup-таблицы в один проход
        let lookup = (0..size).map(|i| {
            if i >= offset && i < end {
                // Локальный индекс сэмпла внутри окна (0 .. width - 1)
                let k = T::from(i - offset).unwrap();
                // Формула: alpha - beta * cos(2*PI * k / (width - 1))
                alpha - beta * (tau * k / denom).cos()
            } else {
                T::zero()
            }
        }).collect();
        Ok(Self {
            lookup,
            dbg,
        })        
    }
    /// ### Оконная функция для подготовки данных к FFT.
    /// 
    /// Окно Блэкмана (Blackman) - Форма суммы трех косинусов, обеспечиват еще юолее плавный спад к краям.
    /// **Плюсы**: Отличное подавление юоковых лепестков (-58 дБ)
    /// **Минусы**: Главный лепесток становиться еще шире (в три раза шире прямоугольного), снижая частотное разрешение.
    /// **Применение**: Анализ сигналов с широким динамическим диапазоном, где важно убрать влияние сильных частот на слабые.
    /// 
    /// - `size` - Размер исходной выбороки.
    /// - `width` - Размер активной зоны окна Блэкмана, не должен превышать размер `size`.
    /// - `offset` - Сдвиг окна `width` внутри выборки `size`, не должен превышать размер `size - width`.
    pub fn blackman(parent: &Dbg, size: usize, width: usize, offset: usize) -> Result<Self, Error> {
        let dbg = Dbg::new(parent, me::<Self>());
        if width > size {
            return Err(Error::new(&dbg, "rect")
                .err(format!("Размер окна width ({width}) превысил размер выборки size ({size})")));
        }
        let end = offset.checked_add(width).ok_or_else(|| {
            Error::new(&dbg, "hann").err("Переполнение (overflow) при вычислении границ окна")
        })?;
        if end > size {
            return Err(Error::new(&dbg, "hann").err(format!(
                "Сдвиг окна offset ({offset}) с учетом ширины ({width}) выходит за пределы выборки size ({size})"
            )));
        }
        // 2. Подготовка констант для формулы Блэкмана
        let denom = if width > 1 {
            T::from(width - 1).unwrap_or_else(T::one)
        } else {
            T::one()
        };
        // Стандартные коэффициенты Блэкмана
        let a0 = T::from(0.42).unwrap();
        let a1 = T::from(0.50).unwrap();
        let a2 = T::from(0.08).unwrap();
        let pi = T::PI();
        let two_pi = pi * T::from(2.0).unwrap();
        let four_pi = pi * T::from(4.0).unwrap();
        // 3. Быстрая генерация массива коэффициентов в один проход
        let lookup = (0..size).map(|i| {
            if i >= offset && i < end {
                // Локальный индекс сэмпла внутри окна (0 .. width - 1)
                let k = T::from(i - offset).unwrap();
                // Аргументы для косинусов
                let arg1 = two_pi * k / denom;
                let arg2 = four_pi * k / denom;
                // Формула: a0 - a1 * cos(2*pi*k / (M-1)) + a2 * cos(4*pi*k / (M-1))
                a0 - a1 * arg1.cos() + a2 * arg2.cos()
            } else {
                T::zero()
            }
        }).collect();
        Ok(Self {
            lookup,
            dbg,
        })        
    }
    /// Вспомогательная функция аппроксимации модифицированной функции Бесселя I0(x)
    /// с помощью степенного ряда (быстро сходится, достаточно ~25 итераций).
    fn bessel_i0(x: T) -> T {
        let mut sum = T::one();
        let mut delta = T::one();
        let x_half = x / T::from(2.0).unwrap();
        // Ограничиваемся 25 итерациями для гарантированной точности f64
        for k in 1..26 {
            let k_t = T::from(k).unwrap();
            delta = delta * (x_half / k_t);
            let term = delta * delta;
            sum = sum + term;
            // Если прибавка стала ничтожно малой, прерываем цикл досрочно
            if term < T::epsilon() {
                break;
            }
        }
        sum
    }
    /// ### Оконная функция для подготовки данных к FFT.
    /// 
    /// Окно Кайзера (Kaiser) - Параметрическое окно на основе модифицированных функций Бесселя
    /// **Плюсы**: Универсальность. Имеет настраиваемый параметр β (или α), который позволяет плавно
    /// регулировать баланс между шириной главного лепестка и уровнем боковых.
    /// **Минусы**: Высокая вычислительная сложность инициализации (расчет функций Бесселя).
    /// **Применение**: Проектирование КИХ-фильтров (FIR) и прецезионный спектральный анализ.
    /// 
    /// - `size` - Размер исходной выбороки.
    /// - `width` - Размер активной зоны окна Кайзера, не должен превышать размер `size`.
    /// - `offset` - Сдвиг окна `width` внутри выборки `size`, не должен превышать размер `size - width`.
    /// - `beta` - Коэффициент формы окна. `beta = 0` дает прямоугольное окно,
    ///     - `beta = 5.44` близко к Хэммингу,
    ///     - `beta = 8.96` дает подавление боковых лепестков до -90 дБ.
    pub fn kaiser(parent: &Dbg, size: usize, width: usize, offset: usize, betta: T) -> Result<Self, Error> {
        let dbg = Dbg::new(parent, me::<Self>());
        if width > size {
            return Err(Error::new(&dbg, "rect")
                .err(format!("Размер окна width ({width}) превысил размер выборки size ({size})")));
        }
        let end = offset.checked_add(width).ok_or_else(|| {
            Error::new(&dbg, "hann").err("Переполнение (overflow) при вычислении границ окна")
        })?;
        if end > size {
            return Err(Error::new(&dbg, "hann").err(format!(
                "Сдвиг окна offset ({offset}) с учетом ширины ({width}) выходит за пределы выборки size ({size})"
            )));
        }
        // 2. Подготовка констант для формулы Кайзера
        let denom_m = if width > 1 {
            T::from(width - 1).unwrap_or_else(T::one)
        } else {
            T::one()
        };
        // Знаменатель формулы — функция Бесселя от самого параметра beta
        let i0_beta = Self::bessel_i0(betta);
        let two = T::from(2.0).unwrap();
        // 3. Быстрая генерация массива коэффициентов в один проход
        let lookup = (0..size).map(|i| {
            if i >= offset && i < end {
                // Локальный индекс сэмпла внутри окна (0 .. width - 1)
                let k = T::from(i - offset).unwrap();
                // Нормированное значение от -1.0 до 1.0 внутри окна: (2*k / (M-1)) - 1.0
                let normalized = (two * k / denom_m) - T::one();
                // Вычисляем подкоренное выражение: 1.0 - normalized^2
                let inner = T::one() - (normalized * normalized);
                // Защита от микроскопических отрицательных значений из-за погрешности float
                let inner_safe = if inner < T::zero() { T::zero() } else { inner };
                // Вычисляем числитель: I0(beta * sqrt(1 - normalized^2))
                let i0_num = Self::bessel_i0(betta * inner_safe.sqrt());
                // Итоговый коэффициент сэмпла
                i0_num / i0_beta
            } else {
                T::zero()
            }
        }).collect();
        Ok(Self {
            lookup,
            dbg,
        })        
    }
    /// Применяет коэффициенты окна к выборке
    #[inline]
    fn eval(&self, samples: &mut [Complex<T>]) {
        for (sample, k) in samples.iter_mut().zip(&self.lookup) {
            sample.re = *k * sample.re;
        }
    }
}
