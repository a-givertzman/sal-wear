use std::path::Path ;
use function_name::named;
use sal_core::{dbg::Dbg, error::Error};
use crate::{Eval, err_pass};
use super::{EvalResult, RetainMode, RetainCtx};

///
/// ### Переименование неактивного файла retain журнала в `.old`.
pub struct MarkOldJournal {
    mode: RetainMode,
    dbg: Dbg,
}
//
impl MarkOldJournal {
    ///
    /// Returns `MarkOldJournal` new instance
    pub fn new(parent: impl Into<String>, mode: RetainMode) -> Self {
        let dbg = Dbg::new(parent, crate::me::<Self>());
        Self {
            mode,
            dbg,
        }
    }
    ///
    /// ### Переименование неактивного файла в `.old`.
    /// Это необходимо что бы при следующей перезагрузке знать в каком режиме писали retain журнал
    /// 
    /// Возвращает `true` усли переименование успешно
    #[named]
    fn mark_inactive_old(dbg: &Dbg, path: &Path, mode: RetainMode) -> Result<(), Error> {
        let src_path = match mode {
            RetainMode::Debug => path.with_extension("dat"),
            RetainMode::Release => path.with_extension("json"),
        };
        let dst_path = src_path.with_added_extension("old");
        if let Err(err) = std::fs::rename(&src_path, &dst_path) {
            if err.kind() != std::io::ErrorKind::NotFound {
                std::fs::remove_file(&src_path).map_err(|err| err_pass!(dbg, err, "Can't rename/remove '{}'", src_path.display()))?;
            }
        }
        Ok(())
    }
}
//
impl Eval<RetainCtx, EvalResult> for MarkOldJournal {
    fn eval(&self, ctx: RetainCtx) -> EvalResult {
        Self::mark_inactive_old(&self.dbg, &ctx.path, self.mode)?;
        Ok(ctx)
    }
    //
    fn exit(&self) {}
}
