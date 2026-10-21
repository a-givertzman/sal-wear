# Проверка соответствия параметров цепочек вычислений

Сначала важное замечание: в репозитории **уже есть файл `design/consistency.md`** с такой таблицей. Я проверил его утверждения по коду — он в целом верен. Ниже — уточнённая и дополненная версия (с найденными расхождениями и новыми замечаниями). Код я не менял, это только рекомендации.

## Легенда

- **✅** — имена согласованы.
- **⚠️** — расхождение имён либо параметр отсутствует в конфиге.
- **❌** — баг / серьёзное расхождение.

---

## Цепочка 1: `ReadEventValues → Autocorrelation → AngularGrid` (`AngularCtx`)

| Параметр (смысл) | В конфиге (YAML / геттер) | В контексте (`AngularCtx`) | В конструкторе (аргумент) | Замечания | Оценка |
|---|---|---|---|---|---|
| Частота дискретизации АЦП | `adc.sample-rate-hz` → `sample_rate_hz` | `dt`, косвенно `omega` | `Autocorrelation::new(sample_rate_hz)`, `AngularCtx::new(f_sample, …)` | Три имени на одну сущность: `sample_rate_hz` / `sample_rate` (поле) / `f_sample` (аргумент) | ⚠️ |
| Размер пакета АЦП | `adc.chunk-size` → `chunk_size` | `phases_size`, `samples` (capacity) | `AngularGrid::new(chunk_size)`, `AngularCtx::new(_, chunk_size)` | Согласовано | ✅ |
| Постоянная составляющая | `adc.dc-offset` → `dc_offset` | — | `Frame::raw(ts, dc_offset, …)` | В doc-комментариях `ds_offset`, в тестах ключ `ds-offset` — опечатка/рассинхрон | ⚠️ |
| Ключ события RPM | **нет в конфиге** | — | `ReadEventValues::new(parent, keys, values)` | Ключ `"rpm"` приходит извне; в тестах захардкожен | ⚠️ |
| Грубый RPM (тахометр) | — (эвент) | `raw_rpm` | — | Пишется `ReadEventValues` | ✅ |
| Грубый период (в сэмплах) | — | `raw_period` | — | Производное `Fs·60/raw_rpm` | ✅ |
| Точный период (в сэмплах) | — | `period` | — | Производное АКФ | ✅ |
| Угловая скорость | — | `omega` | — | `TAU·Fs/period` | ✅ |
| Текущий угол вала | — | `current_theta` | — | Сброс кратно `TAU` в `AngularGrid` | ✅ |
| Ёмкость окна АКФ | **нет в конфиге** | `ac_samples` (capacity) | хардкод `3.0·Fs·60/200` | Магия: `K=3` оборота, `RPMmin=200`. В комментарии рядом фигурирует `/300` и `/600` — код и комментарий расходятся | ⚠️ |
| Шаг окна АКФ (hop) | **нет в конфиге** | `ac_samples` (hop) | хардкод `capacity/8` | Магическая константа | ⚠️ |
| Полуширина поиска пика АКФ | **нет в конфиге** | — | хардкод `raw_period·0.1` | ±10 %, магическая константа | ⚠️ |
| Устаревшее поле | — | `ac_samples` | — | Помечено `#[deprecated]`, подлежит удалению вместе с `Autocorrelation` | ⚠️ |

---

## Цепочка 2: `LowPassSignal → OrderDomainSamples → OrderSpectrum → OrderFeatureFilter → ImbalanceDetector` (`ImbContext`)

| Параметр (смысл) | В конфиге (YAML / геттер) | В контексте (`ImbContext` / вложенные) | В конструкторе (аргумент) | Замечания | Оценка |
|---|---|---|---|---|---|
| Плотность угловой сетки | `analysis.order-tracking.samples-per-rev` → `samples_per_rev()` | — | `ImbContext::new(samples_per_rev,…)`, `OrderDomainSamples::new(samples_per_rev)`, `OrderZone::new(…, samples_per_rev)` | Имена: YAML `samples-per-rev`, геттер `samples_per_rev()`, комментарий `points-per-turn`, design `AngularStep` | ⚠️ |
| Размер FFT-окна | `analysis.n_fft()` (производное) | `fft_buff`, `fft_window` (capacity) | `ImbContext::new(…, n_fft, …)`, `OrderSpectrum::new(n_fft)`, `OrderFeatureFilter::new(n_fft)`, `OrderZone::new(…, n_fft, …)` | Design использует `fft_size`; код — `n_fft`. Одна сущность — два имени | ⚠️ |
| Оборотов в окне FFT | `analysis.fft_turns()` (производное) | — | — (только внутри `n_fft`) | Тест `order_domain_phase_shift_control_test` **ошибочно передаёт `fft_turns()` вместо `n_fft`** в `ImbContext::new` | ❌ |
| Максимальный порядок | `analysis.order-tracking.max-order` → `max_order` | — | — (только в авторасчёте `samples_per_rev`) | Doc говорит «в 2.56 раза», код считает `× 2.5` | ⚠️ |
| Разрешение по порядку | `analysis.order-tracking.order-resolution` → `order_resolution` | — | — (только в авторасчёте `fft_turns`) | Согласовано | ✅ |
| Шаг угловой сетки | `analysis.angular_step_rad()` | — | `OrderFeatureFilter::new(…, angular_step_rad, …)` | Внутри хранится как `phase_offset` | ✅ |
| Верхняя граница НЧ-диапазона (срез LPF) | `analysis.bands.low-order` → `low_cutoff_order()` | — | `LowPassSignal::new(…, cutoff_order, …)` | Имена: YAML `low-order`, геттер `low_cutoff_order()`, аргумент `cutoff_order`, design `conf.imbalance.edge`. **Одна зона используется и как полоса детектора, и как срез ФНЧ**; детектор при этом использует захардкоженные 0.5..3.0 | ⚠️ |
| СЧ-диапазон | `analysis.bands.mid-hz` → `mid_range_max_hz()` | — | — | Не используется (`MidRange = Pass`) | ⚠️ |
| ВЧ-диапазон | `analysis.bands.high-hz` → `high_range_hz()` | — | — | Не используется (`HighRange = Pass`) | ⚠️ |
| Оконная функция | **нет в конфиге** | — | хардкод `WindowFn::kaiser(…, 5.65)` в `VibroSensor::new` | Тип окна и параметр `β=5.65` зашиты в код | ⚠️ |
| Частота дискретизации LPF | — | — | `LowPassSignal` использует `conf.adc.sample_rate_hz` | Согласовано: ФНЧ работает по сырым сэмплам АЦП | ✅ |
| Уточнённый RPM | — (эвент) | `rpm` | — | **Никем не заполняется в проде** (см. баги) | ❌ |
| Целевые порядки Калмана | **нет в конфиге** | `filters` | хардкод `[0.5, 1.0, 1.5, 2.0, 2.5, 3.0]` | Дублируется в `DiagnosticDetector::ORDER_INDEX` | ⚠️ |
| Скорость старения Калмана `q` | **нет в конфиге** | — | хардкод `1e-7` | — | ⚠️ |
| Порог записи тренда | **нет в конфиге** | — | хардкод `0.01` | — | ⚠️ |
| Полуширина захвата зоны | **нет в конфиге** | — | хардкод `half_width = 0.05` | Design задаёт ±0.02…±0.1 | ⚠️ |
| Ширина интегрирования (leakage) | **нет в конфиге** | — | хардкод `3` | — | ⚠️ |
| Окно `ShortSigma` | **нет в конфиге** | — | хардкод `10` | — | ⚠️ |
| Веса / пороги / зоны дефектов | **нет в конфиге** | — | хардкод в `DiagnosticDetector::new()` | Все `weights`, `score_threshold`, `thresholds` зашиты | ⚠️ |

---

## Цепочка 3 (разрабатываемая): `Decimation → ComplexLocalOscillator → LPFTrackingBand` (`AngularCtx.rpm_detection`)

| Параметр (смысл) | В конфиге (YAML / геттер) | В контексте | В конструкторе (аргумент) | Замечания | Оценка |
|---|---|---|---|---|---|
| Частота дискретизации АЦП | `adc.sample-rate-hz` | — | `RpmDetectionCtx::new(sample_rate,…)` → `DecimationCtx::new(sample_rate,…)` | Корректно используется только для `f_sample_dec` | ✅ |
| Коэффициент децимации `M` | **нет в конфиге** | `decimation.factor` | хардкод `20` в `AngularCtx::new` | Имена: design `M`, тест `DECIMATION_COEFF`, поле `factor` | ⚠️ |
| Частота после децимации | — (производное) | `decimation.f_sample_dec` | `sample_rate / factor` | Используется гетеродином, но **не** LPF (баг) | ❌ |
| Размер пакета АЦП | `adc.chunk-size` | — | `RpmDetectionCtx::new(_, samples_capacity, _)` | Четыре имени: `chunk_size` / `samples_capacity` / `signal_len` / `signal_capacity` | ⚠️ |
| Тапы FIR-фильтра | **нет в конфиге** | `dec_fir_state` (capacity) | хардкод `fir_size = 32`, константа `FIR_TAPS_M20_N32` | `fir_size` не связан с `FIR_TAPS_M20_N32.len()`; при `fir_size > 32` цикл свёртки выйдет за границы массива | ⚠️ |
| Частота среза ФНЧ-дециматора | **нет в конфиге** | — | зашита в `FIR_TAPS_M20_N32` (8 кГц) | Производное от `Fs` и `M`, но захардкожено; при смене `Fs` коэффициенты станут неверными | ⚠️ |
| Фазовый счётчик децимации | — | `decimation.dec_counter` | — | Состояние | ✅ |
| Начальная фаза гетеродина | — | `complex_local_oscillator.phi_last` | — | Состояние (непрерывность между чанками) | ✅ |
| Накопленная фаза гетеродина | — | `complex_local_oscillator.phi_net` | — | Поле `phi_net`, а в design/математике — `φ_het` (heterodyne). Имя расходится | ⚠️ |
| Комплексный сигнал I/Q | — | `complex_local_oscillator.complex_signal` | — | Буфер | ✅ |
| Полоса ФНЧ слежения `B` | **нет в конфиге** | — | хардкод `F_CUTOFF = 3.5` Гц | Design задаёт формулу для `B`; в тесте `BAND_HZ = 7.0`, а `f_cutoff = BAND_HZ/2`. Смешение понятий «полоса» и «частота среза» | ⚠️ |
| Добротности биквадов | **нет в конфиге** | `lpf_tracking_band.stage1/2` | хардкод `Q_FACTOR_1 = 0.5412`, `Q_FACTOR_2 = 1.3066` | Согласовано | ✅ |
| Частота дискретизации LPF | — | — | `LPFTrackingBandCtx::new(f_sample, …)` | **Получает `sample_rate` (320 кГц) вместо `f_sample_dec` (16 кГц) — баг** | ❌ |
| Огибающая 1X | — | `lpf_tracking_band.filtered_signal` | — | Буфер | ✅ |
| Флаг валидности, порог амплитуды, `T_max` устаревания | **нет в конфиге** | — | — | Шаги 3.4–3.5 design не реализованы | ❌ |

---

## Сводка несоответствий имён (одна сущность — разные имена)

| Сущность | Имя 1 (конфиг) | Имя 2 (геттер/поле) | Имя 3 (конструктор/тест/design) | Оценка |
|---|---|---|---|---|
| Частота дискретизации | `sample-rate-hz` | `sample_rate` | `f_sample`, `sample_rate_hz`, `f_sample_dec` | ⚠️ |
| Размер пакета | `chunk-size` | `chunk_size` | `samples_capacity`, `signal_capacity`, `signal_len` | ⚠️ |
| Плотность угловой сетки | `samples-per-rev` | `samples_per_rev()` | `points-per-turn`, `AngularStep` | ⚠️ |
| Размер FFT | `n_fft()` | `n_fft` | `fft_size` | ⚠️ |
| Граница НЧ | `low-order` | `low_cutoff_order()` | `cutoff_order`, `conf.imbalance.edge` | ⚠️ |
| Постоянная составляющая | `dc-offset` | `dc_offset` | `ds_offset`, `ds-offset` | ⚠️ |
| Коэффициент децимации | — | `factor` | `M`, `DECIMATION_COEFF` | ⚠️ |
| Фаза гетеродина | — | `phi_net` | `φ_het` | ⚠️ |
| Текущий угол вала | — | `current_theta` (Angular) | `total_phase` (Imb) | ⚠️ |
| Уточнённый RPM | — | `rpm` (Imb) | `raw_rpm` (Angular) | ❌ |

---

## Критические баги (❌) — пояснения

1. **`ImbContext.rpm` не заполняется в проде.**
   В `VibroSensor::eval` вызывается только `low_range_ctx.update(frame)`, которое не трогает `rpm`. Поле остаётся `Rpm(f64::EPSILON)` из конструктора. Как следствие, `LowPassSignal::eval` уходит в ветку `ctx.rpm.value() <= 0.1` и возвращает ошибку `"Low RPM"` — вся НЧ-цепочка в бою останавливается на первом шаге. В тестах значение выставляется вручную (`i_ctx.rpm = Rpm(rpm)`), поэтому баг не виден.

2. **`LPFTrackingBandCtx` инициализируется неверной частотой дискретизации.**
   В `RpmDetectionCtx::new` передаётся `LPFTrackingBandCtx::new(sample_rate, samples_capacity)` — это 320 кГц, тогда как фильтр работает по прореженному сигналу на 16 кГц. Коэффициенты биквадов (частота среза 3.5 Гц) посчитаются для неверной `Fs`, полоса фильтра «уедет» в 20 раз. Тест `lpf_tracking_band_test` подтверждает намерение: там используется `FS = F / 20`.

3. **Ошибка в тесте `order_domain_phase_shift_control_test`.**
   Вызов `ImbContext::new(&dbg, conf.analysis.samples_per_rev(), conf.analysis.fft_turns(), chunk_size, retain)` передаёт в позицию `n_fft` значение `fft_turns()` (число оборотов, ≈64) вместо `n_fft` (≈8192). Тест работает не с той геометрией буфера.

4. **Цепочка 3 не подключена к конвейеру.**
   Узлы `Decimation`, `ComplexLocalOscillator`, `LPFTrackingBand` не инстанцируются нигде, кроме тестов; `RpmDetectionCtx` создаётся в `AngularCtx::new`, но его `eval` не вызывается из `VibroSensor`. Это ожидаемо для разрабатываемой цепочки, но важно зафиксировать.

5. **`ComplexLocalOscillator` игнорирует требования design о возрасте сетевого RPM.**
   Design (шаг 3.2, п. 3) требует применять `f_hat` с учётом штампа времени и правила устаревания. В коде используется `ctx.raw_rpm` «как есть», без времени. Пока не критично, но при подключении приведёт к деградации, не описанной в матрице отказов.

---

## Рекомендации по производительности (отдельно)

1. **`Decimation::eval`: лишнее копирование чанка.**
   `let samples = ctx.samples().to_vec();` копирует весь чанк (512×f64) на каждом вызове. В коде уже стоит `TODO: Постараться избежать копирования!`. Итерация по срезу `ctx.samples()` (с разделением borrow) убирает аллокацию и копирование — самый весомый выигрыш в этой цепочке.

2. **`VecDeque` в горячем цикле КИХ-фильтра.**
   `dec_fir_state` — это `VecDeque<f64>`, обход которого в свёртке менее дружелюбен к кэшу, чем по срезу. Кольцевой буфер на фиксированном массиве (`[f64; 32]` + индекс) даст более предсказуемый доступ и лучше векторизуется.

3. **Свёртка считается только раз на `M` сэмплов — это уже хорошо.**
   Проверка `dec_counter >= factor` стоит до вычисления свёртки, так что стоимость КИХ = `N/M` умножений. Дополнительный резерв — полифазная декомпозиция, но это уже переписывание, а не правка.

4. **Повторные обращения по длинному пути `ctx.rpm_detection.decimation.…` в цикле.**
   После включения `--release` компилятор обычно справляется, но вынос ссылок (`let dec = &mut ctx.rpm_detection.decimation;`) в начало цикла убирает лишние проверки и делает код читаемее.

5. **Влияние на результат.** Пункт 1 уменьшает аллокации и копирование (память/GC-давление), пункт 2 — число промахов кэша. Пункты не меняют математику, только время исполнения. Точный эффект стоит измерять бенчмарком на пакете 512 сэмплов.

---

## Косметика (в самом конце)

- Doc-комментарий `AnalysisConf::samples_per_rev()` говорит «в 2.56 раза», код использует `× 2.5` — привести к одному значению.
- Комментарий в `AngularCtx::new` приводит расчёты с `/300` и `/600`, а код делит на `200` — обновить комментарий.
- `frame.rs`: в doc `ds_offset`, в коде `dc_offset`, в тестах ключ `ds-offset` — унифицировать.
- `ComplexLocalOscillatorCtx::new(signal_len)` и doc `samples_capacity` — согласовать имя аргумента.
- `LPFTrackingBandCtx`: смешение понятий «полоса `B`» и «частота среза `F_CUTOFF`» (в тесте `BAND_HZ = 7.0`, `f_cutoff = BAND_HZ/2`) — уточнить терминологию в комментариях.
- `phi_net` → было бы логичнее `phi_het` по смыслу (heterodyne), как в design.
- В `Design`-псевдокоде (`general.md`) фигурируют имена `conf.imbalance.edge`, `conf.high.range`, `conf.midle.fft_size` — они не соответствуют текущему `Conf`. Стоит синхронизировать документ с кодом.
