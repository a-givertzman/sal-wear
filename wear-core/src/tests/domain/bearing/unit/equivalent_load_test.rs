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
        Context, Eval, MockInputs, ReadInputs, WearCoreConf, AxialLoad, EquivalentLoad, MotorTorque,
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
    /// Тест эквивалентной нагрузки на подшипник [Н]
    /// P = X * Fr + Y * Fa
    #[test]
    fn equiavalent_load() {
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
                    axial_load: 50.0,
                    equivalent_load: 0.0,
                    basic_rating_life: 0.0,
                    limiting_speed: 0.0,
                    actual_speed: 0.0,
                    bearing_accumulated_wear: 0.0,
                    temp_coeff: 0.0,
                    bearing_temp_accumulated_wear: 0.0,
                    err: None,
                },
                0.5,
                0.5,
                MockInputs {
                    rpm: Some(1500.0),
                    motor_p: Some(15.0),
                    t_bearing: Some(0.0),
                },
                Some(0.5 * 50.0 + 0.5 * 50.0),
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
                    axial_load: 10.0,
                    equivalent_load: 0.0,
                    basic_rating_life: 0.0,
                    limiting_speed: 0.0,
                    actual_speed: 0.0,
                    bearing_accumulated_wear: 0.0,
                    temp_coeff: 0.0,
                    bearing_temp_accumulated_wear: 0.0,
                    err: None,
                },
                0.5,
                0.5,
                MockInputs {
                    rpm: Some(1500.0),
                    motor_p: Some(15.0),
                    t_bearing: Some(0.0),
                },
                Some(0.5 * 20.0 + 0.5 * 10.0),
            ),
            (
                3,
                Context {
                    motor_rpm: None,
                    motor_p: None,
                    t_bearing: None,
                    duration: 0.0,
                    motor_torque: 2.5,
                    radial_load: 0.5,
                    axial_load: 0.0,
                    equivalent_load: 0.0,
                    basic_rating_life: 0.0,
                    limiting_speed: 0.0,
                    actual_speed: 0.0,
                    bearing_accumulated_wear: 0.0,
                    temp_coeff: 0.0,
                    bearing_temp_accumulated_wear: 0.0,
                    err: None,
                },
                0.5,
                0.5,
                MockInputs {
                    rpm: Some(3000.0),
                    motor_p: Some(30.0),
                    t_bearing: Some(0.0),
                },
                Some(0.5 * 0.5 + 0.5 * 0.0),
            ),
        ];
        for (step, ctx, X, Y, inputs, expected_load) in test_data {
            let result = EquivalentLoad::new(
                X,
                Y,
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
                    let actual = result.equivalent_load;
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
                        step, result.equivalent_load
                    );
                    log::debug!("Шаг [{}]: Ошибка успешно перехвачена: {:?}", step, result.err);
                }
            }
        }
        test_duration.exit();
    }
}
