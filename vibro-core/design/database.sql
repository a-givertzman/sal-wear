-- 1. Справочник оборудования
CREATE TABLE equipment (
    id          SERIAL PRIMARY KEY,
    name        VARCHAR(255) NOT NULL,       -- Наименование (например, 'Насос НП-101')
    model       VARCHAR(100),                -- Модель/Тип агрегата
    created_at  TIMESTAMPTZ DEFAULT NOW()
);

-- 2. Таблица учета наработки и ресурса (Wear & Lifespan)
CREATE TABLE equipment_wear (
    equipment_id       INTEGER PRIMARY KEY REFERENCES equipment(id) ON DELETE CASCADE,
    operating_hours    REAL NOT NULL DEFAULT 0.0,  -- Фактическая наработка (моточасы)
    nominal_resource   REAL NOT NULL,              -- Номинальный ресурс до кап. ремонта (моточасы)
    updated_at         TIMESTAMPTZ NOT NULL        -- Время последнего обновления наработки
);

-- 3. Обновленная таблица трендов вибрации
CREATE TABLE order_vibration_trends (
    timestamp      TIMESTAMPTZ NOT NULL,
    equipment_id   INTEGER NOT NULL REFERENCES equipment(id) ON DELETE CASCADE,
    order_id       VARCHAR(10) NOT NULL,       -- '1x', '2x', '3x', '0.5x' и т.д.
    rms_value      REAL NOT NULL,              -- Амплитуда (RMS)
    phase          REAL,                       -- Фаза в градусах [0..360)
    rpm            REAL NOT NULL,              -- Текущие обороты вала
    
    PRIMARY KEY (timestamp, equipment_id, order_id)
);

-- Индекс для быстрой фильтрации по конкретному агрегату и гармонике
CREATE INDEX idx_equip_order_trends ON order_vibration_trends (equipment_id, order_id, timestamp DESC);

CREATE TABLE equipment_vibration_thresholds (
    equipment_id  INTEGER REFERENCES equipment(id) ON DELETE CASCADE,
     -- `0.5x`, `1.5x`, `2.5x` - Субгармоники. Механическое ослабление (подшипник «болтается» в корпусе), задевание вала, люфт подшипников скольжения.
     -- `1.0x` - Дисбаланс ротора или погнутость вала.
     -- `2.0x` - Несоосность валов (расцентровка муфты), трещина в валу или механические зазоры.
     -- `3.0x` - Механическое ослабление (люфт опор) или расцентровка трехпальцевых/кулачковых муфт.
     -- `BPFI`- Развитый дефект внутреннего кольца (пики с боковыми полосами + / - 1.0x).
     -- `BPFO`- Развитый дефект наружного кольца (четкий пик и много его гармоник).
     -- `FTF` - Развитый дефект сепаратора (серьезный износ, часто модулирует другие частоты).
     -- `BSF` - Развитый дефект тел качения (шариков/роликов, часто с боковыми полосами + / - FTF).
     -- `BPFI-high`- Ранняя стадия зарождающегося дефекта внутреннего кольца (микросколы).
     -- `BPFO-high`- Ранняя стадия зарождающегося дефекта внешнего кольца (микросколы).
     -- `BSF-high` - Ранняя стадия зарождающегося дефекта тел качения (периодические ВЧ-вспышки при входе шарика в зону нагрузки).
    order_id      VARCHAR(10) NOT NULL,
    
    -- Абсолютные пороги по ГОСТу для конкретной гармоники (в RMS)
    level_b REAL NOT NULL,        -- Желтая зона (1.12 – 2.8 мм/с) - Пригодна для длительной эксплуатации без ограничений.
    level_с REAL NOT NULL,        -- Оранжевая зона (2.8 – 7.1 мм/с) - Непригодна для длительной работы. Требуется планирование ремонта.
    level_d REAL NOT NULL,        -- Красная зона (> 7.1 мм/с) - Опасные вибрации. Риск аварии. Немедленный останов.
    
    -- Порог скорости роста (критический прирост в сутки, например, мм/с в день)
    warn_growth_per_day REAL NOT NULL,  -- Оранжевая зона
    alarm_growth_per_day REAL NOT NULL, -- Красная зона
    
    PRIMARY KEY (equipment_id, order_id)
);
