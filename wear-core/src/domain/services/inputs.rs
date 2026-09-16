/// Хранилище всех входящих событий  
pub struct Inputs {
    /// Текущая частота вращения двигателя [об/мин]
    rpm: Option<f64>,
    /// Текущая мощность двигателя [кВ]
    motor_p: Option<f64>,
    /// Текущая температура подшипникового узла [°C]
    t_bearing: Option<f64>,
    // Число зубьев ведущей шестерни [кол-во]
    z_p: Option<u64>,
    // Делительный диаметр шестерни [м]
    d_p: Option<f64>,
    // Коэффициент нагрузки изгиба 
    kf: Option<f64>,
    // Коэффициент геометрии формы зуба 
    yf: Option<f64>,
}
//
//
impl Inputs {
    pub fn rpm(&self) -> Option<f64> {
        self.rpm
    }
    pub fn motor_p(&self) -> Option<f64> {
        self.motor_p
    }
    pub fn t_bearing(&self) -> Option<f64> {
        self.t_bearing
    }
    pub fn z_p(&self) -> Option<u64> {
        self.z_p
    }
    pub fn d_p(&self) -> Option<f64> {
        self.d_p
    }
    pub fn kf(&self) -> Option<f64> {
        self.kf
    }
    pub fn yf(&self) -> Option<f64> {
        self.yf
    }
}
/// Заглушка хранилище всех входящих событий  
pub struct MockInputs {
    /// Текущая частота вращения двигателя [об/мин]
    pub rpm: Option<f64>,
    /// Текущая мощность двигателя [кВ]
    pub motor_p: Option<f64>,
    /// Текущая температура подшипникового узла [°C]
    pub t_bearing: Option<f64>,    
    // Число зубьев ведущей шестерни [кол-во]
    pub z_p: Option<u64>,
    // Делительный диаметр шестерни [м]
    pub d_p: Option<f64>,
    // Коэффициент нагрузки изгиба 
    pub kf: Option<f64>,
    // Коэффициент геометрии формы зуба 
    pub yf: Option<f64>,
}
//
//
impl MockInputs {
    pub fn new() -> Self {
        Self { 
            rpm: Some(0.1), 
            motor_p: Some(0.1), 
            t_bearing: Some(0.1), 
            z_p: Some(1) ,
            d_p: Some(0.1),
            kf: Some(0.1),
            yf: Some(0.1),
        }
    }
    pub fn rpm(&self) -> Option<f64> {
        self.rpm
    }
    pub fn motor_p(&self) -> Option<f64> {
        self.motor_p
    }
    pub fn t_bearing(&self) -> Option<f64> {
        self.t_bearing
    }
    pub fn z_p(&self) -> Option<u64> {
        self.z_p
    }
    pub fn d_p(&self) -> Option<f64> {
        self.d_p
    }
    pub fn kf(&self) -> Option<f64> {
        self.kf
    }
    pub fn yf(&self) -> Option<f64> {
        self.yf
    }
}