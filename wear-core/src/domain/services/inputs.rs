/// Хранилище всех входящих событий  
pub struct Inputs {
    /// Текущая частота вращения двигателя [об/мин]
    rpm: Option<f64>,
    /// Текущая мощность двигателя [кВ]
    p_motor: Option<f64>,
    /// Текущая температура [°C]
    t_temp: Option<f64>,
    /// Текущая продолжительность [сек]
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
    pub fn t_temp(&self) -> Option<f64> {
        self.t_temp
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
    /// Текущая температура [°C]
    pub t_temp: Option<f64>,
    /// Текущая продолжительность [сек]
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
    pub fn t_temp(&self) -> Option<f64> {
        self.t_temp
    }
    pub fn duration(&self) -> Option<f64> {
        self.duration
    }
}