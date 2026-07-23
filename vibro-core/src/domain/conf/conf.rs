use std::ops::{Bound, RangeBounds};
use serde::{Deserialize, Deserializer};

/// Главная конфигурация конвейера обработки вибросигнала.
/// Инкапсулирует базовые аппаратные константы, параметры сетки и границы фильтров.
#[derive(Clone, Debug, Deserialize)]
pub struct Conf {
    /// Параметры аппаратной части (АЦП, чанки).
    pub hardware: HardwareConf,
    /// Настройки угловой сетки и буферов для Order Tracking.
    pub angular: AngularConf,
    /// Частотные диапазоны для детекторов.
    pub bands: BandsConf,
}

/// Аппаратные параметры источника данных.
#[derive(Clone, Debug, Deserialize)]
pub struct HardwareConf {
    /// Возвращает базовую частоту дискретизации в Гц.
    #[serde(alias = "sample-rate-hz")]
    pub sample_rate_hz: f32,
    /// Возвращает размер пакета данных, поступающего из сети.
    #[serde(alias = "chunk-size")]
    pub chunk_size: usize,
}

/// ### Настройки углового домена (Order Tracking).
#[derive(Clone, Debug, Deserialize)]
pub struct AngularConf {
    /// ### Максимальный порядок (кратность частоты вращения), до которого производится спектральный анализ.
    /// 
    /// Определяет верхнюю границу частотного диапазона в угловой области.
    /// По теории синхронного накопления и Order Tracking, он напрямую связан с максимальной
    /// анализируемой физической частотой:
    /// 
    ///     `F_max [Гц] = max_order * (RPM / 60)`.
    /// 
    /// Например при 3000 об/мин (50 Гц) и max_order = 100, спектр покроет полосу от 0 до 5 кГц.
    /// Или max_order = 300 покроет полосу от 0 до 15 кГц.
    #[serde(alias = "max-order")]
    pub max_order: f32,
    /// ### Требуемая спектральное разрешение в угловом домене.
    /// 
    /// Шаг между соседними спектральными линиями.
    /// Определяет способность алгоритма разделять близко расположенные по частоте дефекты
    /// (например, отличить гармонику вала от близкой частоты перекатывания подшипника).
    /// 
    /// Физически равен единице, деленной на количество полных оборотов в окне БПФ:
    /// 
    ///     `ΔO = 1 / N_rev`.
    /// 
    /// Например, шаг 0.05 порядка создаст на графике "бины" 0.00, 0.05, 0.10 и т.д.
    /// Это позволит отличить дефект на 4.20X от шума на 4.25X.
    #[serde(alias = "resolution")]
    pub resolution: f32,
    /// ### Плотность угловой дискретизации (сэмплов на оборот).
    /// 
    /// Количество отсчетов (сэмплов), фиксируемых или рассчитываемых
    /// за один полный оборот ротора (на 2π радиан).
    /// 
    /// По аналогии с временным доменом (где есть частота дискретизации `Fs` в Гц),
    /// этот параметр является "угловой частотой дискретизации" и измеряется в отсчетах на оборот.
    /// 
    /// Согласно теореме Найквиста-Котельникова, должен быть как минимум в 2 раза
    /// (на практике с учетом спада фильтра — в 2.56 раза) больше, чем `max_order`.
    #[serde(alias = "points-per-turn")]
    n_rev: Option<usize>,
}
impl AngularConf {
    /// ### Плотность угловой дискретизации (сэмплов на оборот).
    /// 
    /// Количество отсчетов (сэмплов), фиксируемых или рассчитываемых
    /// за один полный оборот ротора (на 2π радиан).
    /// 
    /// По аналогии с временным доменом (где есть частота дискретизации `Fs` в Гц),
    /// этот параметр является "угловой частотой дискретизации" и измеряется в отсчетах на оборот.
    /// 
    /// Согласно теореме Найквиста-Котельникова, должен быть как минимум в 2 раза
    /// (на практике с учетом спада фильтра — в 2.56 раза) больше, чем `max_order`.
    ///
    /// Автоматически округляется до ближайшей большей степени двойки.
    #[inline]
    pub fn n_rev(&self) -> usize {
        match self.n_rev {
            Some(ppt) => {
                log::debug!("AngularConf.n_rev | Manually specified: {}", ppt);
                ppt
            }
            None => {
                let min_points = (self.max_order * 2.5).ceil() as usize;
                let ppt = min_points.next_power_of_two();
                log::debug!("AngularConf.n_rev | Auto calculated: {}", ppt);
                ppt
            }
        }
    }
    /// ### Длина окна FFT анализа, выраженная в количестве полных оборотов вала.
    /// 
    /// Теоретически показывает, за какой угловой интервал (`2π x N_rev` радиан)
    /// собираются данные для выполнения одного БПФ. Зависит исключительно от
    /// требуемого разрешения спектра. Округление до ближайшей степени двойки
    /// необходимо для оптимизации алгоритма FFT.
    #[inline]
    pub fn fft_turns(&self) -> usize {
        let min_turns = (1.0 / self.resolution).ceil() as usize;
        min_turns.next_power_of_two()
    }
    /// Вычисляет угловой шаг в радианах.
    /// ### Шаг угловой сетки (дискрет угла) в радианах.
    /// 
    /// Величина изменения фазового угла ротора между двумя соседними ресемплированными
    /// отсчетами в угловом домене. Вычисляется как `Δtheta = 2π / N_rev`.
    /// В Order Tracking этот шаг является строго константным, в отличие от временного шага Δt,
    /// который при изменении скорости вращения постоянно меняется.
    #[inline]
    pub fn angular_step_rad(&self) -> f64 {
        std::f64::consts::TAU / (self.n_rev() as f64)
    }
    /// ### Размер FFT выборки (длина буфера) для спектрального анализа в угловой области.
    /// 
    /// Общее количество эквидистантных по углу отсчетов, подаваемых на вход БПФ.
    /// Равен произведению количества точек на оборот на количество оборотов в окне.
    /// Является прямым аналогом размера БПФ временного сигнала (N_fft).
    ///
    /// **Пример математического расчета*
    /// - Требуемое разрешение `ΔO = 0.01` порядка
    /// - Минимально необходимое число точек: `Nmin = Nrev / ΔO = 64 / 0.01 = 6400` отсчетов.
    /// - Для эффективной FFT значение округляется вверх до ближайшей степени двойки через `next_power_of_two()`.
    /// - Если заданный размер окна: `Nfft = 8192` точки.
    /// - При этом реальное разрешение спектра составляет `ΔO = 64 / 8192 ≈ 0.0078` порядка 
    /// - Данные накапливаются за `≈128` полных оборотов ротора.
    #[inline]
    pub fn n_fft(&self) -> usize {
        self.n_rev() * self.fft_turns()
    }
}

/// Границы частотных диапазонов для фильтрации и анализа.
#[derive(Clone, Debug, Deserialize)]
pub struct BandsConf {
    /// Границы низкочастотной зоны в порядках (Orders).
    #[serde(alias = "low-order", deserialize_with = "parse_range")]
    pub low_order: (Bound<f32>, Bound<f32>),
    /// Верхняя граница среднего диапазона в Герцах (нижняя определяется порядками).
    #[serde(alias = "mid-hz", deserialize_with = "parse_range")]
    pub mid_hz: (Bound<f32>, Bound<f32>),
    /// Границы высокочастотной зоны в Герцах.
    #[serde(alias = "high-hz", deserialize_with = "parse_range")]
    pub high_hz: (Bound<f32>, Bound<f32>),
}
impl BandsConf {
    /// Возвращает верхнюю границу для ФНЧ в порядках (Orders).
    pub fn low_cutoff_order(&self) -> f32 {
        self.low_range_orders().1
    }
    /// Возвращает границы низкочастотной зоны в порядках (Orders).
    pub fn low_range_orders(&self) -> (f32, f32) {
        let (start, end) = extract_bounds(&(self.low_order));
        (start.unwrap(), end.unwrap())
    }
    /// Возвращает границы высокочастотной зоны в Герцах.
    pub fn high_range_hz(&self) -> (f32, f32) {
        let (start, end) = extract_bounds(&(self.high_hz));
        (start.unwrap(), end.unwrap())
    }
    /// Возвращает верхнюю границу среднего диапазона в Герцах (нижняя определяется порядками).
    pub fn mid_range_max_hz(&self) -> f32 {
        let (_, end) = extract_bounds(&(self.mid_hz));
        end.unwrap()
    }
}
fn extract_bounds<T: Clone>(range: &(Bound<T>, Bound<T>)) -> (Option<T>, Option<T>) {
    let f = |b: Bound<&T>| match b {
        Bound::Included(v) | Bound::Excluded(v) => Some(v.clone()),
        Bound::Unbounded => None,
    };
    (f(range.start_bound()), f(range.end_bound()))
}
fn parse_range<'de, D>(deserializer: D) -> Result<(Bound<f32>, Bound<f32>), D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    // Разделяем строку по разделителю ".."
    let (low_str, high_str) = s.split_once("..").ok_or_else(|| {
        serde::de::Error::custom(format!("Неверный формат диапазона: '{}'. Ожидалось 'start..end'", s))
    })?;
    let low_str = low_str.trim(); 
    let high_str = high_str.trim(); 
    let start = if low_str.is_empty() {
        Bound::Unbounded
    } else {
        let val = low_str.parse::<f32>().map_err(serde::de::Error::custom)?;
        Bound::Included(val)
    };
    let end = if high_str.is_empty() {
        Bound::Unbounded
    } else {
        let val = high_str.parse::<f32>().map_err(serde::de::Error::custom)?;
        Bound::Excluded(val)
    };
    Ok((start, end))
}

///
/// Basic Tests
#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;
    /// Тестирование контрактов конфигурации.
    /// Проверяет правильность вычисления внутренних констант для FFT и ресемплинга.
    #[test]
    fn test_conf_contracts() {
        // Допустим, n_rev = 256 [cite: 341] и fft_revolutions = 32 
        let conf: AngularConf = serde_yaml::from_str(r#"
            max-order: 100
            resolution: 0.05
        "#).unwrap();
        // Шаг угла должен вычисляться в f64 [cite: 341]
        let expected_step = 2.0 * PI / 256.0;
        assert!((conf.angular_step_rad() - expected_step).abs() < 1e-12);
        // Буфер FFT должен математически гарантированно быть 8192 [cite: 344]
        assert_eq!(conf.n_fft(), 8192);
    }
}