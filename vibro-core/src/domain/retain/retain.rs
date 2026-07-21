use sal_core::{dbg::Dbg, error::Error};
use serde::{Serialize, de::DeserializeOwned};

pub struct Retain {
    dbg: Dbg,
}
impl Retain {
    /// Returns `Retain` new instance
    pub fn new(parent: impl Into<String>) -> Self {
        let dbg = Dbg::new(parent, crate::me::<Self>());
        Self {
            dbg,
        }
    }
    /// Возвращает актуальное сохраненное значение по ключу
    pub fn get<T: DeserializeOwned>(&self, key: impl Into<String>) -> Option<T> {
        None
    }
    /// Сохраняет актуальное значение
    pub fn store(&self, key: impl Into<String>, value: impl Serialize) -> Result<(), Error> {
        Err(Error::new(&self.dbg, "store").err("Is not implemented yet"))
    }
}