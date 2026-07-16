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
        MockInputs, 
        ReadInputs, 
        MotorTorque
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
    /// Тест расчёта крутящего момента на валу редуктора [Н·м]
    /// M_motor = 9550 * (P_motor/rpm)
    #[test]
    fn motor_torque() {
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
                    t_bearing: Some(0.0),
                },
                Some(9550.0), // 9550 * 50 / 50 = 9550
            ),
            (
                2,
                MockInputs {
                    rpm: Some(25.0),
                    motor_p: Some(50.0),
                    t_bearing: Some(0.0),
                },
                Some(19100.0), // 9550 * 50 / 25 = 19100
            ),
            (
                3,
                MockInputs {
                    rpm: Some(1450.0),
                    motor_p: Some(11.5),
                    t_bearing: Some(0.0),
                },
                Some(9550.0 * 11.5 / 1450.0), // ~75.74137
            ),
            (
                4,
                MockInputs {
                    rpm: Some(1000.0),
                    motor_p: Some(0.0),
                    t_bearing: Some(0.0),
                },
                None, // Ошибка тк motor_p должен быть > 0
            ),
            (
                5,
                MockInputs {
                    rpm: None,
                    motor_p: Some(15.0),
                    t_bearing: Some(0.0),
                },
                None, // Ожидаем, что в контексте вернется ошибка, а не расчет
            ),
            (
                6,
                MockInputs {
                    rpm: Some(1500.0),
                    motor_p: None,
                    t_bearing: None,
                },
                None, // Ожидаем ошибку
            ),
        ];
        for (step, inputs, expected_torque) in test_data {
            let result = MotorTorque::new(
                &parent_dbg,
                ReadInputs::new(
                    &parent_dbg, 
                    Arc::new(inputs)
                ) 
            ).eval(Context::new());
            match expected_torque {
                Some(target) => {
                    assert!(
                        result.err.is_none(), 
                        "Шаг [{}]: Ожидался успешный расчет, но получена ошибка: {:?}", 
                        step, result.err
                    );
                    let actual = result.motor_torque;
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
                        step, result.motor_torque
                    );
                    log::debug!("Шаг [{}]: Ошибка успешно перехвачена: {:?}", step, result.err);
                }
            }
        }
        test_duration.exit();
    }
}
