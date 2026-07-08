use std::sync::Arc;
use sal_core::{
    dbg::Dbg, 
    error::Error
};
use crate::{
    Eval, GetInputs, Inputs, domain::context::Context
};
///
/// Пишет актуальные входные значения в контекст [Context]
/// * `inputs` - хранилище приходящих событий
pub struct ReadInputs<I> {
    inputs: Arc<I>,
    dbg: Dbg,
}
impl<I: GetInputs> ReadInputs<I> {
    ///
    /// Новый экземпляр [ReadInputs]
    /// * `inputs` - хранилище приходящих событий
    /// * `parent` - Идентификатор родительской сущности (для отладки).
    pub fn new(parent: &Dbg, inputs: Arc<I>) -> Self {
        let dbg = Dbg::new(parent, "ReadInputs");
        Self {
            inputs,
            dbg,
        }
    }
}
impl<I: GetInputs> Eval<Context, Context> for ReadInputs<I> {
    fn eval(&self, mut ctx: Context) -> Context {
        match &self.inputs.rpm() {
            Some(rpm) => {
                ctx.motor_rpm = Some(*rpm);
            }
            None => {
                ctx.err = Some(Error::new(&self.dbg, "eval").err("RPM isn't initialized yet."));
            }
        }
        match &self.inputs.p_motor() {
            Some(p_motor) => {
                if *p_motor <= 0.0 {
                    ctx.err = Some(Error::new(&self.dbg, "eval").err("Motor power must be > 0."));
                } else {
                    ctx.motor_p = Some(*p_motor);
                }
            }
            None => {
                ctx.err = Some(Error::new(&self.dbg, "eval").err("Motor power isn't initialized yet."));
            }
        }
        match &self.inputs.t_temp() {
            Some(t_temp) => {
                ctx.motor_t = Some(*t_temp);
            }
            None => {
                ctx.err = Some(Error::new(&self.dbg, "eval").err("Current T isn't initialized yet."));
            }
        }
        match &self.inputs.duration() {
            Some(duration) => {
                ctx.duration = Some(*duration);
            }
            None => {
                ctx.err = Some(Error::new(&self.dbg, "eval").err("Duration isn't initialized yet."));
            }
        }
        ctx
    }
    //
    fn exit(&self) {}
}