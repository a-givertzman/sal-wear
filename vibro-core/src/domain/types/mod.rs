mod types;
pub use types::*;
mod order;
pub use order::*;

macro_rules! impl_math {
    ($Target:ident) => {
        // Сложение двух одинаковых величин (RPM + RPM, Phase + Phase)
        impl<T: Float> std::ops::Add for $Target<T> {
            type Output = Self;
            #[inline] fn add(self, rhs: Self) -> Self { Self(self.0 + rhs.0) }
        }
        impl<T: Float> std::ops::AddAssign for $Target<T> {
            #[inline] fn add_assign(&mut self, rhs: Self) { self.0 = self.0 + rhs.0; }
        }
        // Вычитание двух одинаковых величин
        impl<T: Float> std::ops::Sub for $Target<T> {
            type Output = Self;
            #[inline] fn sub(self, rhs: Self) -> Self { Self(self.0 - rhs.0) }
        }
        impl<T: Float> std::ops::SubAssign for $Target<T> {
            #[inline] fn sub_assign(&mut self, rhs: Self) { self.0 = self.0 - rhs.0; }
        }
        // Умножение одинаковых величин
        impl<T: Float> std::ops::Mul<Self> for $Target<T> {
            type Output = Self;
            #[inline] fn mul(self, rhs: Self) -> Self { Self(self.0 * rhs.0) }
        }
        // Деление одинаковых величин
        impl<T: Float> std::ops::Div<Self> for $Target<T> {
            type Output = Self;
            #[inline] fn div(self, rhs: Self) -> Self { Self(self.0 / rhs.0) }
        }
        // Умножение на скаляр (RPM * 2.0)
        impl<T: Float> std::ops::Mul<T> for $Target<T> {
            type Output = Self;
            #[inline] fn mul(self, rhs: T) -> Self { Self(self.0 * rhs) }
        }
        // Деление на скаляр (RPM / 2.0)
        impl<T: Float> std::ops::Div<T> for $Target<T> {
            type Output = Self;
            #[inline] fn div(self, rhs: T) -> Self { Self(self.0 / rhs) }
        }
    };
}
pub(self) use impl_math;