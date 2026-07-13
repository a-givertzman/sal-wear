use std::f64::consts::TAU;

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
    /// * `samples` - Окно отсчетов для расчета (должно вмещать минимум 2-3 оборота вала).
    /// * `rough_rpm` - Приблизительная частота вращения с тахометра (об/мин).
    /// * `sample_rate` - Частота дискретизации в Гц.
    /// * `initial_angle` - Угол θ на начало окна.
    pub fn new(parent: impl Into<String>, child: Child) -> Self {
        let dbg = Dbg::new(parent, me::<Self>());
        Self {
            child,
            dbg,
        }
    }
}
impl<Child> Eval<Context, Context> for AngularGrid<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    //
    fn eval(&self, ctx: Context) -> Context {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        ctx.phases = [0.0; Frame::SIZE];
        for i in 0..ctx.phases.len() {
            ctx.current_theta += ctx.omega * ctx.dt;
            if ctx.current_theta >= TAU {
                ctx.current_theta -= TAU;
            }
            ctx.phases[i] = ctx.current_theta as f32;
        }
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}