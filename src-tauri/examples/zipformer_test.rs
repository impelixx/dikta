use dikta_lib::asr::{Recognizer, TransducerPaths};
use hound::WavReader;
use std::env;
use std::time::Instant;

fn main() {
    let base = dirs::data_dir().unwrap().join("dikta/models/sherpa-onnx-zipformer-ru-int8-2025-04-20");
    let encoder = base.join("encoder.int8.onnx");
    let decoder = base.join("decoder.onnx");
    let joiner = base.join("joiner.onnx");
    let tokens = base.join("tokens.txt");
    let dim: i32 = env::var("FEAT_DIM").ok().and_then(|s| s.parse().ok()).unwrap_or(64);
    println!("feature_dim = {dim}");

    let load_start = Instant::now();
    let rec = Recognizer::new_transducer_with_dim(
        TransducerPaths {
            encoder: encoder.to_str().unwrap(),
            decoder: decoder.to_str().unwrap(),
            joiner: joiner.to_str().unwrap(),
            tokens: tokens.to_str().unwrap(),
        },
        2,
        dim,
    ).expect("init failed");
    println!("load: {}ms", load_start.elapsed().as_millis());

    for wav in ["0.wav", "1.wav"] {
        let path = base.join("test_wavs").join(wav);
        if !path.exists() { continue; }
        let mut reader = WavReader::open(&path).unwrap();
        let samples: Vec<f32> = reader.samples::<i16>().map(|s| s.unwrap() as f32 / i16::MAX as f32).collect();
        let duration = samples.len() as f64 / 16000.0;
        let t0 = Instant::now();
        let text = rec.decode(&samples);
        let dt = t0.elapsed().as_secs_f64();
        println!("=== {wav} ({duration:.2}s, RTF={:.4}) ===\n{text}\n", dt / duration);
    }
}
