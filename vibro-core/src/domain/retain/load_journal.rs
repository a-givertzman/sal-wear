use std::{fs::File, io::{BufRead, BufReader, Read}, path::Path, sync::Arc};
use function_name::named;
use sal_core::{dbg::Dbg, error::Error};
use crate::{Eval, FxSccHashMap, domain::retain::RetainValue, err, err_pass};
use super::{EvalResult, RetainCtx};

/// Вспомогательная структура для чтения JSON линий из файла.
#[derive(serde::Deserialize)]
struct RawLine<'a> {
    // Используем десериализацию во временную карту с одним элементом
    #[serde(borrow)]
    #[serde(flatten)]
    pub entry: std::collections::HashMap<String, &'a serde_json::value::RawValue>,
}
impl<'a> RawLine<'a> {
    /// Возвращает key и байты чистого JSON
    pub fn entry(self) -> Option<(String, &'a [u8])> {
        let (key, raw_json) = self.entry.into_iter().next()?;
        Some((key, raw_json.get().as_bytes()))
    }
}
///
/// ### Локальный флаг состояния чтения из файла
enum IoState {
    Continue(RetainValue),
    Done,
}
///
/// ### Загрузка состояния журнала в оперативный кэш
pub struct LoadJournal<Child> {
    child: Child,
    dbg: Dbg,
}
//
impl<Child> LoadJournal<Child> {
    ///
    /// Returns `LoadJournal` new instance
    pub fn new(parent: impl Into<String>, child: Child) -> Self {
        let dbg = Dbg::new(parent, crate::me::<Self>());
        Self {
            child,
            dbg,
        }
    }
    ///
    /// ### Парсит одну запись из байтов `postcard` в `(String, RetainState)`
    #[named]
    fn decode_entry(dbg: &Dbg, reader: &mut BufReader<File>, len_buf: &mut [u8; 4], key_buf: &mut Vec<u8>, state_buf: &mut Vec<u8>) -> Result<IoState, Error> {
        if reader.read_exact(len_buf).is_err() { return Ok(IoState::Done); }
        let len = u32::from_le_bytes(*len_buf);
        if len > 1024 {
            return Err(err!(dbg, "Размер ключа retain-записи: {} - превышает 1KB, файл кэша поврежден", len));
        }
        key_buf.resize(len as usize, 0u8);
        reader.read_exact(key_buf).map_err(|err| err_pass!(dbg, err))?;
        let key = std::str::from_utf8(key_buf).map_err(|err| err_pass!(dbg, err))?;
        reader.read_exact(len_buf).map_err(|err| err_pass!(dbg, err))?;
        let len = u32::from_le_bytes(*len_buf);
        if len > 10 * 1024 * 1024 {
            return Err(err!(dbg, "Размер значения retain-записи: {} - превышает 100MB, файл кэша поврежден", len));
        }
        state_buf.resize(len as usize, 0u8);
        reader.read_exact(state_buf).map_err(|err| err_pass!(dbg, err))?;
        // let state = postcard::from_bytes::<RetainState>(state_buf).map_err(|err| err_pass!(dbg, err))?;
        Ok(IoState::Continue(RetainValue::new(key, state_buf.clone())))
    }
    ///
    /// ### Чтение с диска и парсинг `RetainState`
    /// 
    /// - Максимальный размер ключа - 1 KB
    /// - Максимальный размер `RetainState` - 10 MB
    /// 
    /// Устойчив к повреждению хвоста файла. При обнаружении бинарного мусора
    /// или неожиданного конца файла чтение останавливается, а корректно загруженные данные сохраняются.
    fn load(&self, path: &Path, cache: &Arc<FxSccHashMap<String, Vec<u8>>>) -> Result<(), Error> {
        let dat_path = path.with_extension("dat");
        // let file = File::open(&dat_path).map_err(|err| err_pass!(self.dbg, err, "Can't open file '{}'", dat_path.display()))?;
        match File::open(&dat_path) {
            Ok(file) => {
                log::debug!("{}.load | Reading retain cache from '{}'...", self.dbg, dat_path.display());
                let mut reader = BufReader::new(file);
                let mut len_buf = [0u8; 4];
                let mut key_buf = Vec::with_capacity(1024);
                let mut state_buf = Vec::with_capacity(4096);
                loop {
                    match Self::decode_entry(&self.dbg, &mut reader, &mut len_buf, &mut key_buf, &mut state_buf) {
                        Ok(IoState::Done) => break,
                        Ok(IoState::Continue(state)) => {
                            _ = cache.upsert_sync(state.key, state.bytes);
                        }
                        Err(err) => {
                            log::error!("{}.load | Retain файл журнала оборван или поврежден '{}'.\n\tОшибка: {:?}.\n\tТолько часть данных загружено: {:#?}.",
                                self.dbg, dat_path.display(), err, cache);
                            break;
                        }
                    }
                }
                log::debug!("{}.load | Reading retain cache from '{}' - Ok", self.dbg, dat_path.display());
                return Ok(());
            }
            Err(err) => if err.kind() != std::io::ErrorKind::NotFound {
                log::warn!("{}.load | Can't read cache '{}': {:?}", self.dbg, dat_path.display(), err);
            }
        }
        let json_path = path.with_extension("json");
        match File::open(&json_path) {
            Ok(file) => {
                log::debug!("{}.load | Reading retain cache from '{}'...", self.dbg, json_path.display());
                let reader = BufReader::new(file);
                let mut lines = reader.lines();
                while let Some(line) = lines.next() {
                    match line {
                        Ok(line) => {
                            // 1. Парсим строку в легковесный дескриптор (без копирования тела структуры)
                            match serde_json::from_str::<RawLine>(&line) {
                                Ok(raw_line) => {
                                    if let Some((key, bytes)) = raw_line.entry() {
                                        _ = cache.upsert_sync(key, bytes.to_vec());
                                    } else {
                                        log::warn!("{} | Empty JSON object in line", self.dbg);
                                    }
                                }
                                Err(err) => log::warn!("{}.load | Can't parse entry in {}, error: {:?}", self.dbg, json_path.display(), err),
                            }


                            // match serde_json::from_str::<HashMap<String, RetainState>>(&line) {
                            //     Ok(parsed) => {
                            //         if let Some((key, state)) = parsed.into_iter().next() {
                            //             let val = Self::point(&state, txid, &key);
                            //             _ = cache.upsert_sync(key, val);
                            //         }
                            //     }
                            //     Err(err) => log::warn!("{}.load | Can't parse entry in {}, error: {:?}", self.dbg, json_path.display(), err),
                            // }
                        }
                        Err(err) => log::warn!("{}.load | Can't read entry from {}, error: {:?}", self.dbg, json_path.display(), err),
                    }
                }
                log::debug!("{}.load | Reading retain cache from '{}' - Ok", self.dbg, json_path.display());
            }
            Err(err) => if err.kind() != std::io::ErrorKind::NotFound {
                log::warn!("{}.load | Can't read cache '{}': {:?}", self.dbg, json_path.display(), err);
            }
        }
        Ok(())
    }
}
//
impl<Child> Eval<RetainCtx, EvalResult> for LoadJournal<Child>
where
    Child: Eval<RetainCtx, EvalResult>, {
    #[named]
    fn eval(&self, ctx: RetainCtx) -> EvalResult {
        let ctx = self.child.eval(ctx).map_err(|err: Error| err_pass!(self.dbg, err))?;
        self.load(&ctx.path, &ctx.cache).map_err(|err| err_pass!(self.dbg, err))?;
        Ok(ctx)
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
///
/// Basic Tests
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use serde::Serialize;
    use tempfile::NamedTempFile;
    #[derive(Debug, Serialize)]
    struct MockPoint { name: String, ts: chrono::DateTime<chrono::Utc> }
    #[test]
    fn test_load_journal_torn_write_recovery() {
        let mut file = NamedTempFile::new().unwrap();
        let key = "Motor_1_Start";
        let point = MockPoint { name: "test_point".into(), ts: chrono::Utc::now() };
        let state = RetainValue::encode_bytes(key, &point).unwrap();
        let key_len = (key.len() as u32).to_le_bytes();
        file.write_all(&key_len).unwrap();
        file.write_all(key.as_bytes()).unwrap();
        let bytes = postcard::to_allocvec(&point).unwrap();
        let val_len = (bytes.len() as u32).to_le_bytes();
        file.write_all(&val_len).unwrap();
        file.write_all(&bytes).unwrap();
        let key2 = "Motor_2_Start";
        let key_len2 = (key2.len() as u32).to_le_bytes();
        file.write_all(&key_len2).unwrap();
        file.write_all(key2.as_bytes()).unwrap();
        let val_len2 = (bytes.len() as u32).to_le_bytes();
        file.write_all(&val_len2).unwrap();
        file.write_all(&bytes[..bytes.len() / 2]).unwrap();
        file.flush().unwrap();
        let dbg = Dbg::new("Test", "decode_entry");
        let file = File::open(file.path()).unwrap();
        let mut reader = BufReader::new(file);
        let mut len_buf = [0u8; 4];
        let mut key_buf = Vec::new();
        let mut state_buf = Vec::new();
        let res1 = LoadJournal::<()>::decode_entry(&dbg, &mut reader, &mut len_buf, &mut key_buf, &mut state_buf);
        assert!(matches!(res1, Ok(IoState::Continue(e)) if e.key == "Motor_1_Start"));
        let res2 = LoadJournal::<()>::decode_entry(&dbg, &mut reader, &mut len_buf, &mut key_buf, &mut state_buf);
        assert!(res2.is_err(), "Должен поймать ошибку оборванной записи хвоста файла");
    }
}
