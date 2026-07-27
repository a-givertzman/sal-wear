use std::{fs::File, io::BufWriter, time::{Duration, Instant}};
use function_name::named;
use sal_core::{dbg::Dbg, error::Error};
use crate::{Eval, err_pass};
use super::{RetainMode, RetainConf, RetainCtx};

///
/// ### Физическая запись состояния на диск (Compactation).
/// Пишется через атомарную подмену файлов
pub struct CompactateJournal {
    conf: RetainConf,
    dbg: Dbg,
}
//
impl CompactateJournal {
    pub fn new(parent: impl Into<String>, conf: &RetainConf) -> Self {
        let dbg = Dbg::new(parent, crate::me::<Self>());
        Self {
            conf: conf.clone(),
            dbg,
        }
    }
    ///
    /// ### Физическая запись состояния на диск (Compactation)
    /// 
    /// `RetainState` пишется через атомарную подмену файлов
    #[named]
    fn store(dbg: &Dbg, conf: &RetainConf, ctx: &mut RetainCtx) -> Result<(), Error> {
        let (tmp_path, path) = match conf.mode {
            RetainMode::Debug => (ctx.path.with_extension("json.tmp"), ctx.path.with_extension("json")),
            RetainMode::Release => (ctx.path.with_extension("dat.tmp"), ctx.path.with_extension("dat")),
        };
        let file = File::create(&tmp_path)
            .map_err(|err| err_pass!(dbg, err, "Can't open '{}'", tmp_path.display()))?;
        let mut tmp_writer = BufWriter::with_capacity(128 * 1024, file);
        ctx.cache.iter_sync(|key, bytes| {
            if let Err(err) = super::append(dbg, &mut tmp_writer, &conf.mode, key, bytes) {
                log::warn!("{}.store | Can't store '{}' into '{}', error: {:?}", dbg, key, tmp_path.display(), err);
            }
            true
        });
        let file = tmp_writer.into_inner().map_err(|err| err_pass!(dbg, err, "Can't flush '{}'", tmp_path.display()))?;
        file.sync_data().map_err(|err| err_pass!(dbg, err, "Can't Sync '{}'", tmp_path.display()))?;
        drop(file);
        if cfg!(target_os = "windows") {
            ctx.writer_close();
        }
        std::fs::rename(&tmp_path, &path).map_err(|err| err_pass!(dbg, err, "Can't Rename '{}' -> '{}'", tmp_path.display(), path.display()))?;
        log::trace!("{}.store | Compactation done to '{}'", dbg, path.display());
        Ok(())
    }
}
//
impl Eval<RetainCtx, RetainCtx> for CompactateJournal {
    fn eval(&self, mut ctx: RetainCtx) -> RetainCtx {
        if ctx.compactation_trigger.is_exceeded(ctx.file_size_bytes) {
            match Self::store(&self.dbg, &self.conf, &mut ctx) {
                Ok(_) => {
                    ctx.compactation_done();
                    ctx.writer_close();
                    ctx.compactation_trigger.start();
                }
                Err(err) => {
                    log::warn!("{}.run | Store error: {:?}", self.dbg, err);
                }
            }
        }
        ctx
    }
    //
    fn exit(&self) {}
}
///
/// Detection of interval or size exceeded
pub struct Trigger {
    interval: Duration,
    bytes_limit: u64,
    t: std::cell::Cell<Instant>,
}
//
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
        if self.bytes_limit > 0 {
            return self.t.get().elapsed() >= self.interval || bytes.into() >= self.bytes_limit;
        }
        self.t.get().elapsed() >= self.interval
    }
}
//
impl Default for Trigger {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(1),
            bytes_limit: 512,
            t: std::cell::Cell::new(Instant::now()),
        }
    }
}