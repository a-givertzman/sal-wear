mod initial_ctx;
pub(super) use initial_ctx::*;
mod append_journal;
pub(super) use append_journal::*;
mod compactate_journal;
pub(super) use compactate_journal::*;
mod flush_journal;
pub(super) use flush_journal::*;
mod load_journal;
pub(super) use load_journal::*;
mod mark_old_journal;
pub(super) use mark_old_journal::*;
mod open_journal;
pub(super) use open_journal::*;

mod retain_state;
pub use retain_state::*;
mod retain_conf;
pub(crate) use retain_conf::*;
mod retain;
pub use retain::*;

use crate::err_pass;
use function_name::named;

pub(self) type EvalResult = Result<RetainCtx, sal_core::error::Error>;
// ///
// /// `TaskRetain` evaluation
// pub(self) trait Eval<In, Out> {
//     fn eval(&self, _: In) -> Out;
// }
///
/// Context provides tranfer data in the `TaskRetain` evaluation
pub(super) struct RetainCtx {
    pub txid: usize,
    pub cache: std::sync::Arc<crate::FxSccHashMap<String, Vec<u8>>>,
    pub path: std::path::PathBuf,
    pub writer: Option<std::io::BufWriter<std::fs::File>>,
    pub file_size_bytes: u64,
    pub compactation_trigger: compactate_journal::Trigger,
    /// Весь retain cache только что был записан надиск, необходимо очистить буфер в `AppendJournal`
    pub compacted: bool,
    pub flush_trigger: compactate_journal::Trigger,
    pub error: Option<sal_core::error::Error>,
}
//
impl RetainCtx {
    /// Отмечаем что компактация успешно выполнена
    pub fn compactation_done(&mut self) {
        self.compacted = true;
    }
    /// Проверяем была ли компактация
    pub fn compacted(&self) -> bool {
        self.compacted
    }
    pub fn appended(&mut self) {
        self.compacted = false;
    }
    #[named]
    pub fn writer_close(&mut self) {
        if let Some(w) = self.writer.take() {
            if let Err(err) = w.into_inner().map_err(|err| err_pass!(Self, err, "Can't close writer")) {
                log::warn!("{err}");
            }
        }
    }
}
