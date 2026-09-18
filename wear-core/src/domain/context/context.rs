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
    // Число зубьев ведущей шестерни [кол-во]
    pub(crate) z_p: Option<u64>,
    // Число зубьев ведущей шестерни [м]
    pub(crate) d_p: Option<f64>,
    // Коэффициент нагрузки изгиба 
    pub(crate) kf: Option<f64>,
    // Коэффициент геометрии формы зуба 
    pub(crate) yf: Option<f64>,
    // Ширина зубчатого венца [м] 
    pub(crate) b: Option<f64>,
    // Модуль зубчатого колеса [м] 
    pub(crate) m: Option<f64>,
    // Коэффициент нагрузки изгиба
    pub(crate) kh: Option<f64>,
    // Коэффициент геометрии контакта
    pub(crate) zh: Option<f64>,
    // Предел выносливости по изгибу [Па]
    pub(crate) f_lim: Option<f64>,
    // Показатели степени S–N кривой для изгиба 
    pub(crate) m_f: Option<f64>, 
    // Базовое число циклов при напряжении σ_lim для изгиба
    pub(crate) nf_0: Option<f64>,
    // Предел выносливости по контакту [Па]
    pub(crate) h_lim: Option<f64>,
    // Показатели степени S–N кривой для контакта 
    pub(crate) m_h: Option<f64>, 
    // Базовое число циклов при напряжении σ_lim для контакта
    pub(crate) nh_0: Option<f64>,
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
    /// Накопленное повреждение подшипника
    pub(crate) bearing_accumulated_wear: f64,
    /// Температурный коэффициент ускорения износа (Коэффициент Вант-Гоффа)
    pub(crate) temp_coeff: f64,
    /// Накопленное повреждение подшипника c учётом температуры
    pub(crate) bearing_temp_accumulated_wear: f64,
    /// Частота вращения зубчатой передачи
    pub(crate) rotational_frequency: f64,
    /// Частота зацепления
    pub(crate) gear_mesh_frequency: f64,
    /// Число циклов зацепления
    pub(crate) number_mesh_cycles: f64,
    /// Окружная сила
    pub(crate) tangential_force: f64,
    /// Напряжение изгиба
    pub(crate) bending_stresses: f64,
    /// Напряжение контакта
    pub(crate) contact_stresses: f64,
    // Допустимое число циклов (S–N) для изгиба
    pub(crate) bending_num_cycles: f64,
    // Допустимое число циклов (S–N) для контакта
    pub(crate) contact_num_cycles: f64,
    // Доля повреждения от усталости при изгибе
    pub(crate) bending_fatigue_damage: f64,
    // Доля повреждения от усталости при контакте
    pub(crate) contact_fatigue_damage: f64,
    // Повреждение зуба
    pub(crate) tooth_damage: f64,
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
            kf: None,
            yf: None,
            b: None,
            m: None,
            z_p: None,
            d_p: None,
            kh: None,
            zh: None,
            m_f: None,
            f_lim: None,
            nf_0: None,
            m_h: None,
            h_lim: None,
            nh_0: None,
            duration: 0.0,
            motor_torque: 0.0,
            radial_load: 0.0,
            axial_load: 0.0,
            equivalent_load: 0.0,
            basic_rating_life: 0.0,
            limiting_speed: 0.0,
            actual_speed: 0.0,
            bearing_accumulated_wear: 0.0,
            temp_coeff: 0.0,
            bearing_temp_accumulated_wear: 0.0,
            rotational_frequency: 0.0,
            gear_mesh_frequency: 0.0,
            number_mesh_cycles: 0.0,
            tangential_force: 0.0,
            bending_stresses: 0.0,
            contact_stresses: 0.0,
            bending_num_cycles: 0.0,
            contact_num_cycles: 0.0,
            bending_fatigue_damage: 0.0,
            contact_fatigue_damage: 0.0,
            tooth_damage: 0.0,
            err: None,
        }
    }
    ///
    /// Конструктор для использования исключительно в unit-тестах
    #[cfg(test)]
    pub fn new_test(duration: f64,) -> Self {
        Self {
            duration, // Задаем требуемое для тестов время
            ..Self::new()   // Все остальные поля инициализируем дефолтными значениями
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