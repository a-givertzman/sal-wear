///
/// Результат вычисления мгновенной фазы вала 
pub struct InstantShaftPhaseCtx {
    // мгновенная фаза вала
    pub shaft_phase: Vec<f64>,
}
impl Default for InstantShaftPhaseCtx {
    fn default() -> Self {
        Self {
            shaft_phase: Default::default(),
        }
    }
}