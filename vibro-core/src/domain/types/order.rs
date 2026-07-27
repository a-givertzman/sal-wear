/// Номер порядка для квантования
#[derive(Debug, Clone, Copy, PartialOrd)]
pub struct Order(pub f64);
impl Order {
    /// Возвращает внутреннее значение `T`.
    #[inline]
    pub fn value(self) -> f64 {
        self.0
    }
}
impl PartialEq for Order {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // Ваша логика сравнения, например, паника при NaN или приравнивание NaN к NaN
        self.0.to_bits() == other.0.to_bits()
    }
}
impl Eq for Order {}
impl std::hash::Hash for Order {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}