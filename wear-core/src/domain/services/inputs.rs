use crate::domain::bearing::bearing_conf::BearingConf;
/// Хранилище всех входящих событий  
pub struct Inputs {
    conf: BearingConf
}
//
//
impl Inputs {
    /// Новый экземпляр [Inputs]
    pub fn new(conf: BearingConf) -> Self {
        Self { 
            conf 
        }
    }
}