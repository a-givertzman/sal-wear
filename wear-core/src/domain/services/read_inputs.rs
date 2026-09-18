use std::sync::Arc;
use sal_core::{
    dbg::Dbg, 
    error::Error
};
use crate::{
    Eval, GetInputs, domain::context::Context
};
///
/// Пишет актуальные входные значения в контекст [Context]
/// * `inputs` - хранилище приходящих событий
pub struct ReadInputs<I> {
    inputs: Arc<I>,
    dbg: Dbg,
}
impl<I: GetInputs> ReadInputs<I> {
    ///
    /// Новый экземпляр [ReadInputs]
    /// * `inputs` - хранилище приходящих событий
    /// * `parent` - Идентификатор родительской сущности (для отладки).
    pub fn new(parent: &Dbg, inputs: Arc<I>) -> Self {
        let dbg = Dbg::new(parent, "ReadInputs");
        Self {
            inputs,
            dbg,
        }
    }
}
impl<I: GetInputs> Eval<Context, Context> for ReadInputs<I> {
    fn eval(&self, mut ctx: Context) -> Context {
        match &self.inputs.rpm() {
            Some(rpm) => {
                ctx.motor_rpm = Some(*rpm);
            }
            None => {
                ctx.err = Some(Error::new(&self.dbg, "eval").err("RPM isn't initialized yet."));
            }
        }
        match &self.inputs.motor_p() {
            Some(motor_p) => {
                if *motor_p <= 0.0 {
                    ctx.err = Some(Error::new(&self.dbg, "eval").err("Motor power must be > 0."));
                } else {
                    ctx.motor_p = Some(*motor_p);
                }
            }
            None => {
                ctx.err = Some(Error::new(&self.dbg, "eval").err("Motor power isn't initialized yet."));
            }
        }
        match &self.inputs.t_bearing() {
            Some(t_bearing) => {
                ctx.t_bearing = Some(*t_bearing);
            }
            None => {
                ctx.t_bearing = None;
            }
        }       
        match &self.inputs.z_p() {
            Some(z_p) => {
                ctx.z_p = Some(*z_p);
            }
            None => {
                ctx.z_p = None;
            }
        }
        match &self.inputs.d_p() {
            Some(d_p) => {
                ctx.d_p = Some(*d_p);
            }
            None => {
                ctx.d_p = None;
            }
        }
        match &self.inputs.kf() {
            Some(kf) => {
                ctx.kf = Some(*kf);
            }
            None => {
                ctx.kf = None;
            }
        }
        match &self.inputs.yf() {
            Some(yf) => {
                ctx.yf = Some(*yf);
            }
            None => {
                ctx.yf = None;
            }
        }
        match &self.inputs.b() {
            Some(b) => {
                ctx.b = Some(*b);
            }
            None => {
                ctx.b = None;
            }
        }
        match &self.inputs.m() {
            Some(m) => {
                ctx.m = Some(*m);
            }
            None => {
                ctx.m = None;
            }
        }
        match &self.inputs.kh() {
            Some(kh) => {
                ctx.kh = Some(*kh);
            }
            None => {
                ctx.kh = None;
            }
        }
        match &self.inputs.zh() {
            Some(zh) => {
                ctx.zh = Some(*zh);
            }
            None => {
                ctx.zh = None;
            }
        }
        match &self.inputs.m_f() {
            Some(m_f) => {
                ctx.m_f = Some(*m_f);
            }
            None => {
                ctx.m_f = None;
            }
        }
        match &self.inputs.f_lim() {
            Some(f_lim) => {
                ctx.f_lim = Some(*f_lim);
            }
            None => {
                ctx.f_lim = None;
            }
        }
        match &self.inputs.nf_0() {
            Some(nf_0) => {
                ctx.nf_0 = Some(*nf_0);
            }
            None => {
                ctx.nf_0 = None;
            }
        }
        match &self.inputs.m_h() {
            Some(m_h) => {
                ctx.m_h = Some(*m_h);
            }
            None => {
                ctx.m_h = None;
            }
        }
        match &self.inputs.h_lim() {
            Some(h_lim) => {
                ctx.h_lim = Some(*h_lim);
            }
            None => {
                ctx.h_lim = None;
            }
        }
        match &self.inputs.nh_0() {
            Some(nh_0) => {
                ctx.nh_0 = Some(*nh_0);
            }
            None => {
                ctx.nh_0 = None;
            }
        }
        ctx
    }
    //
    fn exit(&self) {}
}