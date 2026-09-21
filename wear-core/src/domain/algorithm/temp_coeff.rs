use sal_core::{dbg::Dbg, error::Error};
use crate::{
    Eval, 
    domain::context::Context
};
///
/// Расчёт температурного коэффициента ускорения износа (Коэффициент Вант-Гоффа): KT
/// См. [раздел 8.2, шаг 5](../../Operion_Diag_Вибродиагностика_и_остаточный_ресурс.pdf)
/// Формула: 
/// KT = Q10^((T_temp - T_ref) / 10)
/// Где:
/// * `Q10` — коэффициент ускорения износа на +10°C
/// * `T_temp` — температура подшипникового узла [°C] (если отсутствуте,то KT = 1)
/// * `T_ref` — опорная температура [°C] (номинальная рабочая температура)
pub struct TempCoeff<Child> {
    // Коэффициент ускорения износа на +10°C
    q10: f64,
    // Опорная температура (номинальная рабочая температура) [°C]
    t_ref: f64,
    child: Child,
    dbg: Dbg,
}
//
//
impl<Child> TempCoeff<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// Новый экземпляр [TempCoeff]
    /// * `q10` - коэффициент ускорения износа на +10°C
    /// * `t_ref` - опорная температура (номинальная рабочая температура) [°C] 
    pub fn new(
        q10: f64,
        t_ref: f64,
        parent: &Dbg, 
        child: Child
    ) -> Self {
        let dbg = Dbg::new(parent, "TempCoeff");
        Self {
            q10,
            t_ref,
            child,
            dbg,
        }
    }
}
impl<Child> Eval<Context, Context> for TempCoeff<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    fn eval(&self, ctx: Context) -> Context {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        let Some(t_bearing) = ctx.t_bearing else {
            ctx.temp_coeff = 1.0;
            return ctx;
        };
        ctx.temp_coeff = self.q10.powf((t_bearing - self.t_ref) / 10.0);
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
