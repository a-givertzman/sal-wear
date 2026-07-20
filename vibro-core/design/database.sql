-- Дефекты
CREATE TABLE order_vibration_trends (
    timestamp   TIMESTAMPTZ NOT NULL,
    order_id    VARCHAR(10) NOT NULL, -- '1x', '2x', '3x', '0.5x' и т.д.
    rms_value   REAL NOT NULL,        -- Амплитуда (RMS) вибрации (например, мм/с)
    phase       REAL,                 -- Фаза гармоники в градусах [0..360)
    rpm         REAL NOT NULL,        -- Текущие обороты вала при замере
    
    PRIMARY KEY (timestamp, order_id)
);

-- Индекс для быстрой выборки тренда по конкретному дефекту
CREATE INDEX idx_order_trends ON order_vibration_trends (order_id, timestamp DESC);


-- SQL-запрос, который считает скорость изменения амплитуды (в единицу времени) между соседними записями:
WITH trend_with_lag AS (
    SELECT 
        timestamp,
        order_id,
        rms_value,
        -- Получаем значения предыдущей точки для этого же order_id
        LAG(rms_value) OVER (PARTITION BY order_id ORDER BY timestamp) as prev_rms,
        LAG(timestamp) OVER (PARTITION BY order_id ORDER BY timestamp) as prev_timestamp
    FROM order_vibration_trends
    WHERE order_id = '1x' -- Смотрим скорость роста конкретно для 1x
)
SELECT 
    timestamp,
    rms_value,
    -- Скорость роста: (Текущее_RMS - Предыдущее_RMS) делить на (Разницу во времени в часах)
    CASE 
        WHEN prev_timestamp IS NOT NULL AND timestamp > prev_timestamp 
        THEN (rms_value - prev_rms) / (EXTRACT(EPOCH FROM (timestamp - prev_timestamp)) / 3600.0)
        ELSE 0 
    END as rms_growth_rate_per_hour
FROM trend_with_lag
ORDER BY timestamp ASC;
