use std::sync::Arc;

use sal_core::dbg::Dbg;
use sal_sync::thread_pool::ThreadPool;

use crate::{AngularGrid, Autocorrelation, Conf};

#[test]
fn complex () {
    let dbg = Dbg::own("complex-test");
    let tp = ThreadPool::new(dbg, Some(8));
    let conf: Conf = serde_yaml::from_str(r#"
        hardware:
            sample-rate-hz: 320000
            chunk-size: 512
        angular:
            resolution: 256
            fft-size: 8192
        bands:
            low-order: 0.5..10
            mid-hz: ..5000
            high-hz: 5000..10000
    "#).unwrap();
    let mut samples = Arc::new([0u16; 512]);
    let angular_grid = AngularGrid::new(samples, rough_rpm, conf.hardware.sample_rate_hz, 0);
    Autocorrelation::new(&dbg,
        
    )
}


struct Inputs {
    rpm: Option<f64>,
}
impl Inputs {
    pub fn rpm(&self) -> Option<f64> {
        self.rpm
    }
}