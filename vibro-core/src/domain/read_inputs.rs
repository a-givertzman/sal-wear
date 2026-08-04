use std::sync::Arc;
use sal_core::{dbg::Dbg, error::Error};
use sal_sync::services::EventValueAccess;
use crate::{AngularCtx, Eval};

/// Пишет актуальные входные значения в контекст
pub struct ReadEventValues<T> {
    values: Arc<T>,
    keys: Vec<String>,
    dbg: Dbg,
}
impl<T> ReadEventValues<T> {
    ///
    /// ### Returns `ReadInputs` new instance
    /// - `parent` - Идентификатор родительской сущности (для отладки).
    pub fn new<K: AsRef<str>>(parent: &Dbg, keys: impl IntoIterator<Item = K>, values: Arc<T>) -> Self {
        let dbg = Dbg::new(parent, crate::me::<Self>());
        Self {
            values,
            keys: keys.into_iter().map(|k| k.as_ref().to_string()).collect(),
            dbg,
        }
    }
}
impl<T: EventValueAccess<str, f64>> Eval<AngularCtx, AngularCtx> for ReadEventValues<T> {
    fn eval(&self, mut ctx: AngularCtx) -> AngularCtx {
        for key in &self.keys {
            match &self.values.get(key) {
                Some(rpm) => ctx.raw_rpm = *rpm,
                None => ctx.err = Some(Error::new(&self.dbg, "eval").err(format!("{key} isn't initialized yet."))),
            }
        }
        ctx
    }
    //
    fn exit(&self) {}
}
