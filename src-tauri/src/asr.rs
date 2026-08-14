use crate::models::{ModelEntry, ModelKind};
use anyhow::{bail, Result};
use sherpa_rs::sherpa_rs_sys;
use std::ffi::{CStr, CString};
use std::mem;

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

/// Обёртка над sherpa-rs-sys — в высокоуровневом sherpa-rs нет готовых
/// модулей под nemo_ctc/nemo transducer, поэтому конфигурируем FFI-структуры
/// напрямую по образцу zipformer.rs из sherpa-rs. Покрывает CTC, Transducer
/// и Whisper через ONNX Runtime.
pub struct SherpaEngine {
    recognizer: *mut sherpa_rs_sys::SherpaOnnxOfflineRecognizer,
    _keep_alive: Vec<CString>,
}

unsafe impl Send for SherpaEngine {}
unsafe impl Sync for SherpaEngine {}

impl SherpaEngine {
    fn new_ctc_with_dim(paths: CtcPaths, num_threads: i32, feature_dim: i32) -> Result<Self> {
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

        Self::create(model_config, decoding_c, vec![model_c, tokens_c, provider_c], feature_dim)
    }

    fn new_transducer_with_dim(paths: TransducerPaths, num_threads: i32, feature_dim: i32) -> Result<Self> {
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
            feature_dim,
        )
    }

    fn new_whisper(paths: WhisperPaths, language: &str, num_threads: i32) -> Result<Self> {
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

    fn decode(&self, samples: &[f32]) -> String {
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

impl Drop for SherpaEngine {
    fn drop(&mut self) {
        unsafe {
            sherpa_rs_sys::SherpaOnnxDestroyOfflineRecognizer(self.recognizer);
        }
    }
}

/// whisper.cpp через whisper-rs — второй движок рядом с sherpa-onnx. Даёт
/// GPU-ускорение (Metal на macOS) для Whisper-моделей в формате GGML/GGUF,
/// как в Handy. GigaAM/Zipformer/Fast Conformer остаются на sherpa-onnx —
/// у whisper.cpp нет этих архитектур.
pub struct WhisperCppEngine {
    ctx: whisper_rs::WhisperContext,
    language: String,
    num_threads: i32,
}

unsafe impl Send for WhisperCppEngine {}
unsafe impl Sync for WhisperCppEngine {}

impl WhisperCppEngine {
    fn new(model_path: &str, language: &str, num_threads: i32) -> Result<Self> {
        let ctx = whisper_rs::WhisperContext::new_with_params(
            model_path,
            whisper_rs::WhisperContextParameters::default(),
        )
        .map_err(|e| anyhow::anyhow!("не удалось загрузить whisper.cpp модель: {e}"))?;
        Ok(Self {
            ctx,
            language: language.to_string(),
            num_threads,
        })
    }

    fn decode(&self, samples: &[f32]) -> String {
        let mut state = match self.ctx.create_state() {
            Ok(s) => s,
            Err(_) => return String::new(),
        };
        let mut params = whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some(&self.language));
        params.set_n_threads(self.num_threads);
        params.set_print_progress(false);
        params.set_print_special(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_translate(false);

        if state.full(params, samples).is_err() {
            return String::new();
        }

        let mut text = String::new();
        for segment in state.as_iter() {
            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(&segment.to_string());
        }
        text.trim().to_string()
    }
}

/// Распознаватель речи — либо sherpa-onnx (CTC/Transducer/Whisper через ONNX),
/// либо whisper.cpp (Whisper через GGML, с GPU-ускорением). Выбор зависит от
/// модели в каталоге, вызывающему код это не важно — везде один `decode()`.
pub enum Recognizer {
    Sherpa(SherpaEngine),
    WhisperCpp(WhisperCppEngine),
}

impl Recognizer {
    pub fn new_ctc_with_dim(paths: CtcPaths, num_threads: i32, feature_dim: i32) -> Result<Self> {
        Ok(Self::Sherpa(SherpaEngine::new_ctc_with_dim(paths, num_threads, feature_dim)?))
    }

    pub fn new_transducer_with_dim(paths: TransducerPaths, num_threads: i32, feature_dim: i32) -> Result<Self> {
        Ok(Self::Sherpa(SherpaEngine::new_transducer_with_dim(paths, num_threads, feature_dim)?))
    }

    pub fn new_whisper(paths: WhisperPaths, language: &str, num_threads: i32) -> Result<Self> {
        Ok(Self::Sherpa(SherpaEngine::new_whisper(paths, language, num_threads)?))
    }

    pub fn new_whisper_cpp(model_path: &str, language: &str, num_threads: i32) -> Result<Self> {
        Ok(Self::WhisperCpp(WhisperCppEngine::new(model_path, language, num_threads)?))
    }

    pub fn from_model_dir(dir: &std::path::Path, kind: ModelKind, num_threads: i32) -> Result<Self> {
        Self::from_model_dir_with_dim(dir, kind, num_threads, 64)
    }

    /// feature_dim (число mel-фильтров) зависит от конкретной модели, а не
    /// только от её "формы" (CTC/Transducer) — GigaAM обучен на 64, а
    /// большинство остальных NeMo/Conformer-моделей ожидают 80. Несовпадение
    /// не возвращает ошибку — роняет процесс целиком (необрабатываемое
    /// исключение из sherpa-onnx C API), поэтому для каждой модели каталога
    /// значение проверено вручную перед добавлением, а не угадано.
    pub fn from_model_dir_with_dim(
        dir: &std::path::Path,
        kind: ModelKind,
        num_threads: i32,
        feature_dim: i32,
    ) -> Result<Self> {
        let tokens = dir.join("tokens.txt");
        let tokens = tokens.to_str().expect("некорректный путь к токенам");
        match kind {
            ModelKind::Ctc => {
                let model = dir.join("model.int8.onnx");
                Self::new_ctc_with_dim(
                    CtcPaths {
                        model: model.to_str().expect("некорректный путь к модели"),
                        tokens,
                    },
                    num_threads,
                    feature_dim,
                )
            }
            ModelKind::Transducer => {
                let encoder = dir.join("encoder.int8.onnx");
                let decoder = dir.join("decoder.onnx");
                let joiner = dir.join("joiner.onnx");
                Self::new_transducer_with_dim(
                    TransducerPaths {
                        encoder: encoder.to_str().expect("некорректный путь к encoder"),
                        decoder: decoder.to_str().expect("некорректный путь к decoder"),
                        joiner: joiner.to_str().expect("некорректный путь к joiner"),
                        tokens,
                    },
                    num_threads,
                    feature_dim,
                )
            }
            // Whisper (ONNX) грузится через from_whisper_dir, whisper.cpp —
            // через from_entry напрямую. Сюда попадать не должен.
            ModelKind::Whisper | ModelKind::WhisperCpp => {
                bail!("для Whisper используйте Recognizer::from_entry")
            }
        }
    }

    /// Собирает распознаватель по элементу каталога — сама решает движок
    /// (sherpa-onnx или whisper.cpp) и специфику загрузки файлов.
    pub fn from_entry(dir: &std::path::Path, entry: &ModelEntry, num_threads: i32) -> Result<Self> {
        match entry {
            ModelEntry::Builtin(info) if info.kind == ModelKind::WhisperCpp => {
                let model = dir.join("model.bin");
                let language = info.whisper_language.unwrap_or("ru");
                Self::new_whisper_cpp(
                    model.to_str().expect("некорректный путь к модели"),
                    language,
                    num_threads,
                )
            }
            ModelEntry::Builtin(info) if info.kind == ModelKind::Whisper => {
                let prefix = info.whisper_file_prefix.unwrap_or("model");
                let language = info.whisper_language.unwrap_or("ru");
                Self::from_whisper_dir(dir, prefix, language, num_threads)
            }
            ModelEntry::Builtin(info) => {
                Self::from_model_dir_with_dim(dir, info.kind, num_threads, info.feature_dim)
            }
            ModelEntry::Custom(_) => Self::from_model_dir(dir, entry.kind(), num_threads),
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
        match self {
            Self::Sherpa(engine) => engine.decode(samples),
            Self::WhisperCpp(engine) => engine.decode(samples),
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
