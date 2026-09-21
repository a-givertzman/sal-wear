use serde::Deserialize;
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BearingType {
    /// Шариковый подшипник
    Ball,
    /// Роликовый подшипник
    Roller,
}

impl BearingType {
    /// Возвращает показатель степени кривой усталости (p)
    pub fn p(&self) -> f64 {
        match self {
            BearingType::Ball => 3.0,
            BearingType::Roller => 10.0 / 3.0, // 3.333333333
        }
    }
}