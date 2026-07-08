use serde::Deserialize;
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
    /// Скорость вращения вала в оборотах в минуту [об/мин]
    #[serde(alias = "rpm")]
    pub rpm: f64,
    /// Радиальная нагрузка на подшипник [H]
    #[serde(alias = "fr-h")]
    pub fr_h: Option<f64>,
    /// Осевая нагрузка на подшипник [H]
    #[serde(alias = "fa-h", default)]
    pub fa_h: f64,
    /// Крутящий момент на валу редуктора [Н·м]
    #[serde(alias = "M-Hm")]
    pub M_Hm: Option<f64>,
    /// Температура подшипникового узла [°C]
    #[serde(alias = "t-temp")]
    pub t_temp: Option<f64>,
    /// Длительность данного устойчивого режима [сек]
    #[serde(alias = "duration")]
    pub duration: f64,
}
///
/// Конфиг подшипника
#[derive(Clone, Debug, Deserialize)]
pub struct BearingConf {
    /// Динамическая грузоподъёмность подшипника [H]
    #[serde(alias = "cr")]
    pub cr: f64,
    /// Показатель степени в формуле ресурса
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
