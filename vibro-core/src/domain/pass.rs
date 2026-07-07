use crate::Eval;

/// Заглушка в конце цепочки вычислений.
/// Ничего не считает, возвращает Context.
pub struct Pass {}
impl Pass {
    /// ### Returns `Pass` new instance
    pub fn new() -> Self {
        Self {}
    }
}
impl<T> Eval<T, T> for Pass {
    //
    fn eval(&self, ctx: T) -> T {
        ctx
    }
    //
    fn exit(&self) {}
}
