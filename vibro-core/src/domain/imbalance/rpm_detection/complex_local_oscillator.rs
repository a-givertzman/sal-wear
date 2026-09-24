use std::f64::consts::PI;

use rustfft::num_complex::Complex;
use sal_core::dbg::Dbg;
use crate::{ImbContext, Eval};

///
/// Комплексный гетеродин (снос 1X на нулевую частоту)
pub struct ComplexLocalOscillator<Child> {
    // Грубая частота вращения вала, об/мин.
    rpm_net : f64,
    /// Частота децимации: f_sample / factor_decimation
    f_decimation: f64, 
    child: Child,
    dbg: Dbg,
}

impl<Child> ComplexLocalOscillator<Child>
where
    Child: Eval<ImbContext, ImbContext> + Send + 'static,
{
    /// ### Returns `ComplexLocalOscillator` new instance
    /// - `parent` - Идентификатор родительской сущности (для отладки)
    /// - `child` - Дочерний (предыдущий) шаг вычислений
    pub fn new(parent: &Dbg, child: Child, rpm_net: f64, f_decimation: f64) -> Self {
        let dbg = Dbg::new(parent, crate::me::<Self>());
        Self {
            rpm_net,
            f_decimation,
            child,
            dbg,
        }
    }
}

impl<Child> Eval<ImbContext, ImbContext> for ComplexLocalOscillator<Child>
where
    Child: Eval<ImbContext, ImbContext> + Send + 'static,
{
    #[inline]
    fn eval(&self, ctx: ImbContext) -> ImbContext {
        // Передаем контекст дальше по цепочке вниз
        let mut ctx = self.child.eval(ctx);
        if ctx.is_err() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        if ctx.complex_local_oscillator.complex_signal.capacity() == 0 {
            ctx.complex_local_oscillator.complex_signal = Vec::with_capacity(ctx.decimation.decimated.len());
            ctx.complex_local_oscillator.phi_net = 0.0;
        }
        ctx.complex_local_oscillator.complex_signal.clear();
        let f_het = self.rpm_net / 60.0;
        for x in ctx.decimation.decimated.iter() {
            let curr_phi_net = (ctx.complex_local_oscillator.phi_net + 2.0 * PI * f_het / self.f_decimation) % (2.0 * PI);
            let euler_phi_net = Complex::new(curr_phi_net.cos(), curr_phi_net.sin());
            let oscillated_value = Complex::new(*x as f64, 0.0) * euler_phi_net;
            ctx.complex_local_oscillator.complex_signal.push(oscillated_value);
            ctx.complex_local_oscillator.phi_net = curr_phi_net;
        }
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}