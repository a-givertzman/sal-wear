use chrono::{DateTime, Utc};
use crate::{FaultKind, Order, Phase, Rms, Rpm, Severity};

/// Результаты диагностики
pub struct DiagResult {
    pub ts: DateTime<Utc>,
    pub order: Order,
    pub order_id: String,
    pub rms: Rms<f64>,
    pub phase: Phase<f64>,
    pub rpm: Rpm<f64>,
    pub kind: FaultKind,
    pub severity: Severity,
}
impl DiagResult {
    pub fn new(ts: DateTime<Utc>, order: Order, order_id: String, rms: Rms<f64>, phase: Phase<f64>, rpm: Rpm<f64>) -> Self {
        Self {
            ts,
            order,
            order_id,
            rms,
            phase,
            rpm,
            kind: FaultKind::Healthy,
            severity: Severity::Green,
        }
    }
    /// Добавляет тип дефекта к результату
    pub fn with_kind(mut self, kind: FaultKind) -> Self {
        self.kind = kind;
        self
    }
    /// Добавляет степень дефекта к результату
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }
}