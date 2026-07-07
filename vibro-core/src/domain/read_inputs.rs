use std::sync::Arc;

use sal_core::{dbg::Dbg, error::Error};
use crate::{Context, Eval, Inputs, me};

/// Пишет актуальные входные значения в контекст
pub struct ReadInputs {
    inputs: Arc<Inputs>,
    dbg: Dbg,
}
impl ReadInputs {
    ///
    /// ### Returns `ReadInputs` new instance
    /// - `parent` - Идентификатор родительской сущности (для отладки).
    pub fn new(parent: &Dbg, inputs: Arc<Inputs>) -> Self {
        let dbg = Dbg::new(parent, me::<Self>());
        Self {
            inputs,
            dbg,
        }
    }
}
impl Eval<Context, Context> for ReadInputs {
    fn eval(&self, mut ctx: Context) -> Context {
        match &self.inputs.rpm() {
            Some(rpm) => {
                ctx.raw_rpm = *rpm;
            }
            None => {
                ctx.err = Some(Error::new(&self.dbg, "eval").err("RPM isn't initialized yet."));
            }
        }
        ctx
    }
    //
    fn exit(&self) {}
}
