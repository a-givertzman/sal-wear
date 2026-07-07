mod domain {
mod conf {
mod conf {
use std::ops::{Bound, Range, RangeBounds, RangeFull};
use serde::{Deserialize, Deserializer};
#[derive(Clone, Debug, Deserialize)]
pub struct Conf {
    pub hardware: HardwareConf,
    pub angular: AngularConf,
    pub bands: BandsConf,
}
#[derive(Clone, Debug, Deserialize)]
pub struct HardwareConf {
    #[serde(alias = "sample-rate-hz")]
    pub sample_rate_hz: f32,
    #[serde(alias = "chunk-size")]
    pub chunk_size: usize,
}
#[derive(Clone, Debug, Deserialize)]
pub struct AngularConf {
    #[serde(alias = "max-order")]
    pub max_order: f32,
    #[serde(alias = "resolution")]
    pub resolution: f32,
}
impl AngularConf {
    pub fn points_per_rev(&self) -> usize {
        let min_points = (self.max_order * 2.5).ceil() as usize;
        min_points.next_power_of_two()
    }
    pub fn fft_turns(&self) -> usize {
        let min_turns = (1.0 / self.resolution).ceil() as usize;
        min_turns.next_power_of_two()
    }
    pub fn angular_step_rad(&self) -> f64 {
        std::f64::consts::TAU / (self.points_per_rev() as f64)
    }
    pub fn fft_buffer_size(&self) -> usize {
        self.points_per_rev() * self.fft_turns()
    }
}
#[derive(Clone, Debug, Deserialize)]
pub struct BandsConf {
    #[serde(alias = "low-order", deserialize_with = "parse_range")]
    pub low_order: (Bound<f32>, Bound<f32>),
    #[serde(alias = "mid-hz", deserialize_with = "parse_range")]
    pub mid_hz: (Bound<f32>, Bound<f32>),
    #[serde(alias = "high-hz", deserialize_with = "parse_range")]
    pub high_hz: (Bound<f32>, Bound<f32>),
}
impl BandsConf {
    pub fn low_range_orders(&self) -> (f32, f32) {
        let (start, end) = extract_bounds(&(self.low_order));
        (start.unwrap(), end.unwrap())
    }
    pub fn high_range_hz(&self) -> (f32, f32) {
        let (start, end) = extract_bounds(&(self.high_hz));
        (start.unwrap(), end.unwrap())
    }
    pub fn mid_range_max_hz(&self) -> f32 {
        let (_, end) = extract_bounds(&(self.mid_hz));
        end.unwrap()
    }
}
fn extract_bounds<T: Clone>(range: &(Bound<T>, Bound<T>)) -> (Option<T>, Option<T>) {
    let f = |b: Bound<&T>| match b {
        Bound::Included(v) | Bound::Excluded(v) => Some(v.clone()),
        Bound::Unbounded => None,
    };
    (f(range.start_bound()), f(range.end_bound()))
}
fn parse_range<'de, D>(deserializer: D) -> Result<(Bound<f32>, Bound<f32>), D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    let (low_str, high_str) = s.split_once("..").ok_or_else(|| {
        serde::de::Error::custom(format!("Неверный формат диапазона: '{}'. Ожидалось 'start..end'", s))
    })?;
    let low_str = low_str.trim();
    let high_str = high_str.trim();
    let start = if low_str.is_empty() {
        Bound::Unbounded
    } else {
        let val = low_str.parse::<f32>().map_err(serde::de::Error::custom)?;
        Bound::Included(val)
    };
    let end = if high_str.is_empty() {
        Bound::Unbounded
    } else {
        let val = high_str.parse::<f32>().map_err(serde::de::Error::custom)?;
        Bound::Excluded(val)
    };
    Ok((start, end))
}
}
pub use conf::*;}
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
pub use conf::*;
mod context {
mod context {
use sal_core::error::Error;
use crate::{MirroredBuffer, TimeDomainSamples};
pub struct Context {
    pub(crate) ac_samples: MirroredBuffer,
    pub(crate) raw_rpm: f64,
    pub(crate) raw_period: f64,
    pub(crate) period: f64,
    pub(crate) omega: f64,
    pub(crate) dt: f64,
    pub current_theta: f64,
    pub phases: [f32; 512],
    pub(crate) err: Option<Error>,
}
impl Context {
    pub fn new() -> Self {
        let capacity = 96_000;
        Self {
            ac_samples: MirroredBuffer::new(capacity),
            raw_rpm: f64::NAN,
            raw_period: f64::NAN,
            period: f64::NAN,
            omega: f64::NAN,
            dt: f64::NAN,
            current_theta: 0.0,
            phases: [0.0; 512],
            err: None,
        }
    }
    pub fn push_chunk(&mut self, samples: &[u16; 512]) {
        self.ac_samples.push_chunk(samples);
        self.err = None;
    }
    pub fn pass_err(mut self, me: impl Into<String>, area: impl Into<String>) -> Context {
        self.err = match self.err {
            Some(err) => Some(Error::new(me, area).pass(err)),
            None => Some(Error::new(me, area)),
        };
        self
    }
}}
pub use context::*;}
pub use context::*;
mod angular_grid {
use sal_core::dbg::Dbg;
use crate::{Context, Eval, domain::PI2, me};
pub struct AngularGrid<Child> {
    child: Child,
    dbg: Dbg,
}
impl<Child> AngularGrid<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// ### Returns `Autocorrelation` new instance
    /// Вычисляет угловую сетку для новой порции данных.
    /// * `samples` - Окно отсчетов для расчета (должно вмещать минимум 2-3 оборота вала).
    /// * `rough_rpm` - Приблизительная частота вращения с тахометра (об/мин).
    /// * `sample_rate` - Частота дискретизации в Гц.
    /// * `initial_angle` - Угол θ на начало окна.
    pub fn new(parent: impl Into<String>, child: Child) -> Self {
        let dbg = Dbg::new(parent, me::<Self>());
        Self {
            child,
            dbg,
        }
    }
}
impl<Child> Eval<Context, Context> for AngularGrid<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    fn eval(&self, ctx: Context) -> Context {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        ctx.phases = [0.0; 512];
        for i in 0..ctx.phases.len() {
            ctx.current_theta += ctx.omega * ctx.dt;
            if ctx.current_theta >= PI2 {
                ctx.current_theta -= PI2;
            }
            ctx.phases[i] = ctx.current_theta as f32;
        }
        ctx
    }
    fn exit(&self) {
        todo!()
    }
}}
pub(crate) use angular_grid::*;
mod autocorrelation {
use sal_core::dbg::Dbg;
use crate::{Context, Eval, domain::PI2, me};
pub struct Autocorrelation<Child> {
    sample_rate: f64,
    child: Child,
    dbg: Dbg,
}
impl<Child> Autocorrelation<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    ///
    /// ### Returns `Autocorrelation` new instance
    /// - `parent` - Идентификатор родительской сущности (для отладки).
    /// - `sample_rate` - Частота дискретизации в Гц.
    /// - `child` - Дочерний (предыдущий) расчетный шаг
    pub fn new(parent: &Dbg, sample_rate_hz: impl Into<f64>, child: Child) -> Self {
        let dbg = Dbg::new(parent, me::<Self>());
        Self {
            sample_rate: sample_rate_hz.into(),
            child,
            dbg,
        }
    }
    /// Вычисляет точный период вращения (в отсчетах) через автокорреляцию.
    /// Ищет максимум функции в узком окне от показаний тахометра.
    #[inline]
    fn find_exact_period(samples: &[u16], raw_period: f64) -> f64 {
        let margin = (raw_period * 0.1) as usize;
        let center = raw_period as usize;
        let start = center.saturating_sub(margin).max(1);
        let end = (center + margin).min(samples.len() / 2);
        let mut max_corr = 0.0;
        let mut best_lag = center;
        for lag in start..=end {
            let mut corr = 0.0;
            for i in 0..(samples.len() - lag) {
                corr += samples[i] as f64 * samples[i + lag] as f64;
            }
            if corr > max_corr {
                max_corr = corr;
                best_lag = lag;
            }
        }
        best_lag as f64
    }
}
impl<Child> Eval<Context, Context> for Autocorrelation<Child>
where
    Child: Eval<Context, Context> + Send + 'static {
    fn eval(&self, ctx: Context) -> Context {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        ctx.raw_period = self.sample_rate * 60.0 / ctx.raw_rpm;
        ctx.period = Self::find_exact_period(ctx.ac_samples.read_window(), ctx.raw_period);
        ctx.omega = PI2 * self.sample_rate / ctx.period;
        ctx.dt = 1.0 / self.sample_rate;
        ctx
    }
    fn exit(&self) {
        todo!()
    }
}
}
pub(crate) use autocorrelation::*;
mod read_inputs {
use std::sync::Arc;
use sal_core::{dbg::Dbg, error::Error};
use crate::{Context, Eval, Inputs, me};
pub struct ReadInputs {
    inputs: Arc<Inputs>,
    dbg: Dbg,
}
impl ReadInputs {
    pub fn new(parent: &Dbg, inputs: Arc<Inputs>) -> Self {
        let dbg = Dbg::new(parent, me::<Self>());
        Self {
            inputs,
            dbg,
        }
    }
}
impl Eval<Context, Context> for ReadInputs {
    fn eval(&self, mut ctx: Context) -> Context {
        match &self.inputs.rpm() {
            Some(rpm) => {
                ctx.raw_rpm = *rpm;
            }
            None => {
                ctx.err = Some(Error::new(&self.dbg, "eval").err("RPM isn't initialized yet."));
            }
        }
        ctx
    }
    fn exit(&self) {}
}
}
pub use read_inputs::*;
pub(crate) const PI2: f64 = std::f64::consts::PI * 2.0;
pub type TimeDomainSamples<const N: usize> = Arc<[u16; N]>;
pub struct Inputs {
    rpm: AtomicU64,
}
impl Inputs {
    pub fn new() -> Self {
        Self { rpm: AtomicU64::new(f64::NAN.to_bits()) }
    }
    pub fn set_rpm(&self, val: f64) {
        self.rpm.store(val.to_bits(), Ordering::Relaxed);
    }
    pub fn rpm(&self) -> Option<f64> {
        let val = f64::from_bits(self.rpm.load(Ordering::Relaxed));
        if val.is_finite() {
            Some(val)
        } else {
            None
        }
    }
}}
pub use domain::*;
mod kernel {
mod eval {
mod eval {
pub trait Eval<In, Out> {
    fn eval(&self, val: In) -> Out;
    fn exit(&self);
}
}
pub use eval::*;}
pub use eval::*;
mod short_type_name {
pub fn short_type_name<T: ?Sized>() -> &'static str {
    let full = std::any::type_name::<T>();
    full.rsplit("::").next().unwrap_or(full)
}
pub use short_type_name as me;
}
pub(crate) use short_type_name::*;
mod mirror_buffer {
/// Зеркальный кольцевой буфер.
pub struct MirroredBuffer {
    /// Внутренний массив размером 2*capacity.
    buffer: Vec<u16>,
    /// Логический размер окна (N), необходимый для автокорреляции.
    capacity: usize,
    /// Текущий индекс записи (от 0 до N-1).
    write_idx: usize,
    /// Текущая длина накопленных элементов
    len: usize,
}
impl MirroredBuffer {
    /// Выделяет память под буфер на куче.
    /// Вызывается строго один раз при инициализации.
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0; capacity * 2],
            capacity,
            write_idx: 0,
            len: 0,
        }
    }
    /// Возвращает длинну коллекции
    pub fn len(&self) -> usize {
        self.len
    }
    /// Возвращает `true` если буфер наполнился
    pub fn is_full(&self) -> bool {
        self.len == self.capacity
    }
    /// Добавляет новую выборку из АЦП в буфер.
    /// Автоматически дублирует данные для обеспечения непрерывного окна чтения.
    /// Агрументы:
    /// * `chunk` - срез новых данных, длина не должна превышать capacity.
    pub fn push_chunk(&mut self, chunk: &[u16]) {
        let m = chunk.len();
        assert!(m <= self.capacity, "Размер выборки превышает вместимость буфера");
        let space_left = self.capacity - self.write_idx;
        if m <= space_left {
            self.buffer[self.write_idx..self.write_idx + m].copy_from_slice(chunk);
            self.buffer[self.write_idx + self.capacity..self.write_idx + self.capacity + m].copy_from_slice(chunk);
            self.write_idx += m;
        } else {
            let part1 = space_left;
            let part2 = m - space_left;
            self.buffer[self.write_idx..self.capacity].copy_from_slice(&chunk[..part1]);
            self.buffer[self.write_idx + self.capacity..self.capacity * 2].copy_from_slice(&chunk[..part1]);
            self.buffer[0..part2].copy_from_slice(&chunk[part1..]);
            self.buffer[self.capacity..self.capacity + part2].copy_from_slice(&chunk[part1..]);
            self.write_idx = part2;
        }
        if self.write_idx == self.capacity {
            self.write_idx = 0;
        }
        if self.len < self.capacity {
            self.len += m;
            if self.len > self.capacity {
                self.len = self.capacity;
            }
        }
    }
    /// Возвращает непрерывный срез памяти длиной N для математического алгоритма.
    /// Срез отсортирован хронологически (от старейшего отсчета к самому новому).
    pub fn read_window(&self) -> &[u16] {
        &self.buffer[self.write_idx..self.write_idx + self.capacity]
    }
}
///
/// Basic Tests
}
pub(crate) use mirror_buffer::*;}
pub use kernel::*;
pub(crate) use kernel::short_type_name;
