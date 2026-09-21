use crate::num_traits::Float;

/// ### RMS | Среднеквадратичное значение амплитуды.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Rms<T: Float>(pub T);
impl<T: Float> Rms<T> {
    /// Возвращает внутреннее значение `T`.
    #[inline]
    pub fn value(self) -> T {
        self.0
    }
    /// Создает RMS из пикового значения (амплитуды) гармонического сигнала.
    /// RMS = Peak / sqrt(2)
    #[inline]
    pub fn from_sine_amplitude(peak: T) -> Self {
        let sqrt_2 = T::from(std::f64::consts::SQRT_2).unwrap();
        Self(peak / sqrt_2)
    }
    /// Рассчитывает пиковое значение (амплитуду) для синусоиды.
    /// Peak = RMS * sqrt(2)
    #[inline]
    pub fn to_sine_amplitude(&self) -> T {
        let sqrt_2 = T::from(std::f64::consts::SQRT_2).unwrap();
        self.0 * sqrt_2
    }
    /// Рассчитывает размах (пик-пик) для синусоиды.
    /// Peak-to-Peak = RMS * 2 * sqrt(2)
    #[inline]
    pub fn to_sine_double_amplitude(&self) -> T {
        let two_sqrt_2 = T::from(2.0 * std::f64::consts::SQRT_2).unwrap();
        self.0 * two_sqrt_2
    }
    /// Рассчитывает RMS из итератора значений амплитуды как сумму квадратов.
    #[inline]
    pub fn from_iter<I>(iter: I) -> Self 
    where 
        I: IntoIterator<Item = T>
    {
        let mut sum_squares = T::zero();
        let mut count = 0usize;

        for x in iter {
            sum_squares = sum_squares + x * x;
            count += 1;
        }

        if count == 0 {
            return Self(T::zero());
        }

        let n = T::from(count).unwrap();
        Self((sum_squares / n).sqrt())
    }
}
impl<T: Float + Default> Default for Rms<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

///
/// ### Phase | Значение фазы.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Phase<T: Float>(pub T);
impl<T: Float> Phase<T> {
    /// Создает Phase из угла в радианах.
    #[inline]
    pub fn from_radians(val: T) -> Self {
        Self(val)
    }
    /// Создает Phase из угла в градусах.
    #[inline]
    pub fn from_degrees(val: T) -> Self {
        Self(val.to_radians())
    }
    /// Возвращает значение фазы в радианах
    #[inline]
    pub fn to_radians(&self) -> T {
        self.0
    }
    /// Возвращает значение фазы в градусах
    #[inline]
    pub fn to_degrees(&self) -> T {
        self.0.to_degrees()
    }
    /// Нормализует фазу к диапазону [-PI; PI] (важно для ЦОС)
    #[inline]
    pub fn normalize_signed(&self) -> Self {
        let pi = T::from(std::f64::consts::PI).unwrap();
        let two_pi = T::from(std::f64::consts::TAU).unwrap();
        let mut r = self.0 % two_pi;
        if r > pi { r = r - two_pi; }
        else if r < -pi { r = r + two_pi; }
        Self(r)
    }
}
impl<T: Float + Default> Default for Phase<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

///
/// ### RPM | Значение частоты вращения об/мин.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Rpm<T: Float>(pub T);
impl<T: Float> Rpm<T> {
    /// Возвращает внутреннее значение `T`.
    #[inline]
    pub fn value(self) -> T {
        self.0
    }
    /// Расчитывает и возвращает частоту в Гц.
    /// frequency = RPM / 60
    #[inline]
    pub fn to_hz(&self) -> T {
        self.0 / T::from(60.0).unwrap()
    }
    /// Создает RPM из частоты в Гц.
    /// RPM = freq * 60
    #[inline]
    pub fn from_hz(val: T) -> Self {
        Self(val * T::from(60.0).unwrap())
    }
    /// Рассчитывает угловую скорость в рад/с.
    /// (omega = RPM * pi / 30)
    #[inline]
    pub fn to_rad_per_sec(&self) -> T {
        self.0 * T::from(std::f64::consts::PI / 30.0).unwrap()
    }
}
impl<T: Float + Default> Default for Rpm<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

super::impl_math!(Rms);
super::impl_math!(Phase);
super::impl_math!(Rpm);
