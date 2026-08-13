//! Реальный замер скорости каждой модели каталога: cargo run --example benchmark_models
//! Декодирует собственный test_wavs/example.wav каждой модели и считает
//! realtime factor (время декодирования / длительность аудио). Результат
//! специфичен для этой машины, но честнее статичной оценки "быстро/медленно".
use dikta_lib::asr::Recognizer;
use dikta_lib::models::{self, ModelEntry};
use hound::WavReader;
use std::time::Instant;

fn read_wav(path: &std::path::Path) -> (Vec<f32>, f64) {
    let mut reader = WavReader::open(path).expect("wav open failed");
    let spec = reader.spec();
    assert_eq!(spec.sample_rate, 16000, "ожидается 16kHz");
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / i16::MAX as f32)
        .collect();
    let duration_sec = samples.len() as f64 / 16000.0;
    (samples, duration_sec)
}

fn main() {
    let mut base = dirs::data_dir().expect("no data dir");
    base.push("dikta");
    base.push("models");

    for info in models::builtin_catalog() {
        let entry = ModelEntry::Builtin(info.clone());
        if !models::is_downloaded(&base, &entry) {
            println!("=== {} === пропущено (не скачано)\n", info.name);
            continue;
        }
        let dir = models::model_root_dir(&base, &entry);
        let wav_path = dir.join("test_wavs").join("example.wav");
        if !wav_path.exists() {
            println!("=== {} === пропущено (нет test_wavs/example.wav)\n", info.name);
            continue;
        }

        let load_start = Instant::now();
        let recognizer = Recognizer::from_model_dir(&dir, info.kind, 2).expect("recognizer init failed");
        let load_ms = load_start.elapsed().as_millis();

        let (samples, duration_sec) = read_wav(&wav_path);

        // Прогрев + 3 замера, берём медиану — первый прогон почти всегда медленнее.
        let mut timings = Vec::new();
        for _ in 0..4 {
            let t0 = Instant::now();
            let text = recognizer.decode(&samples);
            let elapsed = t0.elapsed().as_secs_f64();
            timings.push((elapsed, text));
        }
        timings.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let (median_sec, text) = &timings[timings.len() / 2];
        let rtf = median_sec / duration_sec;

        println!("=== {} ===", info.name);
        println!("загрузка модели: {load_ms}мс");
        println!("аудио: {duration_sec:.2}с, декод (медиана из 4): {median_sec:.3}с, RTF={rtf:.4}");
        println!("текст: {text}\n");
    }
}
