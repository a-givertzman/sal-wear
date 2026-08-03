use std::sync::Arc;
use sal_core::{dbg::Dbg, error::Error};
use sal_sync::services::EventValueAccess;
use crate::{AngularCtx, Eval};

/// Пишет актуальные входные значения в контекст
pub struct ReadEventValuess<T> {
    // dyn EventValueAccess<str, f64> + Send + Sync
    values: Arc<T>,
    dbg: Dbg,
}
impl<T> ReadEventValuess<T> {
    ///
    /// ### Returns `ReadInputs` new instance
    /// - `parent` - Идентификатор родительской сущности (для отладки).
    pub fn new(parent: &Dbg, values: Arc<T>) -> Self {
        let dbg = Dbg::new(parent, crate::me::<Self>());
        Self {
            values,
            dbg,
        }
    }
}
impl<T: EventValueAccess<str, f64>> Eval<AngularCtx, AngularCtx> for ReadEventValuess<T> {
    fn eval(&self, mut ctx: AngularCtx) -> AngularCtx {
        match &self.values.get("rpm") {
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
