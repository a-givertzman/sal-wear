use std::sync::Arc;

use sal_core::{dbg::Dbg, error::Error};
use crate::{AngularGrid, Eval, domain::imbalance::context::ImbContext, me};

/// Выполняет ресемплинг (Order Tracking) отфильтрованного сигнала во временной области 
/// в равномерную сетку угловой области.
/// Использует локальную кубическую интерполяцию Catmull-Rom для предотвращения алиасинга.
pub struct OrderDomainSamples<Child> {
    /// Ссылка на генератор/конфигурацию идеальной сетки углов.
    angular_grid: Arc<AngularGrid<_>>,
    child: Child,
    dbg: Dbg,
}
impl<Child> OrderDomainSamples<Child>
where
    Child: Eval<ImbContext, ImbContext> + Send + 'static {
    /// 2 * PI
    const PI2: f64 = std::f64::consts::PI * 2.0;
    ///
    /// ### Returns `OrderDomainSamples` new instance
    /// - `parent` - Идентификатор родительской сущности (для отладки).
    /// - `angular_grid` - Ссылка на генератор/конфигурацию идеальной сетки углов.
    /// - `child` - Дочерний (предыдущий) расчетный шаг
    pub fn new(parent: &Dbg, angular_grid: Arc<AngularGrid<_>>, child: Child) -> Self {
        let dbg = Dbg::new(parent, me::<Self>());
        Self {
            angular_grid,
            child,
            dbg,
        }
    }
    /// Вычисляет точку с помощью интерполяции Catmull-Rom.
    /// `p0`, `p1`, `p2`, `p3` - четыре соседних отсчета сигнала.
    /// `t` - нормализованное время [0.0; 1.0] между `p1` и `p2`.
    #[inline]
    fn catmull_rom(p0: f64, p1: f64, p2: f64, p3: f64, t: f64) -> f64 {
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
        let ideal_angles = self.angular_grid.get_target_angles(phases[0], phases[phases.len() - 1]);
        let mut idx = 1;
        ctx.order_samples.clear();
        for &target_theta in ideal_angles.iter() {
            // Ищем интервал [idx, idx + 1], в который попадает требуемый угол
            while idx < phases.len() - 2 && phases[idx + 1] < target_theta {
                idx += 1;
            }
            // Пропускаем точки, если для них не хватает истории по краям чанка
            if target_theta < phases[idx] || idx >= phases.len() - 2 {
                continue; 
            }
            let phase_start = phases[idx];
            let phase_end = phases[idx + 1];
            let t = (target_theta - phase_start) / (phase_end - phase_start);
            let p0 = samples[idx - 1] as f64;
            let p1 = samples[idx] as f64;
            let p2 = samples[idx + 1] as f64;
            let p3 = samples[idx + 2] as f64;
            let resampled_val = Self::catmull_rom(p0, p1, p2, p3, t as f64);
            ctx.order_samples.push(resampled_val);
        }
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
