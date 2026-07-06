- Наименование либы: `VibroCore`
- Давай подробно разложим вычислительные шаги, пока без деталей имплементации, но оценим связи и движение данных.
- Сырой сигнал с АЦП уже есть можно брать готовый массив в обработку.
- Сырой сигнал с тахометра тоже есть, получаем с канала, добавлю туда структуру `Inputs`, она читает канал, все значения пишет в поле, в любой момент мы можем получить актуальное значение частоты

- `TimeDomainSamples(Сырые данные)` - высокоэффективный lock-free circular buffer с прямым доступом к элементам

- `TimeDomainSamples` + `Input.rpm`
    - `Autocorrelation(TimeDomainSamples, Inputs.rpm)` Атокорреляция
        - rpm 3000 об/мин $\rightarrow$ $\sim 50$ Гц $\rightarrow$ период $\sim 20$ мс).
        - Ищет пик в узком окне около 20 мс. Находит точное запаздывание $\tau_{peak}$
        - Считаем мгновенную частоту $\omega = 2\pi \cdot \frac{F_s}{\tau_{peak}}$.
        - Пропускаем $\omega$ через медианный фильтр для защиты от выбросов.
        - Интегрируем $\omega$ по времени, чтобы получить массив $\theta[n]$ (текущий угол вала для каждого сэмпла из АЦП).
    - `AngularGrid(Conf.AngularStep, Autocorrelation)` - Сетка угов для текущей выборки
        - Conf.AngularStep = $\Delta\theta = 2\pi / 256$
    - `TimeDomainSamples`
        - `LowPassSignal` - Фильтр низких частот (0.5X..10X)
            - `OrderDomainSamples` - Order Tracking, Resample
            - `OrderSpectrum` - FFT, Копит буфер заданного размера, считает по готовности
            - `ImbalanceDetector` - Детектор изменения энергии в зоне 1x, 2x, 3x
                - `Threshold` для 1x - Дисбаланс, 2/3x - расцентровка, ослабление опор.
                - Наличие изменений отправятся в БД
    - `TimeDomainSamples`
        - `BandpassSignal` - Полосовой Фильтр ВЧ-резонанса (5..10 кГц)
            - `SignalEnvelope` - Детектор огибающей (Full-Wave Rectification + Low Pass Filter)
            - `OrderDomainSamples` - Order Tracking
            - `OrderSpectrum` - FFT, Копит буфер заданного размера, считает по готовности
            - `DefectDetector` - Детектор дефектов BPFI, BPFO, FTF, BSF подшипника
                - `Threshold` для BPFI, BPFO, FTF, BSF на ранней стадии
                - Наличие изменений отправятся в БД
    - `TimeDomainSamples`
        - `BandpassSignal` - Полосовой Фильтр Среднего диапазона (10X..5 кГц)
            - `OrderDomainSamples` - Order Tracking, Resample
            - `OrderSpectrum` - FFT, Копит буфер заданного размера, считает по готовности
            - `DefectDetector` - Детектор дефектов BPFI, BPFO, FTF, BSF подшипника
                - `Threshold` для BPFI, BPFO, FTF, BSF на стадии развитого дефекта
                - Наличие изменений отправятся в БД


Псевдокод что бы проследить связи
Обрати внимание что каждый частотный диапазон - это отдельный самостоятельный объект.
Между диапазонами нет ни каких пересечений, они лишь совместно читают angular_grid и samples.
Ни каких мутаций совместной памяти.
```rust
// берем шедулер из ThreadPool приложения (ThreadPool не дробит и не перекидывает задачи, всегда выполняет job в одном месте)
let scheduler = tp.scheduler();
// подписываемся на канал "Motor.Rpm" у диспетчера приложения
let inputs = Inputs::new(services.subscribe("Motor.Rpm"));
// Раздает lock-free доступ на чтение углов
let angular_grid = Arc::new(AngularGrid::New(
    conf.angular_step,
    Autocorrelation::new(inputs)
));
let imbalance = SqlExport::new(             // Wraps results into sql and export
    api_client_link.clone(),
    ImbalanceDetector::new(
        conf.imbalance.threshold,           // for BPFI, BPFO, FTF, BSF
        OrderSpectrum::new(
            conf.imbalance.fft_size,
            OrderDomainSamples::new(
                angular_grid.clone(),
                LowPassSignal::new(         // Баттерворт 2-го порядка
                    conf.imbalance.edge,    // 10x
                )
            )
        )
    )
);
let high_range = SqlExport::new(            // Wraps results into sql and export
    api_client_link.clone(),
    DefectDetector::new(
        conf.high.threshold,                // for BPFI, BPFO, FTF, BSF
        OrderSpectrum::new(
            conf.high.fft_size,
            OrderDomainSamples::new(
                angular_grid.clone(),
                SignalEnvelope::new(
                    BandpassSignal::new(
                        conf.high.range     // 5..10 кГц
                    )
                )
            )
        )
    )
);
let midle_range = SqlExport::new(           // Wraps results into sql and export
    api_client_link.clone(),
    DefectDetector::new(
        conf.midle.threshold,               // for BPFI, BPFO, FTF, BSF
        OrderSpectrum::new(
            conf.midle.fft_size,
            OrderDomainSamples::new(
                angular_grid.clone(),
                BandpassSignal::new(
                    conf.midle.range        // 10x..5 кГц
                )
            )
        )
    )
);
let mut samples = TimeDomainSamples::new(Vec::with_capacity(512));
loop {
    if Ok(_) = self.parse(&mut samples) {
        angular_grid.eval(&samples);
        scheduler.spawn({
            let samples = samples.clone()
            move || {
            imbalsnce.eval(samples);
        }})
        scheduler.spawn({
            let samples = samples.clone()
            move || {
            high_range.eval(samples);
        }})
        scheduler.spawn({
            let samples = samples.clone()
            move || {
            midle_range.eval(samples);
        }})
    }
}
```