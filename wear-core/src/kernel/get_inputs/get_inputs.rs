use crate::{Inputs, MockInputs};

pub trait GetInputs: Send + Sync {
    fn rpm(&self) -> Option<f64>;
    fn motor_p(&self) -> Option<f64>;
    fn t_bearing(&self) -> Option<f64>;
    fn z_p(&self) -> Option<u64>;
}

impl GetInputs for Inputs {
    fn rpm(&self) -> Option<f64> { self.rpm() }
    fn motor_p(&self) -> Option<f64> { self.motor_p() }
    fn t_bearing(&self) -> Option<f64> { self.t_bearing() }
    fn z_p(&self) -> Option<u64> { self.z_p() }
}

// Реализуем трейт для вашего MockInputs
impl GetInputs for MockInputs {
    fn rpm(&self) -> Option<f64> { self.rpm }
    fn motor_p(&self) -> Option<f64> { self.motor_p }
    fn t_bearing(&self) -> Option<f64> { self.t_bearing }
    fn z_p(&self) -> Option<u64> { self.z_p }
}
