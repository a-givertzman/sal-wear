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
        Context, Eval, GearMeshFrequency, MockInputs, MotorTorque, NumberOfMeshCycles, ReadInputs, RotationalFrequency, TangentialForce,
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
    /// Расчёт окружной силы
    /// См. [раздел 8.3, шаг 4](../../Operion_Diag_Вибродиагностика_и_остаточный_ресурс.pdf)
    /// Формула: 
    /// Ft = 2 * M * d_p [Н]
    /// Где:
    /// * `M` — крутящий момент на валу редуктора [Н·м]
    /// * `d_p` — делительный диаметр шестерни [м]
    #[test]
    fn tangential_force() {
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
                0.1,
                1650.0,
                200.0,
                23151.51515151515,
            ),
            (
                2,
                4.0,
                2000.0,
                300.0,
                716.25,
            ),
        ];
        for (step, d_p, rpm, motor_p, target) in test_data {
            let mut inputs = MockInputs::new();  
            inputs.d_p = Some(d_p);
            inputs.rpm = Some(rpm);
            inputs.motor_p = Some(motor_p);
            let child = ReadInputs::new(
                &parent_dbg, 
                Arc::new(inputs)
            );
            let ctx = Context::new_test(100.0);
            let result = TangentialForce::new(
                &parent_dbg,
                MotorTorque::new(
                    &parent_dbg, 
                    child
                    )
            ).eval(ctx);
            assert!(
                result.err.is_none(), 
                "Шаг [{}]: Ожидался успешный расчет, но получена ошибка: {:?}", 
                step, result.err
            );
            let actual = result.tangential_force;
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
