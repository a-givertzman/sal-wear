use crate::{Phase, num_complex::Complex};
use sal_core::{dbg::Dbg, error::Error};
use crate::{Eval, domain::imbalance::context::ImbContext, me};

/// Выполняет ресемплинг (Order Tracking) отфильтрованного сигнала во временной области 
/// в равномерную сетку угловой области.
/// Использует локальную кубическую интерполяцию Catmull-Rom для предотвращения алиасинга.
pub struct OrderDomainSamples<Child> {
    /// Плотность угловой сетки (точек на оборот).
    samples_per_rev: usize,
    child: Child,
    dbg: Dbg,
}
impl<Child> OrderDomainSamples<Child>
where
    Child: Eval<ImbContext, ImbContext> + Send + 'static {
    ///
    /// ### Returns `OrderDomainSamples` new instance
    /// - `parent` - Идентификатор родительской сущности (для отладки).
    /// - `samples_per_rev` - Плотность угловой сетки (точек на оборот).
    /// - `child` - Дочерний (предыдущий) расчетный шаг
    pub fn new(parent: &Dbg, samples_per_rev: usize, child: Child) -> Self {
        let dbg = Dbg::new(parent, me::<Self>());
        Self {
            samples_per_rev,
            // angles: (0..samples_per_rev).map(|i| (i as f64) * TAU / samples_per_rev as f64).collect(),
            child,
            dbg,
        }
    }
    /// Вычисляет точку с помощью интерполяции Catmull-Rom.
    /// `p0`, `p1`, `p2`, `p3` - четыре соседних отсчета сигнала.
    /// `t` - нормализованное время [0.0; 1.0] между `p1` и `p2`.
    #[inline]
    fn catmull_rom(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
        let t2 = t * t;
        let t3 = t2 * t;
        0.5 * (
            2.0 * p1 +
            (-p0 + p2) * t +
            (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2 +
            (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3
        )
    }
}
impl<Child> Eval<ImbContext, ImbContext> for OrderDomainSamples<Child>
where
    Child: Eval<ImbContext, ImbContext> + Send + 'static {
    //
    #[inline]
    fn eval(&self, ctx: ImbContext) -> ImbContext {
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        let samples = &ctx.samples;
        let phases = &ctx.frame.phases;
        if samples.len() < 4 || phases.len() != samples.len() {
            ctx.err = Some(Error::new(&self.dbg, "eval").err("Недостаточно данных для сплайна"));
            return ctx;
        }
        // Запрашиваем идеальные углы, которые попадают в текущий физический кадр
        let ideal_angles = TargetAngles::new(phases[0], phases[phases.len() - 1], self.samples_per_rev);
        let mut idx = 1;
        ctx.order_samples.clear();
        for target_theta in ideal_angles {
            // Ищем интервал [idx, idx + 1], в который попадает требуемый угол
            while idx < phases.len() - 2 && (phases[idx + 1]) < target_theta as f32 {
                idx += 1;
            }
            // Пропускаем точки, если для них не хватает истории по краям чанка
            if (target_theta as f32) < phases[idx] || idx >= phases.len() - 2 {
                continue; 
            }
            let phase_start = phases[idx];
            let phase_end = phases[idx + 1];
            let t = (target_theta as f32 - phase_start) / (phase_end - phase_start);
            let p0 = samples[idx - 1];
            let p1 = samples[idx];
            let p2 = samples[idx + 1];
            let p3 = samples[idx + 2];
            let resampled_val = Self::catmull_rom(p0, p1, p2, p3, t);
            ctx.order_samples.push(Complex { re: resampled_val, im: 0.0 });
        }
        let delta_theta = std::f64::consts::TAU / (self.samples_per_rev as f64);
        ctx.total_phase = Phase(ctx.total_phase.to_radians() + (ctx.order_samples.len() as f64) * delta_theta);
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
/// Математический генератор идеальной угловой сетки (Zero-Cost).
/// Вычисляет целевые углы на лету без выделения памяти.
pub struct TargetAngles {
    current_idx: isize,
    end_idx: isize,
    delta_theta: f64,
}
impl TargetAngles {
    /// Создает итератор по идеальным отметкам математической сетки,
    /// которые попали в физический промежуток между `theta1` и `theta2`.
    /// Корректно обрабатывает перехлест фазы (конец оборота).
    ///
    /// Аргументы:
    /// - `theta1`: Начальный угол физического чанка (в радианах).
    /// - `theta2`: Конечный угол физического чанка (в радианах).
    /// - `samples_per_rev`: Плотность угловой сетки (точек на оборот).
    pub fn new(theta1: impl Into<f64>, theta2: impl Into<f64>, samples_per_rev: usize) -> Self {
        let theta1 = theta1.into();
        let theta2 = theta2.into();
        let delta_theta = std::f64::consts::TAU / (samples_per_rev as f64);
        let current_idx = (theta1 / delta_theta).ceil() as isize;
        let end_idx = (theta2 / delta_theta).floor() as isize + 1;
        Self {
            current_idx,
            end_idx,
            delta_theta,
        }
    }
}
impl Iterator for TargetAngles {
    type Item = f64;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_idx < self.end_idx {
            let angle = (self.current_idx as f64) * self.delta_theta;
            self.current_idx += 1;
            Some(angle)
        } else {
            None
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{PI, TAU};
    /// Вспомогательная функция для безопасного сравнения векторов с плавающей точкой
    fn assert_angles_eq(actual: Vec<f64>, expected: &[f64]) {
        assert_eq!(
            actual.len(),
            expected.len(),
            "Количество углов не совпадает: получили {}, ожидали {}",
            actual.len(),
            expected.len()
        );
        for (i, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
            assert!(
                (a - e).abs() < 1e-9,
                "Ошибка в индексе {}: {} != {}",
                i,
                a,
                e
            );
        }
    }
    #[test]
    fn test_target_angles_linear_segment() {
        // Сетка 4 точки на оборот: 0, PI/2, PI, 3*PI/2
        // Физический чанк лежит от 1.0 рад до 4.0 рад
        // Должны попасть: PI/2 (1.57) и PI (3.14)
        let angles: Vec<f64> = TargetAngles::new(1.0, 4.0, 4).collect();
        assert_angles_eq(angles, &[PI / 2.0, PI]);
    }
    #[test]
    fn test_target_angles_phase_wrap() {
        // Перехлест оборота в новой концепции (непрерывная ось углов):
        // Чанк начался в конце первого оборота (5.0 рад) 
        // и закончился в начале следующего оборота.
        // Вместо "1.0 рад" мы пишем "TAU + 1.0 рад", показывая движение вперед.
        let theta1 = 5.0;
        let theta2 = std::f64::consts::TAU + 1.0; // ~7.283... рад
        
        // Сетка 4 точки на оборот: 0, PI/2 (~1.57), PI (~3.14), 3*PI/2 (~4.71), TAU (~6.28), TAU + PI/2 (~7.85)
        let angles: Vec<f64> = TargetAngles::new(theta1, theta2, 4).collect();
        
        // В этот диапазон (от 5.0 до 7.283) идеально попадает только точка начала нового оборота (TAU, то есть 2*PI)
        // Предыдущая точка (3*PI/2 = 4.71) осталась позади, а следующая (TAU + PI/2 = 7.85) еще впереди.
        let expected_angle = std::f64::consts::TAU;
        
        assert_eq!(angles.len(), 1);
        assert!((angles[0] - expected_angle).abs() < 1e-9, "Expected {}, got {}", expected_angle, angles[0]);
    }
    
    #[test]
    // fn test_target_angles_phase_wrap() {
    //     // Перехлест оборота: чанк начался на 5.0 рад (конец старого оборота), 
    //     // а закончился на 1.0 рад (начало нового оборота).
    //     // Должны попасть: 3*PI/2 (4.71 - мимо, так как меньше 5.0), 
    //     // 0.0 (перехлест) и мы не доходим до PI/2 (1.57)
    //     let angles: Vec<f64> = TargetAngles::new(5.0, 1.0, 4).collect();
    //     assert_angles_eq(angles, &[0.0]);
    // }
    #[test]
    fn test_target_angles_exact_bounds() {
        // Сетка 8 точек на оборот.
        // Чанк начинается ровно с PI/2 и заканчивается ровно на PI.
        // Должны попасть границы включительно, плюс промежуточная точка 3*PI/4.
        let theta1 = PI / 2.0;
        let theta2 = PI;
        let angles: Vec<f64> = TargetAngles::new(theta1, theta2, 8).collect();
        assert_angles_eq(angles, &[PI / 2.0, 3.0 * PI / 4.0, PI]);
    }
    #[test]
    fn test_target_angles_bounds() {
        // Сетка 8 точек на оборот.
        // Чанк начинается с PI / 8.0 + 0.01 и заканчивается ровно на 5.0 * PI / 8.0 - 0.01.
        let theta1 = PI / 4.0 + 0.01;
        let theta2 = 5.0 * PI / 4.0 - 0.01;
        let angles: Vec<f64> = TargetAngles::new(theta1, theta2, 8).collect();
        assert_angles_eq(angles, &[PI/2.0, 3.0*PI/4.0, PI]);
    }
    #[test]
    fn test_target_angles_empty_range() {
        // Очень короткий чанк, внутрь которого не попадает ни одна идеальная отметка сетки.
        // Например, от 0.1 до 1.0 при сетке из 4 точек (первая точка PI/2 ~ 1.57).
        let angles: Vec<f64> = TargetAngles::new(0.1, 1.0, 4).collect();
        assert!(angles.is_empty(), "Итератор должен быть пустым");
    }
    // Хелпер для сравнения f64 с плавающей точкой
    fn close_enough(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }
    #[test]
    fn test_standard_range() {
        // Сетка 4 точки на оборот (delta = PI/2 = 1.57079...)
        // Диапазон от 1.0 до 4.0. Должны попасть: PI/2 (1.57), PI (3.14)
        let mut it = TargetAngles::new(1.0, 4.0, 4);
        let p1 = it.next().unwrap();
        assert!(close_enough(p1, PI / 2.0), "Expected PI/2, got {}", p1);
        let p2 = it.next().unwrap();
        assert!(close_enough(p2, PI), "Expected PI, got {}", p2);
        assert!(it.next().is_none());
    }

    #[test]
    fn test_exact_boundaries() {
        // Проверяем включение границ, если углы идеально совпадают с сеткой
        // Сетка 4 точки на оборот. Диапазон ровно от PI/2 до PI
        let mut it = TargetAngles::new(PI / 2.0, PI, 4);
        let p1 = it.next().unwrap();
        assert!(close_enough(p1, PI / 2.0));
        let p2 = it.next().unwrap();
        assert!(close_enough(p2, PI));
        assert!(it.next().is_none());
    }
    #[test]
    fn test_negative_angles() {
        // Диапазон от -3.2415 до -0.5. Шаг сетки 1.5707.
        // Должны попасть: -PI (-3.1415) и -PI/2 (-1.5707)
        let mut it = TargetAngles::new(-PI - 0.1, -0.5, 4);
        let p1 = it.next().unwrap();
        assert!(close_enough(p1, -PI), "Expected -PI, got {}", p1);
        let p2 = it.next().unwrap();
        assert!(close_enough(p2, -PI / 2.0), "Expected -PI/2, got {}", p2);
        assert!(it.next().is_none());
    }
    #[test]
    fn test_no_points_in_chunk() {
        // Физический чанк слишком мал и находится между узлами сетки
        // Сетка с шагом PI/2. Чанк от 0.1 до 1.0. Внутри узлов нет.
        let mut it = TargetAngles::new(0.1, 1.0, 4);
        assert!(it.next().is_none());
    }
}
