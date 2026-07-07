use std::sync::Arc;
use crate::domain::services::Inputs;
/// Момент двигателя
pub struct MotorTorque {
    inputs: Arc<Inputs>
}
//
//
impl MotorTorque {
    /// Новый экземпляр [MotorTorque] 
    pub fn new(
        inputs: Arc<Inputs>
    ) -> Self {
        Self {
            inputs
        }
    }
}