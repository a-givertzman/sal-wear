mod tests {
    use std::{
        sync::{
            Arc, 
            Once
        }, 
        time::Duration
    };
    use debugging::session::debug_session::{
        DebugSession, 
        LogLevel
    };
    use sal_core::dbg::Dbg;
    use testing::stuff::max_test_duration::TestDuration;
    use crate::{
        BendingCyclesNum, Context, Eval, MockInputs, ReadInputs,
    };
    ///
    ///
    static INIT: Once = Once::new();
    ///
    /// once called initialisation
    fn init_once() {
        INIT.call_once(|| {
            // implement your initialisation code to be called only once for current test file
        })
    }
    ///
    /// returns:
    ///  - ...
    fn init_each() -> () {}
    ///
    /// Расчёт допустимого числа циклов (S–N) для изгиба
    /// См. [раздел 8.3, шаг 6](../../Operion_Diag_Вибродиагностика_и_остаточный_ресурс.pdf)
    /// Формула: 
    /// NF = NF0 * (σF_lim / σF)^mF
    /// Где:
    /// * `mF` — показатели степени S–N кривой для изгиба 
    /// * `NF0` — базовое число циклов при напряжении σ_lim для изгиба
    /// * `σF_lim` — предел выносливости по изгибу [Па]
    /// * `σF` — значения напряжения на изгибе
    #[test]
    fn bending_cycles_num() {
        DebugSession::new().filter(LogLevel::Debug).init();
        init_once();
        init_each();
        let parent_dbg = Dbg::own("test_session");
        println!("\n{}", parent_dbg);
        let test_duration = TestDuration::new(&parent_dbg, Duration::from_secs(10));
        test_duration.run().unwrap();
        let test_data = [
            (
                1,
                1.0,
                2.0,
                3.0,
                0.1,
                0.15,
            ),
            (
                2,
                4.0,
                5.0,
                6.0,
                0.1,
                9.6e-7,
            ),
        ];
        for (step, m_f, bending_stresses, nf_0, f_lim, target) in test_data {
            let mut inputs = MockInputs::new();  
            inputs.m_f = Some(m_f);
            inputs.f_lim = Some(f_lim);
            inputs.nf_0 = Some(nf_0);
            let child = ReadInputs::new(
                &parent_dbg, 
                Arc::new(inputs)
            );
            let mut ctx = Context::new_test(0.0);
            ctx.bending_stresses = bending_stresses;
            let result = BendingCyclesNum::new(
                &parent_dbg,
                child
            ).eval(ctx);
            assert!(
                result.err.is_none(), 
                "Шаг [{}]: Ожидался успешный расчет, но получена ошибка: {:?}", 
                step, result.err
            );
            let actual = result.bending_num_cycles;
            let epsilon = 1e-5;
            let diff = (actual - target).abs();
            assert!(
                diff < epsilon,
                "\n Шаг: {}\n Получено: {}\n Ожидалось: {}\n Разница: {}", 
                step, actual, target, diff
            );
        }
        test_duration.exit();
    }
}
