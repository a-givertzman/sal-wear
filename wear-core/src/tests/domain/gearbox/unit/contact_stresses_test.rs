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
        ContactStresses, Context, Eval, MockInputs, ReadInputs,
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
    /// Расчёт напряжения контакта
    /// См. [раздел 8.3, шаг 5](../../Operion_Diag_Вибродиагностика_и_остаточный_ресурс.pdf)
    /// Формула: 
    /// σH_i = KH * ZH * sqrt(Ft_i / (b * d_p))
    /// Где:
    /// * `KH` — коэффициент нагрузки изгиба
    /// * `ZH` — коэффициент геометрии контакта
    /// * `Ft` — окружная сила
    /// * `b` — ширина зубчатого венца [м]
    /// * `d_p` — делительный диаметр шестерни [м]
    #[test]
    fn contact_stresses() {
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
                34.64101615137754,
            ),
            (
                2,
                4.0,
                5.0,
                6.0,
                2.0,
                0.1,
                109.54451150103323,
            ),
        ];
        for (step, kh, zh, tangetial_force, b, d_p, target) in test_data {
            let mut inputs = MockInputs::new();  
            inputs.kh = Some(kh);
            inputs.zh = Some(zh);
            inputs.b = Some(b);
            inputs.d_p = Some(d_p);
            let child = ReadInputs::new(
                &parent_dbg, 
                Arc::new(inputs)
            );
            let mut ctx = Context::new_test(0.0);
            ctx.tangential_force = tangetial_force;
            let result = ContactStresses::new(
                &parent_dbg,
                child
            ).eval(ctx);
            assert!(
                result.err.is_none(), 
                "Шаг [{}]: Ожидался успешный расчет, но получена ошибка: {:?}", 
                step, result.err
            );
            let actual = result.contact_stresses;
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
