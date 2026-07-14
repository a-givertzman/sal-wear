use serde::Deserialize;
/// # Шаблон конфигурации YAML (`config.yaml`)
/// ```yaml
/// steady_state:
///   motor-d-m: 0.050        # Диаметр вала двигателя [м]
/// 
/// bearing:
///   cr: 45000.0             # Динамическая грузоподъемность [Н]
///   p: 3.333333333          # Показатель степени кривой усталости (3.0 — шарик, 10/3 — ролик)
///   X: 1.0                  # Коэффициент радиальной нагрузки
///   Y: 0.0                  # Коэффициент осевой нагрузки
/// 
/// temp_model:
///   t-ref: 40.0             # Опорная (номинальная) температура [°C]
///   q10: 2.0                # Коэффициент ускорения износа на каждые +10°C (Вант-Гофф)
///   t-min: -20.0            # Минимально допустимая температура [°C]
///   t-max: 120.0            # Максимально допустимая температура [°C]
/// ```
///
/// Главная конфигурация расчёта остаточного ресурса
#[derive(Clone, Debug, Deserialize)]
pub struct WearCoreConf {
    /// Конфиг текущего устойчивого режима
    pub steady_state: SteadyStateConf,
    /// Конфиг подшипника
    pub bearing: BearingConf,
    /// Конфиг температурной модели
    pub temp_model: TempModelConf,
}
///
/// Конфиг текущего устойчивого режима
#[derive(Clone, Debug, Deserialize)]
pub struct SteadyStateConf {
    /// Диаметр вала двигателя [м]
    #[serde(alias = "motor-d-m")]
    pub motor_d: f64,
}
///
/// Конфиг подшипника
#[derive(Clone, Debug, Deserialize)]
pub struct BearingConf {
    /// Динамическая грузоподъёмность подшипника [H]
    #[serde(alias = "cr")]
    pub cr: f64,
    /// Показатель степени кривой усталости [безразмерная величина]
    #[serde(alias = "p")]
    pub p: f64,
    /// Коэффициент радиальной нагрузки
    #[serde(alias = "X")]
    pub X: f64,
    /// Коэффициент осевой нагрузки
    #[serde(alias = "Y")]
    pub Y: f64,
}
///
/// Конфиг температурной модели
#[derive(Clone, Debug, Deserialize)]
pub struct TempModelConf {
    /// Опорная температура (номинальная рабочая температура) [°C]
    #[serde(alias = "t-ref")]
    pub t_ref: f64,
    /// Коэффициент ускорения износа на +10°C
    #[serde(alias = "q10")]
    pub q10: f64,
    /// Минимально допустимая температура [°C]
    #[serde(alias = "t-min")]
    pub t_min: f64,
    /// Максимально допустимая температура [°C]
    #[serde(alias = "t-max")]
    pub t_max: f64,

}
