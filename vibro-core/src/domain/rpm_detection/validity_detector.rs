use function_name::named;
use sal_core::dbg::Dbg;
use crate::{AngularCtx, Eval, TrackState, err};
/// Детектор валидности фазы (амплитуда огибающей с двухпороговым гистерезисом).
///
/// # Входные данные
/// Амплитуда огибающей `amplitude` (из ФНЧ полосы слежения) и флаги прогрева
/// `ready` (из `InstantShaftPhase`).
///
/// # Математическая суть и гистерезис
/// Модуль (амплитуда) комплексного вектора `A[n] = |I[n] + j*Q[n]|` непрерывно проверяется 
/// на предмет просадок уровня полезного сигнала, вызванных ростом ошибки настройки частоты 
/// или сетевыми сбоями. Чтобы исключить дребезг флага валидности вблизи границы зашумленного 
/// сигнала, в систему заложен двухпороговый гистерезис:
/// 
/// 1. Если текущее состояние валидно (`valid == true`), сброс в невалидное состояние
///    происходит при нефинитной амплитуде или падении ниже нижнего порога: `a < a_low`.
/// 2. Если текущее состояние невалидно (`valid == false`), возврат флага валидности
///    происходит только при конечной амплитуде выше верхнего порога: `a > a_high`.
///
/// # Логика переключения состояний конвейера (TrackState)
/// Детектор управляет глобальным статусом конвейера трекинга вала на основе времени 
/// удержания невалидного флага:
/// * `TrackState::Valid` — Сигнал стабилен, амплитуда выше порогов гистерезиса.
/// * `TrackState::Degraded` — Зафиксировано падение амплитуды (например, идет переходный процесс), 
///   но время невалидности еще не превысило интервал установления фильтра `settle_samples`.
/// * `TrackState::Acquisition` — Сигнал отсутствует или ошибка частоты слишком велика дольше 
///   времени `tau_settle`. Вычисляемая фаза считается разрушенной, конвейер переходит в режим захвата.
/// # Выходные данные
/// Глобальное состояние `TrackState` (Valid / Degraded / Acquisition) и поотсчётная
/// валидность, записанная прямо в `PhaseStatus` полей `shaft_phase` /
/// `shaft_phase_unwrapped` (единый источник валидности для потребителя).
pub struct ValidityDetector<Child> {
    /// Ниже этого порога амплитуда огибающей считается критически малой (сброс валидности).
    a_low: f64,
    /// Выше этого порога амплитуда огибающей считается достаточной для восстановления трекинга.
    a_high: f64,
    /// Время установления переходного процесса (tau_settle), выраженное в отсчетах сигнала 
    /// рабочей частоты дискретизации (рассчитывается примерно как: `(1 / (pi * B)) * fs`).
    settle_samples: usize,
    /// Дочерний вычислительный узел конвейера
    child: Child,
    /// Контекст отладки и логирования
    dbg: Dbg,
}

impl<Child> ValidityDetector<Child>
where
    Child: Eval<AngularCtx, AngularCtx> + Send + 'static,
{
    /// Создает новый экземпляр вычислительного узла `ValidityDetector`.
    /// 
    /// Конструктор выполняет строгую валидацию инварианта гистерезиса: верхний порог 
    /// активации обязан быть строго больше нижнего порога сброса, иначе сборка упадет с паникой.
    ///
    /// # Аргументы
    /// * `parent` - Ссылка на отладочный контекст родительской сущности.
    /// * `child` - Предыдущий вычислительный шаг конвейера.
    /// * `a_low` - Нижний порог амплитуды огибающей для сброса флага валидности.
    /// * `a_high` - Верхний порог амплитуды огибающей для восстановления флага валидности.
    /// * `settle_samples` - Максимально допустимое число невалидных отсчетов до входа в захват.
    pub fn new(parent: &Dbg, child: Child, a_low: f64, a_high: f64, settle_samples: usize) -> Self {
        assert!(a_high > a_low, "a_high должен быть больше a_low (инвариант двухпорогового гистерезиса)");
        Self { 
            a_low, 
            a_high, 
            settle_samples, 
            child, 
            dbg: Dbg::new(parent, crate::me::<Self>()) 
        }
    }
}

impl<Child> Eval<AngularCtx, AngularCtx> for ValidityDetector<Child>
where
    Child: Eval<AngularCtx, AngularCtx> + Send + 'static,
{
    /// Определяет валидность фазы по амплитуде огибающей и обновляет глобальное состояние.
    ///
    /// Читает амплитуду `amplitude` из ФНЧ полосы слежения, продвигает двухпороговый
    /// гистерезис, перезаписывает `PhaseStatus::Invalid` в `shaft_phase` и
    /// `shaft_phase_unwrapped` для невалидных отсчётов, а по длительности просадки
    /// переключает глобальное состояние `TrackState`.
    #[named]
    fn eval(&self, ctx: AngularCtx) -> AngularCtx {
        // Передаем контекст дальше по цепочке вниз для предварительной фильтрации комплексным ФНЧ
        let mut ctx = self.child.eval(ctx);
        if ctx.err.is_some() {
            return ctx.pass_err(&self.dbg, "eval");
        }
        let v = &mut ctx.rpm_detection.validity_detector;
        let lpf = &ctx.rpm_detection.lpf_tracking_band;
        let ph = &mut ctx.rpm_detection.instant_shaft_phase;
        if lpf.amplitude.len() != ph.ready.len() {
            let (a, b) = (lpf.amplitude.len(), ph.ready.len());
            ctx.err = Some(err!(self.dbg, "eval: amplitude ({}) != ready ({})", a, b));
            return ctx;
        }
        for (n, &a) in lpf.amplitude.iter().enumerate() {
            // Логика двухпорогового компаратора
            if v.valid {
                if !a.is_finite() || a < self.a_low {
                    v.valid = false; 
                }
            } else if a.is_finite() && a > self.a_high {
                v.valid = true;
            }
            // Накапливаем или сбрасываем непрерывный счетчик длительности аварийного состояния
            if v.valid { 
                v.invalid_samples = 0; 
            } else { 
                v.invalid_samples += 1; 
            }

            if !(v.valid && ph.ready[n]) {
                ph.shaft_phase[n] = crate::PhaseStatus::Invalid;
                ph.shaft_phase_unwrapped[n] = crate::PhaseStatus::Invalid;
            }
        }
        // Финальный автомат переключения глобального состояния вычислительного ядра
        v.state = if v.valid {
            TrackState::Valid
        } else if v.invalid_samples > self.settle_samples {
            TrackState::Acquisition // Длительная авария: фаза разрушена, уходим в режим повторного захвата частоты
        } else {
            TrackState::Degraded    // Кратковременный сбой: фаза еще вычисляется, но данные помечены как нестабильные
        };
        ctx
    }

    /// Вызывает каскадное завершение для дочерних шагов цепочки вычислений.
    fn exit(&self) { 
        self.child.exit(); 
    }
}
