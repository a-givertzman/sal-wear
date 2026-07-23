use chrono::Utc;
use sal_core::dbg::Dbg;
use sal_sync::collections::FxHashMap;
use crate::{Eval, ImbContext, Order, Phase, Rms};

/// ### Выявление и классификация макро-механических дефектов на низких кратностях частоты вращения (0.5x..3x RPM).
///
/// #### Различаемые дефекты согласно ISO 20816-1
/// * **0.5X, 1.5X, 2.5x RPM (Механические ослабления / люфты опор)** 
/// * **1X RPM (Статический/динамический дисбаланс):** Рост амплитуды строго на первом порядке.
/// * **2X RPM (Несоосность валов / расцентровка муфт):** Доминирование второго порядка, сопровождаемое осевой вибрацией.
/// * **3X RPM (Механические ослабления / люфты опор):** Появление третьей гармоники и субгармоник (0.5X, 1.5X).
/// 
/// [Подробнее о выявлении дефектов](../../../design/imbalance-detector.md)
pub struct ImbalanceDetector<Child> {
    /// Сдвиг угла между началом и концом выборок FFT в радианах.
    diag: DiagnosticDetector,
    /// Предыдущий узел конвейера вычислений (например, спектральный анализ).
    child: Child,
    dbg: Dbg,
}
impl<Child> ImbalanceDetector<Child>
where
    Child: Eval<ImbContext, ImbContext> + Send + 'static {
    ///
    /// ### Returns `ImbalanceDetector` new instance
    /// - `parent` - Идентификатор родительской сущности (для отладки).
    /// - `child` - Дочерний (предыдущий) расчетный шаг
    pub fn new(parent: impl Into<String>, child: Child) -> Self {
        let dbg = Dbg::new(parent, crate::me::<Self>());
        Self {
            diag: DiagnosticDetector::new(),
            child,
            dbg,
        }
    }
}
impl<Child> Eval<ImbContext, ImbContext> for ImbalanceDetector<Child>
where
    Child: Eval<ImbContext, ImbContext> + Send + 'static {
    //
    #[inline]
    fn eval(&self, ctx: ImbContext) -> ImbContext {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        let values: Vec<(Order, Rms<f64>)> = ctx.results.iter()
            .map(|r| (r.order, r.rms))
            .collect();
        let results = self.diag.diagnose(&values);
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}




/// ### Степень развития дефекта
/// 
/// В стандартах по вибродиагностике зоны классифицируются так:
/// - 🟢 A (Green): Отличное или новое состояние.
/// - 🟡 B (Yellow): Пригодно для длительной эксплуатации без ограничений.
/// - 🟠 C (Orange): Предупреждение (Alarm 1). Пригодно для ограниченной эксплуатации, требуется планирование ремонта.
/// - 🔴 D (Red): Преждевременный отказ (Alarm 2). Опасные вибрации, требуется немедленная остановка.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Отличное или новое состояние.
    Green,
    /// Пригодно для длительной эксплуатации без ограничений.
    Yellow,
    /// Предупреждение (Warn). Пригодно для ограниченной эксплуатации, требуется планирование ремонта.
    Orange,
    /// Преждевременный отказ (Alarm). Опасные вибрации, требуется немедленная остановка.
    Red,
}
pub struct SeverityThresholds {
    pub yellow: f64, // Граница Зеленый -> Желтый
    pub orange: f64, // Граница Желтый -> Оранжевый
    pub red: f64,    // Граница Оранжевый -> Красный
}

/// Вид дефекта
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultKind {
    Healthy,
    /// Дисбаланс
    Imbalance,
    /// Расцентровка
    Misalignment,
    /// Механический люфт
    MechanicalLooseness,
}

/// Шаблон дефекта для сопоставления
pub struct FaultPattern {
    pub fault: FaultKind,
    /// Относительные веса для порядков [0.5, 1.0, 1.5, 2.0, 2.5, 3.0]
    /// 1.0 — критически важен для этого дефекта, 0.0 — должен отсутствовать
    pub weights: [f64; DiagnosticDetector::SIZE],
    // Порог совпадения формы (0.0..1.0)
    pub score_threshold: f64,
    // Для оценки тяжести дефекта
    /// Индекс порядка, по которому мерить абсолютную величину
    pub order_idx: usize,
    /// Порог абсолютного значения RMS
    pub thresholds: SeverityThresholds,
}

/// Детектирование вида и степени развития дефектов
#[derive(Debug, Clone, Copy)]
pub struct DiagnosticResult {
    pub fault: FaultKind,
    pub confidence: f64,
    pub severity: Severity,
}

/// ### Выявление, классификация и оценка критичности макро-механических дефектов 
/// роторного оборудования на низких кратностях частоты вращения (0.5x..3.0x RPM).
///
/// Классификация основана на сопоставлении формы текущего нормированного спектра 
/// с эталонными шаблонами дефектов (косинусное сходство) и последующей оценке 
/// абсолютного уровня вибрации (RMS) по четырехзонной шкале критичности (Green/Yellow/Orange/Red) 
/// в соответствии с принципами стандартов ISO 10816/20816.

struct DiagnosticDetector {
    patterns: Vec<FaultPattern>,
}
//
impl DiagnosticDetector {
    /// Количество анализируемых спектральных компонентов (гармоник и субгармоник).
    const SIZE: usize = 6;
    /// Фиксированный перечень целевых порядков (кратностей оборотной частоты),
    /// на основе которых формируется вектор признаков для диагностики дефектов.
    const ORDER_INDEX: [Order; Self::SIZE] = [
        Order(0.5),
        Order(1.0),
        Order(1.5),
        Order(2.0),
        Order(2.5),
        Order(3.0),
    ];
    /// ### Создает новый экземпляр `DiagnosticDetector`
    /// Инициализирует базу эталонных шаблонов 
    /// дефектов (`Imbalance`, `Misalignment`, `MechanicalLooseness`) с их весовыми 
    /// коэффициентами и пороговыми значениями зон опасности.
    pub fn new() -> Self {
        Self {
            patterns: vec![
                FaultPattern {
                    fault: FaultKind::Imbalance,
                    weights: [0.0, 1.0, 0.0, 0.2, 0.0, 0.0], // Доминирует 1X
                    score_threshold: 0.75,
                    order_idx: 1, // Для дисбаланса смотрим строго на 1.0X (индекс 1)
                    thresholds: SeverityThresholds {
                        yellow: 1.8,  // мм/с (или попугаи из Калмана, настроенные под ваш датчик)
                        orange: 4.5,
                        red: 7.1,
                    },
                },
                FaultPattern {
                    fault: FaultKind::Misalignment,
                    weights: [0.0, 0.5, 0.0, 1.0, 0.0, 0.6], // Доминирует 2X, присутствует 1X и 3X
                    score_threshold: 0.70,
                    order_idx: 3, // Для расцентровки главный — 2.0X (индекс 3)
                    thresholds: SeverityThresholds {
                        yellow: 2.3,
                        orange: 5.4,
                        red: 9.2,
                    },
                },
                FaultPattern {
                    fault: FaultKind::MechanicalLooseness,
                    weights: [0.8, 0.6, 0.8, 0.6, 0.8, 0.6], // Хаос, важны дробные порядки
                    score_threshold: 0.65,
                    order_idx: 0, // Для люфта триггером может быть субгармоника 0.5X (индекс 0)
                    thresholds: SeverityThresholds {
                        yellow: 1.1,
                        orange: 3.2,
                        red: 6.5,
                    },
                },
            ],
        }
    }
    /// ### Выполняет поиск среднеквадратического значения (RMS) для заданного порядка 
    /// в плоском срезе результатов спектрального анализа.
    ///
    /// # Аргументы
    /// * `target` - Целевой порядок (`Order`), который необходимо найти.
    /// * `results` - Срез пар (`Order`, `Rms<f64>`), извлеченных из контекста обработки.
    ///
    /// # Возвращаемое значение
    /// Возвращает найденное значение `Rms<f64>`. Если порядок отсутствует в результатах 
    /// спектрального анализа, возвращает `Rms(0.0)`.
    fn get_rms_for_order(target: &Order, results: &[(Order, Rms<f64>)]) -> Rms<f64> {
        results.iter()
            .find(|(order, _)| order == target)
            .map(|(_, rms)| *rms)
            .unwrap_or(Rms(0.0)) // Если порядок не нашелся, считаем его амплитуду нулевой
    }
    /// ### Выполняет комплексную мультидефектную диагностику на основе вектора признаков низких порядков.
    ///
    /// Метод решает три последовательные задачи:
    /// 1. Формирует упорядоченный вектор признаков и нормализует его по амплитуде максимального пика.
    /// 2. Рассчитывает косинусное сходство (метрику уверенности) формы спектра с каждым шаблоном дефекта.
    /// 3. Для совпавших шаблонов вычисляет цветовую зону критичности по абсолютной амплитуде доминантного пика.
    ///
    /// - `values` - Срез пар (`Order`, `Rms<f64>`), полученных после фильтрации и сглаживания Калманом.
    ///
    /// #### Возвращаемое значение
    /// Возвращает вектор `Vec<DiagnosticResult>`, содержащий все обнаруженные дефекты, 
    /// отсортированные по уровню их опасности (от критических к предупредительным). 
    /// Если уровень вибрации ниже порога чувствительности (0.01) или дефекты не обнаружены, 
    /// возвращается пустой вектор.
    pub fn diagnose(&self, values: &[(Order, Rms<f64>)]) -> Vec<DiagnosticResult> {
        let features = Self::ORDER_INDEX.map(|order| {
            Self::get_rms_for_order(&order, values).value()
        });
        let mut results = Vec::with_capacity(self.patterns.len());
        // Нормализуем входной вектор, чтобы оценивать форму спектра, а не абсолютную амплитуду
        let max_val = features.iter().cloned().fold(f64::NAN, f64::max);
        // Базовая защита: если вибрация на нуле, машина гарантированно в порядке
        if max_val < 0.01 {
            return results;
        }
        // Нормализация для анализа формы спектра
        let normalized = features.map(|feature| feature / max_val);
        // Проверяем каждый RMS в результатах по форме (принадлежность дефектам) и классификируем по величине
        for pattern in &self.patterns {
            // Считаем косинусное сходство или скалярное произведение
            let score: f64 = normalized.iter()
                .zip(pattern.weights.iter())
                .map(|(f, w)| f * w)
                .sum();
            // Если форма спектра похожа на данный дефект
            if score > pattern.score_threshold {
                // Оцениваем степень для него по отфильтрованной RMS.
                let rms_amplitude = features[pattern.order_idx];
                let severity = if rms_amplitude >= pattern.thresholds.red {
                    Severity::Red
                } else if rms_amplitude >= pattern.thresholds.orange {
                    Severity::Orange
                } else if rms_amplitude >= pattern.thresholds.yellow {
                    Severity::Yellow
                } else {
                    Severity::Green
                };
                // Добавляем дефект в отчет, если он вышел из зеленой зоны 
                // или если у него очень высокая степень уверенности по форме
                if severity != Severity::Green || score > 0.85 {
                    results.push(DiagnosticResult {
                        fault: pattern.fault,
                        confidence: score,
                        severity,
                    });
                }
            }
        }
        // ЭТАП 3: Сортируем результаты по критичности (сначала Red, потом Orange и т.д.)
        // Это позволит вызывающему коду сразу видеть самые опасные проблемы
        results.sort_by(|a, b| b.severity.partial_cmp(&a.severity).unwrap());
        results
    }
}
