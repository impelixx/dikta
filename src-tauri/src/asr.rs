use crate::models::{ModelEntry, ModelKind};
use anyhow::{bail, Result};
use sherpa_rs::sherpa_rs_sys;
use std::ffi::{CStr, CString};
use std::mem;

/// Тонкая обёртка над sherpa-rs-sys — в высокоуровневом sherpa-rs нет готовых
/// модулей под nemo_ctc/nemo transducer, поэтому конфигурируем FFI-структуры
/// напрямую по образцу zipformer.rs из sherpa-rs. Поддерживает оба семейства
/// моделей GigaAM: CTC (один onnx-файл) и Transducer (encoder/decoder/joiner).
pub struct Recognizer {
    recognizer: *mut sherpa_rs_sys::SherpaOnnxOfflineRecognizer,
    _keep_alive: Vec<CString>,
}

unsafe impl Send for Recognizer {}
unsafe impl Sync for Recognizer {}

fn cstr(s: &str) -> CString {
    CString::new(s).expect("string contains NUL byte")
}

pub struct CtcPaths<'a> {
    pub model: &'a str,
    pub tokens: &'a str,
}

pub struct TransducerPaths<'a> {
    pub encoder: &'a str,
    pub decoder: &'a str,
    pub joiner: &'a str,
    pub tokens: &'a str,
}

pub struct WhisperPaths<'a> {
    pub encoder: &'a str,
    pub decoder: &'a str,
    pub tokens: &'a str,
}

impl Recognizer {
    pub fn new_ctc(paths: CtcPaths, num_threads: i32) -> Result<Self> {
        let model_c = cstr(paths.model);
        let tokens_c = cstr(paths.tokens);
        let provider_c = cstr("cpu");
        let decoding_c = cstr("greedy_search");

        let nemo_ctc_config = sherpa_rs_sys::SherpaOnnxOfflineNemoEncDecCtcModelConfig {
            model: model_c.as_ptr(),
        };

        let model_config = unsafe {
            sherpa_rs_sys::SherpaOnnxOfflineModelConfig {
                tokens: tokens_c.as_ptr(),
                num_threads,
                debug: 0,
                provider: provider_c.as_ptr(),
                nemo_ctc: nemo_ctc_config,
                transducer: mem::zeroed(),
                paraformer: mem::zeroed(),
                whisper: mem::zeroed(),
                tdnn: mem::zeroed(),
                model_type: std::ptr::null(),
                modeling_unit: std::ptr::null(),
                bpe_vocab: std::ptr::null(),
                telespeech_ctc: std::ptr::null(),
                sense_voice: mem::zeroed(),
                moonshine: mem::zeroed(),
                fire_red_asr: mem::zeroed(),
                dolphin: mem::zeroed(),
                zipformer_ctc: mem::zeroed(),
                canary: mem::zeroed(),
            }
        };

        Self::create(model_config, decoding_c, vec![model_c, tokens_c, provider_c], 64)
    }

    pub fn new_transducer(paths: TransducerPaths, num_threads: i32) -> Result<Self> {
        let encoder_c = cstr(paths.encoder);
        let decoder_c = cstr(paths.decoder);
        let joiner_c = cstr(paths.joiner);
        let tokens_c = cstr(paths.tokens);
        let provider_c = cstr("cpu");
        let decoding_c = cstr("greedy_search");

        let transducer_config = sherpa_rs_sys::SherpaOnnxOfflineTransducerModelConfig {
            encoder: encoder_c.as_ptr(),
            decoder: decoder_c.as_ptr(),
            joiner: joiner_c.as_ptr(),
        };

        let model_config = unsafe {
            sherpa_rs_sys::SherpaOnnxOfflineModelConfig {
                tokens: tokens_c.as_ptr(),
                num_threads,
                debug: 0,
                provider: provider_c.as_ptr(),
                transducer: transducer_config,
                nemo_ctc: mem::zeroed(),
                paraformer: mem::zeroed(),
                whisper: mem::zeroed(),
                tdnn: mem::zeroed(),
                model_type: std::ptr::null(),
                modeling_unit: std::ptr::null(),
                bpe_vocab: std::ptr::null(),
                telespeech_ctc: std::ptr::null(),
                sense_voice: mem::zeroed(),
                moonshine: mem::zeroed(),
                fire_red_asr: mem::zeroed(),
                dolphin: mem::zeroed(),
                zipformer_ctc: mem::zeroed(),
                canary: mem::zeroed(),
            }
        };

        Self::create(
            model_config,
            decoding_c,
            vec![encoder_c, decoder_c, joiner_c, tokens_c, provider_c],
            64,
        )
    }

    pub fn new_whisper(paths: WhisperPaths, language: &str, num_threads: i32) -> Result<Self> {
        let encoder_c = cstr(paths.encoder);
        let decoder_c = cstr(paths.decoder);
        let tokens_c = cstr(paths.tokens);
        let provider_c = cstr("cpu");
        let decoding_c = cstr("greedy_search");
        let language_c = cstr(language);
        let task_c = cstr("transcribe");

        let whisper_config = sherpa_rs_sys::SherpaOnnxOfflineWhisperModelConfig {
            encoder: encoder_c.as_ptr(),
            decoder: decoder_c.as_ptr(),
            language: language_c.as_ptr(),
            task: task_c.as_ptr(),
            tail_paddings: 0,
        };

        let model_config = unsafe {
            sherpa_rs_sys::SherpaOnnxOfflineModelConfig {
                tokens: tokens_c.as_ptr(),
                num_threads,
                debug: 0,
                provider: provider_c.as_ptr(),
                whisper: whisper_config,
                nemo_ctc: mem::zeroed(),
                transducer: mem::zeroed(),
                paraformer: mem::zeroed(),
                tdnn: mem::zeroed(),
                model_type: std::ptr::null(),
                modeling_unit: std::ptr::null(),
                bpe_vocab: std::ptr::null(),
                telespeech_ctc: std::ptr::null(),
                sense_voice: mem::zeroed(),
                moonshine: mem::zeroed(),
                fire_red_asr: mem::zeroed(),
                dolphin: mem::zeroed(),
                zipformer_ctc: mem::zeroed(),
                canary: mem::zeroed(),
            }
        };

        Self::create(
            model_config,
            decoding_c,
            vec![encoder_c, decoder_c, tokens_c, provider_c, language_c, task_c],
            80,
        )
    }

    fn create(
        model_config: sherpa_rs_sys::SherpaOnnxOfflineModelConfig,
        decoding_c: CString,
        mut keep_alive: Vec<CString>,
        feature_dim: i32,
    ) -> Result<Self> {
        let feat_config = sherpa_rs_sys::SherpaOnnxFeatureConfig {
            sample_rate: 16000,
            feature_dim,
        };

        let recognizer_config = unsafe {
            sherpa_rs_sys::SherpaOnnxOfflineRecognizerConfig {
                feat_config,
                model_config,
                decoding_method: decoding_c.as_ptr(),
                blank_penalty: 0.0,
                hotwords_file: std::ptr::null(),
                hotwords_score: 0.0,
                lm_config: mem::zeroed(),
                max_active_paths: 0,
                rule_fars: std::ptr::null(),
                rule_fsts: std::ptr::null(),
                hr: mem::zeroed(),
            }
        };

        let recognizer =
            unsafe { sherpa_rs_sys::SherpaOnnxCreateOfflineRecognizer(&recognizer_config) };

        if recognizer.is_null() {
            bail!("не удалось создать распознаватель (проверьте пути к модели/токенам)");
        }

        keep_alive.push(decoding_c);
        Ok(Self {
            recognizer: recognizer as *mut _,
            _keep_alive: keep_alive,
        })
    }

    pub fn from_model_dir(dir: &std::path::Path, kind: ModelKind, num_threads: i32) -> Result<Self> {
        let tokens = dir.join("tokens.txt");
        let tokens = tokens.to_str().expect("некорректный путь к токенам");
        match kind {
            ModelKind::Ctc => {
                let model = dir.join("model.int8.onnx");
                Self::new_ctc(
                    CtcPaths {
                        model: model.to_str().expect("некорректный путь к модели"),
                        tokens,
                    },
                    num_threads,
                )
            }
            ModelKind::Transducer => {
                let encoder = dir.join("encoder.int8.onnx");
                let decoder = dir.join("decoder.onnx");
                let joiner = dir.join("joiner.onnx");
                Self::new_transducer(
                    TransducerPaths {
                        encoder: encoder.to_str().expect("некорректный путь к encoder"),
                        decoder: decoder.to_str().expect("некорректный путь к decoder"),
                        joiner: joiner.to_str().expect("некорректный путь к joiner"),
                        tokens,
                    },
                    num_threads,
                )
            }
            // Whisper грузится через from_whisper_dir (нужен префикс имени файлов
            // и язык) — сюда попадать не должен, но не паникуем на всякий случай.
            ModelKind::Whisper => {
                bail!("для Whisper используйте Recognizer::from_whisper_dir")
            }
        }
    }

    /// Собирает распознаватель по элементу каталога — сама решает, обычный
    /// ли это путь (CTC/Transducer) или Whisper (нужны префикс файлов и язык).
    pub fn from_entry(dir: &std::path::Path, entry: &ModelEntry, num_threads: i32) -> Result<Self> {
        match entry {
            ModelEntry::Builtin(info) if info.kind == ModelKind::Whisper => {
                let prefix = info.whisper_file_prefix.unwrap_or("model");
                let language = info.whisper_language.unwrap_or("ru");
                Self::from_whisper_dir(dir, prefix, language, num_threads)
            }
            _ => Self::from_model_dir(dir, entry.kind(), num_threads),
        }
    }

    /// Whisper-архивы sherpa-onnx именуют файлы с префиксом размера модели
    /// (например "small-encoder.int8.onnx"), поэтому им нужен отдельный путь
    /// загрузки — обычный from_model_dir рассчитан на фиксированные имена.
    pub fn from_whisper_dir(
        dir: &std::path::Path,
        file_prefix: &str,
        language: &str,
        num_threads: i32,
    ) -> Result<Self> {
        let encoder = dir.join(format!("{file_prefix}-encoder.int8.onnx"));
        let decoder = dir.join(format!("{file_prefix}-decoder.int8.onnx"));
        let tokens = dir.join(format!("{file_prefix}-tokens.txt"));
        Self::new_whisper(
            WhisperPaths {
                encoder: encoder.to_str().expect("некорректный путь к encoder"),
                decoder: decoder.to_str().expect("некорректный путь к decoder"),
                tokens: tokens.to_str().expect("некорректный путь к tokens"),
            },
            language,
            num_threads,
        )
    }

    /// Распознаёт моно-сэмплы f32 в диапазоне [-1.0, 1.0] на 16kHz.
    pub fn decode(&self, samples: &[f32]) -> String {
        unsafe {
            let stream = sherpa_rs_sys::SherpaOnnxCreateOfflineStream(self.recognizer);
            sherpa_rs_sys::SherpaOnnxAcceptWaveformOffline(
                stream,
                16000,
                samples.as_ptr(),
                samples.len() as i32,
            );
            sherpa_rs_sys::SherpaOnnxDecodeOfflineStream(self.recognizer, stream);
            let result_ptr = sherpa_rs_sys::SherpaOnnxGetOfflineStreamResult(stream);
            let text = if result_ptr.is_null() {
                String::new()
            } else {
                let raw = &*result_ptr;
                if raw.text.is_null() {
                    String::new()
                } else {
                    CStr::from_ptr(raw.text).to_string_lossy().into_owned()
                }
            };
            sherpa_rs_sys::SherpaOnnxDestroyOfflineRecognizerResult(result_ptr);
            sherpa_rs_sys::SherpaOnnxDestroyOfflineStream(stream);
            text
        }
    }
}

impl Drop for Recognizer {
    fn drop(&mut self) {
        unsafe {
            sherpa_rs_sys::SherpaOnnxDestroyOfflineRecognizer(self.recognizer);
        }
    }
}

/// Грубая оценка "качества сигнала" по RMS-энергии — подставляется вместо
/// недоступной модельной confidence, чтобы график в статистике не был выдумкой.
pub fn signal_quality(samples: &[f32]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();
    let rms = (sum_sq / samples.len() as f64).sqrt();
    (rms * 10.0).clamp(0.0, 1.0)
}
