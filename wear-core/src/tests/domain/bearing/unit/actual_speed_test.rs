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
        Context, Eval, MockInputs, ReadInputs, ActualSpeed, MotorTorque
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
    /// Тест фактического числа оборотов подшипника [об]
    /// n = rpm * (duration / 60)
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
                0.0,
                None, // duration !>0
            ),
            (
                2,
                MockInputs {
                    rpm: Some(25.0),
                    motor_p: Some(50.0),
                    t_bearing: Some(0.0),
                },
                60.0,
                Some(25.0 * (60.0 / 60.0)),
            ),
            (
                3,
                MockInputs {
                    rpm: Some(0.0),
                    motor_p: Some(11.5),
                    t_bearing: Some(0.0),
                },
                60.0,
                None, // rpm !> 0
            ),
            (
                4,
                MockInputs {
                    rpm: None,
                    motor_p: Some(0.0),
                    t_bearing: Some(0.0),
                },
                60.0,
                None, // rpm is none
            ),
            (
                6,
                MockInputs {
                    rpm: Some(1500.0),
                    motor_p: Some(50.0),
                    t_bearing: Some(0.0),
                },
                500.0,
                Some(1500.0 * (500.0 / 60.0)),
            ),
        ];
        for (step, inputs, duration, expected_torque) in test_data {
            let result = ActualSpeed::new(
                &parent_dbg,
                ReadInputs::new(
                    &parent_dbg, 
                    Arc::new(inputs)
                ) 
            ).eval(Context::new_test(duration));
            match expected_torque {
                Some(target) => {
                    assert!(
                        result.err.is_none(), 
                        "Шаг [{}]: Ожидался успешный расчет, но получена ошибка: {:?}", 
                        step, result.err
                    );
                    let actual = result.actual_speed;
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
                        step, result.actual_speed
                    );
                    log::debug!("Шаг [{}]: Ошибка успешно перехвачена: {:?}", step, result.err);
                }
            }
        }
        test_duration.exit();
    }
}
