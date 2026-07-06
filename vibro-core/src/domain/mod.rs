// Тут прикладные типы и классы алгоритмов
mod conf;
use std::sync::Arc;

pub use conf::*;
mod context;
pub use context::*;
mod angular_grid;
pub(crate) use angular_grid::*;
mod autocorrelation;
pub(crate) use autocorrelation::*;

pub type TimeDomainSamples<const N: usize> = Arc<[u16; N]>;