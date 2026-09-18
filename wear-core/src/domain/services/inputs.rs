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
    // Ширина зубчатого венца [м] 
    b: Option<f64>,
    // Модуль зубчатого колеса [м] 
    m: Option<f64>,
    // Коэффициент нагрузки изгиба
    kh: Option<f64>,
    // Коэффициент геометрии контакта
    zh: Option<f64>,
    // Предел выносливости по изгибу [Па]
    f_lim: Option<f64>,
    // Показатели степени S–N кривой для изгиба 
    m_f: Option<f64>, 
    // Базовое число циклов при напряжении σ_lim для изгиба
    nf_0: Option<f64>,
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
    pub fn b(&self) -> Option<f64> {
        self.b
    }
    pub fn m(&self) -> Option<f64> {
        self.m
    }
    pub fn kh(&self) -> Option<f64> {
        self.kh
    }
    pub fn zh(&self) -> Option<f64> {
        self.zh
    }
    pub fn f_lim(&self) -> Option<f64> {
        self.f_lim
    }
    pub fn m_f(&self) -> Option<f64> {
        self.m_f
    }
    pub fn nf_0(&self) -> Option<f64> {
        self.nf_0
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
    // Ширина зубчатого венца [м] 
    pub b: Option<f64>,
    // Модуль зубчатого колеса [м] 
    pub m: Option<f64>,
    // Коэффициент нагрузки изгиба
    pub kh: Option<f64>,
    // Коэффициент геометрии контакта
    pub zh: Option<f64>,
    // Предел выносливости по изгибу [Па]
    pub f_lim: Option<f64>,
    // Показатели степени S–N кривой для изгиба 
    pub m_f: Option<f64>, 
    // Базовое число циклов при напряжении σ_lim для изгиба
    pub nf_0: Option<f64>,
    // Предел выносливости по контакту [Па]
    pub h_lim: Option<f64>,
    // Показатели степени S–N кривой для контакта 
    pub m_h: Option<f64>, 
    // Базовое число циклов при напряжении σ_lim для контакта
    pub nh_0: Option<f64>,
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
            b: Some(0.1),
            m: Some(0.1),
            kh: Some(0.1),
            zh: Some(0.1),
            f_lim: Some(0.1),
            m_f: Some(0.1),
            nf_0: Some(0.1),
            h_lim: Some(0.1),
            m_h: Some(0.1),
            nh_0: Some(0.1),            
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
    pub fn b(&self) -> Option<f64> {
        self.b
    }
    pub fn m(&self) -> Option<f64> {
        self.m
    }
    pub fn kh(&self) -> Option<f64> {
        self.kh
    }
    pub fn zh(&self) -> Option<f64> {
        self.zh
    }
    pub fn f_lim(&self) -> Option<f64> {
        self.f_lim
    }
    pub fn m_f(&self) -> Option<f64> {
        self.m_f
    }
    pub fn nf_0(&self) -> Option<f64> {
        self.nf_0
    }
    pub fn h_lim(&self) -> Option<f64> {
        self.h_lim
    }
    pub fn m_h(&self) -> Option<f64> {
        self.m_h
    }
    pub fn nh_0(&self) -> Option<f64> {
        self.nh_0
    }
}