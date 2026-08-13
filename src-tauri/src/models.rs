use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelKind {
    Ctc,
    Transducer,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub kind: ModelKind,
    pub archive_stem: &'static str,
    pub size_mb: u32,
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
            description: "Быстрее всего, без знаков препинания. Хороший выбор по умолчанию.",
            kind: ModelKind::Ctc,
            archive_stem: "sherpa-onnx-nemo-ctc-giga-am-v3-russian-2025-12-16",
            size_mb: 260,
        },
        ModelInfo {
            id: "giga-ctc-punct-v3",
            name: "GigaAM-CTC v3 + пунктуация",
            description: "Так же быстро, но сама расставляет точки и запятые.",
            kind: ModelKind::Ctc,
            archive_stem: "sherpa-onnx-nemo-ctc-punct-giga-am-v3-russian-2025-12-16",
            size_mb: 260,
        },
        ModelInfo {
            id: "giga-transducer-v3",
            name: "GigaAM-Transducer v3",
            description: "Точнее на сложной речи, но медленнее и без пунктуации.",
            kind: ModelKind::Transducer,
            archive_stem: "sherpa-onnx-nemo-transducer-giga-am-v3-russian-2025-12-16",
            size_mb: 280,
        },
        ModelInfo {
            id: "giga-transducer-punct-v3",
            name: "GigaAM-Transducer v3 + пунктуация",
            description: "Самый точный вариант с пунктуацией — и самый медленный.",
            kind: ModelKind::Transducer,
            archive_stem: "sherpa-onnx-nemo-transducer-punct-giga-am-v3-russian-2025-12-16",
            size_mb: 280,
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
    }
}

/// Скачивает и распаковывает модель в `base`, вызывая `on_progress(downloaded_bytes, total_bytes)`
/// по мере получения данных. Блокирующая функция — вызывающий должен запустить её
/// в отдельном потоке, если не хочет блокировать текущий.
pub fn download(entry: &ModelEntry, base: &Path, mut on_progress: impl FnMut(u64, u64)) -> Result<()> {
    match entry {
        ModelEntry::Builtin(info) => download_builtin(info, base, on_progress),
        ModelEntry::Custom(c) => download_custom(c, base, &mut on_progress),
    }
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
    if !is_downloaded(base, &entry) {
        anyhow::bail!("после распаковки не найдены ожидаемые файлы модели");
    }
    Ok(())
}

fn download_one_file(url: &str, dest: &Path, downloaded_so_far: &mut u64, on_progress: &mut impl FnMut(u64, u64)) -> Result<()> {
    let resp = ureq::get(url).call().with_context(|| format!("не удалось скачать {url}"))?;
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
        on_progress(*downloaded_so_far, 0);
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
