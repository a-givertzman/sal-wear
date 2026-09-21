-- 1. Справочник оборудования
CREATE TABLE equipment (
    id          integer GENERATED ALWAYS AS IDENTITY,
    name        VARCHAR(255) NOT NULL,       -- Наименование (например, 'Насос НП-101')
    model       VARCHAR(100),                -- Модель/Тип агрегата
    created_at  TIMESTAMPTZ DEFAULT clock_timestamp() NOT NULL,
    CONSTRAINT pk_equipment PRIMARY KEY (id),
    CONSTRAINT uq_equipment_name UNIQUE (name)
);

-- 2. Таблица учета наработки и ресурса (Wear & Lifespan)
CREATE TABLE equipment_wear (
    equipment_id       INTEGER PRIMARY KEY REFERENCES equipment(id) ON DELETE CASCADE,
    operating_hours    REAL NOT NULL DEFAULT 0.0,  -- Фактическая наработка (моточасы)
    nominal_resource   REAL NOT NULL,              -- Номинальный ресурс до кап. ремонта (моточасы)
    updated_at         TIMESTAMPTZ NOT NULL        -- Время последнего обновления наработки
);

-- 3. Таблица трендов вибрации
CREATE TABLE vibration_trends (
    timestamp      TIMESTAMPTZ NOT NULL,
    equipment_id   INTEGER NOT NULL REFERENCES equipment(id) ON DELETE CASCADE,
    order_id       VARCHAR(10) NOT NULL,       -- '1x', '2x', '3x', '0.5x' и т.д.
    rms_value      REAL NOT NULL,              -- Амплитуда (RMS)
    phase          REAL NOT NULL,              -- Фаза в градусах [0..360)
    rpm            REAL NOT NULL,              -- Текущие обороты вала
    
    PRIMARY KEY (timestamp, equipment_id, order_id)
);

-- Индекс для быстрой фильтрации по конкретному агрегату и гармонике
CREATE INDEX idx_equip_order_trends ON vibration_trends (equipment_id, order_id, timestamp DESC);

-- Справочник видов дефектов
CREATE TABLE fault_kind (
    id          VARCHAR(64) NOT NULL,
    description TEXT NOT NULL,

    PRIMARY KEY (id)
)
INSERT INTO fault_kind (id, description) VALUES
    ('Imbalance', 'Дисбаланс'),
    ('Misalignment', 'Расцентровка'),
    ('MechanicalLooseness', 'Механический люфт');

-- Степень развития дефекта (Зоны ISO 10816 / 20816)
CREATE TYPE vibration_severity AS ENUM (
    'green',   -- Отличное или новое состояние.
    'yellow',  -- Пригодно для длительной эксплуатации без ограничений.
    'orange',  -- Предупреждение (Warn). Пригодно для ограниченной эксплуатации, требуется планирование ремонта.
    'red'      -- Преждевременный отказ (Alarm). Опасные вибрации, требуется немедленная остановка.
);
COMMENT ON TYPE vibration_severity VALUE 'green' IS 'Отличное или новое состояние.';
COMMENT ON TYPE vibration_severity VALUE 'yellow' IS 'Пригодно для длительной эксплуатации без ограничений.';
COMMENT ON TYPE vibration_severity VALUE 'orange' IS 'Предупреждение (Warn). Пригодно для ограниченной эксплуатации, требуется планирование ремонта.';
COMMENT ON TYPE vibration_severity VALUE 'red' IS 'Преждевременный отказ (Alarm). Опасные вибрации, требуется немедленная остановка.';

-- Оценка состояния оборудования на основании вибрации
CREATE TABLE vibration_faults (
    timestamp      TIMESTAMPTZ NOT NULL,
    equipment_id   INTEGER NOT NULL REFERENCES equipment(id) ON DELETE CASCADE,
    -- Вид неисправности
    fault_kind     VARCHAR(64) NOT NULL REFERENCES fault_kind(id),
    -- Метрика сходства с патерном дефекта [0.0, 1.0]
    score          DOUBLE PRECISION NOT NULL,
    -- Степень опасности текущего дефекта
    -- Green, Yellow, Orange, Red
    severity       vibration_severity NOT NULL,
    -- Текущие обороты расчете, для валидации диагноза
    rpm            DOUBLE PRECISION NOT NULL 
    -- Гарантирует, что для каждого оборудования 
    -- хранится ровно ОДНА запись по конкретному дефекту
    PRIMARY KEY (equipment_id, fault_kind)
);


-- SQL-запрос для вывода списка оборудования с худшим статусом
SELECT 
    e.id AS equipment_id,
    e.name AS equipment_name,
    -- MAX() для ENUM в Postgres выберет самое критическое состояние (последнее в списке ENUM)
    MAX(vf.severity) AS overall_severity,
    -- Собираем список всех обнаруженных дефектов, которые вышли из зоны 'green'
    STRING_AGG(
        CASE WHEN vf.severity != 'green' THEN fk.description END, 
        ', '
    ) AS active_faults,
    MAX(vf.timestamp) AS last_update
FROM equipment e
LEFT JOIN vibration_faults vf ON e.id = vf.equipment_id
LEFT JOIN fault_kind fk ON vf.fault_kind = fk.id
GROUP BY e.id, e.name
ORDER BY 
    -- Сначала показываем самое "красное" и "оранжевое" оборудование
    overall_severity DESC NULLS LAST, 
    e.name;
