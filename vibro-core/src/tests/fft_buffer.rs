use crate::num_complex::Complex;
use slice_ring_buffer::SliceRingBuffer;

/// Высокопроизводительный буфер, удобен для FFT
/// 
/// Используется только для тестирования!
/// 
/// ⚠️ Важно!
/// В крейте slice-ring-buffer обнаружены критические уязвимости безопасности (double-free)
/// на уровне безопасного API, из-за чего он признан небезопасным (RUSTSEC-2025-0044).
/// Кроме того, они сильно привязаны к конкретной ОС (требуют разные хаки для Windows, macOS и Linux).
pub struct FftBuffer<T> {
    size: usize,
    buf: SliceRingBuffer<Complex<T>>,
    step: usize,
    new_samples: usize,
}
impl<T: Copy> FftBuffer<T> {
    pub fn new(size: usize, step: usize) -> Self {
        Self {
            size,
            buf: SliceRingBuffer::with_capacity(size),
            step,
            new_samples: 0,
        }
    }
    pub fn add(&mut self, values: impl IntoIterator<Item = Complex<T>>) {
        for v in values.into_iter() {
            if self.buf.len() >= self.size {
                self.buf.pop_front();
            }
            self.buf.push_back(v);
            self.new_samples += 1;
        }
    }
    pub fn copy_into(&self, target: &mut [Complex<T>]) {
        let src = self.buf.as_slice();
        let len = src.len().min(target.len());
        target[..len].copy_from_slice(&src[..len]);
    }
    pub fn is_full(&self) -> bool {
        log::trace!("FftBuffer.is_full | {}/{}",self.buf.len(), self.size);
        self.buf.len() >= self.size && self.new_samples >= self.step
    }
    pub fn reset(&mut self) {
        self.new_samples = 0;
    }
}
