///
/// Basic Tests
#[cfg(test)]
mod tests {
use sal_core::dbg::Dbg;
    use crate::{Decimation, Eval, AngularCtx, tests::{Frequency, Udp}};
    // Простая заглушка конечного дочернего элемента цепочки
    struct DummyChild;
    impl Eval<AngularCtx, AngularCtx> for DummyChild {
        fn eval(&self, ctx: AngularCtx) -> AngularCtx { ctx } // Просто возвращает контекст без изменений
        fn exit(&self) {}
    }
    #[test]
    fn test_decimation_factor_and_length() {
        let dbg = Dbg::own("test_decimation_factor_and_length");
        let conf_adc_sample_rate_hz = 320000.0;
        let decimator = Decimation::new(
            &dbg, 
            20,
            conf_adc_sample_rate_hz,
            DummyChild
        );
        // Без гармоник — чистая "тишина" на DC-уровне, проверяем только количество элементов
        let mut udp = Udp::new(512, 320_000.0, []);
        let mut samples = [0u16; 512];
        udp.parse(0.0, &mut samples); // rpm=0, Frequency::Rpm-гармоник нет, так что rpm не важен
        let mut ctx = AngularCtx::default();
        ctx.samples = samples.to_vec();
        let result_ctx = decimator.eval(ctx);
        // 512 / 20 = 25.6 -> должно получиться 25 или 26 сэмплов
        assert!(
            result_ctx.rpm_detection.decimation.decimated.len() >= 25 && result_ctx.rpm_detection.decimation.decimated.len() <= 26,
            "Неожиданное количество децимированных сэмплов: {}",
            result_ctx.rpm_detection.decimation.decimated.len()
        );
        assert!(!result_ctx.err.is_some());
    }
    #[test]
    fn test_anti_aliasing_filtering() {
        let dbg = Dbg::own("test_anti_aliasing_filtering");
        let conf_adc_sample_rate_hz = 320000.0;
        let decimator = Decimation::new(
            &dbg, 
            20,
            conf_adc_sample_rate_hz, 
            DummyChild
        );
        let fs = 320_000.0;
        // Полезный сигнал 50Гц + сильная ВЧ-помеха 40кГц (выше среза 8кГц), амплитуда шума вдвое больше
        let mut udp = Udp::new(
            512,
            fs,
            [
                (Frequency::Static(50.0), 500),
                (Frequency::Static(40_000.0), 1000),
            ],
        );
        let mut ctx = AngularCtx::default();
        let mut all_decimated: Vec<f64> = Vec::new();
        for _ in 0..2 {
            let mut samples = [0u16; 512];
            udp.parse(0.0, &mut samples); // rpm не важен — обе гармоники Static
            ctx.samples = samples.to_vec();
            ctx = decimator.eval(ctx);
            all_decimated.extend(&ctx.rpm_detection.decimation.decimated);
        }
        // Высокочастотный шум должен быть подавлен. Порог масштабирован под
        // амплитуду полезного сигнала (500), а не под 1.0, как в нормированной версии -1..1.
        for &sample in &all_decimated {
            assert!(
                sample.abs() < 1.5 * 500.0,
                "Фильтр не подавил высокочастотный шум! Значение: {sample}"
            );
        }
        assert!(!ctx.err.is_some());
    }
}