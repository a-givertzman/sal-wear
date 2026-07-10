use sal_core::dbg::Dbg;
use crate::{
    Eval, 
    domain::context::Context
};
///
/// Расчёт накопленного повреждения подшипника [об]
pub struct BearingAccumulatedWear<Child> {
    child: Child,
    dbg: Dbg,
}
//
//
impl<Child> BearingAccumulatedWear<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// Новый экземпляр [BearingAccumulatedWear]
    pub fn new(
        parent: &Dbg, 
        child: Child
    ) -> Self {
        let dbg = Dbg::new(parent, "BearingAccumulatedWear");
        Self {
            child,
            dbg,
        }
    }
}
impl<Child> Eval<Context, Context> for BearingAccumulatedWear<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    fn eval(&self, ctx: Context) -> Context {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        println!("motor_torque {:?}", ctx.motor_torque);
        println!("radial_load {:?}", ctx.radial_load);
        println!("axial_load {:?}", ctx.axial_load);
        println!("equivalent_load {:?}", ctx.equivalent_load);
        println!("basic_rating_life {:?}", ctx.basic_rating_life);
        println!("limiting_speed {:?}", ctx.limiting_speed);
        println!("actual_speed {:?}", ctx.actual_speed);
        println!("bearing_accumulated_wear {:?}", ctx.bearing_accumulated_wear);


        ctx.bearing_accumulated_wear = ctx.actual_speed / ctx.limiting_speed;
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
