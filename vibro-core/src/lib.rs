mod domain;
pub use domain::*;
mod kernel;
pub use kernel::*;

pub(crate) use sal_core::error::Error as Error;
pub(crate) use kernel::short_type_name;
pub(crate) use rustfft::num_traits as num_traits;
pub(crate) use rustfft::num_complex as num_complex;

#[cfg(test)]
mod tests;