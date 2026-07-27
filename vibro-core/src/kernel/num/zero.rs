/// ### Трейт для типов, обладающих свойством аддитивной идентичности (математического нуля).
///
/// Реализация трейта должна строго соблюдать свойства аддитивного нейтрального элемента:
/// - Правая идентичность: a + 0 = a ∀ a ∈ Self
/// - Левая идентичность: 0 + a = a ∀ a ∈ Self
pub trait Zero: Sized {
    /// ### Возвращает математический ноль для `Self`, `0`.
    /// 
    /// **Требования к чистоте (Purity)**
    ///
    /// Метод zero() обязан быть чистой функцией.
    /// Он должен гарантированно возвращать идентичный результат при каждом вызове.
    /// Запрещено использовать любое внешнее изменяемое состояние,
    /// включая Thread-Local Storage (TLS) или static mut переменные.
    fn zero() -> Self;
    /// Сбрасывает значение `self` в математический ноль `0`.
    fn set_zero(&mut self) {
        *self = Zero::zero();
    }
    /// Возвращает `true` если `self` математический ноль, `0`.
    fn is_zero(&self) -> bool;
}
/// Расширение трейта Zero для типов, чей нулевой элемент может быть вычислен на этапе компиляции.
/// Позволяет использовать нейтральный элемент в контекстах (например, при инициализации статических массивов).
pub trait ConstZero: Zero {
    /// ### Математический ноль, `0`.
    const ZERO: Self;
}

macro_rules! zero_impl {
    ($t:ty, $v:expr) => {
        impl Zero for $t {
            #[inline]
            fn zero() -> $t {
                $v
            }
            #[inline]
            fn is_zero(&self) -> bool {
                *self == $v
            }
        }
        impl ConstZero for $t {
            const ZERO: Self = $v;
        }
    };
}

zero_impl!(usize, 0);
zero_impl!(u8, 0);
zero_impl!(u16, 0);
zero_impl!(u32, 0);
zero_impl!(u64, 0);
zero_impl!(u128, 0);

zero_impl!(isize, 0);
zero_impl!(i8, 0);
zero_impl!(i16, 0);
zero_impl!(i32, 0);
zero_impl!(i64, 0);
zero_impl!(i128, 0);

zero_impl!(f32, 0.0);
zero_impl!(f64, 0.0);

impl<T: Copy + Zero> Zero for crate::num_complex::Complex<T> {
    fn zero() -> Self {
        crate::num_complex::Complex { re: T::zero(), im: T::zero() }
    }
    fn is_zero(&self) -> bool {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_integer_zeros() {
        // --- БЕЗЗНАКОВЫЕ ЦЕЛЫЕ (UNSIGNED INTEGERS) ---
        // usize
        assert_eq!(usize::zero(), 0_usize);
        assert!(0_usize.is_zero());
        assert!(!1_usize.is_zero());
        // u8
        assert_eq!(u8::zero(), 0_u8);
        assert!(0_u8.is_zero());
        assert!(!1_u8.is_zero());
        // u16
        assert_eq!(u16::zero(), 0_u16);
        assert!(0_u16.is_zero());
        assert!(!1_u16.is_zero());
        // u32
        assert_eq!(u32::zero(), 0_u32);
        assert!(0_u32.is_zero());
        assert!(!1_u32.is_zero());
        // u64
        assert_eq!(u64::zero(), 0_u64);
        assert!(0_u64.is_zero());
        assert!(!1_u64.is_zero());
        // u128
        assert_eq!(u128::zero(), 0_u128);
        assert!(0_u128.is_zero());
        assert!(!1_u128.is_zero());
        // --- ЗНАКОВЫЕ ЦЕЛЫЕ (SIGNED INTEGERS) ---
        // isize
        assert_eq!(isize::zero(), 0_isize);
        assert!(0_isize.is_zero());
        assert!(!1_isize.is_zero());
        assert!(!(-1_isize).is_zero());
        // i8
        assert_eq!(i8::zero(), 0_i8);
        assert!(0_i8.is_zero());
        assert!(!1_i8.is_zero());
        assert!(!(-1_i8).is_zero());
        // i16
        assert_eq!(i16::zero(), 0_i16);
        assert!(0_i16.is_zero());
        assert!(!1_i16.is_zero());
        assert!(!(-1_i16).is_zero());
        // i32
        assert_eq!(i32::zero(), 0_i32);
        assert!(0_i32.is_zero());
        assert!(!1_i32.is_zero());
        assert!(!(-1_i32).is_zero());
        // i64
        assert_eq!(i64::zero(), 0_i64);
        assert!(0_i64.is_zero());
        assert!(!1_i64.is_zero());
        assert!(!(-5_i64).is_zero());
        // i128
        assert_eq!(i128::zero(), 0_i128);
        assert!(0_i128.is_zero());
        assert!(!1_i128.is_zero());
        assert!(!(-1_i128).is_zero());
    }
    #[test]
    fn test_const_zero() {
        // Проверка ассоциированных констант
        assert_eq!(<usize as ConstZero>::ZERO, 0);
        assert_eq!(<i32 as ConstZero>::ZERO, 0);
        assert_eq!(<f64 as ConstZero>::ZERO, 0.0);
    }
    #[test]
    fn test_set_zero() {
        let mut val_int = 42_u64;
        val_int.set_zero();
        assert!(val_int.is_zero());
        assert_eq!(val_int, 0);
        let mut val_float = -10.5_f32;
        val_float.set_zero();
        assert!(val_float.is_zero());
        assert_eq!(val_float, 0.0);
    }
    #[test]
    fn test_float_edge_cases() {
        // Базовая проверка f32/f64
        assert!(0.0_f32.is_zero());
        assert!(!1.5_f64.is_zero());
        // Специфика IEEE 754: отрицательный нуль (-0.0)
        let neg_zero_f32 = -0.0_f32;
        let neg_zero_f64 = -0.0_f64;
        assert!(neg_zero_f32.is_zero(), "Negative zero for f32 should be classified as zero");
        assert!(neg_zero_f64.is_zero(), "Negative zero for f64 should be classified as zero");
        // Проверка, что NaN или Бесконечность не определяются как нуль
        assert!(!f32::NAN.is_zero());
        assert!(!f64::INFINITY.is_zero());
    }
    #[test]
    fn test_mathematical_property() {
        // x + 0 == x
        let x = 123_i32;
        assert_eq!(x + i32::zero(), x);
        let y = 55.5_f64;
        assert_eq!(y + f64::zero(), y);
    }
}
