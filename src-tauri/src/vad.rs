/// Детектор тишины на основе RMS-энергии окна сэмплов.
/// sensitivity: 0.0 (шёпот — тишина, отсекает почти всё) .. 1.0 (стройка, отсекает только громкий шум)
pub struct SilenceDetector {
    threshold: f32,
    silence_hangover_ms: u32,
    sample_rate: u32,
    silent_ms_accum: u32,
}

impl SilenceDetector {
    pub fn new(sensitivity: f32, silence_hangover_ms: u32, sample_rate: u32) -> Self {
        let sensitivity = sensitivity.clamp(0.0, 1.0);
        // Порог RMS от "шёпот" (0.002) до "стройка" (0.08), логарифмическая шкала ощущается естественнее.
        let min_thresh: f32 = 0.002;
        let max_thresh: f32 = 0.08;
        let threshold = min_thresh * (max_thresh / min_thresh).powf(sensitivity);
        Self {
            threshold,
            silence_hangover_ms,
            sample_rate,
            silent_ms_accum: 0,
        }
    }

    fn rms(chunk: &[f32]) -> f32 {
        if chunk.is_empty() {
            return 0.0;
        }
        let sum_sq: f32 = chunk.iter().map(|s| s * s).sum();
        (sum_sq / chunk.len() as f32).sqrt()
    }

    /// Скармливаем очередной чанк сэмплов. Возвращает true, если накопленная тишина
    /// превысила hangover и запись пора останавливать.
    pub fn feed(&mut self, chunk: &[f32]) -> bool {
        let energy = Self::rms(chunk);
        let chunk_ms = (chunk.len() as f32 / self.sample_rate as f32 * 1000.0) as u32;
        if energy < self.threshold {
            self.silent_ms_accum += chunk_ms;
        } else {
            self.silent_ms_accum = 0;
        }
        self.silent_ms_accum >= self.silence_hangover_ms
    }

    pub fn reset(&mut self) {
        self.silent_ms_accum = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn silence_chunk(len: usize) -> Vec<f32> {
        vec![0.0; len]
    }

    fn loud_chunk(len: usize) -> Vec<f32> {
        (0..len)
            .map(|i| ((i as f32) * 0.3).sin() * 0.5)
            .collect()
    }

    #[test]
    fn stays_active_while_speaking() {
        let mut vad = SilenceDetector::new(0.5, 800, 16000);
        for _ in 0..10 {
            let stop = vad.feed(&loud_chunk(1600)); // 100ms chunks
            assert!(!stop);
        }
    }

    #[test]
    fn stops_after_hangover_of_silence() {
        let mut vad = SilenceDetector::new(0.5, 500, 16000);
        // 100ms silent chunks, should trigger stop once accumulated >= 500ms
        assert!(!vad.feed(&silence_chunk(1600)));
        assert!(!vad.feed(&silence_chunk(1600)));
        assert!(!vad.feed(&silence_chunk(1600)));
        assert!(!vad.feed(&silence_chunk(1600)));
        assert!(vad.feed(&silence_chunk(1600)));
    }

    #[test]
    fn speech_resets_silence_accumulator() {
        let mut vad = SilenceDetector::new(0.5, 500, 16000);
        vad.feed(&silence_chunk(1600));
        vad.feed(&silence_chunk(1600));
        vad.feed(&silence_chunk(1600));
        // speech interrupts before hangover triggers
        assert!(!vad.feed(&loud_chunk(1600)));
        assert!(!vad.feed(&silence_chunk(1600)));
        assert!(!vad.feed(&silence_chunk(1600)));
    }

    #[test]
    fn higher_sensitivity_raises_threshold() {
        let low = SilenceDetector::new(0.0, 500, 16000);
        let high = SilenceDetector::new(1.0, 500, 16000);
        assert!(low.threshold < high.threshold);
    }
}
