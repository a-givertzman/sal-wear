use function_name::named;
use sal_core::error::Error;
use serde::{Serialize, de::DeserializeOwned};
use crate::err_pass;

///
/// ### Key for retation value
#[derive(Clone)]
pub(super) struct RetainValue {
    pub key: String,
    pub bytes: Vec<u8>,
}
//
impl RetainValue {
    ///
    /// ### Returns `RetainValue` new instance
    pub fn new(key: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self { key: key.into(), bytes }
    }
    /// ### Returns `RetainValue` new instance
    /// with `T` encoded into JSON bytes
    #[named]
    #[inline]
    pub fn encode_json<T: Serialize>(key: impl Into<String>, v: &T) -> Result<Self, Error> {
        let bytes = serde_json::to_vec(v).map_err(|err| err_pass!(Self, err))?;
        Ok(Self { key: key.into(), bytes })
    }
    /// ### Returns `RetainValue` new instance
    /// with `T` encoded into RAW bytes 
    #[named]
    #[inline]
    pub fn encode_bytes<T: Serialize>(key: impl Into<String>, v: &T) -> Result<Self, Error> {
        let bytes = postcard::to_allocvec(v).map_err(|err| err_pass!(Self, err))?;
        Ok(Self { key: key.into(), bytes })
    }
    /// Returns `T` restored from JSON bytes
    #[named]
    #[inline]
    pub fn decode_from_json<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, Error> {
        serde_json::from_slice(bytes).map_err(|err| err_pass!(Self, err))
    }
    /// Returns `T` restored from RAW bytes
    #[named]
    #[inline]
    pub fn decode_from_bytes<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, Error> {
        postcard::from_bytes::<T>(&bytes).map_err(|err| err_pass!(Self, err))
    }
}
