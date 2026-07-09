use core::f64;
use sal_core::error::Error;
///
/// Контейнер для передачи данных между вычислительными шагами
pub struct Context {
    ///
    /// Исходные данные из Inputs
    /// 
    /// Скорость вращения вала в оборотах в минуту [об/мин]
    pub(crate) motor_rpm: Option<f64>,
    /// Текущая мощность двигателя [кВ]
    pub(crate) motor_p: Option<f64>,
    /// Текущая температура подшипникового узла [°C]
    pub(crate) t_bearing: Option<f64>,
    ///
    /// Расчетные значения
    /// 
    /// Текущая продолжительность расчётного интервала [сек]
    pub(crate) duration: f64,
    /// Крутящий момент мотора [Н·м]
    pub(crate) motor_torque: f64,
    /// Радиальная нагрузка на подшипник [H]
    pub(crate) radial_load: f64,
    /// Осевая нагрузка на подшипник [H]
    pub(crate) axial_load: f64,  
    /// Эквивалентная нагрузка на подшипник [H]
    pub(crate) equivalent_load: f64,  
    /// Номинальный ресурс подшипника [10^6 об]
    pub(crate) basic_rating_life: f64,  
    /// Допустимое количество оборотов подшипника [об]
    pub(crate) limiting_speed: f64,  
    /// Фактическое количество оборотов подшипника [об]
    pub(crate) actual_speed: f64,  
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
            t_bearing: None,
            duration: 0.0,
            motor_torque: 0.0,
            radial_load: 0.0,
            axial_load: 0.0,
            equivalent_load: 0.0,
            basic_rating_life: 0.0,
            limiting_speed: 0.0,
            actual_speed: 0.0,
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