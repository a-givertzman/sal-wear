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
        BendingStresses, Context, Eval, GearMeshFrequency, MockInputs, NumberOfMeshCycles, ReadInputs, RotationalFrequency,
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
    /// Расчёт напряжения изгиба
    /// См. [раздел 8.3, шаг 5](../../Operion_Diag_Вибродиагностика_и_остаточный_ресурс.pdf)
    /// Формула: 
    /// σF_i = KF * (Ft / (b * m)) * YF
    /// Где:
    /// * `KF` — коэффициент нагрузки изгиба
    /// * `Ft` — окружная сила
    /// * `b` — ширина зубчатого венца [м]
    /// * `m` — модуль зубчатого колеса [м]
    /// * `YF` — коэффициент геометрии формы зуба 
    #[test]
    fn bending_stresses() {
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
                2.0,
                3.0,
                0.1,
                0.1,
                0.66667,
            ),
            (
                2,
                4.0,
                5.0,
                6.0,
                2.0,
                0.1,
                0.166667,
            ),
        ];
        for (step, kf, tangetial_force, b, m, yf, target) in test_data {
            let mut inputs = MockInputs::new();  
            inputs.kf = Some(kf);
            inputs.yf = Some(yf);
            inputs.b = Some(b);
            inputs.m = Some(m);
            let child = ReadInputs::new(
                &parent_dbg, 
                Arc::new(inputs)
            );
            let mut ctx = Context::new_test(0.0);
            ctx.tangential_force = tangetial_force;
            let result = BendingStresses::new(
                &parent_dbg,
                child
            ).eval(ctx);
            assert!(
                result.err.is_none(), 
                "Шаг [{}]: Ожидался успешный расчет, но получена ошибка: {:?}", 
                step, result.err
            );
            let actual = result.bending_stresses;
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
