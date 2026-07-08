/// Хранилище всех входящих событий  
pub struct Inputs {
    /// Текущая частота вращения двигателя [об/мин]
    rpm: Option<f64>,
    /// Текущая мощность двигателя [кВ]
    p_motor: Option<f64>,
    /// Текущая температура подшипникового узла [°C]
    t_bearing: Option<f64>,
    /// Текущая продолжительность расчётного интервала [сек]
    duration: Option<f64>,
}
//
//
impl Inputs {
    pub fn rpm(&self) -> Option<f64> {
        self.rpm
    }
    pub fn p_motor(&self) -> Option<f64> {
        self.p_motor
    }
    pub fn t_bearing(&self) -> Option<f64> {
        self.t_bearing
    }
    pub fn duration(&self) -> Option<f64> {
        self.duration
    }
}
/// Заглушка хранилище всех входящих событий  
pub struct MockInputs {
    /// Текущая частота вращения двигателя [об/мин]
    pub rpm: Option<f64>,
    /// Текущая мощность двигателя [кВ]
    pub p_motor: Option<f64>,
    /// Текущая температура подшипникового узла [°C]
    pub t_bearing: Option<f64>,
    /// Текущая продолжительность расчётного интервала [сек]
    pub duration: Option<f64>,
}
//
//
impl MockInputs {
    pub fn rpm(&self) -> Option<f64> {
        self.rpm
    }
    pub fn p_motor(&self) -> Option<f64> {
        self.p_motor
    }
    pub fn t_bearing(&self) -> Option<f64> {
        self.t_bearing
    }
    pub fn duration(&self) -> Option<f64> {
        self.duration
    }
}