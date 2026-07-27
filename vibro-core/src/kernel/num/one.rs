/// ### Трейт для типов, обладающих свойством математической единицы.
///
/// Реализация трейта должна строго соблюдать свойства аддитивного нейтрального элемента:
/// - Правая идентичность: a + 1 = a ∀ a ∈ Self
/// - Левая идентичность: 1 + a = a ∀ a ∈ Self
pub trait One: Sized {
    /// ### Возвращает математическую единицу для `Self`, `1`.
    /// 
    /// **Требования к чистоте (Purity)**
    ///
    /// Метод one() обязан быть чистой функцией.
    /// Он должен гарантированно возвращать идентичный результат при каждом вызове.
    /// Запрещено использовать любое внешнее изменяемое состояние,
    /// включая Thread-Local Storage (TLS) или static mut переменные.
    fn one() -> Self;
    /// Сбрасывает значение `self` в математическая единица `1`.
    fn set_one(&mut self) {
        *self = One::one();
    }
    /// Возвращает `true` если `self` математическая единица, `1`.
    fn is_one(&self) -> bool;
}
/// Расширение трейта One для типов, чей единичный элемент может быть вычислен на этапе компиляции.
/// Позволяет использовать единичный элемент в контекстах.
pub trait ConstOne: One {
    /// ### Математическая единица, `1`.
    const ONE: Self;
}

macro_rules! one_impl {
    ($t:ty, $v:expr) => {
        impl One for $t {
            #[inline]
            fn one() -> $t {
                $v
            }
            #[inline]
            fn is_one(&self) -> bool {
                *self == $v
            }
        }
        impl ConstOne for $t {
            const ONE: Self = $v;
        }
    };
}

one_impl!(usize, 1);
one_impl!(u8, 1);
one_impl!(u16, 1);
one_impl!(u32, 1);
one_impl!(u64, 1);
one_impl!(u128, 1);

one_impl!(isize, 1);
one_impl!(i8, 1);
one_impl!(i16, 1);
one_impl!(i32, 1);
one_impl!(i64, 1);
one_impl!(i128, 1);

one_impl!(f32, 1.0);
one_impl!(f64, 1.0);

impl<T: Copy + One> One for crate::num_complex::Complex<T> {
    fn one() -> Self {
        crate::num_complex::Complex { re: T::one(), im: T::one() }
    }
    fn is_one(&self) -> bool {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_integer_ones() {
        // --- БЕЗЗНАКОВЫЕ ЦЕЛЫЕ (UNSIGNED INTEGERS) ---
        // usize
        assert_eq!(usize::one(), 1_usize);
        assert!(1_usize.is_one());
        assert!(!0_usize.is_one());
        // u8
        assert_eq!(u8::one(), 1_u8);
        assert!(1_u8.is_one());
        assert!(!0_u8.is_one());
        // u16
        assert_eq!(u16::one(), 1_u16);
        assert!(1_u16.is_one());
        assert!(!0_u16.is_one());
        // u32
        assert_eq!(u32::one(), 1_u32);
        assert!(1_u32.is_one());
        assert!(!0_u32.is_one());
        // u64
        assert_eq!(u64::one(), 1_u64);
        assert!(1_u64.is_one());
        assert!(!0_u64.is_one());
        // u128
        assert_eq!(u128::one(), 1_u128);
        assert!(1_u128.is_one());
        assert!(!0_u128.is_one());
        // --- ЗНАКОВЫЕ ЦЕЛЫЕ (SIGNED INTEGERS) ---
        // isize
        assert_eq!(isize::one(), 1_isize);
        assert!(1_isize.is_one());
        assert!(!0_isize.is_one());
        assert!(!(-1_isize).is_one());
        // i8
        assert_eq!(i8::one(), 1_i8);
        assert!(1_i8.is_one());
        assert!(!0_i8.is_one());
        assert!(!(-1_i8).is_one());
        // i16
        assert_eq!(i16::one(), 1_i16);
        assert!(1_i16.is_one());
        assert!(!0_i16.is_one());
        assert!(!(-1_i16).is_one());
        // i32
        assert_eq!(i32::one(), 1_i32);
        assert!(1_i32.is_one());
        assert!(!0_i32.is_one());
        assert!(!(-1_i32).is_one());
        // i64
        assert_eq!(i64::one(), 1_i64);
        assert!(1_i64.is_one());
        assert!(!0_i64.is_one());
        assert!(!(-1_i64).is_one());
        // i128
        assert_eq!(i128::one(), 1_i128);
        assert!(1_i128.is_one());
        assert!(!0_i128.is_one());
        assert!(!(-1_i128).is_one());
    }
    #[test]
    fn test_const_one() {
        // Проверка ассоциированных констант
        assert_eq!(<usize as ConstOne>::ONE, 1);
        assert_eq!(<i32 as ConstOne>::ONE, 1);
        assert_eq!(<f64 as ConstOne>::ONE, 1.0);
    }
    #[test]
    fn test_set_one() {
        let mut val_int = 42_u64;
        val_int.set_one();
        assert!(val_int.is_one());
        assert_eq!(val_int, 1);
        let mut val_float = -10.5_f32;
        val_float.set_one();
        assert!(val_float.is_one());
        assert_eq!(val_float, 1.0);
    }
    #[test]
    fn test_float_edge_cases() {
        // Базовая проверка f32/f64
        assert!(1.0_f32.is_one());
        assert!(!1.5_f64.is_one());
        // Специфика IEEE 754: отрицательная единица (-1.0)
        let neg_one_f32 = -1.0_f32;
        let neg_one_f64 = -1.0_f64;
        assert!(!neg_one_f32.is_one(), "Negative one for f32 should not be classified as one");
        assert!(!neg_one_f64.is_one(), "Negative one for f64 should not be classified as one");
        // Проверка, что NaN или Бесконечность не определяются как нуль
        assert!(!f32::NAN.is_one());
        assert!(!f64::INFINITY.is_one());
    }
    #[test]
    fn test_mathematical_property() {
        // x + 1 == x + 1
        let x = 123_i32;
        assert_eq!(x + i32::one(), i32::one() + x);
        let y = 55.5_f64;
        assert_eq!(y + f64::one(), f64::one() + y);
    }
}
