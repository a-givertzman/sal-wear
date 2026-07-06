use sal_core::dbg::Dbg;
use sal_sync::thread_pool::ThreadPool;

use crate::{AngularGrid, Conf};

#[test]
fn complex () {
    let dbg = Dbg::own("complex-test");
    let tp = ThreadPool::new(dbg, Some(8));
    let conf = Conf::new(hardware, angular, bands);
    let mut samples = [0u16; 512];
    let angular_grid = AngularGrid::new(samples, rough_rpm, conf.hardware().sample_rate_hz(), 0);
}


struct Inputs {
    rpm: Option<f64>,
}
impl Inputs {
    pub fn rpm(&self) -> Option<f64> {
        self.rpm
    }
}