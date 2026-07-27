use std::{f64::consts::TAU, ops::{Index, IndexMut}};
use sal_core::dbg::Dbg;
use crate::{Context, Eval, Frame, me};

/// Угловая сетка (фазовый профиль) для заданного окна временных отсчетов.
/// Представляет собой массив углов поворота вала, соответствующих каждому отсчету вибрации.
pub struct AngularGrid<Child> {
    child: Child,
    dbg: Dbg,
}
impl<Child> AngularGrid<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// ### Returns `Autocorrelation` new instance
    /// Вычисляет угловую сетку для новой порции данных.
    pub fn new(parent: impl Into<String>, child: Child) -> Self {
        let dbg = Dbg::new(parent, me::<Self>());
        Self {
            child,
            dbg,
        }
    }
}
impl<Child> Eval<Context, (Context, Phases<f32>)> for AngularGrid<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    /// Возвращает `Context` и угловую сетку `Phases`.
    #[inline]
    fn eval(&self, ctx: Context) -> (Context, Phases<f32>) {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return (ctx.pass_err(&self.dbg, "eval"), Phases::new(0));
        }
        // Делаем сброс ТОЛЬКО кратно полным оборотам (TAU) и только тут, больше ни где сбрасывать не нужно!
        if ctx.current_theta >= TAU {
            let full_turns = (ctx.current_theta / TAU).floor();
            ctx.current_theta -= full_turns * TAU;
        }
        let mut phases = Phases::new(Frame::SIZE);
        for i in 0..ctx.phases_size {
            ctx.current_theta += ctx.omega * ctx.dt;
            phases[i] = ctx.current_theta as f32;
        }
        (ctx, phases)
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
/// 
/// Угловая сетка в радианах (фазовый профиль) для заданного окна временных отсчетов.
/// Представляет собой массив углов поворота вала (в радианах), соответствующих каждому отсчету вибрации.
pub struct Phases<T> {
    vals: Vec<T>,
}
impl<T: crate::num_traits::Float> Phases<T> {
    pub fn new(size: usize) -> Self {
        if size > 0 {
            Self { vals: vec![T::zero(); size] }
        } else {
            Self { vals: vec![] }
        }
    }
    pub fn len(&self) -> usize {
        self.vals.len()
    }
}
// Трейт для чтения по индексу: phases[index]
impl<T> Index<usize> for Phases<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.vals[index] // Делегируем индексацию внутреннему вектору
    }
}

// Трейт для записи по индексу: phases[index] = value
impl<T> IndexMut<usize> for Phases<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.vals[index] // Возвращаем изменяемую ссылку
    }
}