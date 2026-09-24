use std::{f64::consts::PI, sync::Arc};
use chrono::Utc;
use sal_core::dbg::Dbg;
use crate::{ComplexLocalOscillator, Decimation, Eval, Frame, ImbContext, };
struct DummyChild;
impl Eval<ImbContext, ImbContext> for DummyChild {
    fn eval(&self, ctx: ImbContext) -> ImbContext { ctx } // Просто возвращает контекст без изменений
    fn exit(&self) {}
}
///
/// Проверка сноса частоты 1X близкой к нулю
#[test]
fn complex_local_oscillator() {
    let dbg = Dbg::own("complex_local_oscillator_test");
    let rpm_net = 49.5 * 60.0;
    let f_decimation = 16000.0;
    let complex_lo = ComplexLocalOscillator::new(
        &dbg,
        DummyChild,
        rpm_net,
        f_decimation,
    );
    let mut raw: Vec<f64> = Vec::with_capacity(512);
    for t in 0..512 {
        let time = t as f64 / f_decimation;
        raw.push((2.0 * PI * 50.0 * time).sin());
    }
    let raw_u16: Vec<u16> = raw.iter()
        .map(|&v| (v + 2048.0).round() as u16)
        .collect();
    let mut ctx = ImbContext::default();
    let mut prev_phase: Option<f64> = None;
    let dt = 1.0 / 16000.0;
    for chunk in raw_u16.chunks(512) {
        let samples: [u16; 512] = chunk.try_into().expect("ожидается ровно 512 отсчётов");
        let frame = Frame::raw(Utc::now(), samples);
        ctx.frame = Arc::new(frame);
        ctx = complex_lo.eval(ctx);
        for oscillated_value in ctx.complex_local_oscillator.complex_signal.iter() {
            let phase = oscillated_value.im.atan2(oscillated_value.re);
            if let Some(prev) = prev_phase {
                let mut dphase = phase - prev;
                if dphase > PI { dphase -= 2.0 * PI; }
                if dphase < -PI { dphase += 2.0 * PI; }
                let measured_delta_f = dphase / (2.0 * PI * dt);
                assert!(
                    (measured_delta_f - 0.5).abs() < 0.05,
                    "снос 1X не удался: ожидалась остаточная частота ≈0.5Гц, получено {measured_delta_f}"
                );
            }
            prev_phase = Some(phase);
        }
    }
}