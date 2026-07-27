use std::io::Write;
use function_name::named;
use sal_core::dbg::Dbg;
use crate::{Eval, err_pass};
use super::{RetainCtx};

///
/// Проверяет, является ли ошибка ввода-вывода временной и допускает ли она повторное выполнение операции.
const fn is_retryable(kind: std::io::ErrorKind) -> bool {
    matches!(
        kind,
        std::io::ErrorKind::Interrupted | std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
    )
}
///
/// ### Выполняет сброс буфера
/// - При жестком отключении питания остатки буфера будут потеряны.
/// - Гарантируя, что накопленные данные будут переданы OS.
/// - Физическую запись на диск OS выполнит по своему усмотрению.
pub struct FlushJournal<Child> {
    child: Child,
    dbg: Dbg,
}
//
impl<Child> FlushJournal<Child> {
    pub fn new(parent: impl Into<String>, child: Child) -> Self {
        let dbg = Dbg::new(parent, crate::me::<Self>());
        Self {
            child,
            dbg,
        }
    }
}
//
impl<Child> Eval<RetainCtx, RetainCtx> for FlushJournal<Child>
where
    Child: Eval<RetainCtx, RetainCtx>,
{
    #[named]
    fn eval(&self, mut ctx: RetainCtx) -> RetainCtx {
        if ctx.flush_trigger.is_exceeded(0u64) {
            ctx.flush_trigger.start();
            if let Some(writer) = &mut ctx.writer {
                if let Err(err) = writer.flush() {
                    log::warn!("{}.run | Can't flush to '{:?}', error: {:?}", self.dbg, ctx.path.display(), err);
                    if !is_retryable(err.kind()) {
                        ctx.writer_close();
                    }
                }
            }
        }
        let mut ctx = self.child.eval(ctx);
        ctx.error = ctx.error.map(|err| err_pass!(self.dbg, err));
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
