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
        Context, Eval, MockInputs, ReadInputs, TempToothDamage,
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
    /// Расчёт повреждения зуба с учетом температуры
    /// См. [раздел 8.3, шаг 8](../../Operion_Diag_Вибродиагностика_и_остаточный_ресурс.pdf)
    /// Формула: 
    /// D_gear_T = D_gear * KT
    /// Где:
    /// * `D_gear` — повреждения зуба
    /// * `KT` — температурный множитель
    #[test]
    fn temp_tooth_damage() {
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
                2.0,
            ),
            (
                2,
                4.0,
                5.0,
                20.0,
            ),
        ];
        for (step, temp_coeff, tooth_damage, target) in test_data {
            let inputs = MockInputs::new();  
            let child = ReadInputs::new(
                &parent_dbg, 
                Arc::new(inputs)
            );
            let mut ctx = Context::new_test(0.0);
            ctx.temp_coeff = temp_coeff;
            ctx.tooth_damage = tooth_damage;
            let result = TempToothDamage::new(
                &parent_dbg,
                child
            ).eval(ctx);
            assert!(
                result.err.is_none(), 
                "Шаг [{}]: Ожидался успешный расчет, но получена ошибка: {:?}", 
                step, result.err
            );
            let actual = result.temp_tooth_damage;
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
