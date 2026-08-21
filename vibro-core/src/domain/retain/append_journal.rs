use std::{cell::Cell, collections::VecDeque, fs::File, io::{BufWriter, Write}};
use function_name::named;
use sal_core::{dbg::Dbg, error::Error};
use sal_sync::kernel::state::ChangeNotify;
use serde::Serialize;
use crate::{Eval, err_pass};
use super::{RetainValue, RetainMode, RetainCtx};

///
/// Current state of IO
pub(super) enum IoState {
    /// Continue using IO
    Err(Error),
    /// IO is corrupted, must be closed
    Closed(Error),
}
impl IoState {
    #[named]
    pub fn map(dbg: impl ToString, err: std::io::Error) -> Self {
        if !is_retryable(err.kind()) {
            return IoState::Closed(err_pass!(dbg, err));
        }
        IoState::Err(err_pass!(dbg, err))
    }
}
impl std::fmt::Debug for IoState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Err(err) => write!(f, "{:?}", err),
            Self::Closed(err) => write!(f, "{:?}", err),
        }
    }
}
///
/// Проверяет, является ли ошибка ввода-вывода временной и допускает ли она повторное выполнение операции.
const fn is_retryable(kind: std::io::ErrorKind) -> bool {
    matches!(
        kind,
        std::io::ErrorKind::Interrupted | std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
    )
}
///
/// Local Ok or Error state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum State {
    Ok,
    Err,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum BufState {
    Ok,
    Err,
}
///
/// ### Пишет один пакет с кадрированием длины в конец файла
pub struct AppendJournal<Child> {
    mode: RetainMode,
    buffer: Cell<VecDeque<RetainValue>>,
    child: Child,
    notify: ChangeNotify<'static, State, String>,
    buf_notify: ChangeNotify<'static, BufState, String>,
    dbg: Dbg,
}
//
impl<Child> AppendJournal<Child> {
    /// Максимально допустимый размер буффера для аммортизации перед записью в файл
    const MAX_BUFFER_SIZE: usize = 16_000;
    /// Returns `AppendJournal` new instance
    pub fn new(parent: impl Into<String>, mode: RetainMode, child: Child) -> Self {
        let dbg = Dbg::new(parent, crate::me::<Self>());
        let notify = ChangeNotify::builder(&dbg, State::Ok)
            .on(State::Ok, |msg| log::info!("{:?}", msg))
            .on(State::Err, |msg| log::warn!("{:?}", msg))
            .build();
        let buf_notify = ChangeNotify::builder(&dbg, BufState::Ok)
            .on(BufState::Ok, |msg| log::info!("{:?}", msg))
            .on(BufState::Err, |msg| log::warn!("{:?}", msg))
            .build();
        Self {
            mode,
            buffer: Cell::new(VecDeque::new()),
            child,
            notify,
            buf_notify,
            dbg,
        }
    }
}
//
impl<Child> Eval<(Option<RetainValue>, RetainCtx), RetainCtx> for AppendJournal<Child>
where
    Child: Eval<RetainCtx, RetainCtx>, {
    #[named]
    fn eval(&self, (event, mut ctx): (Option<RetainValue>, RetainCtx)) -> RetainCtx {
        let mut buf = self.buffer.take();
        if ctx.compacted() {
            ctx.appended();
            buf.clear();
        }
        if let Some(event) = event {
            _ = ctx.cache.upsert_sync(event.key.clone(), event.bytes.clone());
            if buf.len() >= Self::MAX_BUFFER_SIZE {
                if buf.pop_front().is_some() {
                    self.buf_notify.update(BufState::Err, || format!("{}.run | Buffer state: Overflow. Dropping oldest events to protect memory", self.dbg));
                }
            } else {
                self.buf_notify.update(BufState::Ok, || format!("{}.run | Buffer state: Normal.", self.dbg));
            }
            buf.push_back(event);
        }
        if let Some(writer) = &mut ctx.writer {
            let mut last_err = None;
            buf.retain(|event| {
                if let Some(IoState::Closed(_)) = last_err {
                    return true;
                }
                match append(&self.dbg, writer, &self.mode, &event.key, &event.bytes) {
                    Ok(bytes) => {
                        ctx.file_size_bytes += bytes as u64;
                        false // Успешно записано — удаляем из буфера
                    }
                    Err(err) => {
                        last_err = Some(err);
                        true // Ошибка записи — оставляем в буфере для следующего такта
                    }
                }
            });
            if let Some(err) = last_err {
                if let IoState::Closed(_) = err {
                    ctx.writer_close();
                }
                self.notify.update(State::Err, || format!("{}.run | Retain-storage I/O problems. Persistence impossible, buffering events. Reason: {:?}", self.dbg, err));
            } else {
                self.notify.update(State::Ok, || format!("{}.run | Retain storage I/O operational", self.dbg));
            }
        }
        self.buffer.set(buf);
        let mut ctx = self.child.eval(ctx);
        ctx.error = ctx.error.map(|err| err_pass!(self.dbg, err));
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
///
/// ### Сериализует и пишет один пакет в буфер, возвращая количество записанных байт
/// - `writer` - Указатель на буфер записи
/// - `mode` - Текущий режим, Debug/Release (JSON/RAW)
/// - `key` - Ключь, уникальный идентификатор сохраняемого значения
/// - `bytes` - Готовые байты сохраняемого значения в формате JSON/RAW в зависимости от текущего режима
#[named]
pub(super) fn append(
    dbg: &Dbg,
    writer: &mut BufWriter<File>, 
    mode: &RetainMode,
    key: &str,
    bytes: &[u8],
) -> Result<usize, IoState> {
    match mode {
        RetainMode::Debug => {
            let mut writer = CountingWriter::new(writer);
            // Руками собираем JSON: {"key":bytes}\n
            writer.write_all(b"{\"").map_err(|err| IoState::map(dbg, err))?;
            // Экранируем и пишем ключ (или просто пишем, если в ключе нет кавычек)
            writer.write_all(key.as_bytes()).map_err(|err| IoState::map(dbg, err))?;
            writer.write_all(b"\":").map_err(|err| IoState::map(dbg, err))?;
            // Вставляем уже готовые JSON-байты структуры без аллокаций и парсинга!
            writer.write_all(bytes).map_err(|err| IoState::map(dbg, err))?;
            writer.write_all(b"}\n").map_err(|err| IoState::map(dbg, err))?;
            Ok(writer.bytes_written())
        }
        RetainMode::Release => {
            writer.write_all(&(key.len() as u32).to_le_bytes()).map_err(|err| IoState::map(dbg, err))?;
            writer.write_all(key.as_bytes()).map_err(|err| IoState::map(dbg, err))?;
            // let bytes = postcard::to_allocvec(event).map_err(|err| IoState::Err(err_pass!(dbg, err)))?;
            writer.write_all(&(bytes.len() as u32).to_le_bytes()).map_err(|err| IoState::map(dbg, err))?;
            writer.write_all(bytes).map_err(|err| IoState::map(dbg, err))?;
            Ok(4 + key.len() + 4 + bytes.len())
        }
    }
}
///
/// ### Обертка над Write для подсчета записанных байт
struct CountingWriter<W: Write> {
    inner: W,
    bytes_written: usize,
}
//
impl<W: Write> CountingWriter<W> {
    pub fn new(inner: W) -> Self {
        Self { inner, bytes_written: 0 }
    }
    ///
    /// Получить текущее значение счетчика
    pub fn bytes_written(&self) -> usize {
        self.bytes_written
    }
    pub fn into_inner(self) -> W {
        self.inner
    }
}
//
impl<W: Write> Write for CountingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let result = self.inner.write(buf);
        if let Ok(bytes) = result {
            self.bytes_written += bytes;
        }
        result
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
///
/// Basic Tests
#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, io::BufWriter, sync::Arc, path::PathBuf};
    #[derive(Debug, Serialize)]
    struct MockPoint { name: String, ts: chrono::DateTime<chrono::Utc> }
    struct MockChild;
    impl Eval<RetainCtx, RetainCtx> for MockChild {
        fn eval(&self, ctx: RetainCtx) -> RetainCtx { ctx }
        fn exit(&self) {}
    }
    fn mock_ctx(writer: Option<BufWriter<File>>) -> RetainCtx {
        RetainCtx {
            cache: Arc::new(crate::FxSccHashMap::default()),
            path: PathBuf::from("dummy.log"),
            writer,
            file_size_bytes: 0,
            compactation_trigger: super::super::Trigger::default(),
            compacted: false,
            flush_trigger: super::super::Trigger::default(),
            error: None,
        }
    }
    #[test]
    fn test_append_journal_clears_buffer_on_success() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_journal_success.log");
        let file = File::create(&file_path).unwrap();
        let ctx = mock_ctx(Some(BufWriter::new(file)));
        let journal = AppendJournal::new("test", RetainMode::Release, MockChild);
        let point = MockPoint { name: "test_point".into(), ts: chrono::Utc::now() };
        let event = RetainValue::encode_json("test_point", &point).unwrap();
        journal.eval((Some(event), ctx));
        let buf = journal.buffer.take();
        assert!(buf.is_empty(), "Buffer must be empty after successful append");
        let _ = std::fs::remove_file(file_path);
    }
    #[test]
    fn test_append_journal_accumulates_without_writer() {
        let ctx = mock_ctx(None);
        let journal = AppendJournal::new("test", RetainMode::Release, MockChild);
        let point = MockPoint { name: "test_point".into(), ts: chrono::Utc::now() };
        let event = RetainValue::encode_json("test_point", &point).unwrap();
        journal.eval((Some(event), ctx));
        let buf = journal.buffer.take();
        assert_eq!(buf.len(), 1, "Buffer must accumulate events if writer is missing");
    }
}
