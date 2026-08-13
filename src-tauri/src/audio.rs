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

struct StreamHandle {
    #[allow(dead_code)] // держим поток живым через RAII, не читаем его напрямую
    stream: cpal::Stream,
    sample_rate: u32,
    channels: u16,
    device_name: Option<String>,
}

/// Владеет открытым входным потоком cpal. start()/stop() лишь включают/выключают
/// запись сэмплов в буфер — поток не пересоздаётся на каждую диктовку, только при
/// явной смене устройства через `switch_device`.
pub struct AudioEngine {
    handle: Mutex<StreamHandle>,
    buffer: Arc<Mutex<SharedBuffer>>,
    silence_triggered: Arc<AtomicBool>,
    vad: Arc<Mutex<Option<SilenceDetector>>>,
}

unsafe impl Send for AudioEngine {}
unsafe impl Sync for AudioEngine {}

pub fn list_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    match host.input_devices() {
        Ok(devices) => devices.filter_map(|d| d.name().ok()).collect(),
        Err(_) => Vec::new(),
    }
}

fn build_stream(
    device_name: Option<&str>,
    buffer: Arc<Mutex<SharedBuffer>>,
    silence_triggered: Arc<AtomicBool>,
    vad: Arc<Mutex<Option<SilenceDetector>>>,
) -> Result<StreamHandle> {
    let host = cpal::default_host();
    let device = match device_name {
        Some(name) => host
            .input_devices()?
            .find(|d| d.name().map(|n| n == name).unwrap_or(false))
            .with_context(|| format!("устройство ввода «{name}» не найдено"))?,
        None => host
            .default_input_device()
            .context("не найдено устройство ввода звука")?,
    };
    let resolved_name = device.name().ok();
    let config = device.default_input_config().context("нет конфигурации входа")?;
    let sample_rate = config.sample_rate().0;
    let channels = config.channels();

    let err_fn = |err| eprintln!("[audio] ошибка потока: {err}");
    let stream = device.build_input_stream(
        &config.into(),
        move |data: &[f32], _| {
            let mut buf = buffer.lock().unwrap();
            if !buf.recording {
                return;
            }
            buf.samples.extend_from_slice(data);
            if let Some(vad) = vad.lock().unwrap().as_mut() {
                if vad.feed(data) {
                    silence_triggered.store(true, Ordering::SeqCst);
                }
            }
        },
        err_fn,
        None,
    )?;
    stream.play()?;

    Ok(StreamHandle {
        stream,
        sample_rate,
        channels,
        device_name: resolved_name,
    })
}

impl AudioEngine {
    pub fn new(device_name: Option<&str>) -> Result<Self> {
        let buffer = Arc::new(Mutex::new(SharedBuffer {
            recording: false,
            samples: Vec::new(),
        }));
        let silence_triggered = Arc::new(AtomicBool::new(false));
        let vad: Arc<Mutex<Option<SilenceDetector>>> = Arc::new(Mutex::new(None));

        let handle = build_stream(device_name, buffer.clone(), silence_triggered.clone(), vad.clone())?;

        Ok(Self {
            handle: Mutex::new(handle),
            buffer,
            silence_triggered,
            vad,
        })
    }

    /// Пересоздаёт входной поток на выбранном устройстве (None = системное по умолчанию).
    pub fn switch_device(&self, device_name: Option<&str>) -> Result<()> {
        let new_handle = build_stream(
            device_name,
            self.buffer.clone(),
            self.silence_triggered.clone(),
            self.vad.clone(),
        )?;
        *self.handle.lock().unwrap() = new_handle;
        Ok(())
    }

    pub fn current_device_name(&self) -> Option<String> {
        self.handle.lock().unwrap().device_name.clone()
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
        let handle = self.handle.lock().unwrap();
        let mono = downmix_to_mono(&raw, handle.channels);
        resample_linear(&mono, handle.sample_rate, 16000)
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
