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
        Context, Eval, MockInputs, ReadInputs, WearCoreConf, axial_load::AxialLoad, limiting_speed::LimitingSpeed, motor_torque::MotorTorque,
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
    /// Тест допустимого число оборотов подшипника [Н]
    /// N = L10 * 10e6
    #[test]
    fn limiting_speed() {
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
                Context {
                    motor_rpm: None,
                    motor_p: None,
                    t_bearing: None,
                    duration: 0.0,
                    motor_torque: 50.0,
                    radial_load: 50.0,
                    equivalent_load: 0.0,
                    axial_load: 0.0,
                    basic_rating_life: 110.0,
                    limiting_speed: 0.0,
                    actual_speed: 0.0,
                    bearing_accumulated_wear: 0.0,
                    err: None,
                },
                MockInputs {
                    rpm: Some(1500.0),
                    motor_p: Some(15.0),
                    t_bearing: Some(0.0),
                    duration: Some(0.0),
                },
                Some(110.0 * 10e6),
            ),
            (
                2,
                Context {
                    motor_rpm: None,
                    motor_p: None,
                    t_bearing: None,
                    duration: 0.0,
                    motor_torque: 25.0,
                    radial_load: 20.0,
                    axial_load: 0.0,
                    equivalent_load: 0.0,   
                    basic_rating_life: 1110.0,
                    limiting_speed: 0.0,
                    actual_speed: 0.0,
                    bearing_accumulated_wear: 0.0,
                    err: None,
                },
                MockInputs {
                    rpm: Some(1500.0),
                    motor_p: Some(15.0),
                    t_bearing: Some(0.0),
                    duration: Some(0.0),
                },
                Some(1110.0 * 10e6),
            ),
        ];
        for (step, ctx, inputs, expected_load) in test_data {
            let result = LimitingSpeed::new(
                &parent_dbg,
                ReadInputs::new(
                    &parent_dbg, 
                    Arc::new(inputs)
                ) 
            ).eval(ctx);
            match expected_load {
                Some(target) => {
                    assert!(
                        result.err.is_none(), 
                        "Шаг [{}]: Ожидался успешный расчет, но получена ошибка: {:?}", 
                        step, result.err
                    );
                    let actual = result.limiting_speed;
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
                        step, result.limiting_speed
                    );
                    log::debug!("Шаг [{}]: Ошибка успешно перехвачена: {:?}", step, result.err);
                }
            }
        }
        test_duration.exit();
    }
}
