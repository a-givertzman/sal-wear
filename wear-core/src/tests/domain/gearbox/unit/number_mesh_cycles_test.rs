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
        Context, 
        Eval, 
        GearMeshFrequency, 
        MockInputs, 
        NumberOfMeshCycles, 
        ReadInputs, 
        RotationalFrequency,
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
    /// Расчёт числа циклов зацепления
    /// См. [раздел 8.3, шаг 3](../../Operion_Diag_Вибродиагностика_и_остаточный_ресурс.pdf)
    /// Формула: 
    /// n_mesh = f_GMF * duration [Гц]
    /// Где:
    /// * `f_GMF` — частота зацепления [Гц]
    /// * `duration` — длительность данного устойчивого режима [с] (обязательно больше нуля)
    #[test]
    fn number_mesh_cycles() {
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
                2,
                3.0,
                0.1,
            ),
            (
                2,
                4.0,
                5,
                6.0,
                2.0,
            ),
        ];
        for (step, rpm, z_p, duration, target) in test_data {
            let mut inputs = MockInputs::new();  
            inputs.z_p = Some(z_p);
            inputs.rpm = Some(rpm);
            let child = ReadInputs::new(
                &parent_dbg, 
                Arc::new(inputs)
            );
            let ctx = Context::new_test(duration);
            let result = NumberOfMeshCycles::new(
                &parent_dbg,
                GearMeshFrequency::new(
                    &parent_dbg,
                    RotationalFrequency::new(
                        &parent_dbg, 
                        child
                        )
                    ) 
            ).eval(ctx);
            assert!(
                result.err.is_none(), 
                "Шаг [{}]: Ожидался успешный расчет, но получена ошибка: {:?}", 
                step, result.err
            );
            let actual = result.number_mesh_cycles;
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
