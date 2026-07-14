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
        Context, Eval, MockInputs, ReadInputs, TempCoeff, actual_speed::ActualSpeed, motor_torque::MotorTorque
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
    /// Тест температурного коэффициента ускорения износа (Коэффициент Вант-Гоффа)
    /// KT = Q10^((t_bearing-t_ref)/10.0)
    #[test]
    fn temp_coeff() {
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
                MockInputs {
                    rpm: Some(50.0),
                    motor_p: Some(50.0),
                    t_bearing: Some(50.0),
                },
                50.0,
                100.0,
                Some(50.0_f64.powf((50.0-100.0) / 10.0)),
            ),
            (
                1,
                MockInputs {
                    rpm: Some(50.0),
                    motor_p: Some(50.0),
                    t_bearing: None,
                },
                50.0,
                100.0,
                Some(1.0),
            ),
        ];
        for (step, inputs, q_10, t_ref, expected_coeff) in test_data {
            let result = TempCoeff::new(
                q_10,
                t_ref,
                &parent_dbg,
                ReadInputs::new(
                    &parent_dbg, 
                    Arc::new(inputs)
                ) 
            ).eval(Context::new());
            match expected_coeff {
                Some(target) => {
                    assert!(
                        result.err.is_none(), 
                        "Шаг [{}]: Ожидался успешный расчет, но получена ошибка: {:?}", 
                        step, result.err
                    );
                    let actual = result.temp_coeff;
                    let epsilon = 1e-5;
                    let diff = (actual - target).abs();
                    assert!(
                        diff < epsilon,
                        "\n Шаг: {}\n Получено: {}\n Ожидалось: {}\n Разница: {}", 
                        step, actual, target, diff
                    );
                },
                None => {
                    assert!(
                        result.err.is_some(), 
                        "Шаг [{}]: Ожидалась ошибка из-за отсутствия данных, но расчет прошел. Результат: {:?}", 
                        step, result.temp_coeff
                    );
                    log::debug!("Шаг [{}]: Ошибка успешно перехвачена: {:?}", step, result.err);
                }
            }
        }
        test_duration.exit();
    }
}
