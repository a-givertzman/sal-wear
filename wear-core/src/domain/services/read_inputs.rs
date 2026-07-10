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
        match &self.inputs.motor_p() {
            Some(motor_p) => {
                if *motor_p <= 0.0 {
                    ctx.err = Some(Error::new(&self.dbg, "eval").err("Motor power must be > 0."));
                } else {
                    ctx.motor_p = Some(*motor_p);
                }
            }
            None => {
                ctx.err = Some(Error::new(&self.dbg, "eval").err("Motor power isn't initialized yet."));
            }
        }
        match &self.inputs.t_bearing() {
            Some(t_bearing) => {
                ctx.t_bearing = Some(*t_bearing);
            }
            None => {
                ctx.t_bearing = None;
            }
        }
        match &self.inputs.duration() {
            Some(duration) => {
                ctx.duration = *duration;
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