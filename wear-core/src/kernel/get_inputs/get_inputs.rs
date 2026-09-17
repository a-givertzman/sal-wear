use crate::{Inputs, MockInputs};

pub trait GetInputs: Send + Sync {
    fn rpm(&self) -> Option<f64>;
    fn motor_p(&self) -> Option<f64>;
    fn t_bearing(&self) -> Option<f64>;
    fn z_p(&self) -> Option<u64>;
    fn d_p(&self) -> Option<f64>;
    fn kf(&self) -> Option<f64>;
    fn yf(&self) -> Option<f64>;
    fn b(&self) -> Option<f64>;
    fn m(&self) -> Option<f64>;
    fn kh(&self) -> Option<f64>;
    fn zh(&self) -> Option<f64>;
    fn m_f(&self) -> Option<f64>;
    fn f_lim(&self) -> Option<f64>;
    fn nf_0(&self) -> Option<f64>;
    fn m_h(&self) -> Option<f64>;
    fn h_lim(&self) -> Option<f64>;
    fn nh_0(&self) -> Option<f64>;
}

impl GetInputs for Inputs {
    fn rpm(&self) -> Option<f64> { self.rpm() }
    fn motor_p(&self) -> Option<f64> { self.motor_p() }
    fn t_bearing(&self) -> Option<f64> { self.t_bearing() }
    fn z_p(&self) -> Option<u64> { self.z_p() }
    fn d_p(&self) -> Option<f64> { self.d_p() }
    fn kf(&self) -> Option<f64> { self.kf() }
    fn yf(&self) -> Option<f64> { self.yf() }
    fn b(&self) -> Option<f64> { self.b() }
    fn m(&self) -> Option<f64> { self.m() }
    fn kh(&self) -> Option<f64> { self.kh() }
    fn zh(&self) -> Option<f64> { self.zh() }
    fn m_f(&self) -> Option<f64> { self.m_f() }
    fn f_lim(&self) -> Option<f64> { self.f_lim() }
    fn nf_0(&self) -> Option<f64> { self.nf_0() }
    fn m_h(&self) -> Option<f64> { self.m_h() }
    fn h_lim(&self) -> Option<f64> { self.h_lim() }
    fn nh_0(&self) -> Option<f64> { self.nh_0() }
}

// Реализуем трейт для вашего MockInputs
impl GetInputs for MockInputs {
    fn rpm(&self) -> Option<f64> { self.rpm }
    fn motor_p(&self) -> Option<f64> { self.motor_p }
    fn t_bearing(&self) -> Option<f64> { self.t_bearing }
    fn z_p(&self) -> Option<u64> { self.z_p }
    fn d_p(&self) -> Option<f64> { self.d_p }
    fn kf(&self) -> Option<f64> { self.kf }
    fn yf(&self) -> Option<f64> { self.yf }
    fn b(&self) -> Option<f64> { self.b }
    fn m(&self) -> Option<f64> { self.m }
    fn kh(&self) -> Option<f64> { self.kh }
    fn zh(&self) -> Option<f64> { self.zh }
    fn m_f(&self) -> Option<f64> { self.m_f() }
    fn f_lim(&self) -> Option<f64> { self.f_lim() }
    fn nf_0(&self) -> Option<f64> { self.nf_0() }
    fn m_h(&self) -> Option<f64> { self.m_h() }
    fn h_lim(&self) -> Option<f64> { self.h_lim() }
    fn nh_0(&self) -> Option<f64> { self.nh_0() }
}
