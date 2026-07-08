use crate::{Inputs, MockInputs};

pub trait GetInputs: Send + Sync {
    fn rpm(&self) -> Option<f64>;
    fn p_motor(&self) -> Option<f64>;
    fn t_temp(&self) -> Option<f64>;
    fn duration(&self) -> Option<f64>;
}

impl GetInputs for Inputs {
    fn rpm(&self) -> Option<f64> { self.rpm() }
    fn p_motor(&self) -> Option<f64> { self.p_motor() }
    fn t_temp(&self) -> Option<f64> { self.t_temp() }
    fn duration(&self) -> Option<f64> { self.duration() }
}

// Реализуем трейт для вашего MockInputs
impl GetInputs for MockInputs {
    fn rpm(&self) -> Option<f64> { self.rpm }
    fn p_motor(&self) -> Option<f64> { self.p_motor }
    fn t_temp(&self) -> Option<f64> { self.t_temp }
    fn duration(&self) -> Option<f64> { self.duration }
}
