use core::f64;
use sal_core::error::Error;
///
/// Контейнер для передачи данных между вычислительными шагами
pub struct Context {
    /// Скорость вращения вала в оборотах в минуту [об/мин]
    pub(crate) rpm: Option<f64>,
    /// Мощность двигателя [кВ]
    pub(crate) p_motor: Option<f64>,
    /// Текущая температура [°C]
    pub(crate) t_temp: Option<f64>,
    /// Текущая продолжительность [сек]
    pub(crate) duration: Option<f64>,
    /// Крутящий момент мотора [Н·м]
    pub(crate) motor_torque: Option<f64>,
    /// Текущая ошибка вычислений
    /// Будет `Some(Error)` если шаг вычислений вернул ошибку, остальные шали эскалируют наверх.
    pub(crate) err: Option<Error>,
}
impl Context {
    ///
    /// Новый экземпляр [Context]
    pub fn new() -> Self {
        Self {
            rpm: None,
            p_motor: None,
            t_temp: None,
            duration: None,
            motor_torque: None,
            err: None,
        }
    }
    ///
    /// Добавление контекст ошибки или 
    /// инициализация новой ошибки в текущем контексте
    pub fn pass_err(mut self, me: impl Into<String>, area: impl Into<String>) -> Context {
        self.err = match self.err {
            Some(err) => Some(Error::new(me, area).pass(err)),
            None => Some(Error::new(me, area)),
        };
        self
    }
}