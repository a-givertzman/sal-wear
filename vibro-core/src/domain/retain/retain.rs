use std::{io::Write, path::PathBuf, sync::Arc, time::{Duration, Instant}};
use function_name::named;
use sal_core::{dbg::Dbg, error::Error};
use sal_sync::{kernel::state::ExitNotify, services::{Service, Services, entity::{Name, Object, PointConf}}, sync::{Handles, Owner}, thread_pool::Scheduler};
use serde::{Serialize, de::DeserializeOwned};
use crate::{Eval, FxSccHashMap, RECV_TIMEOUT, Receiver, RecvTimeoutError, Sender, RetainConf, err, err_pass};
use super::{RetainValue, AppendJournal, CompactateJournal, FlushJournal, InitialCtx, LoadJournal, MarkOldJournal, OpenJournal};

///
/// ### Хранение пар `Key-Value` на диске
/// 
/// - **Формат данных на диске в режиме `Debug`**
/// ```json
/// { "Key": Value-json }
/// ```
/// - **Формат данных на диске в режиме `Release`**
/// 
///  Key len | Key |  Value bytes len | Value bytes
///   :---: | :---: | :---: | :---:
///  `u32`<br>(4 bytes) | `String`<br>( Key len bytes) | `u32`<br>(4 bytes) | `Retain Value`<br>(Value len bytes)
///
/// Отложенная запись retain значений для оптимизации работы с диском.
/// Получает Key-Point в канале, сохраняет в единый для `Retain` файл добавлением в конец.
/// Периодически актуализирует весь журнал.
pub struct Retain {
    name: Name,
    cache: Arc<FxSccHashMap<String, Vec<u8>>>,
    conf: RetainConf,
    path: PathBuf,
    send: Sender<RetainValue>,
    recv: Owner<Receiver<RetainValue>>,
    scheduler: Option<Scheduler>,
    handles: Handles<()>,
    exit: Arc<ExitNotify>,
    dbg: Dbg,
}
//
#[allow(unused)]
impl Retain {
    const BUFFER_SIZE: usize = 16 * 1024;
    ///
    /// Returns `Retain` new instance
    /// - `parent` - Родительский сервис `Task`.
    /// - `txid` - Идентификатор сервиса отправителя (в данном случае родительского `Task`).
    #[named]
    pub fn new(parent: &Name, txid: usize, conf: RetainConf, services: &Arc<Services>, scheduler: Scheduler) -> Result<Self, Error> {
        let name = Name::new(parent, crate::me::<Self>());
        let dbg = Dbg::new(parent, crate::me::<Self>());
        let Some(retain_path) = services.retain().path else {
            return Err(err!(dbg, "Retain: path - missed in Application config"));
        };
        let cw_dir = std::env::current_dir().map_err(|err| err_pass!(dbg, err))?;
        let dir = cw_dir.join(retain_path).join(parent.join().trim_start_matches('/'));
        std::fs::create_dir_all(&dir).map_err(|err| err_pass!(dbg, err, "Error creating dir: '{}'", dir.display()))?;
        let path = dir.join("retain").with_extension("json");
        let (send, recv) = crate::channel_bounded(Self::BUFFER_SIZE);
        Ok(Self {
            name,
            cache: Arc::new(FxSccHashMap::default()),
            conf,
            path,
            send,
            recv: Owner::new(recv),
            scheduler: Some(scheduler),
            handles: Handles::new(parent),
            exit: Arc::new(ExitNotify::new(parent, None, None)),
            dbg,
        })
    }
    ///
    /// Returns `Retain` test instance
    /// - `cache` - Коллекция пар (key, bytes), где bytes - это байты сериализованного объекта, который нужно сохранить в Retain
    pub fn mock(parent: impl Into<String>, cache: impl IntoIterator<Item = (String, Vec<u8>)>) -> Self {
        let name = Name::new(parent, crate::me::<Self>());
        let dbg = Dbg::new(name.parent(), crate::me::<Self>());
        let (send, recv) = crate::channel_bounded(Self::BUFFER_SIZE);
        Self {
            name,
            cache: Arc::new(cache.into_iter().collect()),
            conf: RetainConf::default(),
            path: PathBuf::new(),
            send,
            recv: Owner::new(recv),
            scheduler: None,
            handles: Handles::new(&dbg),
            exit: Arc::new(ExitNotify::new(&dbg, None, None)),
            dbg,
        }
    }
    /// ### Returns retained `T` for the specified `key`
    #[inline]
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let cached: Option<Result<T, Error>> = self.cache.read_sync(key, |_, bytes| {
            match self.conf.mode {
                super::RetainMode::Debug => RetainValue::decode_from_json(bytes),
                super::RetainMode::Release => RetainValue::decode_from_bytes(bytes),
            }
        });
        match cached {
            Some(Err(err)) => {
                log::debug!("{}.get | Can't parse retained '{key}': {:?}", self.dbg, err);
                None
            }
            None => {
                log::debug!("{}.get | Can't find retained '{key}'", self.dbg);
                None
            }
            Some(Ok(value)) => Some(value),
        }
    }
    /// Retains passed `value` into the `key`
    #[named]
    #[inline]
    pub fn store(&self, key: impl Into<String>, value: impl Serialize) -> Result<(), Error> {
        let event = match self.conf.mode {
            super::RetainMode::Debug => RetainValue::encode_json(key, &value).map_err(|err| err_pass!(self.dbg, err))?,
            super::RetainMode::Release => RetainValue::encode_bytes(key, &value).map_err(|err| err_pass!(self.dbg, err))?,
        };
        _ = self.send.send(event);
        Ok(())
    }
}
//
impl Object for Retain {
    fn name(&self) -> Name {
        self.name.clone()
    }
}
//
impl std::fmt::Debug for Retain {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Retain")
            .field("name", &self.name)
            .finish()
    }
}
//
impl Service for Retain {
    //
    // fn get_link(&self, _: &str) -> Sender<Point> {
    //     self.send.clone()
    // }
    //
    #[named]
    fn run(&self) -> Result<(), Error> {
        log::info!("{}.run | Starting...", self.dbg);
        let dbg = self.dbg.clone();
        let exit = self.exit.clone();
        let conf = self.conf.clone();
        let rx_recv = self.recv.take().ok_or_else(|| err!(dbg, "Can't take recv"))?;
        let ctx = InitialCtx::new(&dbg, &conf, &self.path,
            LoadJournal::new(&dbg,
                MarkOldJournal::new(&dbg, conf.mode),
            ),
        ).eval(self.cache.clone())?;
        match self.scheduler.as_ref() {
            Some(scheduler) => {
                let handle = scheduler.spawn({
                    let dbg = dbg.clone();
                    let retain = OpenJournal::new(&dbg, &conf.journal.flush, ctx,
                        AppendJournal::new(&dbg, conf.mode,
                            FlushJournal::new(&dbg,
                                CompactateJournal::new(&dbg, &conf),
                            ),
                        ),
                    );
                    move || {
                    'main: while !exit.get() {
                        match rx_recv.recv_timeout(RECV_TIMEOUT) {  // 100ms
                            Ok(event) => {
                                log::trace!("{dbg}.run | point '{}': {:?}", event.key, event.bytes);
                                if let Err(err) = retain.eval(Some(event)).map_err(|err| err_pass!(dbg, err)) {
                                    log::warn!("{err}");
                                }
                            }
                            Err(RecvTimeoutError::Timeout) => {
                                if let Err(err) = retain.eval(None).map_err(|err| err_pass!(dbg, err)) {
                                    log::warn!("{err}");
                                }
                            }
                            Err(err) => {
                                log::debug!("{dbg}.run | Receiv error: {:?}", err);
                                break 'main;
                            }
                        }
                    }
                    if let Err(err) = retain.close() {
                        log::error!("{dbg}.run | Can't close retain journal: {:?}", err);
                    }
                    log::info!("{dbg}.run | Exit");
                }});
                match handle {
                    Ok(handle) => {
                        log::info!("{dbg}.run | Starting - ok");
                        self.handles.push(handle);
                        Ok(())
                    }
                    Err(err) => Err(err_pass!(dbg, err, "Start failed")),
                }
            }
            None => {
                let handle = std::thread::spawn({
                    let dbg = dbg.clone();
                    let cache = self.cache.clone();
                    move || {
                    'main: while !exit.get() {
                        match rx_recv.recv_timeout(RECV_TIMEOUT) {
                            Ok(event) => {
                                // log::trace!("{dbg}.run | point '{}': {:?}", event.key, event.p);
                                _ = cache.upsert_sync(event.key, event.bytes);
                            }
                            Err(RecvTimeoutError::Timeout) => {}
                            Err(err) => {
                                log::trace!("{dbg}.run | Receiv error: {:?}", err);
                                break 'main;
                            }
                        };
                    }
                    log::info!("{dbg}.run | Exit");
                }});
                self.handles.push(handle);
                log::info!("{dbg}.run | Starting (Mock) - ok");
                Ok(())
            }
        }
    }
    //
    fn points(&self) -> Vec<PointConf> {
        self.conf.points()
    }
    //
    fn wait(&self) -> Result<(), Error> {
        self.handles.wait()
    }
    //
    fn is_finished(&self) -> bool {
        self.handles.is_finished()
    }
    //
    fn exit(&self) {
        self.exit.exit();
    }
}
///
/// Cycle measuring
#[allow(unused)]
struct Trigger {
    interval: Duration,
    bytes_limit: u64,
    t: std::cell::Cell<Instant>,
}
//
#[allow(unused)]
impl Trigger {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            bytes_limit: 0,
            t: std::cell::Cell::new(Instant::now()),
        }
    }
    ///
    /// Maximum buffer length allowed before exceeded, MB.
    pub fn with_mb_limit(self, mb: impl Into<u64>) -> Self {
        Self {
            interval: self.interval,
            bytes_limit: mb.into() * 1024 * 1024,
            t: self.t,
        }
    }
    pub fn start(&self) {
        self.t.replace(Instant::now());
    }
    ///
    /// ### Returns `true` if time interval or bytes limit is exceeded
    /// - `bytes`: Current size in bytes
    pub fn is_exceeded(&self, bytes: impl Into<u64>) -> bool {
        if self.t.get().elapsed() >= self.interval {
            return true;
        }
        if self.bytes_limit > 0 {
            return bytes.into() >= self.bytes_limit;
        }
        false
    }
}
///
/// Wraps a writer and counts the total number of bytes written.
#[allow(unused)]
struct CountingWriter<W: Write> {
    inner: W,
    bytes_written: usize,
}
//
#[allow(unused)]
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
        match self.inner.write(buf) {
            Ok(bytes) => {
                self.bytes_written += bytes;
                Ok(bytes)
            }
            Err(err) => Err(err),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
