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
        BendingCyclesNum, BendingFatigueDamage, BendingStresses, ContactCyclesNum, ContactFatigueDamage, ContactStresses, Context, Eval, GearMeshFrequency, MockInputs, NumberOfMeshCycles, ReadInputs, RotationalFrequency, TangentialForce, ToothDamage,
    };

    static INIT: Once = Once::new();

    fn init_once() {
        INIT.call_once(|| {
            // Инициализация кода, если требуется
        })
    }

    fn init_each() -> () {}
    ///
    /// Расчёт повреждения зуба
    /// См. [раздел 8.3, шаг 7](../../Operion_Diag_Вибродиагностика_и_остаточный_ресурс.pdf)
    /// Формула: 
    /// D_gear = max(DF, DH)
    #[test]
    fn tooth_damage_complex() {
        DebugSession::new().filter(LogLevel::Debug).init();
        init_once();
        init_each();
        let parent_dbg = Dbg::own("test_session");
        println!("\n{}", parent_dbg);
        let test_duration = TestDuration::new(&parent_dbg, Duration::from_secs(10));
        test_duration.run().unwrap();
        // Структура данных:
        // (step, rpm, torque, duration, z_p, d_p, kf, yf, b, m, kh, zh, f_lim, m_f, nf_0, h_lim, m_h, nh_0, target_D_gear)
        let test_data = [
            (
                1,          // Шаг 1: Тест-кейс с крупным модулем (m = 0.1, d_p = 2.0)
                600.0,      // rpm (дает f_rot = 10 Гц)
                1000.0,     // torque -> запишется в ctx.motor_torque
                10.0,       // duration -> запишется в Context::new_test()
                Some(20),   // z_p 
                Some(2.0),  // d_p (2.0 метра)
                Some(1.0),  // kf
                Some(1.0),  // yf
                Some(0.1),  // b (0.1 метра)
                Some(0.1),  // m (0.1 метра)
                Some(1.0),  // kh
                Some(1.0),  // zh
                Some(1_000_000.0),   // f_lim
                Some(6.0),           // m_f
                Some(4.0),   // nf_0
                Some(1_000.0),       // h_lim
                Some(8.0),           // m_h
                Some(5.0),   // nh_0      
                0.0005,    // Точный расчетный target_D_gear
            ),
            (
                2,          // Шаг 2: Альтернативный тест-кейс (m = 0.15, d_p = 3.0)
                900.0,      // rpm (f_rot = 15 Гц)
                1500.0,     // torque
                20.0,       // duration
                Some(20),   // z_p
                Some(3.0),  // d_p (3.0 метра)
                Some(1.2),  // kf
                Some(1.1),  // yf
                Some(0.2),  // b
                Some(0.15), // m
                Some(1.1),  // kh
                Some(1.05), // zh
                Some(2_000_000.0),   // f_lim
                Some(6.0),           // m_f
                Some(5.0),   // nf_0
                Some(2_000.0),       // h_lim
                Some(8.0),           // m_h
                Some(6.0),   // nh_0    
                0.000000136055, // Точный расчетный target_D_gear
            ),
        ];
        for (step, rpm, torque, duration, z_p, d_p, kf, yf, b, m, kh, zh, f_lim, m_f, nf_0, h_lim, m_h, nh_0, target) in test_data {
            let mut inputs = MockInputs::new();  
            inputs.rpm = Some(rpm);
            inputs.z_p = z_p;  
            inputs.d_p = d_p;  
            inputs.kf = kf;  
            inputs.yf = yf;  
            inputs.b = b;  
            inputs.m = m;  
            inputs.kh = kh;  
            inputs.zh = zh;  
            inputs.f_lim = f_lim;  
            inputs.m_f = m_f;  
            inputs.nf_0 = nf_0;  
            inputs.h_lim = h_lim;  
            inputs.m_h = m_h;  
            inputs.nh_0 = nh_0;  
            let child = ReadInputs::new(
                &parent_dbg, 
                Arc::new(inputs)
            );
            let mut ctx = Context::new_test(duration);
            ctx.motor_torque = torque;
            let result = ToothDamage::new(
                &parent_dbg,
                ContactFatigueDamage::new(
                    &parent_dbg, 
                    BendingFatigueDamage::new(
                        &parent_dbg, 
                        ContactCyclesNum::new(
                            &parent_dbg, 
                            BendingCyclesNum::new(
                                &parent_dbg, 
                                ContactStresses::new(
                                    &parent_dbg, 
                                    BendingStresses::new(
                                        &parent_dbg, 
                                        TangentialForce::new(
                                            &parent_dbg, 
                                            NumberOfMeshCycles::new(
                                                &parent_dbg, 
                                                GearMeshFrequency::new(
                                                    &parent_dbg, 
                                                    RotationalFrequency::new(
                                                        &parent_dbg, 
                                                        child
                                                    )
                                                )
                                            )
                                        )
                                    )
                                )
                            )
                        )
                    )
                )
            ).eval(ctx);
            assert!(
                result.err.is_none(), 
                "Шаг [{}]: Ожидался успешный расчет, но получена ошибка: {:?}", 
                step, result.err
            );
            let actual = result.tooth_damage;
            let epsilon = 1e-12; 
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
