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
        Context, Eval, MockInputs, ReadInputs, WearCoreConf, BearingAccumulatedWear, AxialLoad, EquivalentLoad, MotorTorque,
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
    /// Тест накопленного повреждения подшипника 
    /// D_bearing = n / N
    #[test]
    fn bearing_accumulated_wear() {
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
                    limiting_speed: 100.0,
                    actual_speed: 50.0,
                    bearing_accumulated_wear: 0.0,
                    temp_coeff: 0.0,
                    bearing_temp_accumulated_wear: 0.0,
                    err: None,
                },
                MockInputs {
                    rpm: Some(1500.0),
                    motor_p: Some(15.0),
                    t_bearing: Some(0.0),
                },
                Some(50.0 / 100.0),
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
                    limiting_speed: 50.0,
                    actual_speed: 0.5,
                    bearing_accumulated_wear: 0.0,
                    temp_coeff: 0.0,
                    bearing_temp_accumulated_wear: 0.0,
                    err: None,
                },
                MockInputs {
                    rpm: Some(1500.0),
                    motor_p: Some(15.0),
                    t_bearing: Some(0.0),
                },
                Some(0.5 / 50.0),
            ),
        ];
        for (step, ctx, inputs, expected_load) in test_data {
            let result = BearingAccumulatedWear::new(
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
                    let actual = result.bearing_accumulated_wear;
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
                        step, result.bearing_accumulated_wear
                    );
                    log::debug!("Шаг [{}]: Ошибка успешно перехвачена: {:?}", step, result.err);
                }
            }
        }
        test_duration.exit();
    }
}
