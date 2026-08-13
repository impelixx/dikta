use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::vad::SilenceDetector;

/// Простой линейный ресемплер моно f32 в целевую частоту — достаточно по качеству
/// для распознавания речи, без тяжёлых зависимостей (rubato и т.п.).
fn resample_linear(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate || input.is_empty() {
        return input.to_vec();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let out_len = (input.len() as f64 / ratio).floor() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_pos = i as f64 * ratio;
        let idx = src_pos.floor() as usize;
        let frac = (src_pos - idx as f64) as f32;
        let a = input.get(idx).copied().unwrap_or(0.0);
        let b = input.get(idx + 1).copied().unwrap_or(a);
        out.push(a + (b - a) * frac);
    }
    out
}

fn downmix_to_mono(interleaved: &[f32], channels: u16) -> Vec<f32> {
    if channels <= 1 {
        return interleaved.to_vec();
    }
    let channels = channels as usize;
    interleaved
        .chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

struct SharedBuffer {
    recording: bool,
    samples: Vec<f32>,
}

/// Владеет открытым входным потоком cpal на всё время жизни приложения.
/// start()/stop() лишь включают/выключают запись сэмплов в буфер — сам поток
/// не пересоздаётся на каждую диктовку, это дешевле и надёжнее.
pub struct AudioEngine {
    _stream: cpal::Stream,
    buffer: Arc<Mutex<SharedBuffer>>,
    silence_triggered: Arc<AtomicBool>,
    vad: Arc<Mutex<Option<SilenceDetector>>>,
    device_sample_rate: u32,
    device_channels: u16,
}

unsafe impl Send for AudioEngine {}
unsafe impl Sync for AudioEngine {}

impl AudioEngine {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .context("не найдено устройство ввода звука")?;
        let config = device.default_input_config().context("нет конфигурации входа")?;
        let sample_rate = config.sample_rate().0;
        let channels = config.channels();

        let buffer = Arc::new(Mutex::new(SharedBuffer {
            recording: false,
            samples: Vec::new(),
        }));
        let silence_triggered = Arc::new(AtomicBool::new(false));

        let buffer_cb = buffer.clone();
        let silence_cb = silence_triggered.clone();
        // VAD настраивается заново при каждом start(), но поток должен захватить
        // изменяемое состояние — держим его тоже в Mutex рядом с буфером.
        let vad: Arc<Mutex<Option<SilenceDetector>>> = Arc::new(Mutex::new(None));
        let vad_cb = vad.clone();

        let err_fn = |err| eprintln!("[audio] ошибка потока: {err}");

        let stream = device.build_input_stream(
            &config.into(),
            move |data: &[f32], _| {
                let mut buf = buffer_cb.lock().unwrap();
                if !buf.recording {
                    return;
                }
                buf.samples.extend_from_slice(data);
                if let Some(vad) = vad_cb.lock().unwrap().as_mut() {
                    if vad.feed(data) {
                        silence_cb.store(true, Ordering::SeqCst);
                    }
                }
            },
            err_fn,
            None,
        )?;
        stream.play()?;

        Ok(Self {
            _stream: stream,
            buffer,
            silence_triggered,
            vad,
            device_sample_rate: sample_rate,
            device_channels: channels,
        })
    }

    /// Начинает копить сэмплы. `vad` = Some, если для этой сессии включён автостоп по тишине.
    pub fn start(&self, vad: Option<SilenceDetector>) {
        *self.vad.lock().unwrap() = vad;
        let mut buf = self.buffer.lock().unwrap();
        buf.recording = true;
        buf.samples.clear();
        self.silence_triggered.store(false, Ordering::SeqCst);
    }

    /// Была ли зафиксирована тишина достаточной длительности с последнего start().
    pub fn silence_triggered(&self) -> bool {
        self.silence_triggered.load(Ordering::SeqCst)
    }

    /// Останавливает запись и возвращает накопленные сэмплы, приведённые к 16kHz mono.
    pub fn stop(&self) -> Vec<f32> {
        let mut buf = self.buffer.lock().unwrap();
        buf.recording = false;
        let raw = std::mem::take(&mut buf.samples);
        drop(buf);
        let mono = downmix_to_mono(&raw, self.device_channels);
        resample_linear(&mono, self.device_sample_rate, 16000)
    }

    /// RMS громкости последнего небольшого хвоста буфера — для живой волны в UI.
    /// Не претендует на точность, только на "живость" индикатора.
    pub fn recent_level(&self) -> f32 {
        let buf = self.buffer.lock().unwrap();
        let tail_len = 2048.min(buf.samples.len());
        if tail_len == 0 {
            return 0.0;
        }
        let tail = &buf.samples[buf.samples.len() - tail_len..];
        let sum_sq: f32 = tail.iter().map(|s| s * s).sum();
        (sum_sq / tail_len as f32).sqrt().clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_identity_when_same_rate() {
        let input = vec![0.1, 0.2, 0.3, 0.4];
        let out = resample_linear(&input, 16000, 16000);
        assert_eq!(out, input);
    }

    #[test]
    fn resample_halves_length_when_downsampling_by_two() {
        let input: Vec<f32> = (0..1000).map(|i| i as f32).collect();
        let out = resample_linear(&input, 32000, 16000);
        assert!((out.len() as i64 - 500).abs() <= 2);
    }

    #[test]
    fn downmix_averages_channels() {
        let stereo = vec![1.0, 3.0, 2.0, 4.0]; // two frames, L/R
        let mono = downmix_to_mono(&stereo, 2);
        assert_eq!(mono, vec![2.0, 3.0]);
    }

    #[test]
    fn downmix_noop_for_mono() {
        let mono_in = vec![0.5, 0.6, 0.7];
        let out = downmix_to_mono(&mono_in, 1);
        assert_eq!(out, mono_in);
    }
}
