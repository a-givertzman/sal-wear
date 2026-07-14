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
        Context, Eval, MockInputs, ReadInputs, WearCoreConf, accumulated_wear::BearingAccumulatedWear, actual_speed::ActualSpeed, axial_load::AxialLoad, basic_rating_life::BasicRatingLife, equivalent_load::EquivalentLoad, limiting_speed::LimitingSpeed, motor_torque::MotorTorque, radial_load::RadialLoad,
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
    /// Комплексный тест накопленного повреждения подшипника 
    /// D_bearing = n / N
    #[test]
    fn accumulated_wear_complex() {
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
                    duration: 60.0,
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
                    motor_p: Some(5550.0),
                    t_bearing: Some(0.0),
                },
                0.2,
                0.038, // диаметр вала
                0.56, // X
                1.2, // Y
                22000.0, // Сr
                10.0/3.0,
                None, // диаметр вала около нуля
            ),
            (
                2,
                Context {
                    motor_rpm: None,
                    motor_p: None,
                    t_bearing: None,
                    duration: 3600.0,
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
                    rpm: Some(3000.0),
                    motor_p: Some(45000.0),
                    t_bearing: Some(0.0),
                },
                0.2,
                0.65, 
                1.0,
                0.0,
                110000.0,
                10.0/3.0,
                Some(18.39),
            ),
        ];
        for (step, ctx, inputs, fa_to_fr, motor_d, X, Y, Cr, p, expected_load) in test_data {
            let result = BearingAccumulatedWear::new(
                &parent_dbg,
                ActualSpeed::new(
                    &parent_dbg,
                    LimitingSpeed::new(
                        &parent_dbg,
                        BasicRatingLife::new(
                            Cr, 
                            p,
                            &parent_dbg, 
                            EquivalentLoad::new(
                                X, 
                                Y, 
                                &parent_dbg, 
                                AxialLoad::new(
                                    fa_to_fr,
                                    &parent_dbg,
                                    RadialLoad::new(
                                        motor_d, 
                                        &parent_dbg,
                                        MotorTorque::new(
                                            &parent_dbg, 
                                            ReadInputs::new(
                                                &parent_dbg, 
                                                Arc::new(inputs)
                                            )
                                        ) 
                                    ) 
                                )
                            )
                        ) 
                    ) 
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
                    let epsilon = 1e-2;
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
