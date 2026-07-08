use core::f64;
use sal_core::error::Error;
///
/// Контейнер для передачи данных между вычислительными шагами
pub struct Context {
    /// Скорость вращения вала в оборотах в минуту [об/мин]
    pub(crate) motor_rpm: Option<f64>,
    /// Мощность двигателя [кВ]
    pub(crate) motor_p: Option<f64>,
    /// Текущая температура [°C]
    pub(crate) motor_t: Option<f64>,
    /// Текущая продолжительность [сек]
    pub(crate) duration: f64,
    /// Крутящий момент мотора [Н·м]
    pub(crate) motor_torque: f64,
    /// Текущая ошибка вычислений
    /// Будет `Some(Error)` если шаг вычислений вернул ошибку, остальные шали эскалируют наверх.
    pub(crate) err: Option<Error>,
}
impl Context {
    ///
    /// Новый экземпляр [Context]
    pub fn new() -> Self {
        Self {
            motor_rpm: None,
            motor_p: None,
            motor_t: None,
            duration: 0.0,
            motor_torque: 0.0,
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