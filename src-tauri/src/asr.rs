use anyhow::{bail, Result};
use sherpa_rs::sherpa_rs_sys;
use std::ffi::{CStr, CString};
use std::mem;

/// Тонкая обёртка над sherpa-rs-sys для NeMo CTC (GigaAM) — в высокоуровневом
/// sherpa-rs нет готового модуля под nemo_ctc, поэтому конфигурируем FFI-структуры
/// напрямую по образцу zipformer.rs из sherpa-rs.
pub struct GigaAmRecognizer {
    recognizer: *mut sherpa_rs_sys::SherpaOnnxOfflineRecognizer,
    _model_cstr: CString,
    _tokens_cstr: CString,
    _provider_cstr: CString,
    _decoding_cstr: CString,
}

unsafe impl Send for GigaAmRecognizer {}
unsafe impl Sync for GigaAmRecognizer {}

fn cstr(s: &str) -> CString {
    CString::new(s).expect("string contains NUL byte")
}

impl GigaAmRecognizer {
    pub fn new(model_path: &str, tokens_path: &str, num_threads: i32) -> Result<Self> {
        let model_cstr = cstr(model_path);
        let tokens_cstr = cstr(tokens_path);
        let provider_cstr = cstr("cpu");
        let decoding_cstr = cstr("greedy_search");

        let nemo_ctc_config = sherpa_rs_sys::SherpaOnnxOfflineNemoEncDecCtcModelConfig {
            model: model_cstr.as_ptr(),
        };

        let model_config = unsafe {
            sherpa_rs_sys::SherpaOnnxOfflineModelConfig {
                tokens: tokens_cstr.as_ptr(),
                num_threads,
                debug: 0,
                provider: provider_cstr.as_ptr(),
                nemo_ctc: nemo_ctc_config,
                // Прочие варианты моделей нам не нужны — зануляем.
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

        let feat_config = sherpa_rs_sys::SherpaOnnxFeatureConfig {
            sample_rate: 16000,
            feature_dim: 64,
        };

        let recognizer_config = unsafe {
            sherpa_rs_sys::SherpaOnnxOfflineRecognizerConfig {
                feat_config,
                model_config,
                decoding_method: decoding_cstr.as_ptr(),
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
            bail!("не удалось создать распознаватель GigaAM (проверьте пути к модели/токенам)");
        }

        Ok(Self {
            recognizer: recognizer as *mut _,
            _model_cstr: model_cstr,
            _tokens_cstr: tokens_cstr,
            _provider_cstr: provider_cstr,
            _decoding_cstr: decoding_cstr,
        })
    }

    /// Распознаёт моно-сэмплы f32 в диапазоне [-1.0, 1.0] на 16kHz.
    /// Возвращает текст и грубую оценку качества сигнала (не confidence модели —
    /// sherpa-onnx C API не отдаёт per-token вероятности для greedy CTC-декода).
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

impl Drop for GigaAmRecognizer {
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
