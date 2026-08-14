use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelKind {
    Ctc,
    Transducer,
    /// Whisper через sherpa-onnx (ONNX Runtime).
    Whisper,
    /// Whisper через whisper.cpp (GGML/GGUF, с GPU-ускорением — как в Handy).
    WhisperCpp,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub kind: ModelKind,
    pub archive_stem: &'static str,
    pub size_mb: u32,
    pub languages: &'static [&'static str],
    pub punctuation: bool,
    /// Реальный замер на M-серии Mac (cargo run --example benchmark_models),
    /// не универсальная гарантия — на другом железе будет отличаться.
    /// RTF = время декода / длительность аудио, меньше = быстрее.
    pub measured_rtf: f32,
    pub measured_load_ms: u32,
    /// Только для Whisper: файлы в архиве именуются с префиксом размера
    /// модели (например "small-encoder.int8.onnx"), и язык распознавания
    /// нужно указать явно — Whisper многоязычный, а не заточен под русский.
    pub whisper_file_prefix: Option<&'static str>,
    pub whisper_language: Option<&'static str>,
    /// Число mel-фильтров, которое ждёт конкретная модель (для CTC/Transducer;
    /// Whisper фиксирован на 80 внутри asr.rs). GigaAM обучен на 64 — почти
    /// все остальные NeMo/Conformer-модели на 80. Несовпадение не даёт
    /// нормальную ошибку, а роняет процесс целиком, так что значение для
    /// каждой модели проверено вручную (cargo run --example ... с реальным
    /// аудио), а не угадано по умолчанию.
    pub feature_dim: i32,
    /// Только для WhisperCpp: прямая ссылка на .bin файл модели (GGML),
    /// не архив — скачивается одним файлом, без tar/bzip2.
    pub ggml_url: Option<&'static str>,
}

/// Модель, добавленная пользователем по произвольному репозиторию Hugging Face.
/// Файлы могут называться как угодно на HF — при скачивании мы сохраняем их
/// локально под стандартными именами (как у asr::Recognizer::from_model_dir),
/// чтобы не плодить отдельный путь загрузки под каждое соглашение об именах.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomModel {
    pub id: String,
    pub name: String,
    pub repo_id: String,
    pub kind: ModelKind,
    pub files: CustomFiles,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum CustomFiles {
    Ctc { model: String, tokens: String },
    Transducer {
        encoder: String,
        decoder: String,
        joiner: String,
        tokens: String,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "source")]
pub enum ModelEntry {
    Builtin(ModelInfo),
    Custom(CustomModel),
}

impl ModelEntry {
    pub fn id(&self) -> &str {
        match self {
            ModelEntry::Builtin(i) => i.id,
            ModelEntry::Custom(c) => &c.id,
        }
    }
    pub fn name(&self) -> &str {
        match self {
            ModelEntry::Builtin(i) => i.name,
            ModelEntry::Custom(c) => &c.name,
        }
    }
    pub fn kind(&self) -> ModelKind {
        match self {
            ModelEntry::Builtin(i) => i.kind,
            ModelEntry::Custom(c) => c.kind,
        }
    }
}

const RELEASE_BASE: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models";

pub fn builtin_catalog() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "giga-ctc-v3",
            name: "GigaAM-CTC v3",
            description: "Без знаков препинания. Быстрее всего переключается (лёгкая загрузка).",
            kind: ModelKind::Ctc,
            archive_stem: "sherpa-onnx-nemo-ctc-giga-am-v3-russian-2025-12-16",
            size_mb: 260,
            languages: &["ru"],
            punctuation: false,
            measured_rtf: 0.085,
            measured_load_ms: 920,
            whisper_file_prefix: None,
            whisper_language: None,
            feature_dim: 64,
            ggml_url: None,
        },
        ModelInfo {
            id: "giga-ctc-punct-v3",
            name: "GigaAM-CTC v3 + пунктуация",
            description: "Сама расставляет точки и запятые. Хороший выбор по умолчанию.",
            kind: ModelKind::Ctc,
            archive_stem: "sherpa-onnx-nemo-ctc-punct-giga-am-v3-russian-2025-12-16",
            size_mb: 260,
            languages: &["ru"],
            punctuation: true,
            measured_rtf: 0.084,
            measured_load_ms: 935,
            whisper_file_prefix: None,
            whisper_language: None,
            feature_dim: 64,
            ggml_url: None,
        },
        ModelInfo {
            id: "giga-transducer-v3",
            name: "GigaAM-Transducer v3",
            description: "Без пунктуации. Дольше загружается при переключении (3 onnx-сессии).",
            kind: ModelKind::Transducer,
            archive_stem: "sherpa-onnx-nemo-transducer-giga-am-v3-russian-2025-12-16",
            size_mb: 280,
            languages: &["ru"],
            punctuation: false,
            measured_rtf: 0.086,
            measured_load_ms: 2300,
            whisper_file_prefix: None,
            whisper_language: None,
            feature_dim: 64,
            ggml_url: None,
        },
        ModelInfo {
            id: "giga-transducer-punct-v3",
            name: "GigaAM-Transducer v3 + пунктуация",
            description: "С пунктуацией. Дольше загружается при переключении (3 onnx-сессии).",
            kind: ModelKind::Transducer,
            archive_stem: "sherpa-onnx-nemo-transducer-punct-giga-am-v3-russian-2025-12-16",
            size_mb: 280,
            languages: &["ru"],
            punctuation: true,
            measured_rtf: 0.082,
            measured_load_ms: 1990,
            whisper_file_prefix: None,
            whisper_language: None,
            feature_dim: 64,
            ggml_url: None,
        },
        ModelInfo {
            id: "whisper-cpp-base",
            name: "Whisper base (whisper.cpp)",
            description: "Многоязычная модель OpenAI через whisper.cpp — GPU-ускорение (Metal на macOS), как в Handy. На русском чаще ошибается в деталях, чем GigaAM, но одна модель понимает ~99 языков.",
            kind: ModelKind::WhisperCpp,
            archive_stem: "whisper-cpp-base",
            size_mb: 148,
            languages: &["ru", "en", "+97 других"],
            punctuation: true,
            measured_rtf: 0.0157,
            measured_load_ms: 5941,
            whisper_file_prefix: None,
            whisper_language: Some("ru"),
            feature_dim: 0,
            ggml_url: Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin"),
        },
        ModelInfo {
            id: "whisper-cpp-small",
            name: "Whisper small (whisper.cpp)",
            description: "Та же модель через whisper.cpp, крупнее и точнее base на русском — с GPU-ускорением ощутимо быстрее, чем ONNX-вариант такого же размера.",
            kind: ModelKind::WhisperCpp,
            archive_stem: "whisper-cpp-small",
            size_mb: 488,
            languages: &["ru", "en", "+97 других"],
            punctuation: true,
            measured_rtf: 0.0428,
            measured_load_ms: 166,
            whisper_file_prefix: None,
            whisper_language: Some("ru"),
            feature_dim: 0,
            ggml_url: Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin"),
        },
        ModelInfo {
            id: "whisper-cpp-tiny",
            name: "Whisper tiny (whisper.cpp)",
            description: "Самая маленькая и быстрая модель Whisper — для слабых машин или когда важнее скорость, чем точность. Заметно чаще ошибается на русском, чем base/small.",
            kind: ModelKind::WhisperCpp,
            archive_stem: "whisper-cpp-tiny",
            size_mb: 74,
            languages: &["ru", "en", "+97 других"],
            punctuation: true,
            measured_rtf: 0.0099,
            measured_load_ms: 88,
            whisper_file_prefix: None,
            whisper_language: Some("ru"),
            feature_dim: 0,
            ggml_url: Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin"),
        },
        ModelInfo {
            id: "whisper-cpp-medium",
            name: "Whisper medium (whisper.cpp)",
            description: "Крупнее small, заметно точнее на сложной речи и акцентах — но и тяжелее по памяти/загрузке. Разумный выбор, когда точность важнее скорости переключения.",
            kind: ModelKind::WhisperCpp,
            archive_stem: "whisper-cpp-medium",
            size_mb: 1533,
            languages: &["ru", "en", "+97 других"],
            punctuation: true,
            measured_rtf: 0.0889,
            measured_load_ms: 667,
            whisper_file_prefix: None,
            whisper_language: Some("ru"),
            feature_dim: 0,
            ggml_url: Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin"),
        },
        ModelInfo {
            id: "nemo-ctc-en-conformer-medium",
            name: "NeMo Conformer-CTC medium (English)",
            description: "Официальная модель NVIDIA NeMo (stt_en_conformer_ctc_medium) для английского — без пунктуации, но точная и стабильная на чистой речи.",
            kind: ModelKind::Ctc,
            archive_stem: "sherpa-onnx-nemo-ctc-en-conformer-medium",
            size_mb: 158,
            languages: &["en"],
            punctuation: false,
            measured_rtf: 0.0268,
            measured_load_ms: 628,
            whisper_file_prefix: None,
            whisper_language: None,
            feature_dim: 80,
            ggml_url: None,
        },
        ModelInfo {
            id: "fast-conformer-multilingual",
            name: "Fast Conformer (10 языков Европы)",
            description: "Одна модель на белорусский, немецкий, английский, испанский, французский, хорватский, итальянский, польский, русский и украинский — быстрее GigaAM.",
            kind: ModelKind::Ctc,
            archive_stem: "sherpa-onnx-nemo-fast-conformer-ctc-be-de-en-es-fr-hr-it-pl-ru-uk-20k-int8",
            size_mb: 130,
            languages: &["be", "de", "en", "es", "fr", "hr", "it", "pl", "ru", "uk"],
            punctuation: false,
            measured_rtf: 0.027,
            measured_load_ms: 700,
            whisper_file_prefix: None,
            whisper_language: None,
            feature_dim: 80,
            ggml_url: None,
        },
        ModelInfo {
            id: "zipformer-ru",
            name: "Zipformer RU (Vosk)",
            description: "Ещё одна модель только на русском (архитектура Zipformer, экспорт проекта Vosk/alphacep) — не от Sber, самая быстрая в каталоге.",
            kind: ModelKind::Transducer,
            archive_stem: "sherpa-onnx-zipformer-ru-int8-2025-04-20",
            size_mb: 84,
            languages: &["ru"],
            punctuation: false,
            measured_rtf: 0.013,
            measured_load_ms: 1725,
            whisper_file_prefix: None,
            whisper_language: None,
            feature_dim: 80,
            ggml_url: None,
        },
    ]
}

pub fn full_catalog(custom: &[CustomModel]) -> Vec<ModelEntry> {
    let mut v: Vec<ModelEntry> = builtin_catalog().into_iter().map(ModelEntry::Builtin).collect();
    v.extend(custom.iter().cloned().map(ModelEntry::Custom));
    v
}

pub fn find(id: &str, custom: &[CustomModel]) -> Option<ModelEntry> {
    full_catalog(custom).into_iter().find(|m| m.id() == id)
}

fn download_url(info: &ModelInfo) -> String {
    format!("{RELEASE_BASE}/{}.tar.bz2", info.archive_stem)
}

pub fn model_root_dir(base: &Path, entry: &ModelEntry) -> PathBuf {
    match entry {
        ModelEntry::Builtin(info) => base.join(info.archive_stem),
        ModelEntry::Custom(c) => base.join(&c.id),
    }
}

pub fn is_downloaded(base: &Path, entry: &ModelEntry) -> bool {
    let dir = model_root_dir(base, entry);
    match entry.kind() {
        ModelKind::Ctc => dir.join("model.int8.onnx").exists() && dir.join("tokens.txt").exists(),
        ModelKind::Transducer => {
            dir.join("encoder.int8.onnx").exists()
                && dir.join("decoder.onnx").exists()
                && dir.join("joiner.onnx").exists()
                && dir.join("tokens.txt").exists()
        }
        ModelKind::Whisper => {
            let prefix = match entry {
                ModelEntry::Builtin(info) => info.whisper_file_prefix.unwrap_or("model"),
                ModelEntry::Custom(_) => "model",
            };
            dir.join(format!("{prefix}-encoder.int8.onnx")).exists()
                && dir.join(format!("{prefix}-decoder.int8.onnx")).exists()
                && dir.join(format!("{prefix}-tokens.txt")).exists()
        }
        ModelKind::WhisperCpp => dir.join("model.bin").exists(),
    }
}

/// Скачивает и распаковывает модель в `base`, вызывая `on_progress(downloaded_bytes, total_bytes)`
/// по мере получения данных. Блокирующая функция — вызывающий должен запустить её
/// в отдельном потоке, если не хочет блокировать текущий.
pub fn download(entry: &ModelEntry, base: &Path, mut on_progress: impl FnMut(u64, u64)) -> Result<()> {
    match entry {
        ModelEntry::Builtin(info) if info.kind == ModelKind::WhisperCpp => {
            download_ggml(info, base, &mut on_progress)
        }
        ModelEntry::Builtin(info) => download_builtin(info, base, on_progress),
        ModelEntry::Custom(c) => download_custom(c, base, &mut on_progress),
    }
}

/// Модели whisper.cpp — один .bin файл (GGML), без архива.
fn download_ggml(info: &ModelInfo, base: &Path, on_progress: &mut impl FnMut(u64, u64)) -> Result<()> {
    let url = info.ggml_url.context("у модели не задан ggml_url")?;
    let dir = base.join(info.archive_stem);
    std::fs::create_dir_all(&dir)?;
    let mut downloaded = 0u64;
    download_one_file(url, &dir.join("model.bin"), &mut downloaded, on_progress)?;

    let entry = ModelEntry::Builtin(info.clone());
    if !is_downloaded(base, &entry) {
        anyhow::bail!("после скачивания не найден файл модели");
    }
    Ok(())
}

fn download_builtin(info: &ModelInfo, base: &Path, mut on_progress: impl FnMut(u64, u64)) -> Result<()> {
    std::fs::create_dir_all(base)?;
    let url = download_url(info);
    let resp = ureq::get(&url).call().context("не удалось начать скачивание")?;
    let total: u64 = resp
        .header("Content-Length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let tmp_path = base.join(format!("{}.tar.bz2.part", info.id));
    let mut file = std::fs::File::create(&tmp_path)?;
    let mut reader = resp.into_reader();
    let mut buf = [0u8; 64 * 1024];
    let mut downloaded: u64 = 0;
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        downloaded += n as u64;
        on_progress(downloaded, total);
    }
    drop(file);

    let tar_bz2 = std::fs::File::open(&tmp_path)?;
    let decompressed = bzip2::read::BzDecoder::new(tar_bz2);
    let mut archive = tar::Archive::new(decompressed);
    archive.unpack(base).context("не удалось распаковать архив модели")?;
    std::fs::remove_file(&tmp_path).ok();

    let entry = ModelEntry::Builtin(info.clone());
    let dir = model_root_dir(base, &entry);

    // Архивы Whisper содержат и fp32, и int8 версии encoder/decoder, а мы
    // всегда используем только int8 — удаляем неиспользуемые fp32-файлы
    // (для whisper-small это лишний ~1ГБ на диске просто так).
    if info.kind == ModelKind::Whisper {
        if let Some(prefix) = info.whisper_file_prefix {
            std::fs::remove_file(dir.join(format!("{prefix}-encoder.onnx"))).ok();
            std::fs::remove_file(dir.join(format!("{prefix}-decoder.onnx"))).ok();
        }
    }

    // Некоторые Transducer-архивы (например zipformer-ru) называют квантованный
    // joiner "joiner.int8.onnx" вместо "joiner.onnx" — приводим к ожидаемому
    // имени, не храня лишнюю fp32-копию.
    if info.kind == ModelKind::Transducer {
        let joiner = dir.join("joiner.onnx");
        let joiner_int8 = dir.join("joiner.int8.onnx");
        if !joiner.exists() && joiner_int8.exists() {
            std::fs::rename(&joiner_int8, &joiner).ok();
        }
    }

    if !is_downloaded(base, &entry) {
        anyhow::bail!("после распаковки не найдены ожидаемые файлы модели");
    }

    Ok(())
}

fn download_one_file(url: &str, dest: &Path, downloaded_so_far: &mut u64, on_progress: &mut impl FnMut(u64, u64)) -> Result<()> {
    let resp = ureq::get(url).call().with_context(|| format!("не удалось скачать {url}"))?;
    let total: u64 = resp.header("Content-Length").and_then(|v| v.parse().ok()).unwrap_or(0);
    let mut file = std::fs::File::create(dest)?;
    let mut reader = resp.into_reader();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        *downloaded_so_far += n as u64;
        on_progress(*downloaded_so_far, total);
    }
    Ok(())
}

fn download_custom(c: &CustomModel, base: &Path, on_progress: &mut impl FnMut(u64, u64)) -> Result<()> {
    let dir = base.join(&c.id);
    std::fs::create_dir_all(&dir)?;
    let mut downloaded = 0u64;

    let resolve = |filename: &str| format!("https://huggingface.co/{}/resolve/main/{}", c.repo_id, filename);

    match &c.files {
        CustomFiles::Ctc { model, tokens } => {
            download_one_file(&resolve(model), &dir.join("model.int8.onnx"), &mut downloaded, on_progress)?;
            download_one_file(&resolve(tokens), &dir.join("tokens.txt"), &mut downloaded, on_progress)?;
        }
        CustomFiles::Transducer { encoder, decoder, joiner, tokens } => {
            download_one_file(&resolve(encoder), &dir.join("encoder.int8.onnx"), &mut downloaded, on_progress)?;
            download_one_file(&resolve(decoder), &dir.join("decoder.onnx"), &mut downloaded, on_progress)?;
            download_one_file(&resolve(joiner), &dir.join("joiner.onnx"), &mut downloaded, on_progress)?;
            download_one_file(&resolve(tokens), &dir.join("tokens.txt"), &mut downloaded, on_progress)?;
        }
    }

    let entry = ModelEntry::Custom(c.clone());
    if !is_downloaded(base, &entry) {
        anyhow::bail!("после скачивания не найдены ожидаемые файлы модели");
    }
    Ok(())
}

/// Список .onnx/.txt файлов в указанном репозитории Hugging Face — для ручного
/// сопоставления файлов ролям (токены/модель/энкодер/декодер/joiner) в UI.
/// Имена в разных репозиториях не унифицированы, поэтому точный подбор остаётся
/// за пользователем — мы только отфильтровываем нерелевантные файлы.
pub fn hf_list_files(repo_id: &str) -> Result<Vec<String>> {
    let url = format!("https://huggingface.co/api/models/{repo_id}");
    let resp = ureq::get(&url)
        .call()
        .with_context(|| format!("не удалось получить список файлов репозитория {repo_id}"))?;
    let json: serde_json::Value = resp.into_json()?;
    let siblings = json
        .get("siblings")
        .and_then(|s| s.as_array())
        .cloned()
        .unwrap_or_default();
    let files: Vec<String> = siblings
        .into_iter()
        .filter_map(|s| s.get("rfilename").and_then(|f| f.as_str()).map(String::from))
        .filter(|f| f.ends_with(".onnx") || f.ends_with(".txt"))
        .collect();
    if files.is_empty() {
        anyhow::bail!("в репозитории не нашлось .onnx/.txt файлов — это точно sherpa-onnx модель?");
    }
    Ok(files)
}
